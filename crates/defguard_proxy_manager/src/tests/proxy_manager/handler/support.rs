use std::sync::atomic::{AtomicU64, Ordering};

use defguard_common::db::{
    Id,
    models::{
        Device, DeviceType, User, WireguardNetwork,
        polling_token::PollingToken,
        wireguard::{LocationMfaMode, ServiceLocationMode},
    },
};
use defguard_core::{
    db::models::enrollment::{ENROLLMENT_TOKEN_TYPE, Token},
    enterprise::license::{License, LicenseTier, set_cached_license},
};
use defguard_proto::proxy::{
    CoreRequest, CoreResponse, DeviceConfigResponse, DeviceInfo, EnrollmentStartRequest,
    core_request, core_response,
};
use sqlx::PgPool;
use ipnetwork::IpNetwork;

use crate::tests::common::HandlerTestContext;

// ---------------------------------------------------------------------------
// Per-module counters (separate from the global TEST_ID in common/mod.rs)
// ---------------------------------------------------------------------------

static USER_CTR: AtomicU64 = AtomicU64::new(0);
static NET_CTR: AtomicU64 = AtomicU64::new(0);
static DEV_CTR: AtomicU64 = AtomicU64::new(0);

// ---------------------------------------------------------------------------
// Assertion helpers
// ---------------------------------------------------------------------------

pub(crate) fn assert_initial_info_received(response: &CoreResponse) {
    assert!(
        matches!(
            response.payload,
            Some(core_response::Payload::InitialInfo(_))
        ),
        "expected InitialInfo as first response from handler, got: {:?}",
        response.payload.as_ref().map(|p| std::mem::discriminant(p))
    );
}

/// Consume the `InitialInfo` message that the handler sends immediately after
/// establishing the bidi stream.  Most lifecycle tests call this before
/// injecting any business messages.
pub(crate) async fn complete_proxy_handshake(context: &mut HandlerTestContext) {
    let response = context.mock_proxy_mut().recv_outbound().await;
    assert_initial_info_received(&response);
}

/// Assert that a `CoreResponse` carries a `DeviceConfig` payload and return a
/// reference to it.
pub(crate) fn assert_device_config_response(response: &CoreResponse) -> &DeviceConfigResponse {
    match &response.payload {
        Some(core_response::Payload::DeviceConfig(cfg)) => cfg,
        other => panic!(
            "expected DeviceConfig response, got: {:?}",
            other.as_ref().map(|p| std::mem::discriminant(p))
        ),
    }
}

/// Assert that a `CoreResponse` carries a `CoreError` payload and return the
/// tonic status code.
pub(crate) fn assert_error_response(response: &CoreResponse) -> tonic::Code {
    match &response.payload {
        Some(core_response::Payload::CoreError(err)) => tonic::Code::from_i32(err.status_code),
        other => panic!(
            "expected CoreError response, got: {:?}",
            other.as_ref().map(|p| std::mem::discriminant(p))
        ),
    }
}

// ---------------------------------------------------------------------------
// License helpers
// ---------------------------------------------------------------------------

/// Install a Business-tier license into the global cache for the duration of a
/// test.
pub(crate) fn set_test_license_business() {
    let license = License {
        customer_id: "test-customer-id".into(),
        subscription: false,
        valid_until: None,
        limits: None,
        version_date_limit: None,
        tier: LicenseTier::Business,
    };
    set_cached_license(Some(license));
}

/// Remove the global license (so tests that require no license can clear one
/// that was previously set).
pub(crate) fn clear_test_license() {
    set_cached_license(None);
}

// ---------------------------------------------------------------------------
// Misc. helpers
// ---------------------------------------------------------------------------

/// Return a minimal `DeviceInfo` suitable for test requests.
///
/// `parse_client_ip_agent` in the enrollment server requires a non-`None`
/// `DeviceInfo` with a valid IP address.  Tests that call `NewDevice` or
/// `ExistingDevice` must pass this instead of `None`.
pub(crate) fn make_device_info() -> DeviceInfo {
    DeviceInfo {
        ip_address: "127.0.0.1".to_string(),
        user_agent: Some("test-client/1.0".to_string()),
        version: None,
        platform: None,
    }
}

