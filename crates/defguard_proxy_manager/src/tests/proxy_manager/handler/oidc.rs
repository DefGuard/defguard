// Phase 5: OIDC handler tests
//
// AuthCallback tests (Phase 5b):
//  1. test_auth_callback_creates_new_user_on_first_login
//     — no pre-existing user; code contains unknown sub/email; verifies core
//       creates the user and returns an enrollment token
//  2. test_auth_callback_exchanges_code_for_enrollment_token
//     — pre-existing user; verifies the handler matches by email and returns
//       a valid enrollment token id
//  3. test_mfa_oidc_full_flow
//     — ClientMfaStart (OIDC) → ClientMfaOidcAuthenticate → ClientMfaFinish
//       → PSK + GatewayEvent + BidiEvent + VpnClientSession in DB
//
// AuthInfo tests (Phase 5c):
//  4. test_auth_info_enrollment_returns_authorize_url
//     — valid license + provider; AuthFlowType::Enrollment returns an OIDC
//       authorization URL pointing at the mock provider
//  5. test_auth_info_mfa_returns_authorize_url
//     — same as above but AuthFlowType::Mfa
//  6. test_auth_info_requires_license
//     — no license → FailedPrecondition error
//  7. test_auth_info_requires_oidc_provider
//     — valid license, no provider in DB → NotFound error

#![allow(deprecated)]
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

use defguard_core::db::models::enrollment::Token;
use defguard_proto::proxy::{
    AuthCallbackRequest, AuthFlowType, AuthInfoRequest, ClientMfaOidcAuthenticateRequest,
    CoreRequest, MfaMethod, core_request, core_response,
};

use super::support::{
    assert_error_response, assert_vpn_session_exists, clear_test_license, complete_proxy_handshake,
    create_external_mfa_network, create_oidc_provider, create_user, create_user_with_device,
    expect_bidi_mfa_success, make_device_info, make_oidc_code, send_mfa_finish, send_mfa_start,
    set_public_proxy_url, set_test_license_business,
};
use crate::tests::common::{HandlerTestContext, MockOidcProvider};

// ---------------------------------------------------------------------------
// 1. AuthCallback creates a new user when sub/email are unknown
// ---------------------------------------------------------------------------

#[sqlx::test]
async fn test_auth_callback_creates_new_user_on_first_login(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;
    set_test_license_business();

    // Spin up a mock OIDC provider and register it in the DB.
    let mock = MockOidcProvider::start().await;
    let _provider = create_oidc_provider(&context.pool, &mock).await;

    // Point the edge callback URL at the mock so `edge_callback_url` works.
    set_public_proxy_url(&context.pool, &mock.base_url).await;

    // Choose a sub/email that does NOT exist in the DB yet.
    let sub = "new-oidc-user-sub";
    let email = "newoidcuser@example.com";
    let raw_nonce = "test-nonce-1";
    let code = make_oidc_code(sub, email, raw_nonce);

    context.mock_proxy().send_request(CoreRequest {
        id: 10,
        device_info: None,
        payload: Some(core_request::Payload::AuthCallback(AuthCallbackRequest {
            code: code.clone(),
            nonce: raw_nonce.to_string(),
            callback_url: String::new(), // ignored in v2 path (handler uses settings)
        })),
    });

    let response = context.mock_proxy_mut().recv_outbound().await;
    let auth_cb = match &response.payload {
        Some(core_response::Payload::AuthCallback(r)) => r,
        Some(core_response::Payload::CoreError(e)) => panic!(
            "test_auth_callback_creates_new_user_on_first_login: got CoreError status={} msg={}",
            e.status_code, e.message
        ),
        other => panic!(
            "expected AuthCallback response, got: {:?}",
            other.as_ref().map(std::mem::discriminant)
        ),
    };

    // The token id must be non-empty.
    assert!(
        !auth_cb.token.is_empty(),
        "expected non-empty enrollment token id"
    );

    // The URL should be non-empty (proxy public URL from settings).
    assert!(
        !auth_cb.url.is_empty(),
        "expected non-empty proxy public URL"
    );

    // The enrollment token must exist in the DB.
    let token = Token::find_by_id(&context.pool, &auth_cb.token)
        .await
        .expect("db query failed for enrollment token");

    // The token's email must match what we sent.
    assert_eq!(
        token.email.as_deref(),
        Some(email),
        "enrollment token email mismatch"
    );

    clear_test_license();
    context.finish().await.expect_server_finished().await;
}

