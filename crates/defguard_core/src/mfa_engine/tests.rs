use std::{
    collections::HashSet,
    net::{IpAddr, Ipv4Addr},
    time::{Instant, SystemTime},
};

use base64::{
    Engine as _,
    prelude::{BASE64_STANDARD, BASE64_URL_SAFE_NO_PAD},
};
use chrono::{TimeDelta, Utc};
use defguard_common::{
    db::{
        Id,
        models::{
            Device, DeviceType, ThrottleScope, User, WebAuthn, WireguardNetwork,
            biometric_auth::{BiometricAuth, BiometricChallenge},
            device::WireguardNetworkDevice,
            mfa_flow::{LocationMfaFlowAssignment, MfaFlow},
            settings::{Settings, initialize_current_settings},
            user::{TOTP_CODE_DIGITS, TOTP_CODE_VALIDITY_PERIOD},
            vpn_client_mfa_session::{
                EphemeralState, MFA_FAILED_ATTEMPT_CAP, VPN_MFA_SESSION_TIMEOUT,
                VpnClientMfaSession, VpnMfaFlowKind, hash_token,
            },
            vpn_client_session::{VpnClientMfaMethod, VpnClientSession},
            wireguard::ServiceLocationMode,
        },
        setup_pool,
    },
    testing::smtp::configure_working_smtp,
};
use defguard_proto::client_types::MfaMethod;
use ed25519_dalek::{Signer, SigningKey};
use futures::future::join_all;
use ipnetwork::IpNetwork;
use sha2::{Digest, Sha256};
use sqlx::{
    PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
};
use tokio::sync::mpsc;
use tonic::{Code, Status};
use totp_lite::{Sha1, totp_custom};
use uuid::Uuid;
use webauthn_authenticator_rs::{WebauthnAuthenticator, prelude::Url, softpasskey::SoftPasskey};
use webauthn_rs::prelude::Passkey;

use super::{MfaEngine, POLL_LIMIT, POLL_WINDOW, PollWindows};
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
        error::StartError,
        legacy::{FinishError, LegacyProof},
        method::{InitiateError, Verdict, VerifyError, check_mobile_approval, verify},
        multi_step::{
            Fido2Assertion, StartRejectionReason, StartResult, StepCredential, StepError,
            StepFinishError, StepProof,
        },
        types::{FinishOutcome, VerificationProof},
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
    mpsc::UnboundedReceiver<GatewayCommand>,
) {
    let (gateway_tx, gateway_rx) = mpsc::unbounded_channel();
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
) -> (Id, Vec<HashSet<VpnClientMfaMethod>>) {
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
        (
            Status::from(StartError::AttemptLimit),
            Code::FailedPrecondition,
            "Too many failed MFA attempts. Try again later.",
        ),
        (
            Status::from(InitiateError::TooManyRequests),
            Code::FailedPrecondition,
            "Too many MFA requests. Try again later.",
        ),
    ] {
        assert_eq!(status.code(), code);
        assert_eq!(status.message(), message);
    }

    // OIDC and method-switching flows return OK; license checks happen at start.
}

#[test]
fn test_mobile_approve_empty_proof_reads_approval_flag() {
    let mut ephemeral = EphemeralState {
        step_attempt_id: "attempt".to_owned(),
        selected_method: VpnClientMfaMethod::MobileApprove,
        openid_auth_completed: false,
        mobile_approved: false,
        mobile_auth_device_name: None,
        biometric_challenge: None,
    };
    assert_eq!(check_mobile_approval(&ephemeral), Verdict::NotYet,);

    ephemeral.mobile_approved = true;
    assert_eq!(check_mobile_approval(&ephemeral), Verdict::Proved);
}

#[sqlx::test]
async fn test_multi_step_mobile_approval_is_mark_only_and_attempt_bound(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool)
        .await
        .expect("failed to init settings");
    let location = create_mfa_location(&pool).await;
    let user = create_user(&pool).await;
    let device = create_device(&pool, user.id).await;
    attach_device_to_location(&pool, location.id, device.id).await;

    let signing_key = SigningKey::from_bytes(&[7; 32]);
    let auth_pub_key = BASE64_STANDARD.encode(signing_key.verifying_key().to_bytes());
    BiometricAuth::new(device.id, auth_pub_key.clone())
        .save(&pool)
        .await
        .expect("failed to register mobile authenticator");
    let challenge = BiometricChallenge::new();
    let mut transaction = pool.begin().await.expect("failed to begin transaction");
    let (flow, _) = MfaFlow::create(
        &mut transaction,
        "Mobile approval flow".to_owned(),
        vec![vec![VpnClientMfaMethod::MobileApprove]],
    )
    .await
    .expect("failed to create flow");
    let (_, outcome) = VpnClientMfaSession::<Id>::start(
        &mut transaction,
        location.id,
        device.id,
        user.id,
        flow.id,
        vec![vec![VpnClientMfaMethod::MobileApprove]],
        VpnMfaFlowKind::MultiStep,
        VpnClientMfaMethod::MobileApprove,
        Some(challenge.clone()),
        VPN_MFA_SESSION_TIMEOUT,
    )
    .await
    .expect("failed to start mobile approval session");
    transaction
        .commit()
        .await
        .expect("failed to commit mobile approval session");

    let (engine, mut event_rx, mut gateway_rx) = make_engine(pool.clone());
    let signature = sign_challenge(&signing_key, &challenge.challenge);
    let checked_proof = engine
        .check_legacy_mobile_proof(&outcome.token, &signature, &auth_pub_key)
        .await
        .expect("valid proof check should succeed")
        .expect("valid mobile proof should return the current attempt");
    assert_eq!(checked_proof.step_attempt_id, outcome.step_attempt_id);
    assert_eq!(checked_proof.username, user.username);
    assert_eq!(checked_proof.device_id, device.id);
    assert_eq!(checked_proof.location_name, location.name);
    let legacy_proof_key = format!("legacy-mobile:{}", hash_token(&outcome.token));
    assert_eq!(
        ThrottleScope::VpnMfaCode
            .attempts(&pool, &legacy_proof_key)
            .await,
        Some(0),
        "a valid legacy signature is refunded"
    );

    let unmarked = VpnClientMfaSession::<Id>::find_active_by_token(&pool, &outcome.token)
        .await
        .expect("session lookup should succeed")
        .expect("mobile approval session must remain active");
    assert_eq!(unmarked.failed_attempts, 0);
    assert!(
        !unmarked
            .ephemeral_state
            .as_ref()
            .expect("attempt must remain active")
            .0
            .mobile_approved
    );
    assert!(event_rx.try_recv().is_err());
    assert!(gateway_rx.try_recv().is_err());

    engine
        .approve_mobile_step(
            outcome.token.clone(),
            super::multi_step::MobileApprovalProof {
                signature,
                auth_pub_key,
                step_attempt_id: outcome.step_attempt_id.clone(),
            },
            test_ip(),
        )
        .await
        .expect("valid mobile approval should be accepted");

    let marked = VpnClientMfaSession::<Id>::find_active_by_token(&pool, &outcome.token)
        .await
        .expect("session lookup should succeed")
        .expect("mobile approval session must remain active");
    assert_eq!(marked.failed_attempts, 0);
    assert!(
        marked
            .ephemeral_state
            .as_ref()
            .expect("attempt must remain active")
            .0
            .mobile_approved
    );
    assert!(event_rx.try_recv().is_err());
    assert!(gateway_rx.try_recv().is_err());

    let result = engine
        .finish_step(
            outcome.token,
            super::multi_step::StepProof {
                step_attempt_id: outcome.step_attempt_id,
                credential: None,
            },
            test_ip(),
        )
        .await
        .expect("approved mobile step should finish the flow");
    assert!(matches!(result, FinishOutcome::Completed { .. }));
}

