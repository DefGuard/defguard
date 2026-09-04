#![allow(deprecated)]
use base64::{Engine, prelude::BASE64_STANDARD};
use defguard_common::db::{
    Id,
    models::{
        User,
        settings::{Settings, update_current_settings},
        vpn_client_mfa_session::VpnClientMfaSession,
    },
};
use defguard_core::{
    db::models::enrollment::Token,
    enterprise::{
        handlers::openid_login::{MfaOidcState, build_state},
        license::{License, LicenseTier, SupportType, set_cached_license},
        limits::update_counts,
    },
    events::{ApiEvent, ApiEventType},
    grpc::proto::enterprise::license::LicenseLimits,
};
use defguard_proto::{
    client_types::{AuthFlowType, AuthInfoRequest, MfaMethod},
    proxy::{
        AuthCallbackRequest, ClientMfaOidcAuthenticateRequest, CoreRequest, core_request,
        core_response,
    },
};
use reqwest::Url;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use tokio::time::timeout;
use tonic::Code;

use super::support::{
    EmailVerified, assert_error_response, assert_error_response_details, assert_vpn_session_exists,
    clear_test_license, complete_proxy_handshake, create_external_mfa_network,
    create_oidc_provider, create_user, create_user_with_device, expect_bidi_mfa_success,
    link_user_oidc_identity, make_device_info, make_oidc_code, make_oidc_code_with_email_verified,
    send_mfa_finish, send_mfa_start, set_public_proxy_url, set_test_license_business,
};
use crate::tests::common::{HandlerTestContext, MockOidcProvider, RECEIVE_TIMEOUT};

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
            nonce: raw_nonce.to_owned(),
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
            state: None,
            auth_flow_type: AuthFlowType::Enrollment as i32,
            ..Default::default()
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

