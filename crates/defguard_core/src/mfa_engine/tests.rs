use std::{
    net::{IpAddr, Ipv4Addr},
    time::SystemTime,
};

use chrono::{TimeDelta, Utc};
use defguard_common::{
    db::{
        Id,
        models::{
            Device, DeviceType, User, WireguardNetwork,
            biometric_auth::BiometricAuth,
            device::WireguardNetworkDevice,
            mfa_flow::{LocationMfaFlowAssignment, MfaFlow},
            settings::initialize_current_settings,
            user::{TOTP_CODE_DIGITS, TOTP_CODE_VALIDITY_PERIOD},
            vpn_client_mfa_session::{
                EphemeralState, MFA_FAILED_ATTEMPT_CAP, MfaSessionContext, VPN_MFA_SESSION_TIMEOUT,
                VpnClientMfaSession, hash_token,
            },
            vpn_client_session::{VpnClientMfaMethod, VpnClientSession},
            wireguard::ServiceLocationMode,
        },
        setup_pool,
    },
    testing::smtp::configure_working_smtp,
};
use defguard_proto::client_types::MfaMethod;
use ipnetwork::IpNetwork;
use sqlx::{
    PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
};
use tokio::sync::{broadcast, mpsc};
use tonic::{Code, Status};
use totp_lite::{Sha1, totp_custom};

use super::MfaEngine;
use crate::{
    enterprise::{
        db::models::openid_provider::{
            DirectorySyncTarget, DirectorySyncUserBehavior, OpenIdProvider, OpenIdProviderKind,
        },
        license::{License, LicenseTier, SupportType, set_cached_license},
        limits::{Counts, set_counts},
    },
    events::{BidiStreamEvent, BidiStreamEventType, DesktopClientMfaEvent},
    grpc::{GatewayCommand, proto::enterprise::license::LicenseLimits},
    mfa_engine::{
        error::{FinishError, StepError},
        method::{Verdict, verify},
        types::{FinishOutcome, Proof, StartRejectionReason, StartResult},
    },
};

fn set_test_license_business() {
    let license = License::new(
        "test".to_owned(),
        true,
        Some(Utc::now() + TimeDelta::days(1)),
        Some(LicenseLimits {
            users: 100,
            devices: 100,
            locations: 100,
            network_devices: Some(100),
        }),
        None,
        LicenseTier::Business,
        SupportType::Basic,
        Vec::new(),
    );
    set_cached_license(Some(license));
    set_counts(Counts::new(1, 1, 1, 1));
}

fn clear_test_license() {
    set_cached_license(None);
}

fn make_engine(
    pool: PgPool,
) -> (
    MfaEngine,
    mpsc::UnboundedReceiver<BidiStreamEvent>,
    broadcast::Receiver<GatewayCommand>,
) {
    let (gateway_tx, gateway_rx) = broadcast::channel(8);
    let (bidi_event_tx, bidi_event_rx) = mpsc::unbounded_channel();
    (
        MfaEngine::new(pool, gateway_tx, bidi_event_tx),
        bidi_event_rx,
        gateway_rx,
    )
}

async fn create_user(pool: &PgPool) -> User<Id> {
    User::new(
        "mfa-engine-test-user".to_owned(),
        Some("pass123"),
        "Tester".to_owned(),
        "MfaEngine".to_owned(),
        "mfa-engine-test@example.com".to_owned(),
        None,
    )
    .save(pool)
    .await
    .expect("failed to create user")
}

async fn create_device(pool: &PgPool, user_id: Id) -> Device<Id> {
    Device::new(
        "mfa-engine-test-device".to_owned(),
        "mfa-engine-test-pubkey".to_owned(),
        user_id,
        DeviceType::User,
        None,
        true,
    )
    .save(pool)
    .await
    .expect("failed to create device")
}

async fn create_mfa_location(pool: &PgPool) -> WireguardNetwork<Id> {
    WireguardNetwork::new(
        "mfa-engine-test-location".to_owned(),
        51820,
        "vpn.example.com".to_owned(),
        None,
        [IpNetwork::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0).unwrap()],
        true,
        false,
        false,
        false,
        true, // mfa_enabled
        ServiceLocationMode::Disabled,
    )
    .set_address([IpNetwork::new(IpAddr::V4(Ipv4Addr::new(10, 10, 0, 1)), 24).unwrap()])
    .expect("failed to set location address")
    .save(pool)
    .await
    .expect("failed to create location")
}

async fn attach_device_to_location(pool: &PgPool, location_id: Id, device_id: Id) {
    WireguardNetworkDevice::new(
        location_id,
        device_id,
        vec![IpAddr::V4(Ipv4Addr::new(10, 10, 0, 10))],
    )
    .insert(pool)
    .await
    .expect("failed to attach device to location");
}

async fn create_and_assign_flow(
    pool: &PgPool,
    location_id: Id,
    steps: Vec<Vec<VpnClientMfaMethod>>,
) {
    let mut tx = pool.begin().await.expect("failed to begin tx");
    let (flow, _) = MfaFlow::create(&mut tx, "mfa-engine-test-flow".to_owned(), steps)
        .await
        .expect("failed to create flow");
    MfaFlow::assign_to_location(
        &mut tx,
        location_id,
        &[LocationMfaFlowAssignment {
            flow_id: flow.id,
            is_default: true,
            group_ids: Vec::new(),
        }],
    )
    .await
    .expect("failed to assign flow");
    tx.commit().await.expect("failed to commit tx");
}

async fn resolve_flow(
    pool: &PgPool,
    location_id: Id,
    user_id: Id,
) -> (Id, Vec<Vec<VpnClientMfaMethod>>) {
    let mut conn = pool.acquire().await.expect("failed to acquire conn");
    let (flow, steps) = MfaFlow::resolve_for_user(&mut conn, location_id, user_id)
        .await
        .expect("failed to resolve flow")
        .expect("flow should resolve");
    (flow.id, steps.into_iter().map(|s| s.methods).collect())
}