fn sign_challenge(signing_key: &SigningKey, challenge: &str) -> String {
    BASE64_STANDARD.encode(signing_key.sign(challenge.as_bytes()).to_bytes())
}

#[sqlx::test]
async fn test_legacy_mobile_proof_check_throttles_invalid_signatures(
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
        "Legacy mobile proof check",
        vec![vec![VpnClientMfaMethod::MobileApprove]],
        VpnMfaFlowKind::MultiStep,
    )
    .await;
    let legacy_proof_key = format!("legacy-mobile:{}", hash_token(&token));
    let signing_key = SigningKey::from_bytes(&[12; 32]);
    let auth_pub_key = BASE64_STANDARD.encode(signing_key.verifying_key().to_bytes());
    BiometricAuth::new(session.device_id, auth_pub_key.clone())
        .save(&pool)
        .await
        .expect("failed to register mobile authenticator");

    let (engine, mut event_rx, mut gateway_rx) = make_engine(pool.clone());
    let signature_without_challenge = sign_challenge(&signing_key, "challenge");
    assert_eq!(
        engine
            .check_legacy_mobile_proof(&token, &signature_without_challenge, &auth_pub_key)
            .await
            .expect("proof check should succeed"),
        None,
    );
    assert_eq!(
        ThrottleScope::VpnMfaCode
            .attempts(&pool, &legacy_proof_key)
            .await,
        None,
        "a proof without a current challenge is not charged"
    );

    let started = engine
        .step_start(token.clone(), VpnClientMfaMethod::MobileApprove)
        .await
        .expect("mobile attempt should start");
    let challenge = started
        .challenge
        .expect("mobile attempt needs a signature challenge");
    let wrong_signature =
        sign_challenge(&signing_key, &format!("{challenge} is not the challenge"));
    assert_eq!(
        engine
            .check_legacy_mobile_proof(&token, &wrong_signature, &auth_pub_key)
            .await
            .expect("proof check should succeed"),
        None,
    );

    let unowned_signing_key = SigningKey::from_bytes(&[13; 32]);
    let unowned_pub_key = BASE64_STANDARD.encode(unowned_signing_key.verifying_key().to_bytes());
    let unowned_signature = sign_challenge(&unowned_signing_key, &challenge);
    assert_eq!(
        engine
            .check_legacy_mobile_proof(&token, &unowned_signature, &unowned_pub_key)
            .await
            .expect("proof check should succeed"),
        None,
    );
    assert_eq!(
        engine
            .check_legacy_mobile_proof(&token, "", &auth_pub_key)
            .await
            .expect("proof check should succeed"),
        None,
    );
    assert_eq!(
        engine
            .check_legacy_mobile_proof(&token, &wrong_signature, "")
            .await
            .expect("proof check should succeed"),
        None,
    );
    assert_eq!(
        engine
            .check_legacy_mobile_proof("unknown-token", &wrong_signature, &auth_pub_key)
            .await
            .expect("proof check should succeed"),
        None,
    );

    let limit = ThrottleScope::VpnMfaCode.limit();
    assert_eq!(
        ThrottleScope::VpnMfaCode
            .attempts(&pool, &legacy_proof_key)
            .await,
        Some(2),
        "only cryptographically rejected proofs are charged"
    );
    for _ in 2..limit {
        assert_eq!(
            engine
                .check_legacy_mobile_proof(&token, &wrong_signature, &auth_pub_key)
                .await
                .expect("proof check should succeed"),
            None,
        );
    }
    assert!(
        ThrottleScope::VpnMfaCode
            .is_blocked(&pool, &legacy_proof_key)
            .await
            .expect("throttle lookup should succeed")
    );
    let valid_signature = sign_challenge(&signing_key, &challenge);
    assert_eq!(
        engine
            .check_legacy_mobile_proof(&token, &valid_signature, &auth_pub_key)
            .await
            .expect("rate-limited proof check should succeed"),
        None,
        "an exhausted legacy token remains opaque"
    );
    assert_eq!(
        ThrottleScope::VpnMfaCode
            .attempts(&pool, &legacy_proof_key)
            .await,
        Some(limit + 1)
    );

    let unchanged = VpnClientMfaSession::<Id>::find_active_by_token(&pool, &token)
        .await
        .expect("session lookup should succeed")
        .expect("session must remain active");
    assert_eq!(unchanged.failed_attempts, 0);
    let unchanged_attempt = unchanged
        .ephemeral_state
        .as_ref()
        .expect("attempt must remain active")
        .0
        .clone();
    assert_eq!(unchanged_attempt.step_attempt_id, started.step_attempt_id);
    assert!(!unchanged_attempt.mobile_approved);

    sqlx::query("UPDATE vpn_client_mfa_session SET ephemeral_state = NULL WHERE id = $1")
        .bind(session.id)
        .execute(&pool)
        .await
        .expect("failed to clear the MFA attempt");
    assert_eq!(
        engine
            .check_legacy_mobile_proof(&token, &wrong_signature, &auth_pub_key)
            .await
            .expect("proof check should succeed"),
        None,
    );
    let uninitialized = VpnClientMfaSession::<Id>::find_active_by_token(&pool, &token)
        .await
        .expect("session lookup should succeed")
        .expect("session must remain active");
    assert_eq!(uninitialized.failed_attempts, 0);
    assert!(uninitialized.ephemeral_state.is_none());
    assert_eq!(
        ThrottleScope::VpnMfaCode
            .attempts(&pool, &legacy_proof_key)
            .await,
        Some(limit + 1),
        "a session without a current attempt is not charged"
    );
    assert!(event_rx.try_recv().is_err());
    assert!(gateway_rx.try_recv().is_err());
    ThrottleScope::VpnMfaCode
        .end_window(&pool, &legacy_proof_key)
        .await;
}