// ---------------------------------------------------------------------------
// 4. AuthInfo with Enrollment flow type returns a valid OIDC authorization URL
// ---------------------------------------------------------------------------

#[sqlx::test]
async fn test_auth_info_enrollment_returns_authorize_url(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;
    set_test_license_business();

    let mock = MockOidcProvider::start().await;
    let provider = create_oidc_provider(&context.pool, &mock).await;
    set_public_proxy_url(&context.pool, &mock.base_url).await;

    context.mock_proxy().send_request(CoreRequest {
        id: 40,
        device_info: None,
        payload: Some(core_request::Payload::AuthInfo(AuthInfoRequest {
            redirect_url: String::new(), // deprecated; ignored when auth_flow_type is set
            state: None,
            auth_flow_type: AuthFlowType::Enrollment as i32,
        })),
    });

    let response = context.mock_proxy_mut().recv_outbound().await;
    let auth_info = match &response.payload {
        Some(core_response::Payload::AuthInfo(r)) => r,
        Some(core_response::Payload::CoreError(e)) => panic!(
            "test_auth_info_enrollment_returns_authorize_url: got CoreError status={} msg={}",
            e.status_code, e.message
        ),
        other => panic!(
            "expected AuthInfo response, got: {:?}",
            other.as_ref().map(std::mem::discriminant)
        ),
    };

    // The URL must be non-empty and point at the mock OIDC authorization endpoint.
    assert!(
        !auth_info.url.is_empty(),
        "expected non-empty authorization URL"
    );
    assert!(
        auth_info.url.starts_with(&mock.base_url),
        "authorization URL should start with mock base URL; got: {}",
        auth_info.url
    );

    // CSRF token and nonce must be non-empty.
    assert!(
        !auth_info.csrf_token.is_empty(),
        "expected non-empty csrf_token"
    );
    assert!(!auth_info.nonce.is_empty(), "expected non-empty nonce");

    // The button display name must match the provider's display name.
    assert_eq!(
        auth_info.button_display_name.as_deref(),
        provider.display_name.as_deref(),
        "button_display_name should match provider display_name"
    );

    clear_test_license();
    context.finish().await.expect_server_finished().await;
}

// ---------------------------------------------------------------------------
// 5. AuthInfo with Mfa flow type returns a valid OIDC authorization URL
// ---------------------------------------------------------------------------

#[sqlx::test]
async fn test_auth_info_mfa_returns_authorize_url(_: PgPoolOptions, options: PgConnectOptions) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;
    set_test_license_business();

    let mock = MockOidcProvider::start().await;
    let _provider = create_oidc_provider(&context.pool, &mock).await;
    set_public_proxy_url(&context.pool, &mock.base_url).await;

    context.mock_proxy().send_request(CoreRequest {
        id: 50,
        device_info: None,
        payload: Some(core_request::Payload::AuthInfo(AuthInfoRequest {
            redirect_url: String::new(),
            state: None,
            auth_flow_type: AuthFlowType::Mfa as i32,
        })),
    });

    let response = context.mock_proxy_mut().recv_outbound().await;
    let auth_info = match &response.payload {
        Some(core_response::Payload::AuthInfo(r)) => r,
        Some(core_response::Payload::CoreError(e)) => panic!(
            "test_auth_info_mfa_returns_authorize_url: got CoreError status={} msg={}",
            e.status_code, e.message
        ),
        other => panic!(
            "expected AuthInfo response, got: {:?}",
            other.as_ref().map(std::mem::discriminant)
        ),
    };

    assert!(
        !auth_info.url.is_empty(),
        "expected non-empty authorization URL"
    );
    assert!(
        auth_info.url.starts_with(&mock.base_url),
        "authorization URL should start with mock base URL; got: {}",
        auth_info.url
    );
    assert!(
        !auth_info.csrf_token.is_empty(),
        "expected non-empty csrf_token"
    );
    assert!(!auth_info.nonce.is_empty(), "expected non-empty nonce");

    clear_test_license();
    context.finish().await.expect_server_finished().await;
}

