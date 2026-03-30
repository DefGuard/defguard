use std::sync::atomic::{AtomicU64, Ordering};

use defguard_common::db::{
    Id,
    models::{
        Device, DeviceType, User, WireguardNetwork,
        polling_token::PollingToken,
        vpn_client_session::VpnClientSession,
        wireguard::{LocationMfaMode, ServiceLocationMode},
    },
};
use defguard_core::{
    db::models::enrollment::{ENROLLMENT_TOKEN_TYPE, Token},
    enterprise::license::{License, LicenseTier, set_cached_license},
    events::{BidiStreamEvent, BidiStreamEventType, DesktopClientMfaEvent},
    grpc::GatewayEvent,
};
use defguard_proto::proxy::{
    AwaitRemoteMfaFinishRequest, ClientMfaFinishRequest, ClientMfaStartRequest,
    ClientMfaTokenValidationRequest, CoreRequest, CoreResponse, DeviceConfigResponse, DeviceInfo,
    EnrollmentStartRequest, MfaMethod, core_request, core_response,
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

// ---------------------------------------------------------------------------
// Factory helpers — MFA networks
// ---------------------------------------------------------------------------

/// Insert a WireGuard network with `LocationMfaMode::Internal`, returning the
/// saved `WireguardNetwork<Id>`.  Use this for any test that exercises the MFA
/// flow (the default `create_network` uses `LocationMfaMode::Disabled`).
pub(crate) async fn create_mfa_network(pool: &PgPool) -> WireguardNetwork<Id> {
    let n = NET_CTR.fetch_add(1, Ordering::Relaxed);
    WireguardNetwork::new(
        format!("test-mfa-network-{n}"),
        41820 + i32::try_from(n % 10_000).unwrap(),
        "10.1.0.1".to_string(),
        None,
        Vec::<IpNetwork>::new(),
        true,  // allow_all_groups
        false, // acl_enabled
        false, // acl_default_allow
        LocationMfaMode::Internal,
        ServiceLocationMode::default(),
    )
    .try_set_address("10.1.0.1/24")
    .expect("failed to set mfa network address")
    .save(pool)
    .await
    .expect("failed to save test mfa wireguard network")
}

/// Insert a WireGuard network with `LocationMfaMode::External`.
pub(crate) async fn create_external_mfa_network(pool: &PgPool) -> WireguardNetwork<Id> {
    let n = NET_CTR.fetch_add(1, Ordering::Relaxed);
    WireguardNetwork::new(
        format!("test-ext-mfa-network-{n}"),
        31820 + i32::try_from(n % 10_000).unwrap(),
        "10.2.0.1".to_string(),
        None,
        Vec::<IpNetwork>::new(),
        true,  // allow_all_groups
        false, // acl_enabled
        false, // acl_default_allow
        LocationMfaMode::External,
        ServiceLocationMode::default(),
    )
    .try_set_address("10.2.0.1/24")
    .expect("failed to set ext mfa network address")
    .save(pool)
    .await
    .expect("failed to save test external mfa wireguard network")
}

// ---------------------------------------------------------------------------
// MFA user setup helpers
// ---------------------------------------------------------------------------

/// Enable email MFA for `user`, returning the currently-valid MFA code.
///
/// The code is valid immediately and can be passed directly to
/// `ClientMfaFinishRequest::code`.
pub(crate) async fn setup_user_email_mfa(pool: &PgPool, user: &mut User<Id>) -> String {
    user.new_email_secret(pool).await.expect("new_email_secret");
    user.enable_email_mfa(pool).await.expect("enable_email_mfa");
    // generate_email_mfa_code uses the in-memory secret; note that
    // start_client_mfa_login also calls generate_email_mfa_code internally —
    // the two calls will produce the same code because the in-memory secret
    // hasn't changed. But we need the code *after* the start call, so the
    // caller should call this helper before start and pass the code to finish.
    user.generate_email_mfa_code().expect("generate_email_mfa_code")
}

// ---------------------------------------------------------------------------
// MFA flow helpers
// ---------------------------------------------------------------------------

static MFA_CTR: AtomicU64 = AtomicU64::new(2000);

/// Send `ClientMfaStart` and return `(response_id, start_token)`.
///
/// Panics if the handler returns an error.
pub(crate) async fn send_mfa_start(
    context: &mut HandlerTestContext,
    location_id: Id,
    pubkey: &str,
    method: MfaMethod,
) -> (u64, String) {
    let id = MFA_CTR.fetch_add(1, Ordering::Relaxed);
    context.mock_proxy().send_request(CoreRequest {
        id,
        device_info: None,
        payload: Some(core_request::Payload::ClientMfaStart(ClientMfaStartRequest {
            location_id,
            pubkey: pubkey.to_string(),
            method: method as i32,
        })),
    });
    let response = context.mock_proxy_mut().recv_outbound().await;
    let token = match &response.payload {
        Some(core_response::Payload::ClientMfaStart(r)) => r.token.clone(),
        Some(core_response::Payload::CoreError(e)) => panic!(
            "send_mfa_start: got CoreError status={} msg={}",
            e.status_code, e.message
        ),
        other => panic!(
            "send_mfa_start: expected ClientMfaStart response, got: {:?}",
            other.as_ref().map(|p| std::mem::discriminant(p))
        ),
    };
    (id, token)
}

/// Send `ClientMfaFinish` and return `(response, preshared_key)`.
///
/// Requires `device_info` because the handler calls `parse_client_ip_agent`.
/// Panics if the handler returns an error.
pub(crate) async fn send_mfa_finish(
    context: &mut HandlerTestContext,
    token: &str,
    code: Option<&str>,
) -> (CoreResponse, String) {
    let id = MFA_CTR.fetch_add(1, Ordering::Relaxed);
    context.mock_proxy().send_request(CoreRequest {
        id,
        device_info: Some(make_device_info()),
        payload: Some(core_request::Payload::ClientMfaFinish(ClientMfaFinishRequest {
            token: token.to_string(),
            code: code.map(str::to_string),
            auth_pub_key: None,
        })),
    });
    let response = context.mock_proxy_mut().recv_outbound().await;
    let psk = match &response.payload {
        Some(core_response::Payload::ClientMfaFinish(r)) => r.preshared_key.clone(),
        Some(core_response::Payload::CoreError(e)) => panic!(
            "send_mfa_finish: got CoreError status={} msg={}",
            e.status_code, e.message
        ),
        other => panic!(
            "send_mfa_finish: expected ClientMfaFinish response, got: {:?}",
            other.as_ref().map(|p| std::mem::discriminant(p))
        ),
    };
    (response, psk)
}

/// Send `ClientMfaTokenValidation` and return `token_valid`.
pub(crate) async fn send_token_validation(
    context: &mut HandlerTestContext,
    token: &str,
) -> bool {
    let id = MFA_CTR.fetch_add(1, Ordering::Relaxed);
    context.mock_proxy().send_request(CoreRequest {
        id,
        device_info: None,
        payload: Some(core_request::Payload::ClientMfaTokenValidation(
            ClientMfaTokenValidationRequest { token: token.to_string() },
        )),
    });
    let response = context.mock_proxy_mut().recv_outbound().await;
    match &response.payload {
        Some(core_response::Payload::ClientMfaTokenValidation(r)) => r.token_valid,
        Some(core_response::Payload::CoreError(e)) => panic!(
            "send_token_validation: got CoreError status={} msg={}",
            e.status_code, e.message
        ),
        other => panic!(
            "send_token_validation: expected ClientMfaTokenValidation response, got: {:?}",
            other.as_ref().map(|p| std::mem::discriminant(p))
        ),
    }
}

// ---------------------------------------------------------------------------
// MFA assertion helpers
// ---------------------------------------------------------------------------

/// Assert that the next `GatewayEvent` broadcast is `MfaSessionAuthorized` and
/// return `(location_id, device)`.
pub(crate) async fn expect_gateway_mfa_authorized(
    wireguard_tx: &tokio::sync::broadcast::Sender<GatewayEvent>,
) -> Id {
    use tokio::time::{timeout, Duration};
    let mut rx = wireguard_tx.subscribe();
    let event = timeout(Duration::from_secs(5), rx.recv())
        .await
        .expect("timed out waiting for GatewayEvent::MfaSessionAuthorized")
        .expect("gateway event channel closed");
    match event {
        GatewayEvent::MfaSessionAuthorized(loc_id, _, _) => loc_id,
        other => panic!("expected MfaSessionAuthorized, got: {other:?}"),
    }
}

/// Assert that the next `BidiStreamEvent` is `DesktopClientMfa(Success)` and
/// return the location id from the event.
pub(crate) async fn expect_bidi_mfa_success(
    bidi_rx: &mut tokio::sync::mpsc::UnboundedReceiver<BidiStreamEvent>,
) -> Id {
    use tokio::time::{timeout, Duration};
    let event = timeout(Duration::from_secs(5), bidi_rx.recv())
        .await
        .expect("timed out waiting for BidiStreamEvent DesktopClientMfa(Success)")
        .expect("bidi event channel closed");
    match event.event {
        BidiStreamEventType::DesktopClientMfa(e) => match *e {
            DesktopClientMfaEvent::Success { location, .. } => location.id,
            other => panic!("expected DesktopClientMfaEvent::Success, got: {other:?}"),
        },
        other => panic!("expected BidiStreamEventType::DesktopClientMfa, got: {other:?}"),
    }
}

/// Assert that a `CoreResponse` carries a `CoreError` and return tonic code.
/// Alias kept for backwards-compat with enrollment / polling tests.
pub(crate) fn assert_mfa_error_response(response: &CoreResponse) -> tonic::Code {
    assert_error_response(response)
}

/// Assert that the `VpnClientSession` for a given location and device exists in
/// the DB and return it.
pub(crate) async fn assert_vpn_session_exists(
    pool: &PgPool,
    location_id: Id,
    device_id: Id,
) -> VpnClientSession<Id> {
    VpnClientSession::try_get_active_session(pool, location_id, device_id)
        .await
        .expect("db query failed")
        .unwrap_or_else(|| {
            panic!("expected active VpnClientSession for location={location_id} device={device_id}")
        })
}