async fn session_count(pool: &PgPool, location_id: Id, device_id: Id) -> i64 {
    sqlx::query_scalar!(
        "SELECT count(*) FROM vpn_client_mfa_session WHERE location_id = $1 AND device_id = $2",
        location_id,
        device_id,
    )
    .fetch_one(pool)
    .await
    .unwrap()
    .unwrap_or(0)
}

#[test]
fn test_status_table_messages() {
    for (status, code, message) in [
        (
            Status::from(FinishError::Unauthorized),
            Code::Unauthenticated,
            "unauthorized",
        ),
        (
            Status::from(FinishError::SessionNotFound),
            Code::InvalidArgument,
            "login session not found",
        ),
        (
            Status::from(FinishError::AttemptLimit),
            Code::PermissionDenied,
            "Too many failed MFA attempts. Please try connecting again.",
        ),
        (
            Status::from(FinishError::StaleAttempt),
            Code::InvalidArgument,
            "stale MFA attempt",
        ),
        (
            Status::from(FinishError::UninitializedStep),
            Code::InvalidArgument,
            "no MFA attempt in progress",
        ),
    ] {
        assert_eq!(status.code(), code);
        assert_eq!(status.message(), message);
    }

    for (status, code, message) in [
        (
            Status::from(StepError::SessionNotFound),
            Code::InvalidArgument,
            "login session not found",
        ),
        (
            Status::from(StepError::MethodNotInStep),
            Code::InvalidArgument,
            "MFA method is not in the current step",
        ),
        (
            Status::from(StepError::MethodNotConfigured),
            Code::FailedPrecondition,
            "MFA method is not configured for this user",
        ),
    ] {
        assert_eq!(status.code(), code);
        assert_eq!(status.message(), message);
    }

    // OIDC's unresolved new-protocol outcome and legal method switching return OK, not a status.
    // License validation is frozen at Start, so the old StepStart license-loss row is obsolete.
}

#[sqlx::test]
async fn test_mobile_approve_empty_proof_reads_approval_flag(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let user = create_user(&pool).await;
    let context = MfaSessionContext {
        location: create_mfa_location(&pool).await,
        device: create_device(&pool, user.id).await,
        user,
    };
    let mut ephemeral = EphemeralState {
        step_attempt_id: "attempt".to_owned(),
        selected_method: VpnClientMfaMethod::MobileApprove,
        openid_auth_completed: false,
        mobile_approved: false,
        mobile_auth_device_name: None,
        biometric_challenge: None,
    };
    let proof = Proof {
        code: None,
        auth_pub_key: None,
        step_attempt_id: None,
    };

    assert_eq!(
        verify(&pool, &context, &ephemeral, &proof)
            .await
            .expect("empty mobile-approve proof must verify"),
        Verdict::NotYet,
    );

    ephemeral.mobile_approved = true;
    assert_eq!(
        verify(&pool, &context, &ephemeral, &proof)
            .await
            .expect("approved mobile-approve proof must verify"),
        Verdict::Proved,
    );
}

#[sqlx::test]
async fn test_start_multi_step_valid_totp_email_plan_returns_token(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    set_test_license_business();
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool)
        .await
        .expect("failed to init settings");
    let _smtp = configure_working_smtp(&pool).await;

    let location = create_mfa_location(&pool).await;
    create_and_assign_flow(
        &pool,
        location.id,
        vec![
            vec![VpnClientMfaMethod::Totp],
            vec![VpnClientMfaMethod::Email],
        ],
    )
    .await;
    let mut user = create_user(&pool).await;
    user.enable_totp(&pool)
        .await
        .expect("failed to enable TOTP");
    user.enable_email_mfa(&pool)
        .await
        .expect("failed to enable email MFA");
    let device = create_device(&pool, user.id).await;
    attach_device_to_location(&pool, location.id, device.id).await;

    let (flow_id, step_methods) = resolve_flow(&pool, location.id, user.id).await;
    let (engine, _event_rx, _gateway_rx) = make_engine(pool.clone());

    let result = engine
        .start_multi_step(
            &location,
            &device,
            &user,
            flow_id,
            step_methods,
            vec![VpnClientMfaMethod::Totp, VpnClientMfaMethod::Email],
        )
        .await
        .expect("start should succeed");
    let StartResult::Accepted(outcome) = result else {
        panic!("expected an accepted plan")
    };
    assert!(!outcome.token.is_empty());
    assert!(outcome.superseded_token_hash.is_none());
    assert!(
        VpnClientMfaSession::<Id>::find_active_by_token(&pool, &outcome.token)
            .await
            .unwrap()
            .is_some()
    );
}

#[sqlx::test]
async fn test_start_multi_step_supersedes_prior_session(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    set_test_license_business();
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool)
        .await
        .expect("failed to init settings");
    let _smtp = configure_working_smtp(&pool).await;

    let location = create_mfa_location(&pool).await;
    create_and_assign_flow(
        &pool,
        location.id,
        vec![
            vec![VpnClientMfaMethod::Totp],
            vec![VpnClientMfaMethod::Email],
        ],
    )
    .await;
    let mut user = create_user(&pool).await;
    user.enable_totp(&pool)
        .await
        .expect("failed to enable TOTP");
    user.enable_email_mfa(&pool)
        .await
        .expect("failed to enable email MFA");
    let device = create_device(&pool, user.id).await;
    attach_device_to_location(&pool, location.id, device.id).await;

    let (flow_id, step_methods) = resolve_flow(&pool, location.id, user.id).await;
    let (engine, _event_rx, _gateway_rx) = make_engine(pool.clone());
    let plan = vec![VpnClientMfaMethod::Totp, VpnClientMfaMethod::Email];

    let first = engine
        .start_multi_step(
            &location,
            &device,
            &user,
            flow_id,
            step_methods.clone(),
            plan.clone(),
        )
        .await
        .expect("first start should succeed");
    let StartResult::Accepted(first_outcome) = first else {
        panic!("expected an accepted plan")
    };

    let second = engine
        .start_multi_step(&location, &device, &user, flow_id, step_methods, plan)
        .await
        .expect("second start should succeed");
    let StartResult::Accepted(second_outcome) = second else {
        panic!("expected an accepted plan")
    };

    assert_eq!(
        second_outcome.superseded_token_hash,
        Some(hash_token(&first_outcome.token))
    );
    assert!(
        VpnClientMfaSession::<Id>::find_active_by_token(&pool, &first_outcome.token)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        VpnClientMfaSession::<Id>::find_active_by_token(&pool, &second_outcome.token)
            .await
            .unwrap()
            .is_some()
    );
}