// ---------------------------------------------------------------------------
// 6. AuthInfo requires a business license
// ---------------------------------------------------------------------------

#[sqlx::test]
async fn test_auth_info_requires_license(_: PgPoolOptions, options: PgConnectOptions) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;

    // Ensure no license is active.
    clear_test_license();

    context.mock_proxy().send_request(CoreRequest {
        id: 60,
        device_info: None,
        payload: Some(core_request::Payload::AuthInfo(AuthInfoRequest {
            redirect_url: String::new(),
            state: None,
            auth_flow_type: AuthFlowType::Enrollment as i32,
        })),
    });

    let response = context.mock_proxy_mut().recv_outbound().await;
    let code = assert_error_response(&response);
    assert_eq!(
        code,
        tonic::Code::FailedPrecondition,
        "expected FailedPrecondition when no license"
    );

    context.finish().await.expect_server_finished().await;
}

// ---------------------------------------------------------------------------
// 7. AuthInfo returns NotFound when no OIDC provider is configured
// ---------------------------------------------------------------------------

#[sqlx::test]
async fn test_auth_info_requires_oidc_provider(_: PgPoolOptions, options: PgConnectOptions) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;
    set_test_license_business();

    // No OIDC provider is inserted — but we still need a valid public proxy URL
    // so that edge_callback_url() does not fail before the provider lookup.
    set_public_proxy_url(&context.pool, "http://proxy.example.com").await;

    context.mock_proxy().send_request(CoreRequest {
        id: 70,
        device_info: None,
        payload: Some(core_request::Payload::AuthInfo(AuthInfoRequest {
            redirect_url: String::new(),
            state: None,
            auth_flow_type: AuthFlowType::Enrollment as i32,
        })),
    });

    let response = context.mock_proxy_mut().recv_outbound().await;
    let code = assert_error_response(&response);
    assert_eq!(
        code,
        tonic::Code::NotFound,
        "expected NotFound when no OIDC provider configured"
    );

    clear_test_license();
    context.finish().await.expect_server_finished().await;
}

// ---------------------------------------------------------------------------
// 3. Full OIDC MFA flow: Start → OidcAuthenticate → Finish
// ---------------------------------------------------------------------------