#[sqlx::test]
async fn test_legacy_mobile_proof_check_rejects_non_mobile_method_without_side_effects(
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
        "Legacy proof non-mobile method",
        vec![vec![VpnClientMfaMethod::Totp]],
        VpnMfaFlowKind::MultiStep,
    )
    .await;
    let (engine, mut event_rx, mut gateway_rx) = make_engine(pool.clone());

    assert_eq!(
        engine
            .check_legacy_mobile_proof(&token, "signature", "key")
            .await
            .expect("proof check should succeed"),
        None,
    );

    let unchanged = VpnClientMfaSession::<Id>::find_active_by_token(&pool, &token)
        .await
        .expect("session lookup should succeed")
        .expect("session must remain active");
    assert_eq!(unchanged.id, session.id);
    assert_eq!(unchanged.failed_attempts, 0);
    assert_eq!(
        unchanged
            .ephemeral_state
            .expect("attempt must remain active")
            .0
            .selected_method,
        VpnClientMfaMethod::Totp
    );
    assert!(event_rx.try_recv().is_err());
    assert!(gateway_rx.try_recv().is_err());
}

#[sqlx::test]
async fn test_legacy_mobile_proof_check_rejects_legacy_flow_without_side_effects(
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
        "Legacy proof legacy flow",
        vec![vec![VpnClientMfaMethod::MobileApprove]],
        VpnMfaFlowKind::Legacy,
    )
    .await;
    let (engine, mut event_rx, mut gateway_rx) = make_engine(pool.clone());

    assert_eq!(
        engine
            .check_legacy_mobile_proof(&token, "signature", "key")
            .await
            .expect("proof check should succeed"),
        None,
    );

    let unchanged = VpnClientMfaSession::<Id>::find_active_by_token(&pool, &token)
        .await
        .expect("session lookup should succeed")
        .expect("legacy session must remain active");
    assert_eq!(unchanged.id, session.id);
    assert_eq!(unchanged.flow_kind, VpnMfaFlowKind::Legacy);
    assert_eq!(unchanged.failed_attempts, 0);
    assert!(
        !unchanged
            .ephemeral_state
            .expect("attempt must remain active")
            .0
            .mobile_approved
    );
    assert!(event_rx.try_recv().is_err());
    assert!(gateway_rx.try_recv().is_err());
}

#[sqlx::test]
async fn test_mobile_approval_rejects_superseded_attempt_without_side_effects(
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
        "Mobile approval stale flow",
        vec![vec![VpnClientMfaMethod::MobileApprove]],
        VpnMfaFlowKind::MultiStep,
    )
    .await;
    let signing_key = SigningKey::from_bytes(&[8; 32]);
    let auth_pub_key = BASE64_STANDARD.encode(signing_key.verifying_key().to_bytes());
    BiometricAuth::new(session.device_id, auth_pub_key.clone())
        .save(&pool)
        .await
        .expect("failed to register mobile authenticator");

    let (engine, mut event_rx, mut gateway_rx) = make_engine(pool.clone());
    let first = engine
        .step_start(token.clone(), VpnClientMfaMethod::MobileApprove)
        .await
        .expect("first mobile attempt should start");
    let second = engine
        .step_start(token.clone(), VpnClientMfaMethod::MobileApprove)
        .await
        .expect("reissued mobile attempt should start");
    let first_challenge = first.challenge.expect("mobile attempt needs a challenge");
    let signature = sign_challenge(&signing_key, &first_challenge);

    let error = engine
        .approve_mobile_step(
            token.clone(),
            super::multi_step::MobileApprovalProof {
                signature,
                auth_pub_key,
                step_attempt_id: first.step_attempt_id,
            },
            test_ip(),
        )
        .await
        .expect_err("a superseded mobile approval must be rejected");
    assert!(matches!(
        error,
        super::multi_step::StepFinishError::StaleAttempt
    ));

    let current = VpnClientMfaSession::<Id>::find_active_by_token(&pool, &token)
        .await
        .expect("session lookup should succeed")
        .expect("the session must remain active");
    assert_eq!(current.failed_attempts, 0);
    assert_eq!(
        current
            .ephemeral_state
            .expect("current attempt must remain")
            .0
            .step_attempt_id,
        second.step_attempt_id
    );
    assert!(
        VpnClientSession::get_all_active_device_sessions_in_location(
            &pool,
            current.location_id,
            current.device_id,
        )
        .await
        .expect("session lookup should succeed")
        .is_empty()
    );
    assert!(event_rx.try_recv().is_err());
    assert!(gateway_rx.try_recv().is_err());
}

