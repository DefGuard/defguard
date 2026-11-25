use std::mem::discriminant;

use defguard_common::db::{
    Id,
    models::{
        DeviceState,
        device_verification::{
            DEVICE_VERIFICATION_CODE_DIGITS, DEVICE_VERIFICATION_CODE_PERIOD, DeviceVerification,
            DeviceVerificationOutcome,
        },
    },
};
use defguard_proto::{
    client_common::{
        self, DeviceVerificationStartResponse, DeviceVerificationStatus, TotpAlgorithm,
    },
    proxy::{CoreErrorCode, CoreRequest, CoreResponse, core_request, core_response},
};
use sqlx::{
    PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
    query,
};
use tonic::Code;

use super::support::{
    assert_error_response_full, clear_test_license, complete_proxy_handshake, create_polling_token,
    create_user, create_user_with_device, set_test_license_business, set_test_license_enterprise,
};
use crate::tests::common::HandlerTestContext;

fn error_codes(response: &CoreResponse) -> (Code, CoreErrorCode) {
    let (code, _, error_code) = assert_error_response_full(response);
    (code, error_code)
}

async fn send_start(context: &mut HandlerTestContext, token: &str) -> CoreResponse {
    context.mock_proxy().send_request(CoreRequest {
        id: 40,
        device_info: None,
        payload: Some(core_request::Payload::DeviceVerificationStart(
            client_common::DeviceVerificationStartRequest {
                token: token.to_owned(),
            },
        )),
    });
    context.mock_proxy_mut().recv_outbound().await
}

async fn start(context: &mut HandlerTestContext, token: &str) -> DeviceVerificationStartResponse {
    match send_start(context, token).await.payload {
        Some(core_response::Payload::DeviceVerificationStart(response)) => response,
        other => panic!(
            "expected DeviceVerificationStart response, got: {:?}",
            other.as_ref().map(discriminant)
        ),
    }
}

async fn send_status(
    context: &mut HandlerTestContext,
    token: &str,
    session_id: Id,
) -> CoreResponse {
    context.mock_proxy().send_request(CoreRequest {
        id: 41,
        device_info: None,
        payload: Some(core_request::Payload::DeviceVerificationStatus(
            client_common::DeviceVerificationStatusRequest {
                token: token.to_owned(),
                session_id,
            },
        )),
    });
    let response = context.mock_proxy_mut().recv_outbound().await;
    assert_eq!(response.id, 41, "response must answer the status request");
    response
}

async fn status(
    context: &mut HandlerTestContext,
    token: &str,
    session_id: Id,
) -> DeviceVerificationStatus {
    match send_status(context, token, session_id).await.payload {
        Some(core_response::Payload::DeviceVerificationStatus(response)) => response,
        other => panic!(
            "expected DeviceVerificationStatus response, got: {:?}",
            other.as_ref().map(discriminant)
        ),
    }
}

async fn resolve(pool: &PgPool, device_id: Id, outcome: DeviceVerificationOutcome) {
    DeviceVerification::resolve_active_for_device(pool, device_id, outcome)
        .await
        .expect("failed to resolve verification")
        .expect("no active verification");
}