#[sqlx::test]
async fn test_start_and_step_start_reject_unconfigured_biometric_method(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    set_test_license_business();
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool)
        .await
        .expect("failed to init settings");

    let location = create_mfa_location(&pool).await;
    create_and_assign_flow(
        &pool,
        location.id,
        vec![
            vec![VpnClientMfaMethod::Totp],
            vec![VpnClientMfaMethod::Biometric],
        ],
    )
    .await;
    let mut user = create_user(&pool).await;
    user.enable_totp(&pool)
        .await
        .expect("failed to enable TOTP");
    let device = create_device(&pool, user.id).await;
    attach_device_to_location(&pool, location.id, device.id).await;

    let (flow_id, step_methods) = resolve_flow(&pool, location.id, user.id).await;
    let (engine, mut event_rx, _gateway_rx) = make_engine(pool.clone());

    let result = engine
        .start_multi_step(
            &location,
            &device,
            &user,
            flow_id,
            step_methods.clone(),
            vec![VpnClientMfaMethod::Totp, VpnClientMfaMethod::Biometric],
        )
        .await
        .expect("start should return a rejection, not an error");
    let StartResult::Rejected(rejections) = result else {
        panic!("expected a rejected plan")
    };
    assert_eq!(rejections.len(), 1);
    assert_eq!(rejections[0].step, 1);
    assert_eq!(rejections[0].reason, StartRejectionReason::StepUnavailable);
    assert!(
        event_rx.try_recv().is_err(),
        "a rejection must not emit an event"
    );
    assert_eq!(session_count(&pool, location.id, device.id).await, 0);

    let mut transaction = pool.begin().await.expect("failed to begin transaction");
    let (_, outcome) = VpnClientMfaSession::<Id>::start(
        &mut transaction,
        location.id,
        device.id,
        user.id,
        flow_id,
        step_methods,
        VpnClientMfaMethod::Totp,
        None,
        VPN_MFA_SESSION_TIMEOUT,
    )
    .await
    .expect("failed to create test MFA session");
    transaction
        .commit()
        .await
        .expect("failed to commit test MFA session");
    let session = VpnClientMfaSession::<Id>::find_active_by_token(&pool, &outcome.token)
        .await
        .expect("failed to load test MFA session")
        .expect("test MFA session must exist");
    let mut connection = pool
        .acquire()
        .await
        .expect("failed to acquire database connection");
    session
        .advance(
            &mut connection,
            session.current_step,
            None,
            VpnClientMfaMethod::Totp,
        )
        .await
        .expect("failed to advance test MFA session")
        .expect("test MFA session must advance");

    let error = engine
        .step_start(outcome.token, VpnClientMfaMethod::Biometric)
        .await
        .expect_err("StepStart must reject the same unconfigured method");
    let status = Status::from(error);
    assert_eq!(status.code(), Code::FailedPrecondition);
    assert_eq!(
        status.message(),
        "MFA method is not configured for this user"
    );
    assert!(
        event_rx.try_recv().is_err(),
        "a rejected StepStart must not emit an event"
    );
}

#[sqlx::test]
async fn test_step_start_oidc_survives_license_lapse(_: PgPoolOptions, options: PgConnectOptions) {
    set_test_license_business();
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool)
        .await
        .expect("failed to init settings");
    OpenIdProvider::new(
        "Test".to_owned(),
        "https://idp.example.com".to_owned(),
        OpenIdProviderKind::Google,
        "client_id".to_owned(),
        "client_secret".to_owned(),
        None,
        None,
        None,
        None,
        true,
        60,
        DirectorySyncUserBehavior::Keep,
        DirectorySyncUserBehavior::Keep,
        DirectorySyncTarget::All,
        None,
        None,
        Vec::new(),
        None,
        false,
        false,
        None,
    )
    .save(&pool)
    .await
    .expect("failed to configure OpenID provider");

    let location = create_mfa_location(&pool).await;
    create_and_assign_flow(
        &pool,
        location.id,
        vec![
            vec![VpnClientMfaMethod::Totp],
            vec![VpnClientMfaMethod::Oidc],
        ],
    )
    .await;
    let mut user = create_user(&pool).await;
    user.new_totp_secret(&pool)
        .await
        .expect("failed to generate TOTP secret");
    user.enable_totp(&pool)
        .await
        .expect("failed to enable TOTP");
    user.openid_sub = Some("oidc-sub".to_owned());
    user.save(&pool).await.expect("failed to link OIDC user");
    let device = create_device(&pool, user.id).await;
    attach_device_to_location(&pool, location.id, device.id).await;

    let (flow_id, steps) = resolve_flow(&pool, location.id, user.id).await;
    let (engine, _event_rx, _gateway_rx) = make_engine(pool.clone());
    let StartResult::Accepted(start) = engine
        .start_multi_step(
            &location,
            &device,
            &user,
            flow_id,
            steps,
            vec![VpnClientMfaMethod::Totp, VpnClientMfaMethod::Oidc],
        )
        .await
        .expect("licensed start should succeed")
    else {
        panic!("expected an accepted plan")
    };
    engine
        .finish(
            start.token.clone(),
            Proof {
                code: Some(totp_code(&user)),
                auth_pub_key: None,
                step_attempt_id: None,
            },
            test_ip(),
        )
        .await
        .expect("TOTP step should advance");

    clear_test_license();
    let step = engine
        .step_start(start.token, VpnClientMfaMethod::Oidc)
        .await
        .expect("OIDC step must survive a license lapse");
    assert!(!step.step_attempt_id.is_empty());
}