#[sqlx::test]
async fn test_mobile_approval_rejection_increments_failure_once(
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
        "Mobile approval rejection flow",
        vec![vec![VpnClientMfaMethod::MobileApprove]],
        VpnMfaFlowKind::MultiStep,
    )
    .await;
    let registered_key = SigningKey::from_bytes(&[9; 32]);
    let invalid_key = SigningKey::from_bytes(&[10; 32]);
    let auth_pub_key = BASE64_STANDARD.encode(registered_key.verifying_key().to_bytes());
    BiometricAuth::new(session.device_id, auth_pub_key.clone())
        .save(&pool)
        .await
        .expect("failed to register mobile authenticator");

    let (engine, mut event_rx, mut gateway_rx) = make_engine(pool.clone());
    let attempt = engine
        .step_start(token.clone(), VpnClientMfaMethod::MobileApprove)
        .await
        .expect("mobile attempt should start");
    let challenge = attempt.challenge.expect("mobile attempt needs a challenge");
    let signature = sign_challenge(&invalid_key, &challenge);

    let error = Status::from(
        engine
            .approve_mobile_step(
                token.clone(),
                super::multi_step::MobileApprovalProof {
                    signature,
                    auth_pub_key,
                    step_attempt_id: attempt.step_attempt_id,
                },
                test_ip(),
            )
            .await
            .expect_err("an invalid mobile signature must be rejected"),
    );
    assert_eq!(error.code(), Code::Unauthenticated);
    assert_eq!(error.message(), "unauthorized");

    let current = VpnClientMfaSession::<Id>::find_active_by_token(&pool, &token)
        .await
        .expect("session lookup should succeed")
        .expect("the session must remain active");
    assert_eq!(current.failed_attempts, 1);
    assert!(
        !current
            .ephemeral_state
            .expect("current attempt must remain")
            .0
            .mobile_approved
    );
    assert!(
        VpnClientSession::get_all_active_device_sessions_in_location(
            &pool,
            current.location_id,
            current.device_id,
        )
        .await
        .expect("session lookup should succeed")
        .is_empty()
    );
    match event_rx
        .try_recv()
        .expect("invalid approval must emit one event")
        .event
    {
        BidiStreamEventType::DesktopClientMfa(event) => {
            assert!(matches!(*event, DesktopClientMfaEvent::Failed { .. }));
        }
        other => panic!("unexpected stream event: {other:?}"),
    }
    assert!(event_rx.try_recv().is_err());
    assert!(gateway_rx.try_recv().is_err());
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
    let session = VpnClientMfaSession::<Id>::find_active_by_token(&pool, &outcome.token)
        .await
        .unwrap()
        .expect("multi-step start must persist a session");
    assert_eq!(session.flow_kind, VpnMfaFlowKind::MultiStep);
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
        step_methods
            .iter()
            .map(VpnClientMfaMethod::ordered_set)
            .collect(),
        VpnMfaFlowKind::MultiStep,
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
            None,
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
        "client_secret".to_owned().into(),
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
        .finish_step(
            start.token.clone(),
            code_proof(start.step_attempt_id.clone(), totp_code(&user)),
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
async fn test_start_legacy_rejects_unlicensed_oidc(_: PgPoolOptions, options: PgConnectOptions) {
    clear_test_license();
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool)
        .await
        .expect("failed to init settings");

    OpenIdProvider::new(
        "Test".to_owned(),
        "https://idp.example.com".to_owned(),
        OpenIdProviderKind::Google,
        "client_id".to_owned(),
        "client_secret".to_owned().into(),
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
    create_and_assign_flow(&pool, location.id, vec![vec![VpnClientMfaMethod::Oidc]]).await;
    let mut user = create_user(&pool).await;
    user.openid_sub = Some("oidc-sub".to_owned());
    user.save(&pool).await.expect("failed to link OIDC user");
    let device = create_device(&pool, user.id).await;
    attach_device_to_location(&pool, location.id, device.id).await;

    let (flow_id, step_methods) = resolve_flow(&pool, location.id, user.id).await;
    let first_step = step_methods
        .into_iter()
        .next()
        .expect("the flow must have a first step");
    let (engine, _event_rx, _gateway_rx) = make_engine(pool.clone());
    let error = engine
        .start_legacy(
            &location,
            &device,
            &user,
            flow_id,
            first_step,
            VpnClientMfaMethod::Oidc,
        )
        .await
        .expect_err("an unlicensed OIDC method must be rejected");

    assert!(matches!(error, StartError::MethodNotInStep));
    assert_eq!(session_count(&pool, location.id, device.id).await, 0);
}

#[sqlx::test]
async fn test_start_legacy_rejects_fido2(_: PgPoolOptions, options: PgConnectOptions) {
    set_test_license_business();
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool)
        .await
        .expect("failed to init settings");

    let location = create_mfa_location(&pool).await;
    create_and_assign_flow(
        &pool,
        location.id,
        vec![vec![VpnClientMfaMethod::Totp, VpnClientMfaMethod::Fido2]],
    )
    .await;
    let user = create_user(&pool).await;
    let device = create_device(&pool, user.id).await;
    attach_device_to_location(&pool, location.id, device.id).await;

    let (flow_id, step_methods) = resolve_flow(&pool, location.id, user.id).await;
    let first_step = step_methods
        .into_iter()
        .next()
        .expect("the flow must have a first step");
    let (engine, _event_rx, _gateway_rx) = make_engine(pool.clone());
    let error = engine
        .start_legacy(
            &location,
            &device,
            &user,
            flow_id,
            first_step,
            VpnClientMfaMethod::Fido2,
        )
        .await
        .expect_err("FIDO2 must be rejected by the legacy contract");

    assert!(matches!(
        error,
        StartError::Initiate(InitiateError::UnsupportedMethod)
    ));
    assert_eq!(session_count(&pool, location.id, device.id).await, 0);
}

#[sqlx::test]
async fn test_fido2_session_rejects_code_credential_before_totp_verification(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool)
        .await
        .expect("failed to init settings");
    let mut user = create_user(&pool).await;
    user.new_totp_secret(&pool)
        .await
        .expect("failed to generate TOTP secret");
    user.enable_totp(&pool)
        .await
        .expect("failed to enable TOTP");
    let (_session, token, _flow) = start_session_with_flow(
        &pool,
        user.id,
        "FIDO2 method selection flow",
        vec![vec![VpnClientMfaMethod::Fido2]],
        VpnMfaFlowKind::MultiStep,
    )
    .await;
    let (engine, _event_rx, _gateway_rx) = make_engine(pool.clone());
    let loaded = engine
        .load_finish_context(&token, VpnMfaFlowKind::MultiStep, test_ip())
        .await
        .expect("FIDO2 session context should load");
    let proof = VerificationProof::Code(totp_code(&user));

    let error = verify(&pool, &loaded.ctx, &loaded.ephemeral, Some(&proof))
        .await
        .expect_err("a TOTP credential must not be verified for a FIDO2 session");
    assert!(matches!(
        error,
        VerifyError::MalformedProof {
            message: "MFA credential does not match the selected method",
            event: None,
        }
    ));
    let session = VpnClientMfaSession::<Id>::find_active_by_token(&pool, &token)
        .await
        .expect("session lookup should succeed")
        .expect("the rejected session must remain active");
    assert_eq!(session.failed_attempts, 0);
}