#[sqlx::test]
async fn test_mfa_oidc_full_flow(_: PgPoolOptions, options: PgConnectOptions) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;
    set_test_license_business();

    // External MFA network is required for OIDC MFA method.
    let network = create_external_mfa_network(&context.pool).await;
    let (user, device) = create_user_with_device(&context.pool).await;

    let mock = MockOidcProvider::start().await;
    let _provider = create_oidc_provider(&context.pool, &mock).await;
    set_public_proxy_url(&context.pool, &mock.base_url).await;

    // Subscribe to gateway events before sending MFA finish.
    let _gateway_rx = context.wireguard_tx.subscribe();

    // ---- Step 1: ClientMfaStart with Oidc method ----
    let (_, mfa_token) = send_mfa_start(
        &mut context,
        network.id,
        &device.wireguard_pubkey,
        MfaMethod::Oidc,
    )
    .await;

    // ---- Step 2: ClientMfaOidcAuthenticate ----
    // Build the `state` field by encoding the mfa_token inside it.
    let state =
        defguard_core::enterprise::handlers::openid_login::build_state(Some(mfa_token.clone()))
            .secret()
            .clone();

    let raw_nonce = "mfa-oidc-nonce";
    let code = make_oidc_code(&user.email, &user.email, raw_nonce);

    context.mock_proxy().send_request(CoreRequest {
        id: 30,
        device_info: Some(make_device_info()),
        payload: Some(core_request::Payload::ClientMfaOidcAuthenticate(
            ClientMfaOidcAuthenticateRequest {
                code: code.clone(),
                state: state.clone(),
                callback_url: String::new(), // unused in handler (uses settings)
                nonce: raw_nonce.to_string(),
            },
        )),
    });

    // The handler returns an Empty payload on success.
    let response = context.mock_proxy_mut().recv_outbound().await;
    assert!(
        matches!(response.payload, Some(core_response::Payload::Empty(()))),
        "expected Empty after OidcAuthenticate, got: {:?}",
        response.payload.as_ref().map(std::mem::discriminant)
    );

    // ---- Step 3: ClientMfaFinish (no TOTP code — session is OIDC-completed) ----
    let (_, psk) = send_mfa_finish(&mut context, &mfa_token, None).await;
    assert!(
        !psk.is_empty(),
        "expected non-empty PSK after OIDC MFA finish"
    );

    // Verify VpnClientSession was created.
    assert_vpn_session_exists(&context.pool, network.id, device.id).await;

    // Verify BidiStreamEvent::DesktopClientMfa(Success) was emitted.
    let location_id = expect_bidi_mfa_success(&mut context.bidi_events_rx).await;
    assert_eq!(location_id, network.id);

    clear_test_license();
    context.finish().await.expect_server_finished().await;
}

// ---------------------------------------------------------------------------
// 2. AuthCallback matches existing user by email and returns enrollment token
// ---------------------------------------------------------------------------

/// When the OIDC code's email matches a pre-existing user the handler must
/// return a valid enrollment token bound to that user (not create a new one).
#[sqlx::test]
async fn test_auth_callback_exchanges_code_for_enrollment_token(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;
    set_test_license_business();

    // Create a user whose email will be matched by the OIDC callback.
    let user = create_user(&context.pool).await;

    // Spin up mock OIDC provider and register it in the DB.
    let mock = MockOidcProvider::start().await;
    let _provider = create_oidc_provider(&context.pool, &mock).await;
    set_public_proxy_url(&context.pool, &mock.base_url).await;

    // Build an OIDC code whose email matches the pre-existing user.
    let raw_nonce = "test-nonce-existing-user";
    let code = make_oidc_code(&user.email, &user.email, raw_nonce);

    context.mock_proxy().send_request(CoreRequest {
        id: 11,
        device_info: None,
        payload: Some(core_request::Payload::AuthCallback(AuthCallbackRequest {
            code: code.clone(),
            nonce: raw_nonce.to_string(),
            callback_url: String::new(),
        })),
    });

    let response = context.mock_proxy_mut().recv_outbound().await;
    let auth_cb = match &response.payload {
        Some(core_response::Payload::AuthCallback(r)) => r,
        Some(core_response::Payload::CoreError(e)) => panic!(
            "test_auth_callback_exchanges_code_for_enrollment_token: got CoreError status={} msg={}",
            e.status_code, e.message
        ),
        other => panic!(
            "expected AuthCallback response, got: {:?}",
            other.as_ref().map(std::mem::discriminant)
        ),
    };

    assert!(
        !auth_cb.token.is_empty(),
        "expected non-empty enrollment token id"
    );
    assert!(
        !auth_cb.url.is_empty(),
        "expected non-empty proxy public URL"
    );

    // The enrollment token must exist in the DB and be bound to the existing user.
    let token = Token::find_by_id(&context.pool, &auth_cb.token)
        .await
        .expect("db query failed for enrollment token");

    assert_eq!(
        token.user_id, user.id,
        "enrollment token must belong to the pre-existing user"
    );

    clear_test_license();
    context.finish().await.expect_server_finished().await;
}