#[sqlx::test]
async fn test_start_multi_step_unlicensed_fails_closed(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    clear_test_license();
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool)
        .await
        .expect("failed to init settings");

    let location = create_mfa_location(&pool).await;
    create_and_assign_flow(
        &pool,
        location.id,
        vec![
            vec![VpnClientMfaMethod::Totp],
            vec![VpnClientMfaMethod::Email],
        ],
    )
    .await;
    let user = create_user(&pool).await;
    let device = create_device(&pool, user.id).await;
    attach_device_to_location(&pool, location.id, device.id).await;

    let (flow_id, step_methods) = resolve_flow(&pool, location.id, user.id).await;
    let (engine, mut event_rx, _gateway_rx) = make_engine(pool.clone());

    let err = engine
        .start_multi_step(
            &location,
            &device,
            &user,
            flow_id,
            step_methods,
            vec![VpnClientMfaMethod::Totp, VpnClientMfaMethod::Email],
        )
        .await
        .expect_err("an unlicensed multi-step plan must fail closed");
    let err = Status::from(err);
    assert_eq!(err.code(), Code::FailedPrecondition);
    assert_eq!(
        err.message(),
        "multi-step MFA is not available for this location"
    );
    assert!(
        !err.message().contains("no valid license"),
        "the license gate message must not contain 'no valid license'"
    );
    assert!(event_rx.try_recv().is_err());
    assert_eq!(session_count(&pool, location.id, device.id).await, 0);
}

#[sqlx::test]
async fn test_start_multi_step_rejects_unconfigured_method(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    set_test_license_business();
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool)
        .await
        .expect("failed to init settings");

    let location = create_mfa_location(&pool).await;
    create_and_assign_flow(
        &pool,
        location.id,
        vec![
            vec![VpnClientMfaMethod::Totp],
            vec![VpnClientMfaMethod::Email],
        ],
    )
    .await;
    // TOTP is configured; email is not, so only the email step is unavailable.
    let mut user = create_user(&pool).await;
    user.enable_totp(&pool)
        .await
        .expect("failed to enable TOTP");
    let device = create_device(&pool, user.id).await;
    attach_device_to_location(&pool, location.id, device.id).await;

    let (flow_id, step_methods) = resolve_flow(&pool, location.id, user.id).await;
    let (engine, mut event_rx, _gateway_rx) = make_engine(pool.clone());

    let result = engine
        .start_multi_step(
            &location,
            &device,
            &user,
            flow_id,
            step_methods,
            vec![VpnClientMfaMethod::Totp, VpnClientMfaMethod::Email],
        )
        .await
        .expect("start should return a rejection, not an error");
    let StartResult::Rejected(rejections) = result else {
        panic!("expected a rejected plan")
    };
    assert_eq!(rejections.len(), 1);
    assert_eq!(rejections[0].step, 1);
    assert_eq!(rejections[0].reason, StartRejectionReason::StepUnavailable);
    assert!(event_rx.try_recv().is_err());
    assert_eq!(session_count(&pool, location.id, device.id).await, 0);
}

/// The TOTP/Email method restriction applies only to multi-step flows, so it must not reject a
/// single-step plan.
#[sqlx::test]
async fn test_start_multi_step_single_step_non_boundary_method_accepted(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    set_test_license_business();
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool)
        .await
        .expect("failed to init settings");

    let location = create_mfa_location(&pool).await;
    create_and_assign_flow(
        &pool,
        location.id,
        vec![vec![VpnClientMfaMethod::MobileApprove]],
    )
    .await;
    let user = create_user(&pool).await;
    let device = create_device(&pool, user.id).await;
    attach_device_to_location(&pool, location.id, device.id).await;
    BiometricAuth::new(device.id, "single-step-test-key".to_owned())
        .save(&pool)
        .await
        .expect("failed to register mobile-approve key");

    let (flow_id, step_methods) = resolve_flow(&pool, location.id, user.id).await;
    let (engine, _event_rx, _gateway_rx) = make_engine(pool.clone());

    let result = engine
        .start_multi_step(
            &location,
            &device,
            &user,
            flow_id,
            step_methods,
            vec![VpnClientMfaMethod::MobileApprove],
        )
        .await
        .expect("a single-step non-TOTP/Email plan must start");
    assert!(
        matches!(result, StartResult::Accepted(_)),
        "expected an accepted plan, got a rejection"
    );
}