async fn register_test_fido2_passkey(
    pool: &PgPool,
    user_id: Id,
    username: &str,
) -> (SigningKey, Vec<u8>) {
    let settings = Settings::get_current_settings();
    let webauthn = settings
        .build_webauthn()
        .expect("failed to build WebAuthn configuration");
    let origin = Url::parse(&settings.defguard_url).expect("invalid WebAuthn origin");
    let (challenge, registration_state) = webauthn
        .start_passkey_registration(Uuid::new_v4(), username, username, None)
        .expect("failed to start passkey registration");
    let mut authenticator = WebauthnAuthenticator::new(SoftPasskey::new(true));
    let registration = authenticator
        .do_registration(origin, challenge)
        .expect("failed to register software passkey");
    let passkey = webauthn
        .finish_passkey_registration(&registration, &registration_state)
        .expect("failed to finish passkey registration");

    let signing_key = SigningKey::from_bytes(&[11; 32]);
    let mut serialized_passkey = serde_json::to_value(&passkey).expect("serialize passkey");
    serialized_passkey["cred"]["cred"]["key"] = serde_json::json!({
        "EC_OKP": {
            "curve": "ED25519",
            "x": BASE64_URL_SAFE_NO_PAD.encode(signing_key.verifying_key().as_bytes()),
        }
    });
    let passkey: Passkey = serde_json::from_value(serialized_passkey).expect("deserialize passkey");
    let credential_id = passkey.cred_id().as_ref().to_vec();
    WebAuthn::new(user_id, "registered test key".to_owned(), &passkey)
        .expect("failed to serialize passkey")
        .save(pool)
        .await
        .expect("failed to save passkey");
    (signing_key, credential_id)
}

async fn start_fido2_test_attempt(
    pool: &PgPool,
    user_id: Id,
) -> (
    MfaEngine,
    mpsc::UnboundedReceiver<BidiStreamEvent>,
    mpsc::UnboundedReceiver<GatewayCommand>,
    String,
    String,
) {
    initialize_current_settings(pool)
        .await
        .expect("failed to init settings");
    let (session, token, _) = start_session_with_flow(
        pool,
        user_id,
        "FIDO2 assertion flow",
        vec![vec![VpnClientMfaMethod::Fido2]],
        VpnMfaFlowKind::MultiStep,
    )
    .await;
    let mut conn = pool.acquire().await.expect("failed to acquire connection");
    let attempt_id = session
        .begin_attempt(
            &mut conn,
            VpnClientMfaMethod::Fido2,
            Some(BiometricChallenge::new()),
        )
        .await
        .expect("failed to begin FIDO2 attempt");
    let (engine, event_rx, gateway_rx) = make_engine(pool.clone());
    (engine, event_rx, gateway_rx, token, attempt_id)
}

#[sqlx::test]
async fn test_finish_step_rejects_empty_fido2_credential_id_without_counting(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let user = create_user(&pool).await;
    let (engine, mut event_rx, _gateway_rx, token, attempt_id) =
        start_fido2_test_attempt(&pool, user.id).await;
    let rp_id_hash = vec![1; 32];
    let mut authenticator_data = rp_id_hash.clone();
    authenticator_data.push(2);

    let status = Status::from(
        engine
            .finish_step(
                token.clone(),
                StepProof {
                    step_attempt_id: attempt_id,
                    credential: Some(StepCredential::Fido2(Fido2Assertion {
                        rp_id_hash,
                        authenticator_data,
                        signature: vec![3],
                        credential_id: Vec::new(),
                    })),
                },
                test_ip(),
            )
            .await
            .expect_err("an empty FIDO2 credential ID must be malformed"),
    );
    assert_eq!(status.code(), Code::InvalidArgument);
    assert_eq!(status.message(), "FIDO2 credential ID is empty");

    let session = VpnClientMfaSession::<Id>::find_active_by_token(&pool, &token)
        .await
        .expect("session lookup should succeed")
        .expect("malformed proof must preserve the session");
    assert_eq!(session.failed_attempts, 0);
    assert!(event_rx.try_recv().is_err());
}

#[sqlx::test]
async fn test_finish_step_rejects_empty_fido2_signature_without_counting(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let user = create_user(&pool).await;
    let (engine, mut event_rx, _gateway_rx, token, attempt_id) =
        start_fido2_test_attempt(&pool, user.id).await;
    let rp_id_hash = vec![1; 32];
    let mut authenticator_data = rp_id_hash.clone();
    authenticator_data.push(2);

    let status = Status::from(
        engine
            .finish_step(
                token.clone(),
                StepProof {
                    step_attempt_id: attempt_id,
                    credential: Some(StepCredential::Fido2(Fido2Assertion {
                        rp_id_hash,
                        authenticator_data,
                        signature: Vec::new(),
                        credential_id: vec![4],
                    })),
                },
                test_ip(),
            )
            .await
            .expect_err("an empty FIDO2 signature must be malformed"),
    );
    assert_eq!(status.code(), Code::InvalidArgument);
    assert_eq!(status.message(), "FIDO2 signature is empty");

    let session = VpnClientMfaSession::<Id>::find_active_by_token(&pool, &token)
        .await
        .expect("session lookup should succeed")
        .expect("malformed proof must preserve the session");
    assert_eq!(session.failed_attempts, 0);
    assert!(event_rx.try_recv().is_err());
}

#[sqlx::test]
async fn test_finish_step_unknown_nonempty_fido2_credential_id_stays_counted_to_prevent_enumeration(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool)
        .await
        .expect("failed to init settings");
    let user = create_user(&pool).await;
    let (_signing_key, registered_credential_id) =
        register_test_fido2_passkey(&pool, user.id, &user.username).await;
    let mut unknown_credential_id = registered_credential_id;
    unknown_credential_id.push(0);
    let (engine, mut event_rx, _gateway_rx, token, attempt_id) =
        start_fido2_test_attempt(&pool, user.id).await;
    let rp_id_hash = vec![1; 32];
    let mut authenticator_data = rp_id_hash.clone();
    authenticator_data.push(2);

    let status = Status::from(
        engine
            .finish_step(
                token.clone(),
                StepProof {
                    step_attempt_id: attempt_id,
                    credential: Some(StepCredential::Fido2(Fido2Assertion {
                        rp_id_hash,
                        authenticator_data,
                        signature: vec![3],
                        credential_id: unknown_credential_id,
                    })),
                },
                test_ip(),
            )
            .await
            .expect_err("an unknown non-empty credential ID must fail verification"),
    );
    assert_eq!(status.code(), Code::Unauthenticated);
    assert_eq!(status.message(), "unauthorized");

    let session = VpnClientMfaSession::<Id>::find_active_by_token(&pool, &token)
        .await
        .expect("session lookup should succeed")
        .expect("verification failure must preserve the session");
    assert_eq!(session.failed_attempts, 1);
    match event_rx
        .try_recv()
        .expect("verification failure must emit a failed event")
        .event
    {
        BidiStreamEventType::DesktopClientMfa(event) => {
            assert!(matches!(*event, DesktopClientMfaEvent::Failed { .. }));
        }
        other => panic!("unexpected stream event: {other:?}"),
    }
    assert!(event_rx.try_recv().is_err());
}