#[sqlx::test]
async fn test_verification_start_creates_and_joins_session(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;
    set_test_license_enterprise();

    let (_user, device) = create_user_with_device(&context.pool).await;
    let token = create_polling_token(&context.pool, device.id).await;

    let first = start(&mut context, &token).await;
    let stored = DeviceVerification::find_active_for_device(&context.pool, device.id)
        .await
        .unwrap()
        .expect("client start should store a verification");
    assert_eq!(first.session_id, stored.id);
    assert_eq!(first.secret, stored.secret);
    assert_eq!(stored.started_by, None);
    assert_eq!(first.algorithm(), TotpAlgorithm::Sha1);
    assert_eq!(first.digits, DEVICE_VERIFICATION_CODE_DIGITS);
    assert_eq!(u64::from(first.period), DEVICE_VERIFICATION_CODE_PERIOD);
    assert_eq!(first.expires_at, stored.expires_at.and_utc().timestamp());

    let second = start(&mut context, &token).await;
    assert_eq!(second.session_id, first.session_id);
    assert_eq!(second.secret, first.secret);

    clear_test_license();
    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_verification_start_joins_admin_session(_: PgPoolOptions, options: PgConnectOptions) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;
    set_test_license_enterprise();

    let admin = create_user(&context.pool).await;
    let (_user, device) = create_user_with_device(&context.pool).await;
    let token = create_polling_token(&context.pool, device.id).await;
    let mut conn = context.pool.acquire().await.unwrap();
    let (admin_session, _) = DeviceVerification::new(device.id, Some(admin.id))
        .start_or_get(&mut conn)
        .await
        .unwrap();
    drop(conn);

    let response = start(&mut context, &token).await;
    assert_eq!(response.session_id, admin_session.id);
    assert_eq!(response.secret, admin_session.secret);

    clear_test_license();
    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_verification_status_reports_decisions(_: PgPoolOptions, options: PgConnectOptions) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;
    set_test_license_enterprise();

    let (_user, mut device) = create_user_with_device(&context.pool).await;
    let token = create_polling_token(&context.pool, device.id).await;

    let mut first_session = None;
    let mut blocked_session = None;
    for (outcome, expected) in [
        (
            DeviceVerificationOutcome::Cancelled,
            client_common::DeviceVerificationOutcome::Cancelled,
        ),
        (
            DeviceVerificationOutcome::Verified,
            client_common::DeviceVerificationOutcome::Verified,
        ),
        (
            DeviceVerificationOutcome::Blocked,
            client_common::DeviceVerificationOutcome::Blocked,
        ),
    ] {
        let session = start(&mut context, &token).await;
        first_session.get_or_insert(session.session_id);
        blocked_session = Some(session.session_id);

        let pending = status(&mut context, &token, session.session_id).await;
        assert_eq!(pending.session_id, session.session_id);
        assert_eq!(
            pending.outcome(),
            client_common::DeviceVerificationOutcome::Pending
        );
        assert_eq!(pending.expires_at, session.expires_at);
        assert_eq!(pending.resolved_at, None);

        resolve(&context.pool, device.id, outcome).await;
        let resolved = status(&mut context, &token, session.session_id).await;
        assert_eq!(resolved.outcome(), expected);
        assert!(resolved.resolved_at.is_some());
    }

    assert_eq!(
        error_codes(&send_status(&mut context, &token, first_session.unwrap()).await),
        (Code::NotFound, CoreErrorCode::VerificationSessionNotFound),
        "a replaced session should be gone"
    );

    device.state = DeviceState::Blocked;
    device.save(&context.pool).await.unwrap();
    assert_eq!(
        error_codes(&send_start(&mut context, &token).await),
        (Code::PermissionDenied, CoreErrorCode::DeviceBlocked),
        "a blocked device cannot start"
    );
    assert_eq!(
        status(&mut context, &token, blocked_session.unwrap())
            .await
            .outcome(),
        client_common::DeviceVerificationOutcome::Blocked,
        "a blocked device still learns the outcome"
    );

    clear_test_license();
    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_verification_status_reports_expiry(_: PgPoolOptions, options: PgConnectOptions) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;
    set_test_license_enterprise();

    let (_user, device) = create_user_with_device(&context.pool).await;
    let token = create_polling_token(&context.pool, device.id).await;
    let session = start(&mut context, &token).await;
    query(
        "UPDATE device_verification \
        SET expires_at = (now() AT TIME ZONE 'UTC') - interval '1 second' WHERE device_id = $1",
    )
    .bind(device.id)
    .execute(&context.pool)
    .await
    .unwrap();

    let response = status(&mut context, &token, session.session_id).await;
    assert_eq!(
        response.outcome(),
        client_common::DeviceVerificationOutcome::Expired
    );
    assert_eq!(response.resolved_at, None);

    clear_test_license();
    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_verification_rejects_other_devices_session(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;
    set_test_license_enterprise();

    let (_user, device) = create_user_with_device(&context.pool).await;
    let (_other_user, other_device) = create_user_with_device(&context.pool).await;
    let token = create_polling_token(&context.pool, device.id).await;
    let other_token = create_polling_token(&context.pool, other_device.id).await;
    let session = start(&mut context, &token).await;

    assert_eq!(
        error_codes(&send_status(&mut context, &other_token, session.session_id).await),
        (Code::NotFound, CoreErrorCode::VerificationSessionNotFound)
    );

    clear_test_license();
    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_verification_rejects_invalid_callers(_: PgPoolOptions, options: PgConnectOptions) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;
    set_test_license_enterprise();

    assert_eq!(
        error_codes(&send_start(&mut context, "no-such-token").await),
        (Code::Unauthenticated, CoreErrorCode::DeviceTokenInvalid)
    );
    assert_eq!(
        error_codes(&send_start(&mut context, "").await),
        (Code::Unauthenticated, CoreErrorCode::DeviceTokenInvalid)
    );

    let (mut user, device) = create_user_with_device(&context.pool).await;
    let token = create_polling_token(&context.pool, device.id).await;
    user.is_active = false;
    user.save(&context.pool).await.unwrap();
    assert_eq!(
        error_codes(&send_start(&mut context, &token).await),
        (Code::PermissionDenied, CoreErrorCode::UserInactive)
    );

    set_test_license_business();
    assert_eq!(
        error_codes(&send_start(&mut context, &token).await),
        (Code::FailedPrecondition, CoreErrorCode::LicenseRequired),
        "device verification requires the Enterprise tier"
    );

    clear_test_license();
    let response = send_start(&mut context, &token).await;
    let (code, message, error_code) = assert_error_response_full(&response);
    assert_eq!(code, Code::FailedPrecondition);
    assert_eq!(error_code, CoreErrorCode::LicenseRequired);
    assert_eq!(message, "no valid license", "Edge maps this message to 402");

    context.finish().await.expect_server_finished().await;
}