async fn start_two_step_session(pool: &PgPool, user_id: Id) -> (VpnClientMfaSession<Id>, String) {
    let location = create_mfa_location(pool).await;
    let device = create_device(pool, user_id).await;
    attach_device_to_location(pool, location.id, device.id).await;
    let mut tx = pool.begin().await.unwrap();
    let (session, outcome) = VpnClientMfaSession::<Id>::start(
        &mut tx,
        location.id,
        device.id,
        user_id,
        1,
        vec![
            vec![VpnClientMfaMethod::Totp],
            vec![VpnClientMfaMethod::Email],
        ],
        VpnClientMfaMethod::Totp,
        None,
        VPN_MFA_SESSION_TIMEOUT,
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    (session, outcome.token)
}

async fn start_session_with_flow(
    pool: &PgPool,
    user_id: Id,
    title: &str,
    steps: Vec<Vec<VpnClientMfaMethod>>,
) -> (VpnClientMfaSession<Id>, String, MfaFlow<Id>) {
    let location = create_mfa_location(pool).await;
    let device = create_device(pool, user_id).await;
    attach_device_to_location(pool, location.id, device.id).await;
    let mut tx = pool.begin().await.unwrap();
    let (flow, _) = MfaFlow::create(&mut tx, title.to_owned(), steps.clone())
        .await
        .unwrap();
    let (session, outcome) = VpnClientMfaSession::<Id>::start(
        &mut tx,
        location.id,
        device.id,
        user_id,
        flow.id,
        steps.clone(),
        steps[0][0],
        None,
        VPN_MFA_SESSION_TIMEOUT,
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    (session, outcome.token, flow)
}

async fn advance_session(pool: &PgPool, session: &VpnClientMfaSession<Id>) {
    let mut conn = pool.acquire().await.unwrap();
    session
        .advance(
            &mut conn,
            session.current_step,
            None,
            VpnClientMfaMethod::Totp,
        )
        .await
        .unwrap()
        .expect("advance should match the current step");
}

#[sqlx::test]
async fn test_step_start_mints_an_id(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool)
        .await
        .expect("failed to init settings");
    let _smtp = configure_working_smtp(&pool).await;

    let mut user = create_user(&pool).await;
    user.new_email_secret(&pool)
        .await
        .expect("failed to generate email secret");
    user.enable_email_mfa(&pool)
        .await
        .expect("failed to enable email MFA");
    let (session, token) = start_two_step_session(&pool, user.id).await;
    advance_session(&pool, &session).await;

    let (engine, _event_rx, _gateway_rx) = make_engine(pool.clone());
    let started = engine
        .step_start(token, VpnClientMfaMethod::Email)
        .await
        .expect("step start should succeed");
    assert!(!started.step_attempt_id.is_empty());
    assert!(started.challenge.is_none());
}

#[sqlx::test]
async fn test_step_start_recall_mints_fresh_id_and_resends(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool)
        .await
        .expect("failed to init settings");
    let smtp = configure_working_smtp(&pool).await;

    let mut user = create_user(&pool).await;
    user.new_email_secret(&pool)
        .await
        .expect("failed to generate email secret");
    user.enable_email_mfa(&pool)
        .await
        .expect("failed to enable email MFA");
    let (session, token) = start_two_step_session(&pool, user.id).await;
    advance_session(&pool, &session).await;

    let (engine, _event_rx, _gateway_rx) = make_engine(pool.clone());
    let first = engine
        .step_start(token.clone(), VpnClientMfaMethod::Email)
        .await
        .expect("first step start should succeed");
    let second = engine
        .step_start(token, VpnClientMfaMethod::Email)
        .await
        .expect("second step start should succeed");

    // A same-method re-call is a retry, not a no-op: it supersedes the prior attempt and
    // re-runs `initiate`, which is what makes "resend the code" work.
    assert_ne!(
        first.step_attempt_id, second.step_attempt_id,
        "a re-call must mint a fresh attempt id"
    );
    smtp.wait_for_count(2).await;
    assert_eq!(
        smtp.message_count(),
        2,
        "a re-call must re-send the email so the user can request a new code"
    );
}

#[sqlx::test]
async fn test_mfa_actions_reject_missing_session(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (engine, _event_rx, _gateway_rx) = make_engine(pool);

    let status = Status::from(
        engine
            .step_start("missing-token".to_owned(), VpnClientMfaMethod::Totp)
            .await
            .expect_err("missing session must be rejected"),
    );
    assert_eq!(status.code(), Code::InvalidArgument);
    assert_eq!(status.message(), "login session not found");

    let status = Status::from(
        engine
            .finish(
                "missing-token".to_owned(),
                Proof {
                    code: Some("000000".to_owned()),
                    auth_pub_key: None,
                    step_attempt_id: Some("attempt".to_owned()),
                },
                test_ip(),
            )
            .await
            .expect_err("missing session must be rejected"),
    );
    assert_eq!(status.code(), Code::InvalidArgument);
    assert_eq!(status.message(), "login session not found");
}

#[sqlx::test]
async fn test_step_start_rejects_method_not_in_step(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool)
        .await
        .expect("failed to init settings");

    let user = create_user(&pool).await;
    let (_session, token) = start_two_step_session(&pool, user.id).await;

    let (engine, _event_rx, _gateway_rx) = make_engine(pool.clone());
    let err = engine
        .step_start(token, VpnClientMfaMethod::Email)
        .await
        .expect_err("a method outside the current step must be rejected");
    let err = Status::from(err);
    assert_eq!(err.code(), Code::InvalidArgument);
    assert_eq!(err.message(), "MFA method is not in the current step");
}

#[sqlx::test]
async fn test_step_start_rejects_unconfigured_method(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool)
        .await
        .expect("failed to init settings");

    // Email is not configured (email_mfa_enabled is false by default).
    let user = create_user(&pool).await;
    let (session, token) = start_two_step_session(&pool, user.id).await;
    advance_session(&pool, &session).await;

    let (engine, _event_rx, _gateway_rx) = make_engine(pool.clone());
    let err = engine
        .step_start(token, VpnClientMfaMethod::Email)
        .await
        .expect_err("an unconfigured method must be rejected");
    let err = Status::from(err);
    assert_eq!(err.code(), Code::FailedPrecondition);
    assert_eq!(err.message(), "MFA method is not configured for this user");
}

async fn setup_user_totp_and_email(pool: &PgPool, user: &mut User<Id>) {
    user.new_totp_secret(pool).await.expect("new_totp_secret");
    user.enable_totp(pool).await.expect("enable_totp");
    user.new_email_secret(pool).await.expect("new_email_secret");
    user.enable_email_mfa(pool).await.expect("enable_email_mfa");
}

fn totp_code(user: &User<Id>) -> String {
    let secret = user.totp_secret.as_ref().expect("totp_secret must be set");
    let ts = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("system time before epoch")
        .as_secs();
    totp_custom::<Sha1>(TOTP_CODE_VALIDITY_PERIOD, TOTP_CODE_DIGITS, secret, ts)
}

fn email_code(user: &User<Id>) -> String {
    user.generate_email_mfa_code()
        .expect("email_mfa_secret must be set")
}