#[sqlx::test]
async fn test_finish_step_completes_with_valid_fido2_assertion(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool)
        .await
        .expect("failed to init settings");
    let user = create_user(&pool).await;
    let (signing_key, credential_id) =
        register_test_fido2_passkey(&pool, user.id, &user.username).await;
    let (engine, _event_rx, _gateway_rx, token, attempt_id) =
        start_fido2_test_attempt(&pool, user.id).await;
    let session = VpnClientMfaSession::<Id>::find_active_by_token(&pool, &token)
        .await
        .expect("session lookup should succeed")
        .expect("FIDO2 attempt should persist");
    let challenge = session
        .ephemeral_state
        .as_ref()
        .expect("FIDO2 attempt should have ephemeral state")
        .0
        .biometric_challenge
        .as_ref()
        .expect("FIDO2 attempt should have a challenge")
        .challenge
        .clone();
    let rp_id = Settings::get_current_settings()
        .webauthn_rp_id()
        .expect("RP ID should be configured");
    let rp_id_hash = Sha256::digest(rp_id.as_bytes()).to_vec();
    let mut authenticator_data = rp_id_hash.clone();
    authenticator_data.push(0x05);
    authenticator_data.extend_from_slice(&1_u32.to_be_bytes());
    let mut signed_data = authenticator_data.clone();
    signed_data.extend_from_slice(&Sha256::digest(challenge.as_bytes()));
    let signature = signing_key.sign(&signed_data).to_bytes().to_vec();

    let outcome = engine
        .finish_step(
            token.clone(),
            StepProof {
                step_attempt_id: attempt_id,
                credential: Some(StepCredential::Fido2(Fido2Assertion {
                    rp_id_hash,
                    authenticator_data,
                    signature,
                    credential_id,
                })),
            },
            test_ip(),
        )
        .await
        .expect("valid FIDO2 assertion should complete the step");
    assert!(matches!(outcome, FinishOutcome::Completed { .. }));
    assert!(
        VpnClientMfaSession::<Id>::find_active_by_token(&pool, &token)
            .await
            .expect("session lookup should succeed")
            .is_none(),
        "a completed flow should remove its MFA session"
    );
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
    // The unconfigured Email step should be the only rejection.
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
        VpnMfaFlowKind::MultiStep,
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
    flow_kind: VpnMfaFlowKind,
) -> (VpnClientMfaSession<Id>, String, MfaFlow<Id>) {
    let location = create_mfa_location(pool).await;
    let device = create_device(pool, user_id).await;
    attach_device_to_location(pool, location.id, device.id).await;
    start_session_with_context(
        pool,
        location.id,
        device.id,
        user_id,
        title,
        steps,
        flow_kind,
    )
    .await
}

async fn start_session_with_context(
    pool: &PgPool,
    location_id: Id,
    device_id: Id,
    user_id: Id,
    title: &str,
    steps: Vec<Vec<VpnClientMfaMethod>>,
    flow_kind: VpnMfaFlowKind,
) -> (VpnClientMfaSession<Id>, String, MfaFlow<Id>) {
    let mut tx = pool.begin().await.unwrap();
    let (flow, _) = MfaFlow::create(&mut tx, title.to_owned(), steps.clone())
        .await
        .unwrap();
    let (session, outcome) = VpnClientMfaSession::<Id>::start(
        &mut tx,
        location_id,
        device_id,
        user_id,
        flow.id,
        steps.clone(),
        flow_kind,
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
            None,
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
            .finish_step(
                "missing-token".to_owned(),
                code_proof("attempt", "000000"),
                test_ip(),
            )
            .await
            .expect_err("missing session must be rejected"),
    );
    assert_eq!(status.code(), Code::InvalidArgument);
    assert_eq!(status.message(), "login session not found");
}

#[sqlx::test]
async fn test_engine_rejects_cross_contract_calls_without_side_effects(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool)
        .await
        .expect("failed to init settings");
    let user = create_user(&pool).await;
    let (legacy_session, legacy_token, _) = start_session_with_flow(
        &pool,
        user.id,
        "Cross-contract legacy flow",
        vec![vec![VpnClientMfaMethod::Totp]],
        VpnMfaFlowKind::Legacy,
    )
    .await;
    let legacy_attempt = legacy_session
        .ephemeral_state
        .as_ref()
        .expect("legacy session must have an attempt")
        .step_attempt_id
        .clone();
    let (engine, mut event_rx, _gateway_rx) = make_engine(pool.clone());

    assert!(matches!(
        engine
            .step_start(legacy_token.clone(), VpnClientMfaMethod::Totp)
            .await
            .expect_err("multi-step start must reject a legacy session"),
        StepError::SessionNotFound
    ));
    assert!(matches!(
        engine
            .finish_step(
                legacy_token.clone(),
                code_proof(legacy_attempt.clone(), "000000"),
                test_ip(),
            )
            .await
            .expect_err("multi-step finish must reject a legacy session"),
        super::multi_step::StepFinishError::SessionNotFound
    ));
    assert!(matches!(
        engine
            .approve_mobile_step(
                legacy_token.clone(),
                super::multi_step::MobileApprovalProof {
                    signature: "signature".to_owned(),
                    auth_pub_key: "key".to_owned(),
                    step_attempt_id: legacy_attempt,
                },
                test_ip(),
            )
            .await
            .expect_err("multi-step approval must reject a legacy session"),
        super::multi_step::StepFinishError::SessionNotFound
    ));

    let legacy_session = VpnClientMfaSession::<Id>::find_active_by_token(&pool, &legacy_token)
        .await
        .expect("legacy session lookup must succeed")
        .expect("legacy session must survive cross-contract calls");
    assert_eq!(legacy_session.failed_attempts, 0);
    assert_eq!(legacy_session.current_step, 0);
    assert!(event_rx.try_recv().is_err());

    let location_id = legacy_session.location_id;
    let device_id = legacy_session.device_id;
    let mut tx = pool.begin().await.expect("transaction must start");
    legacy_session
        .delete(&mut *tx)
        .await
        .expect("legacy session cleanup must succeed");
    tx.commit()
        .await
        .expect("legacy session cleanup must commit");
    let (_multi_session, multi_token, _) = start_session_with_context(
        &pool,
        location_id,
        device_id,
        user.id,
        "Cross-contract multi-step flow",
        vec![vec![VpnClientMfaMethod::Totp]],
        VpnMfaFlowKind::MultiStep,
    )
    .await;
    let err = engine
        .finish_legacy(
            multi_token.clone(),
            LegacyProof {
                code: Some("000000".to_owned()),
                auth_pub_key: None,
            },
            test_ip(),
        )
        .await
        .expect_err("legacy finish must reject a multi-step session");
    assert!(matches!(err, FinishError::SessionNotFound));
    let multi_session = VpnClientMfaSession::<Id>::find_active_by_token(&pool, &multi_token)
        .await
        .expect("multi-step session lookup must succeed")
        .expect("multi-step session must survive cross-contract calls");
    assert_eq!(multi_session.failed_attempts, 0);
    assert_eq!(multi_session.current_step, 0);
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