// ---------------------------------------------------------------------------
// Factory helpers — Users
// ---------------------------------------------------------------------------

/// Insert a test user, returning the saved `User<Id>`.
pub(crate) async fn create_user(pool: &PgPool) -> User<Id> {
    let n = USER_CTR.fetch_add(1, Ordering::Relaxed);
    let username = format!("test-user-{n}");
    User::new(
        username.clone(),
        None,
        "Test".to_string(),
        "User".to_string(),
        format!("{username}@test.example"),
        None,
    )
    .save(pool)
    .await
    .expect("failed to save test user")
}

// ---------------------------------------------------------------------------
// Factory helpers — Networks
// ---------------------------------------------------------------------------

/// Insert a minimal WireGuard network, returning the saved `WireguardNetwork<Id>`.
pub(crate) async fn create_network(pool: &PgPool) -> WireguardNetwork<Id> {
    let n = NET_CTR.fetch_add(1, Ordering::Relaxed);
    WireguardNetwork::new(
        format!("test-network-{n}"),
        51820 + i32::try_from(n % 10_000).unwrap(),
        "10.0.0.1".to_string(),
        None,
        Vec::<IpNetwork>::new(),
        true,  // allow_all_groups
        false, // acl_enabled
        false, // acl_default_allow
        LocationMfaMode::default(),
        ServiceLocationMode::default(),
    )
    .try_set_address("10.0.0.1/24")
    .expect("failed to set network address")
    .save(pool)
    .await
    .expect("failed to save test wireguard network")
}

// ---------------------------------------------------------------------------
// Factory helpers — Devices
// ---------------------------------------------------------------------------

/// Pre-generated valid 32-byte WireGuard public keys (base64, 44 chars each).
/// Used by `create_device_for_user` so that `Device::validate_pubkey` passes.
static DEVICE_PUBKEYS: &[&str] = &[
    "HCk2Q1BdaneEkZ6ruMXS3+z5BhMgLTpHVGFue4iVoq8=",
    "IzA9SldkcX6LmKWyv8zZ5vMADRonNEFOW2h1go+cqbY=",
    "KjdEUV5reIWSn6y5xtPg7foHFCEuO0hVYm98iZajsL0=",
    "MT5LWGVyf4yZprPAzdrn9AEOGyg1Qk9caXaDkJ2qt8Q=",
    "OEVSX2x5hpOgrbrH1OHu+wgVIi88SVZjcH2Kl6Sxvss=",
    "P0xZZnOAjZqntMHO2+j1Ag8cKTZDUF1qd4SRnqu4xdI=",
    "RlNgbXqHlKGuu8jV4u/8CRYjMD1KV2RxfouYpbK/zNk=",
    "TVpndIGOm6i1ws/c6fYDEB0qN0RRXmt4hZKfrLnG0+A=",
    "VGFue4iVoq+8ydbj8P0KFyQxPktYZXJ/jJmms8DN2uc=",
    "W2h1go+cqbbD0N3q9wQRHis4RVJfbHmGk6CtusfU4e4=",
    "Ym98iZajsL3K1+Tx/gsYJTI/TFlmc4CNmqe0wc7b6PU=",
    "aXaDkJ2qt8TR3uv4BRIfLDlGU2BteoeUoa67yNXi7/w=",
    "cH2Kl6SxvsvY5fL/DBkmM0BNWmd0gY6bqLXCz9zp9gM=",
    "d4SRnqu4xdLf7PkGEyAtOkdUYW57iJWir7zJ1uPw/Qo=",
    "fouYpbK/zNnm8wANGic0QU5baHWCj5yptsPQ3er3BBE=",
    "hZKfrLnG0+Dt+gcUIS47SFVib3yJlqOwvcrX5PH+Cxg=",
];