fn test_ip() -> IpAddr {
    IpAddr::V4(Ipv4Addr::new(10, 0, 0, 7))
}

#[sqlx::test]
async fn test_finish_advanced_then_completed(_: PgPoolOptions, options: PgConnectOptions) {
    set_test_license_business();
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool)
        .await
        .expect("failed to init settings");
    let _smtp = configure_working_smtp(&pool).await;

    let location = create_mfa_location(&pool).await;
    create_and_assign_flow(
        &pool,
        location.id,
        vec![
            vec![VpnClientMfaMethod::Totp],
            vec![VpnClientMfaMethod::Email],
        ],
    )
    .await;
    let mut user = create_user(&pool).await;
    setup_user_totp_and_email(&pool, &mut user).await;
    let device = create_device(&pool, user.id).await;
    attach_device_to_location(&pool, location.id, device.id).await;

    let (flow_id, step_methods) = resolve_flow(&pool, location.id, user.id).await;
    let (engine, mut event_rx, _gateway_rx) = make_engine(pool.clone());

    let result = engine
        .start_multi_step(
            &location,
            &device,
            &user,
            flow_id,
            step_methods,
            vec![VpnClientMfaMethod::Totp, VpnClientMfaMethod::Email],
        )
        .await
        .expect("start should succeed");
    let StartResult::Accepted(outcome) = result else {
        panic!("expected an accepted plan")
    };
    let token = outcome.token;

    // Step 0 (TOTP) is not final: finish returns Advanced without authorizing.
    let (outcome, _) = engine
        .finish(
            token.clone(),
            Proof {
                code: Some(totp_code(&user)),
                auth_pub_key: None,
                step_attempt_id: None,
            },
            test_ip(),
        )
        .await
        .expect("finish of step 0 should succeed");
    assert_eq!(outcome, FinishOutcome::Advanced { next_step: 1 });
    assert!(
        VpnClientSession::get_all_active_device_sessions_in_location(&pool, location.id, device.id)
            .await
            .unwrap()
            .is_empty(),
        "no session may be authorized before the final step"
    );
    assert!(event_rx.try_recv().is_err());

    // Initialize and finish step 1 (Email): this completes the flow.
    engine
        .step_start(token.clone(), VpnClientMfaMethod::Email)
        .await
        .expect("step_start should succeed");
    let (outcome, _) = engine
        .finish(
            token.clone(),
            Proof {
                code: Some(email_code(&user)),
                auth_pub_key: None,
                step_attempt_id: None,
            },
            test_ip(),
        )
        .await
        .expect("finish of step 1 should succeed");
    let FinishOutcome::Completed { preshared_key } = outcome else {
        panic!("expected a completed flow")
    };
    assert!(!preshared_key.is_empty());

    let sessions =
        VpnClientSession::get_all_active_device_sessions_in_location(&pool, location.id, device.id)
            .await
            .unwrap();
    assert_eq!(sessions.len(), 1);
    assert!(sessions[0].is_mfa_session);
    assert!(
        VpnClientMfaSession::<Id>::find_active_by_token(&pool, &token)
            .await
            .unwrap()
            .is_none(),
        "the in-progress session must be deleted on completion"
    );

    // A single Success event with the ordered satisfied methods.
    let event = event_rx.try_recv().expect("expected a success event");
    match event.event {
        BidiStreamEventType::DesktopClientMfa(event) => match *event {
            DesktopClientMfaEvent::Success { attribution, .. } => {
                assert_eq!(attribution.snapshot.steps.len(), 2);
                assert_eq!(
                    attribution.snapshot.steps[0].satisfied,
                    Some(VpnClientMfaMethod::Totp)
                );
                assert_eq!(
                    attribution.snapshot.steps[1].satisfied,
                    Some(VpnClientMfaMethod::Email)
                );
            }
            other => panic!("unexpected event: {other:?}"),
        },
        other => panic!("unexpected stream event: {other:?}"),
    }
}

/// A proof for step 0 must never be able to satisfy step 1 as well.
///
/// The attack it rules out: in a `[TOTP, Email]` flow, an attacker holding only the TOTP secret
/// replays one valid code twice. Both calls verify against the same ephemeral state, both
/// advance the cursor, and the second sees `current_step == total_steps` and authorizes the
/// peer with the Email step never proved.
#[sqlx::test]
async fn test_finish_replayed_proof_cannot_skip_a_step(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    set_test_license_business();
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool)
        .await
        .expect("failed to init settings");
    let _smtp = configure_working_smtp(&pool).await;

    let location = create_mfa_location(&pool).await;
    create_and_assign_flow(
        &pool,
        location.id,
        vec![
            vec![VpnClientMfaMethod::Totp],
            vec![VpnClientMfaMethod::Email],
        ],
    )
    .await;
    let mut user = create_user(&pool).await;
    setup_user_totp_and_email(&pool, &mut user).await;
    let device = create_device(&pool, user.id).await;
    attach_device_to_location(&pool, location.id, device.id).await;

    let (flow_id, step_methods) = resolve_flow(&pool, location.id, user.id).await;
    let (engine, mut event_rx, _gateway_rx) = make_engine(pool.clone());

    let result = engine
        .start_multi_step(
            &location,
            &device,
            &user,
            flow_id,
            step_methods,
            vec![VpnClientMfaMethod::Totp, VpnClientMfaMethod::Email],
        )
        .await
        .expect("start should succeed");
    let StartResult::Accepted(outcome) = result else {
        panic!("expected an accepted plan")
    };
    let token = outcome.token;

    let code = totp_code(&user);
    let (outcome, _) = engine
        .finish(
            token.clone(),
            Proof {
                code: Some(code.clone()),
                auth_pub_key: None,
                step_attempt_id: None,
            },
            test_ip(),
        )
        .await
        .expect("finish of step 0 should succeed");
    assert_eq!(outcome, FinishOutcome::Advanced { next_step: 1 });

    // Replay the very same proof. It must not complete the flow.
    let err = engine
        .finish(
            token.clone(),
            Proof {
                code: Some(code),
                auth_pub_key: None,
                step_attempt_id: None,
            },
            test_ip(),
        )
        .await
        .expect_err("a replayed step-0 proof must not satisfy step 1");
    let err = Status::from(err);
    assert_eq!(err.code(), Code::InvalidArgument);
    assert_eq!(err.message(), "no MFA attempt in progress");

    // The security property the status code alone does not prove: no peer was authorized.
    assert!(
        VpnClientSession::get_all_active_device_sessions_in_location(&pool, location.id, device.id)
            .await
            .unwrap()
            .is_empty(),
        "a replayed proof must not authorize a peer"
    );
    assert!(
        event_rx.try_recv().is_err(),
        "a replayed proof must not emit a success event"
    );

    // The flow is still waiting on step 1, not completed.
    let session = VpnClientMfaSession::<Id>::find_active_by_token(&pool, &token)
        .await
        .unwrap()
        .expect("the MFA session must survive a rejected replay");
    assert_eq!(session.current_step, 1);
    assert_eq!(
        session.steps_snapshot.0.steps[1].satisfied, None,
        "the Email step must remain unsatisfied"
    );
}