fn code_proof(step_attempt_id: impl Into<String>, code: impl Into<String>) -> StepProof {
    StepProof {
        step_attempt_id: step_attempt_id.into(),
        credential: Some(StepCredential::Code(code.into())),
    }
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
    let first_attempt_id = outcome.step_attempt_id;

    // The first TOTP proof advances without authorizing the peer.
    let outcome = engine
        .finish_step(
            token.clone(),
            code_proof(first_attempt_id, totp_code(&user)),
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

    let second_attempt = engine
        .step_start(token.clone(), VpnClientMfaMethod::Email)
        .await
        .expect("step_start should succeed");
    let outcome = engine
        .finish_step(
            token.clone(),
            code_proof(second_attempt.step_attempt_id, email_code(&user)),
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
    let first_attempt_id = outcome.step_attempt_id;

    let code = totp_code(&user);
    let outcome = engine
        .finish_step(
            token.clone(),
            code_proof(first_attempt_id.clone(), code.clone()),
            test_ip(),
        )
        .await
        .expect("finish of step 0 should succeed");
    assert_eq!(outcome, FinishOutcome::Advanced { next_step: 1 });

    // Replay the very same proof. It must not complete the flow.
    let err = engine
        .finish_step(token.clone(), code_proof(first_attempt_id, code), test_ip())
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
    let first_attempt_id = outcome.step_attempt_id;

    engine
        .finish_step(
            token.clone(),
            code_proof(first_attempt_id, totp_code(&user)),
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
        .finish_step(
            token.clone(),
            code_proof(first.step_attempt_id, email_code(&user)),
            test_ip(),
        )
        .await
        .expect_err("a superseded attempt id must be rejected");
    let err = Status::from(err);
    assert_eq!(err.code(), Code::InvalidArgument);
    assert_eq!(err.message(), "stale MFA attempt");

    // The current attempt still works, so the guard rejects staleness, not the method.
    let outcome = engine
        .finish_step(
            token,
            code_proof(second.step_attempt_id, email_code(&user)),
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
        VpnMfaFlowKind::MultiStep,
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
                .finish_step(
                    token.clone(),
                    code_proof(attempt_id.clone(), "000000"),
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
            .finish_step(token.clone(), code_proof(attempt_id, "000000"), test_ip())
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
            .finish_step(token, code_proof("attempt", "000000"), test_ip())
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
        VpnMfaFlowKind::MultiStep,
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
                .finish_step(
                    token.clone(),
                    code_proof(email_attempt.step_attempt_id.clone(), "000000"),
                    test_ip(),
                )
                .await
                .expect_err("a wrong code must be rejected"),
        );
        assert_eq!(err.code(), Code::Unauthenticated);
    }
    let err = Status::from(
        engine
            .finish_step(
                token,
                code_proof(email_attempt.step_attempt_id, "000000"),
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
async fn test_finish_legacy_cap_deletes_session_and_emits_abort(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool)
        .await
        .expect("failed to init settings");

    let user = create_user(&pool).await;
    let (session, token, flow) = start_session_with_flow(
        &pool,
        user.id,
        "Legacy cap attribution flow",
        vec![
            vec![VpnClientMfaMethod::Totp],
            vec![VpnClientMfaMethod::Email],
        ],
        VpnMfaFlowKind::Legacy,
    )
    .await;

    let (engine, mut event_rx, _gateway_rx) = make_engine(pool.clone());
    for _ in 0..MFA_FAILED_ATTEMPT_CAP {
        let err = engine
            .finish_legacy(
                token.clone(),
                LegacyProof {
                    code: Some("000000".to_owned()),
                    auth_pub_key: None,
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
        Some("Legacy cap attribution flow")
    );
    assert_eq!(attribution.snapshot, session.steps_snapshot.0);
    assert!(
        event_rx.try_recv().is_err(),
        "only one abort must be emitted"
    );
}

async fn finish_with_code(
    engine: &MfaEngine,
    token: &str,
    step_attempt_id: &str,
    code: String,
) -> Result<FinishOutcome, StepFinishError> {
    engine
        .finish_step(
            token.to_owned(),
            StepProof {
                step_attempt_id: step_attempt_id.to_owned(),
                credential: Some(StepCredential::Code(code)),
            },
            test_ip(),
        )
        .await
}

async fn start_multi_step_totp(
    engine: &MfaEngine,
    location: &WireguardNetwork<Id>,
    device: &Device<Id>,
    user: &User<Id>,
    flow_id: Id,
    steps: &[HashSet<VpnClientMfaMethod>],
) -> Result<super::types::MultiStepStartOutcome, StartError> {
    match engine
        .start_multi_step(
            location,
            device,
            user,
            flow_id,
            steps.to_vec(),
            vec![VpnClientMfaMethod::Totp],
        )
        .await?
    {
        StartResult::Accepted(outcome) => Ok(outcome),
        StartResult::Rejected(_) => panic!("TOTP plan was unexpectedly rejected"),
    }
}

/// Regression test for DefGuard/defguard#3571
#[sqlx::test]
async fn test_code_throttle_survives_new_sessions_and_concurrent_proofs(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool)
        .await
        .expect("failed to init settings");

    let location = create_mfa_location(&pool).await;
    create_and_assign_flow(&pool, location.id, vec![vec![VpnClientMfaMethod::Totp]]).await;
    let mut user = create_user(&pool).await;
    user.new_totp_secret(&pool).await.expect("new_totp_secret");
    user.enable_totp(&pool).await.expect("enable_totp");
    let device = create_device(&pool, user.id).await;
    attach_device_to_location(&pool, location.id, device.id).await;
    let key = super::throttle_key(location.id, device.id);

    let (flow_id, steps) = resolve_flow(&pool, location.id, user.id).await;
    let (engine, mut event_rx, _gateway_rx) = make_engine(pool.clone());
    let limit = ThrottleScope::VpnMfaCode.limit();
    let start = || start_multi_step_totp(&engine, &location, &device, &user, flow_id, &steps);

    let started = start().await.expect("start should succeed");
    join_all((0..2 * limit).map(|_| {
        finish_with_code(
            &engine,
            &started.token,
            &started.step_attempt_id,
            "000000".into(),
        )
    }))
    .await;
    let mut failed_events = 0;
    while let Ok(event) = event_rx.try_recv() {
        if let BidiStreamEventType::DesktopClientMfa(event) = event.event
            && matches!(*event, DesktopClientMfaEvent::Failed { .. })
        {
            failed_events += 1;
        }
    }
    assert!(
        failed_events <= limit,
        "{failed_events} guesses were verified, the limit is {limit}"
    );
    ThrottleScope::VpnMfaCode.end_window(&pool, &key).await;

    // New sessions keep the count: 5 + 4 + 1 wrong codes use up the limit of 10.
    for wrong_codes in [5, 4, 1] {
        let started = start().await.expect("start should succeed");
        for _ in 0..wrong_codes {
            let err = finish_with_code(
                &engine,
                &started.token,
                &started.step_attempt_id,
                "000000".into(),
            )
            .await
            .expect_err("a wrong code must be rejected");
            assert!(matches!(
                err,
                StepFinishError::Unauthorized | StepFinishError::AttemptLimit
            ));
        }
        if wrong_codes == 1 {
            let err = Status::from(
                finish_with_code(
                    &engine,
                    &started.token,
                    &started.step_attempt_id,
                    totp_code(&user),
                )
                .await
                .expect_err("the throttle must refuse the attempt"),
            );
            assert_eq!(err.code(), Code::PermissionDenied);
        }
    }
    let err = Status::from(
        start()
            .await
            .expect_err("a start with a used-up limit must be refused"),
    );
    assert_eq!(err.code(), Code::FailedPrecondition);
    assert_eq!(
        err.message(),
        "Too many failed MFA attempts. Try again later."
    );

    // The correct code is refunded, so it leaves no charged attempt.
    ThrottleScope::VpnMfaCode.end_window(&pool, &key).await;
    let started = start().await.expect("start should succeed");
    let outcome = finish_with_code(
        &engine,
        &started.token,
        &started.step_attempt_id,
        totp_code(&user),
    )
    .await
    .expect("the correct code must complete the flow");
    assert!(matches!(outcome, FinishOutcome::Completed { .. }));
    assert_eq!(
        ThrottleScope::VpnMfaCode.attempts(&pool, &key).await,
        Some(0)
    );
}

#[test]
fn test_poll_windows_limit_each_session_per_window() {
    let start = Instant::now();
    let mut polls = PollWindows::new(start);
    for _ in 0..POLL_LIMIT {
        assert!(polls.hit("a", start));
    }
    assert!(!polls.hit("a", start));
    assert!(polls.hit("b", start), "another session keeps its own count");

    let next_window = start + POLL_WINDOW;
    assert!(
        polls.hit("a", next_window),
        "a new window starts the count again"
    );
    assert_eq!(
        polls.windows.len(),
        1,
        "the prune drops the ended window of b"
    );
}

/// Regression test for DefGuard/defguard#3585: a client could poll `finish` with no limit.
#[sqlx::test]
async fn test_finish_poll_throttle_allows_approved_mobile_completion(
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
        "Poll flow",
        vec![vec![VpnClientMfaMethod::MobileApprove]],
        VpnMfaFlowKind::MultiStep,
    )
    .await;
    let attempt_id = session
        .ephemeral_state
        .as_ref()
        .expect("session must have an initial attempt")
        .step_attempt_id
        .clone();
    let (engine, _event_rx, _gateway_rx) = make_engine(pool.clone());
    let poll = || {
        engine.finish_step(
            token.clone(),
            StepProof {
                step_attempt_id: attempt_id.clone(),
                credential: None,
            },
            test_ip(),
        )
    };

    for _ in 0..POLL_LIMIT {
        let outcome = poll().await.expect("a poll must succeed");
        assert_eq!(outcome, FinishOutcome::AwaitingExternal);
    }
    assert_eq!(
        poll()
            .await
            .expect("an unapproved poll past the limit must answer"),
        FinishOutcome::AwaitingExternal
    );

    // A stored approval bypasses the client polling limit so the flow can complete.
    let mut conn = pool.acquire().await.unwrap();
    assert!(
        session
            .mark_mobile_approved(&mut conn, &attempt_id, None)
            .await
            .unwrap()
    );
    let outcome = poll().await.expect("approved mobile flow should complete");
    assert!(matches!(outcome, FinishOutcome::Completed { .. }));
}

/// Regression test for DefGuard/defguard#3585: `step_start` could send email codes with no limit.
#[sqlx::test]
async fn test_initiate_throttle_bounds_starts_and_step_starts(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool)
        .await
        .expect("failed to init settings");

    let location = create_mfa_location(&pool).await;
    create_and_assign_flow(&pool, location.id, vec![vec![VpnClientMfaMethod::Totp]]).await;
    let mut user = create_user(&pool).await;
    user.new_totp_secret(&pool).await.expect("new_totp_secret");
    user.enable_totp(&pool).await.expect("enable_totp");
    let device = create_device(&pool, user.id).await;
    attach_device_to_location(&pool, location.id, device.id).await;

    let (flow_id, steps) = resolve_flow(&pool, location.id, user.id).await;
    let (engine, _event_rx, _gateway_rx) = make_engine(pool.clone());
    let start = || start_multi_step_totp(&engine, &location, &device, &user, flow_id, &steps);

    // Starts and step starts share one count: limit - 1 starts and one step start use it up.
    let limit = ThrottleScope::VpnMfaInitiate.limit();
    for _ in 0..limit - 2 {
        start().await.expect("start should succeed");
    }
    let token = start().await.expect("start should succeed").token;
    engine
        .step_start(token.clone(), VpnClientMfaMethod::Totp)
        .await
        .expect("step start should succeed");

    let err = Status::from(
        engine
            .step_start(token, VpnClientMfaMethod::Totp)
            .await
            .expect_err("the step start must be refused"),
    );
    assert_eq!(err.code(), Code::FailedPrecondition);
    assert_eq!(err.message(), "Too many MFA requests. Try again later.");
    let err = Status::from(start().await.expect_err("the start must be refused"));
    assert_eq!(err.code(), Code::FailedPrecondition);
    assert_eq!(err.message(), "Too many MFA requests. Try again later.");
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
        .finish_step(token, code_proof("attempt", "000000"), test_ip())
        .await
        .expect_err("finish on an uninitialized step must be rejected");
    let err = Status::from(err);
    assert_eq!(err.code(), Code::InvalidArgument);
    assert_eq!(err.message(), "no MFA attempt in progress");
}