#[sqlx::test]
async fn test_auth_info_mfa_returns_authorize_url(_: PgPoolOptions, options: PgConnectOptions) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;
    set_test_license_business();

    let mock = MockOidcProvider::start().await;
    let _provider = create_oidc_provider(&context.pool, &mock).await;
    set_public_proxy_url(&context.pool, &mock.base_url).await;

    // The MFA flow requires an active session whose token rides in `state`. Start one for the
    // external (OIDC) network so `start_client_mfa_login` accepts the Oidc method.
    let network = create_external_mfa_network(&context.pool).await;
    let (mut user, device) = create_user_with_device(&context.pool).await;
    link_user_oidc_identity(&context.pool, &mut user).await;
    let (_id, mfa_token) = send_mfa_start(
        &mut context,
        network.id,
        &device.wireguard_pubkey,
        MfaMethod::Oidc,
    )
    .await;
    let session = VpnClientMfaSession::<Id>::find_active_by_token(&context.pool, &mfa_token)
        .await
        .expect("failed to find active MFA session")
        .expect("expected an active MFA session");
    let attempt_id = session
        .ephemeral_state
        .as_ref()
        .expect("expected an attempt in progress")
        .step_attempt_id
        .clone();

    context.mock_proxy().send_request(CoreRequest {
        id: 50,
        device_info: None,
        payload: Some(core_request::Payload::AuthInfo(AuthInfoRequest {
            state: Some(mfa_token.clone()),
            auth_flow_type: AuthFlowType::Mfa as i32,
            ..Default::default()
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

    // The authorize URL's `state` must carry "<csrf>.<token>.<step_attempt_id>". The csrf
    // prefix is the browser's CSRF nonce; the tail is what the callback parses back out.
    let url = Url::parse(&auth_info.url).expect("failed to parse authorize URL");
    let state_param = url
        .query_pairs()
        .find(|(key, _)| key == "state")
        .map(|(_, value)| value.into_owned())
        .expect("authorize URL must carry a state parameter");
    let decoded = BASE64_STANDARD
        .decode(state_param.as_bytes())
        .expect("state must be base64");
    let decoded = String::from_utf8(decoded).expect("state must be UTF-8");
    let (csrf, tail) = decoded
        .split_once('.')
        .expect("state must be <csrf>.<payload>");
    assert!(!csrf.is_empty(), "state must carry a csrf prefix");
    assert_eq!(tail, MfaOidcState::build(&mfa_token, &attempt_id));

    clear_test_license();
    context.finish().await.expect_server_finished().await;
}

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
            state: None,
            auth_flow_type: AuthFlowType::Enrollment as i32,
            ..Default::default()
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

#[sqlx::test]
async fn test_auth_info_requires_oidc_provider(_: PgPoolOptions, options: PgConnectOptions) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;
    set_test_license_business();

    // No OIDC provider is inserted - but we still need a valid public proxy URL
    // so that edge_callback_url() does not fail before the provider lookup.
    set_public_proxy_url(&context.pool, "http://proxy.example.com").await;

    context.mock_proxy().send_request(CoreRequest {
        id: 70,
        device_info: None,
        payload: Some(core_request::Payload::AuthInfo(AuthInfoRequest {
            state: None,
            auth_flow_type: AuthFlowType::Enrollment as i32,
            ..Default::default()
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

#[sqlx::test]
async fn test_auth_callback_missing_user_without_account_creation_returns_permission_denied(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;
    set_test_license_business();

    let mock = MockOidcProvider::start().await;
    let _provider = create_oidc_provider(&context.pool, &mock).await;
    set_public_proxy_url(&context.pool, &mock.base_url).await;

    let mut settings = Settings::get_current_settings();
    settings.openid_create_account = false;
    update_current_settings(&context.pool, settings)
        .await
        .expect("failed to disable OpenID account creation");

    let raw_nonce = "test-nonce-account-creation-disabled";
    let code = make_oidc_code("missing-user-sub", "missing-user@example.com", raw_nonce);
    context.mock_proxy().send_request(CoreRequest {
        id: 80,
        device_info: None,
        payload: Some(core_request::Payload::AuthCallback(AuthCallbackRequest {
            code,
            nonce: raw_nonce.to_owned(),
        })),
    });

    let response = context.mock_proxy_mut().recv_outbound().await;
    let code = assert_error_response(&response);
    assert_eq!(
        code,
        tonic::Code::PermissionDenied,
        "expected PermissionDenied when account creation is disabled"
    );

    let mut settings = Settings::get_current_settings();
    settings.openid_create_account = true;
    update_current_settings(&context.pool, settings)
        .await
        .expect("failed to re-enable OpenID account creation");

    clear_test_license();
    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_auth_callback_requires_oidc_provider(_: PgPoolOptions, options: PgConnectOptions) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;
    set_public_proxy_url(&context.pool, "http://proxy.example.com").await;

    context.mock_proxy().send_request(CoreRequest {
        id: 90,
        device_info: None,
        payload: Some(core_request::Payload::AuthCallback(AuthCallbackRequest {
            code: "code".to_owned(),
            nonce: "nonce".to_owned(),
        })),
    });

    let response = context.mock_proxy_mut().recv_outbound().await;
    let code = assert_error_response(&response);
    assert_eq!(
        code,
        tonic::Code::NotFound,
        "expected NotFound when no OIDC provider is configured"
    );

    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_auth_callback_invalid_provider_url_returns_invalid_argument(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;

    let mock = MockOidcProvider::start().await;
    let mut provider = create_oidc_provider(&context.pool, &mock).await;
    provider.base_url = "not a url".to_owned();
    provider
        .save(&context.pool)
        .await
        .expect("failed to save invalid OIDC provider URL");
    set_public_proxy_url(&context.pool, &mock.base_url).await;

    context.mock_proxy().send_request(CoreRequest {
        id: 100,
        device_info: None,
        payload: Some(core_request::Payload::AuthCallback(AuthCallbackRequest {
            code: "code".to_owned(),
            nonce: "nonce".to_owned(),
        })),
    });

    let response = context.mock_proxy_mut().recv_outbound().await;
    let code = assert_error_response(&response);
    assert_eq!(
        code,
        tonic::Code::InvalidArgument,
        "expected InvalidArgument when OIDC provider URL is invalid"
    );

    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_mfa_oidc_full_flow(_: PgPoolOptions, options: PgConnectOptions) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;
    set_test_license_business();

    // External MFA network is required for OIDC MFA method.
    let network = create_external_mfa_network(&context.pool).await;
    let (mut user, device) = create_user_with_device(&context.pool).await;
    link_user_oidc_identity(&context.pool, &mut user).await;

    let mock = MockOidcProvider::start().await;
    let _provider = create_oidc_provider(&context.pool, &mock).await;
    set_public_proxy_url(&context.pool, &mock.base_url).await;

    // Subscribe to gateway events before sending MFA finish.
    let _gateway_rx = context.gateway_tx.subscribe();

    // ---- Step 1: ClientMfaStart with Oidc method ----
    let (_, mfa_token) = send_mfa_start(
        &mut context,
        network.id,
        &device.wireguard_pubkey,
        MfaMethod::Oidc,
    )
    .await;

    // ---- Step 2: ClientMfaOidcAuthenticate ----
    // Build the `state` field the way the authorize-URL builder does for the MFA flow:
    // encode "<mfa_token>.<step_attempt_id>".
    let session = VpnClientMfaSession::<Id>::find_active_by_token(&context.pool, &mfa_token)
        .await
        .expect("failed to find active MFA session")
        .expect("expected an active MFA session");
    let attempt_id = session
        .ephemeral_state
        .as_ref()
        .expect("expected an attempt in progress")
        .step_attempt_id
        .clone();
    let state = build_state(Some(MfaOidcState::build(&mfa_token, &attempt_id)))
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
                nonce: raw_nonce.to_owned(),
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

    // ---- Step 3: ClientMfaFinish (no TOTP code - session is OIDC-completed) ----
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

/// An MFA callback presenting an unrecognised provider identity is refused and leaves no trace.
///
/// The `sub` is unlinked but the email matches a real account, which MFA must not resolve to. The
/// rejection lands after identity resolution, so a write there would survive it.
#[sqlx::test]
async fn test_mfa_oidc_unknown_identity_does_not_link_account(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;
    set_test_license_business();

    let network = create_external_mfa_network(&context.pool).await;
    let (mut user, device) = create_user_with_device(&context.pool).await;
    link_user_oidc_identity(&context.pool, &mut user).await;

    // A second account, never linked to any provider identity, whose email the callback presents.
    let bystander = create_user(&context.pool).await;
    assert!(bystander.openid_sub.is_none());

    let mock = MockOidcProvider::start().await;
    let _provider = create_oidc_provider(&context.pool, &mock).await;
    set_public_proxy_url(&context.pool, &mock.base_url).await;

    let (_, mfa_token) = send_mfa_start(
        &mut context,
        network.id,
        &device.wireguard_pubkey,
        MfaMethod::Oidc,
    )
    .await;

    let session = VpnClientMfaSession::<Id>::find_active_by_token(&context.pool, &mfa_token)
        .await
        .expect("failed to find active MFA session")
        .expect("expected an active MFA session");
    let attempt_id = session
        .ephemeral_state
        .as_ref()
        .expect("expected an attempt in progress")
        .step_attempt_id
        .clone();
    let state = build_state(Some(MfaOidcState::build(&mfa_token, &attempt_id)))
        .secret()
        .clone();

    let raw_nonce = "mfa-oidc-unknown-identity-nonce";
    // An unknown `sub`, carrying the bystander's email.
    let code = make_oidc_code("unlinked-provider-sub", &bystander.email, raw_nonce);

    context.mock_proxy().send_request(CoreRequest {
        id: 32,
        device_info: Some(make_device_info()),
        payload: Some(core_request::Payload::ClientMfaOidcAuthenticate(
            ClientMfaOidcAuthenticateRequest {
                code,
                state,
                nonce: raw_nonce.to_owned(),
            },
        )),
    });

    let response = context.mock_proxy_mut().recv_outbound().await;
    let code = assert_error_response(&response);
    assert_eq!(code, Code::Unauthenticated);

    // The bystander is still unlinked: the refused callback wrote nothing.
    let bystander = User::find_by_id(&context.pool, bystander.id)
        .await
        .expect("failed to reload bystander")
        .expect("bystander should still exist");
    assert!(
        bystander.openid_sub.is_none(),
        "a refused MFA callback linked an account to the provider identity"
    );

    clear_test_license();
    context.finish().await.expect_server_finished().await;
}

/// A callback whose state-carried `step_attempt_id` does not match the live row is rejected.
///
/// The genuine end-to-end case - a callback from an attempt superseded by a re-issue on the SAME
/// token - is not reachable until #3045 adds re-attempts; no production path re-issues an attempt
/// on a live token yet. This test constructs a stale id against a live row instead, so the gap is
/// covered explicitly rather than left looking tested.
#[sqlx::test]
async fn test_mfa_oidc_rejects_stale_attempt_id(_: PgPoolOptions, options: PgConnectOptions) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;
    set_test_license_business();

    let network = create_external_mfa_network(&context.pool).await;
    let (mut user, device) = create_user_with_device(&context.pool).await;
    link_user_oidc_identity(&context.pool, &mut user).await;

    let mock = MockOidcProvider::start().await;
    let _provider = create_oidc_provider(&context.pool, &mock).await;
    set_public_proxy_url(&context.pool, &mock.base_url).await;

    let (_id, mfa_token) = send_mfa_start(
        &mut context,
        network.id,
        &device.wireguard_pubkey,
        MfaMethod::Oidc,
    )
    .await;

    // Build a state carrying a stale attempt id that does not match the live row.
    let state = build_state(Some(MfaOidcState::build(&mfa_token, "stale-attempt-id")))
        .secret()
        .clone();

    let raw_nonce = "mfa-oidc-stale-nonce";
    let oidc_code = make_oidc_code(&user.email, &user.email, raw_nonce);

    context.mock_proxy().send_request(CoreRequest {
        id: 31,
        device_info: Some(make_device_info()),
        payload: Some(core_request::Payload::ClientMfaOidcAuthenticate(
            ClientMfaOidcAuthenticateRequest {
                code: oidc_code,
                state,
                nonce: raw_nonce.to_owned(),
            },
        )),
    });

    // The handler must reject the stale attempt rather than silently ignore it.
    let response = context.mock_proxy_mut().recv_outbound().await;
    let error_code = assert_error_response(&response);
    assert_eq!(
        error_code,
        tonic::Code::InvalidArgument,
        "expected InvalidArgument for a stale attempt id"
    );

    // The live attempt is untouched: the mark was a no-op, so the session is still pending OIDC.
    let session = VpnClientMfaSession::<Id>::find_active_by_token(&context.pool, &mfa_token)
        .await
        .expect("failed to find active MFA session")
        .expect("expected the session to remain live");
    assert!(
        !session
            .ephemeral_state
            .as_ref()
            .expect("expected an attempt in progress")
            .openid_auth_completed,
        "stale callback must not mark the attempt complete"
    );

    clear_test_license();
    context.finish().await.expect_server_finished().await;
}

/// A callback carrying a stale `step_attempt_id` must not destroy the live attempt, even when the
/// callback would otherwise fail on a path that deletes the session.
///
/// Every abort path in the handler (wrong method, bad callback URL, wrong account, bad code)
/// deletes the row. The attempt-id check therefore has to run before all of them: otherwise a late
/// callback from a superseded attempt tears down the attempt that replaced it. This drives an
/// unverifiable OIDC code so the request would reach the delete-on-failure branch, and asserts the
/// session is still live afterwards.
#[sqlx::test]
async fn test_mfa_oidc_stale_attempt_id_does_not_delete_session(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;
    set_test_license_business();

    let network = create_external_mfa_network(&context.pool).await;
    let (mut user, device) = create_user_with_device(&context.pool).await;
    link_user_oidc_identity(&context.pool, &mut user).await;

    let mock = MockOidcProvider::start().await;
    let _provider = create_oidc_provider(&context.pool, &mock).await;
    set_public_proxy_url(&context.pool, &mock.base_url).await;

    let (_id, mfa_token) = send_mfa_start(
        &mut context,
        network.id,
        &device.wireguard_pubkey,
        MfaMethod::Oidc,
    )
    .await;

    let state = build_state(Some(MfaOidcState::build(&mfa_token, "stale-attempt-id")))
        .secret()
        .clone();

    // A code for an account that does not exist: verification fails, and that failure path is one
    // of the branches that deletes the session.
    let raw_nonce = "mfa-oidc-stale-delete-nonce";
    let oidc_code = make_oidc_code("no-such-sub", "no-such-user@example.com", raw_nonce);

    context.mock_proxy().send_request(CoreRequest {
        id: 32,
        device_info: Some(make_device_info()),
        payload: Some(core_request::Payload::ClientMfaOidcAuthenticate(
            ClientMfaOidcAuthenticateRequest {
                code: oidc_code,
                state,
                nonce: raw_nonce.to_owned(),
            },
        )),
    });

    // Rejected for the stale attempt id, not for the bad code: the binding is checked first.
    let response = context.mock_proxy_mut().recv_outbound().await;
    let error_code = assert_error_response(&response);
    assert_eq!(
        error_code,
        tonic::Code::InvalidArgument,
        "expected InvalidArgument for a stale attempt id"
    );

    // The decisive assertion: the live session survived a failing callback bound to a dead attempt.
    let session = VpnClientMfaSession::<Id>::find_active_by_token(&context.pool, &mfa_token)
        .await
        .expect("failed to find active MFA session")
        .expect("a stale callback must not delete the live session");
    assert!(
        !session
            .ephemeral_state
            .as_ref()
            .expect("expected an attempt in progress")
            .openid_auth_completed,
        "stale callback must not mark the attempt complete"
    );

    clear_test_license();
    context.finish().await.expect_server_finished().await;
}

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
            nonce: raw_nonce.to_owned(),
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

/// When the provider marks the email as unverified, the callback must not merge
/// the identity into a pre-existing account: it returns `PermissionDenied` and
/// leaves the target user's `openid_sub` unset.
#[sqlx::test]
async fn test_auth_callback_unverified_email_does_not_merge_into_existing_account(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;
    set_test_license_business();

    // Target account: pre-existing and never used OIDC, so `openid_sub` is NULL.
    let target = create_user(&context.pool).await;

    let mock = MockOidcProvider::start().await;
    let _provider = create_oidc_provider(&context.pool, &mock).await;
    set_public_proxy_url(&context.pool, &mock.base_url).await;

    // The attacker's `sub` is unknown, the email matches the target, and the
    // provider reports it unverified.
    let raw_nonce = "test-nonce-unverified-email";
    let code = make_oidc_code_with_email_verified(
        "attacker-sub",
        &target.email,
        raw_nonce,
        EmailVerified::Unverified,
    );

    context.mock_proxy().send_request(CoreRequest {
        id: 12,
        device_info: None,
        payload: Some(core_request::Payload::AuthCallback(AuthCallbackRequest {
            code,
            nonce: raw_nonce.to_owned(),
        })),
    });

    let response = context.mock_proxy_mut().recv_outbound().await;
    let (status, message) = assert_error_response_details(&response);
    assert_eq!(
        status,
        tonic::Code::PermissionDenied,
        "expected PermissionDenied when the provider reports the email unverified"
    );
    assert!(
        message.contains("did not verify the email address"),
        "expected the unverified-email rejection, got: {message}"
    );

    // The target account must not be bound to the attacker's identity.
    let target = User::find_by_email(&context.pool, &target.email)
        .await
        .expect("db query failed for target user")
        .expect("target user should still exist");
    assert!(
        target.openid_sub.is_none(),
        "unverified email must not set openid_sub on the existing account"
    );

    clear_test_license();
    context.finish().await.expect_server_finished().await;
}

/// Many providers omit `email_verified` altogether, so an absent claim must stay
/// permissive: the identity still links to the account matching its email. Pins the
/// compatibility decision behind rejecting only an explicit `false`.
#[sqlx::test]
async fn test_auth_callback_absent_email_verified_claim_still_links_account(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;
    set_test_license_business();

    let user = create_user(&context.pool).await;

    let mock = MockOidcProvider::start().await;
    let _provider = create_oidc_provider(&context.pool, &mock).await;
    set_public_proxy_url(&context.pool, &mock.base_url).await;

    let raw_nonce = "test-nonce-absent-email-verified";
    let code = make_oidc_code_with_email_verified(
        "absent-claim-sub",
        &user.email,
        raw_nonce,
        EmailVerified::Absent,
    );

    context.mock_proxy().send_request(CoreRequest {
        id: 14,
        device_info: None,
        payload: Some(core_request::Payload::AuthCallback(AuthCallbackRequest {
            code,
            nonce: raw_nonce.to_owned(),
        })),
    });

    let response = context.mock_proxy_mut().recv_outbound().await;
    let auth_cb = match &response.payload {
        Some(core_response::Payload::AuthCallback(r)) => r,
        Some(core_response::Payload::CoreError(e)) => panic!(
            "an absent email_verified claim must not block login: status={} msg={}",
            e.status_code, e.message
        ),
        other => panic!(
            "expected AuthCallback response, got: {:?}",
            other.as_ref().map(std::mem::discriminant)
        ),
    };

    let token = Token::find_by_id(&context.pool, &auth_cb.token)
        .await
        .expect("db query failed for enrollment token");
    assert_eq!(
        token.user_id, user.id,
        "enrollment token must belong to the matched user"
    );

    clear_test_license();
    context.finish().await.expect_server_finished().await;
}

/// Emails are unique, so an account created from an unverified address claims that
/// identity for good. With no account to merge into, the callback must still fail
/// and create nothing.
#[sqlx::test]
async fn test_auth_callback_unverified_email_does_not_create_account(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;
    set_test_license_business();

    let mock = MockOidcProvider::start().await;
    let _provider = create_oidc_provider(&context.pool, &mock).await;
    set_public_proxy_url(&context.pool, &mock.base_url).await;

    // No account holds this address, so this exercises the creation path.
    let email = "no-such-user@example.com";
    let raw_nonce = "test-nonce-unverified-email-no-account";
    let code = make_oidc_code_with_email_verified(
        "attacker-sub",
        email,
        raw_nonce,
        EmailVerified::Unverified,
    );

    context.mock_proxy().send_request(CoreRequest {
        id: 13,
        device_info: None,
        payload: Some(core_request::Payload::AuthCallback(AuthCallbackRequest {
            code,
            nonce: raw_nonce.to_owned(),
        })),
    });

    let response = context.mock_proxy_mut().recv_outbound().await;
    let (status, message) = assert_error_response_details(&response);
    assert_eq!(
        status,
        tonic::Code::PermissionDenied,
        "expected PermissionDenied when the provider reports the email unverified"
    );
    assert!(
        message.contains("did not verify the email address"),
        "expected the unverified-email rejection, got: {message}"
    );

    assert!(
        User::find_by_email(&context.pool, email)
            .await
            .expect("db query failed for the claimed email")
            .is_none(),
        "unverified email must not create an account"
    );

    clear_test_license();
    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_auth_callback_blocked_by_license_limit_emits_user_import_blocked_event(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;

    // Reach the user limit: one existing user, license capped at one user.
    create_user(&context.pool).await;
    update_counts(&context.pool)
        .await
        .expect("failed to refresh license usage counts");
    set_cached_license(Some(License {
        customer_id: "test-customer-id".into(),
        subscription: false,
        valid_until: None,
        limits: Some(LicenseLimits {
            users: 1,
            devices: 100,
            locations: 100,
            network_devices: Some(100),
        }),
        version_date_limit: None,
        tier: LicenseTier::Business,
        support_type: SupportType::Basic,
        features: vec![],
    }));

    let mock = MockOidcProvider::start().await;
    let _provider = create_oidc_provider(&context.pool, &mock).await;
    set_public_proxy_url(&context.pool, &mock.base_url).await;

    let raw_nonce = "test-nonce-license-limit";
    let email = "blocked-oidc-user@example.com";
    let code = make_oidc_code("blocked-oidc-user-sub", email, raw_nonce);

    context.mock_proxy().send_request(CoreRequest {
        id: 20,
        device_info: None,
        payload: Some(core_request::Payload::AuthCallback(AuthCallbackRequest {
            code,
            nonce: raw_nonce.to_owned(),
        })),
    });

    let response = context.mock_proxy_mut().recv_outbound().await;
    let code = assert_error_response(&response);
    assert_eq!(
        code,
        tonic::Code::ResourceExhausted,
        "expected ResourceExhausted status when license user limit is reached"
    );

    let ApiEvent { event, .. } = timeout(RECEIVE_TIMEOUT, context.event_rx.recv())
        .await
        .expect("timed out waiting for UserImportBlocked activity log event")
        .expect("event channel closed");
    match *event {
        ApiEventType::UserImportBlocked {
            email: blocked_email,
            user_count,
            limit,
            ..
        } => {
            assert_eq!(blocked_email, email);
            assert_eq!(user_count, 1);
            assert_eq!(limit, 1);
        }
        other => panic!("expected UserImportBlocked event, got: {other:?}"),
    }

    clear_test_license();
    context.finish().await.expect_server_finished().await;
}