/// A proof carrying a superseded `step_attempt_id` must be rejected. Re-calling `step_start`
/// mints a fresh attempt, and the previous one stops being spendable at that moment.
#[sqlx::test]
async fn test_finish_rejects_superseded_attempt_id(_: PgPoolOptions, options: PgConnectOptions) {
    set_test_license_business();
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool)
        .await
        .expect("failed to init settings");
    let _smtp = configure_working_smtp(&pool).await;

    let location = create_mfa_location(&pool).await;
    create_and_assign_flow(
        &pool,
        location.id,
        vec![
            vec![VpnClientMfaMethod::Totp],
            vec![VpnClientMfaMethod::Email],
        ],
    )
    .await;
    let mut user = create_user(&pool).await;
    setup_user_totp_and_email(&pool, &mut user).await;
    let device = create_device(&pool, user.id).await;
    attach_device_to_location(&pool, location.id, device.id).await;

    let (flow_id, step_methods) = resolve_flow(&pool, location.id, user.id).await;
    let (engine, _event_rx, _gateway_rx) = make_engine(pool.clone());

    let result = engine
        .start_multi_step(
            &location,
            &device,
            &user,
            flow_id,
            step_methods,
            vec![VpnClientMfaMethod::Totp, VpnClientMfaMethod::Email],
        )
        .await
        .expect("start should succeed");
    let StartResult::Accepted(outcome) = result else {
        panic!("expected an accepted plan")
    };
    let token = outcome.token;

    engine
        .finish(
            token.clone(),
            Proof {
                code: Some(totp_code(&user)),
                auth_pub_key: None,
                step_attempt_id: None,
            },
            test_ip(),
        )
        .await
        .expect("finish of step 0 should succeed");

    let first = engine
        .step_start(token.clone(), VpnClientMfaMethod::Email)
        .await
        .expect("first step_start should succeed");
    let second = engine
        .step_start(token.clone(), VpnClientMfaMethod::Email)
        .await
        .expect("second step_start should succeed");
    assert_ne!(first.step_attempt_id, second.step_attempt_id);

    // Spend the superseded attempt id: rejected even though the code itself is valid.
    let err = engine
        .finish(
            token.clone(),
            Proof {
                code: Some(email_code(&user)),
                auth_pub_key: None,
                step_attempt_id: Some(first.step_attempt_id),
            },
            test_ip(),
        )
        .await
        .expect_err("a superseded attempt id must be rejected");
    let err = Status::from(err);
    assert_eq!(err.code(), Code::InvalidArgument);
    assert_eq!(err.message(), "stale MFA attempt");

    // The current attempt still works, so the guard rejects staleness, not the method.
    let (outcome, _) = engine
        .finish(
            token,
            Proof {
                code: Some(email_code(&user)),
                auth_pub_key: None,
                step_attempt_id: Some(second.step_attempt_id),
            },
            test_ip(),
        )
        .await
        .expect("the current attempt must still complete the flow");
    assert!(matches!(outcome, FinishOutcome::Completed { .. }));
}

#[sqlx::test]
async fn test_finish_cap_with_attempt_id_returns_restart_status_for_one_step_flow(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool)
        .await
        .expect("failed to init settings");

    let user = create_user(&pool).await;
    let (session, token, _) = start_session_with_flow(
        &pool,
        user.id,
        "One-step cap flow",
        vec![vec![VpnClientMfaMethod::Totp]],
    )
    .await;
    let attempt_id = session
        .ephemeral_state
        .as_ref()
        .expect("session must have an initial attempt")
        .step_attempt_id
        .clone();
    let (engine, _event_rx, _gateway_rx) = make_engine(pool.clone());

    for _ in 0..MFA_FAILED_ATTEMPT_CAP - 1 {
        let err = Status::from(
            engine
                .finish(
                    token.clone(),
                    Proof {
                        code: Some("000000".to_owned()),
                        auth_pub_key: None,
                        step_attempt_id: Some(attempt_id.clone()),
                    },
                    test_ip(),
                )
                .await
                .expect_err("a wrong code must be rejected"),
        );
        assert_eq!(err.code(), Code::Unauthenticated);
        assert_eq!(err.message(), "unauthorized");
    }

    let err = Status::from(
        engine
            .finish(
                token.clone(),
                Proof {
                    code: Some("000000".to_owned()),
                    auth_pub_key: None,
                    step_attempt_id: Some(attempt_id),
                },
                test_ip(),
            )
            .await
            .expect_err("the cap must require a restart"),
    );
    assert_eq!(err.code(), Code::PermissionDenied);
    assert_eq!(
        err.message(),
        "Too many failed MFA attempts. Please try connecting again."
    );

    let err = Status::from(
        engine
            .finish(
                token,
                Proof {
                    code: Some("000000".to_owned()),
                    auth_pub_key: None,
                    step_attempt_id: None,
                },
                test_ip(),
            )
            .await
            .expect_err("a capped session must be gone"),
    );
    assert_eq!(err.code(), Code::InvalidArgument);
    assert_eq!(err.message(), "login session not found");
}