/// Insert a test device for the given user, returning the saved `Device<Id>`.
/// The device is automatically added to all existing networks so that
/// `WireguardNetworkDevice` join records exist (required for config-building).
pub(crate) async fn create_device_for_user(pool: &PgPool, user_id: Id) -> Device<Id> {
    let n = DEV_CTR.fetch_add(1, Ordering::Relaxed);
    // Use a pre-generated valid 32-byte base64 WireGuard public key.
    let pubkey = DEVICE_PUBKEYS[n as usize % DEVICE_PUBKEYS.len()].to_string();
    let mut conn = pool.acquire().await.expect("failed to acquire DB connection");
    let device = Device::new(
        format!("test-device-{n}"),
        pubkey,
        user_id,
        DeviceType::User,
        None,
        true,
    )
    .save(&mut *conn)
    .await
    .expect("failed to save test device");
    // Add to all networks that exist at this point so WireguardNetworkDevice
    // join rows are created (needed by build_device_config_response).
    device
        .add_to_all_networks(&mut conn)
        .await
        .expect("failed to add device to networks");
    device
}

/// Insert a test user AND a test device for that user, returning both.
pub(crate) async fn create_user_with_device(pool: &PgPool) -> (User<Id>, Device<Id>) {
    let user = create_user(pool).await;
    let device = create_device_for_user(pool, user.id).await;
    (user, device)
}

// ---------------------------------------------------------------------------
// Factory helpers — Enrollment tokens
// ---------------------------------------------------------------------------

/// Insert a valid enrollment token for the given user.
///
/// The token expires in one hour, so it is always valid in tests.
///
/// `admin_id` should be the ID of the user who is initiating enrollment
/// (typically an admin).  The enrollment welcome-page template references
/// `{{ admin_first_name }}` etc., so Tera will fail to render it when those
/// variables are absent.  Pass `Some(user_id)` to populate those fields
/// (using the user as their own admin is fine for tests).
pub(crate) async fn create_enrollment_token(pool: &PgPool, user_id: Id, admin_id: Option<Id>) -> Token {
    let token = Token::new(
        user_id,
        admin_id,
        None,
        3600, // 1 hour
        Some(ENROLLMENT_TOKEN_TYPE.to_string()),
    );
    token
        .save(pool)
        .await
        .expect("failed to save enrollment token");
    token
}

// ---------------------------------------------------------------------------
// Factory helpers — Polling tokens
// ---------------------------------------------------------------------------

/// Insert a polling token for the given device, returning the raw token string.
pub(crate) async fn create_polling_token(pool: &PgPool, device_id: Id) -> String {
    PollingToken::new(device_id)
        .save(pool)
        .await
        .expect("failed to save polling token")
        .token
}

// ---------------------------------------------------------------------------
// Enrollment session helpers
// ---------------------------------------------------------------------------

/// Send an `EnrollmentStart` request to the handler and consume the response.
///
/// The enrollment server requires `start_session()` to be called (setting
/// `Token::used_at`) before any subsequent `NewDevice` or `ExistingDevice`
/// request can be processed.  Call this helper with a fresh token immediately
/// after `complete_proxy_handshake` to open the enrollment session.
///
/// The function sends a single `EnrollmentStartRequest` with the given token
/// ID and waits for the `EnrollmentStartResponse` (or any payload — panicking
/// if the stream closes without a response).
pub(crate) async fn start_enrollment_session(
    context: &mut HandlerTestContext,
    token_id: &str,
) {
    static ENROLL_CTR: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1000);
    let id = ENROLL_CTR.fetch_add(1, Ordering::Relaxed);

    context.mock_proxy().send_request(CoreRequest {
        id,
        device_info: Some(make_device_info()),
        payload: Some(core_request::Payload::EnrollmentStart(
            EnrollmentStartRequest {
                token: token_id.to_string(),
            },
        )),
    });

    // Consume the response (EnrollmentStartResponse or an error).
    let response = context.mock_proxy_mut().recv_outbound().await;
    match &response.payload {
        Some(core_response::Payload::EnrollmentStart(_)) => { /* success */ }
        Some(core_response::Payload::CoreError(e)) => {
            panic!(
                "start_enrollment_session: got CoreError status={} msg={}",
                e.status_code, e.message
            );
        }
        other => panic!(
            "start_enrollment_session: expected EnrollmentStart response, got: {:?}",
            other.as_ref().map(|p| std::mem::discriminant(p))
        ),
    }
}