#[sqlx::test]
async fn test_finish_cap_emits_frozen_partial_abort_attribution(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool)
        .await
        .expect("failed to init settings");

    let _smtp = configure_working_smtp(&pool).await;
    let mut user = create_user(&pool).await;
    user.new_email_secret(&pool)
        .await
        .expect("failed to generate email secret");
    user.enable_email_mfa(&pool)
        .await
        .expect("failed to enable email MFA");
    let (session, token, mut flow) = start_session_with_flow(
        &pool,
        user.id,
        "Cap attribution flow",
        vec![
            vec![VpnClientMfaMethod::Totp],
            vec![VpnClientMfaMethod::Email],
        ],
    )
    .await;
    advance_session(&pool, &session).await;
    let (engine, mut event_rx, _gateway_rx) = make_engine(pool.clone());
    let email_attempt = engine
        .step_start(token.clone(), VpnClientMfaMethod::Email)
        .await
        .expect("email step must initialize");

    for _ in 0..MFA_FAILED_ATTEMPT_CAP - 1 {
        let err = Status::from(
            engine
                .finish(
                    token.clone(),
                    Proof {
                        code: Some("000000".to_owned()),
                        auth_pub_key: None,
                        step_attempt_id: Some(email_attempt.step_attempt_id.clone()),
                    },
                    test_ip(),
                )
                .await
                .expect_err("a wrong code must be rejected"),
        );
        assert_eq!(err.code(), Code::Unauthenticated);
    }
    let err = Status::from(
        engine
            .finish(
                token,
                Proof {
                    code: Some("000000".to_owned()),
                    auth_pub_key: None,
                    step_attempt_id: Some(email_attempt.step_attempt_id),
                },
                test_ip(),
            )
            .await
            .expect_err("the cap must abort the flow"),
    );
    assert_eq!(err.code(), Code::PermissionDenied);
    flow.title = "Renamed after abort".to_owned();
    flow.save(&pool).await.expect("flow rename must succeed");

    for _ in 0..MFA_FAILED_ATTEMPT_CAP {
        assert!(matches!(
            event_rx.try_recv().expect("expected a failed event").event,
            BidiStreamEventType::DesktopClientMfa(event)
                if matches!(*event, DesktopClientMfaEvent::Failed { .. })
        ));
    }
    let event = event_rx.try_recv().expect("expected an abort event");
    let BidiStreamEventType::DesktopClientMfa(event) = event.event else {
        panic!("unexpected stream event");
    };
    let DesktopClientMfaEvent::Aborted { attribution, .. } = *event else {
        panic!("expected MFA abort event");
    };
    assert_eq!(attribution.snapshot.flow_id, flow.id);
    assert_eq!(
        attribution.flow_name.as_deref(),
        Some("Cap attribution flow")
    );
    assert_eq!(
        attribution.snapshot.steps[0].satisfied,
        Some(VpnClientMfaMethod::Totp)
    );
    assert_eq!(attribution.snapshot.steps[1].satisfied, None);
    assert!(
        event_rx.try_recv().is_err(),
        "only one abort must be emitted"
    );
}

#[sqlx::test]
async fn test_finish_cap_deletes_session_and_emits_failed(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool)
        .await
        .expect("failed to init settings");

    let user = create_user(&pool).await;
    let (_session, token) = start_two_step_session(&pool, user.id).await;

    let (engine, mut event_rx, _gateway_rx) = make_engine(pool.clone());
    for _ in 0..MFA_FAILED_ATTEMPT_CAP {
        let err = engine
            .finish(
                token.clone(),
                Proof {
                    code: Some("000000".to_owned()),
                    auth_pub_key: None,
                    step_attempt_id: None,
                },
                test_ip(),
            )
            .await
            .expect_err("a wrong code must be rejected");
        let err = Status::from(err);
        assert_eq!(err.code(), Code::Unauthenticated);
        assert_eq!(err.message(), "unauthorized");
    }

    assert!(
        VpnClientMfaSession::<Id>::find_active_by_token(&pool, &token)
            .await
            .unwrap()
            .is_none(),
        "the session must be deleted at the attempt cap"
    );

    for _ in 0..MFA_FAILED_ATTEMPT_CAP {
        let event = event_rx.try_recv().expect("expected a failed event");
        match event.event {
            BidiStreamEventType::DesktopClientMfa(event) => match *event {
                DesktopClientMfaEvent::Failed {
                    method, message, ..
                } => {
                    assert_eq!(method, MfaMethod::Totp);
                    assert_eq!(message, "invalid TOTP code");
                }
                other => panic!("unexpected event: {other:?}"),
            },
            other => panic!("unexpected stream event: {other:?}"),
        }
    }
    assert!(
        event_rx.try_recv().is_err(),
        "legacy requests must not emit an abort"
    );
}

#[sqlx::test]
async fn test_finish_on_uninitialized_step(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool)
        .await
        .expect("failed to init settings");

    let user = create_user(&pool).await;
    let (session, token) = start_two_step_session(&pool, user.id).await;
    advance_session(&pool, &session).await;

    let (engine, _event_rx, _gateway_rx) = make_engine(pool.clone());
    let err = engine
        .finish(
            token,
            Proof {
                code: Some("000000".to_owned()),
                auth_pub_key: None,
                step_attempt_id: None,
            },
            test_ip(),
        )
        .await
        .expect_err("finish on an uninitialized step must be rejected");
    let err = Status::from(err);
    assert_eq!(err.code(), Code::InvalidArgument);
    assert_eq!(err.message(), "no MFA attempt in progress");
}
