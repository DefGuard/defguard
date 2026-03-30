// Phase 5: OIDC handler tests
//
// Covers:
//  1. test_auth_callback_creates_new_user_on_first_login
//     — no pre-existing user; code contains unknown sub/email; verifies core
//       creates the user and returns an enrollment token
//  2. test_auth_callback_exchanges_code_for_enrollment_token
//     — pre-existing user; verifies the handler matches by email and returns
//       a valid enrollment token id
//  3. test_mfa_oidc_full_flow
//     — ClientMfaStart (OIDC) → ClientMfaOidcAuthenticate → ClientMfaFinish
//       → PSK + GatewayEvent + BidiEvent + VpnClientSession in DB

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
            other.as_ref().map(|p| std::mem::discriminant(p))
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

    context.finish().await.expect_server_finished().await;
}

// ---------------------------------------------------------------------------
// 2. AuthCallback returns enrollment token for a pre-existing user
// ---------------------------------------------------------------------------

#[sqlx::test]
async fn test_auth_callback_exchanges_code_for_enrollment_token(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;
    set_test_license_business();

    let mock = MockOidcProvider::start().await;
    let _provider = create_oidc_provider(&context.pool, &mock).await;
    set_public_proxy_url(&context.pool, &mock.base_url).await;

    // Pre-create the user.
    let user = create_user(&context.pool).await;
    let raw_nonce = "test-nonce-2";
    // Use the user's ID as the sub (any stable string works; `user_from_claims`
    // tries to match by `sub` first, then by `email`).
    let code = make_oidc_code("some-sub", &user.email, raw_nonce);

    context.mock_proxy().send_request(CoreRequest {
        id: 20,
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
            other.as_ref().map(|p| std::mem::discriminant(p))
        ),
    };

    assert!(!auth_cb.token.is_empty(), "expected non-empty enrollment token id");

    // The DB token must reference the correct user.
    let token = Token::find_by_id(&context.pool, &auth_cb.token)
        .await
        .expect("db query failed for enrollment token");
    assert_eq!(token.user_id, user.id, "enrollment token references wrong user");

    context.finish().await.expect_server_finished().await;
}

// ---------------------------------------------------------------------------
// 3. Full OIDC MFA flow: Start → OidcAuthenticate → Finish
// ---------------------------------------------------------------------------

#[sqlx::test]
async fn test_mfa_oidc_full_flow(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
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

    // Ensure the business license is still active immediately before the MFA
    // start call (another concurrent test might have cleared it).
    set_test_license_business();

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
    let state = defguard_core::enterprise::handlers::openid_login::build_state(Some(
        mfa_token.clone(),
    ))
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
        response.payload.as_ref().map(|p| std::mem::discriminant(p))
    );

    // ---- Step 3: ClientMfaFinish (no TOTP code — session is OIDC-completed) ----
    let (_, psk) = send_mfa_finish(&mut context, &mfa_token, None).await;
    assert!(!psk.is_empty(), "expected non-empty PSK after OIDC MFA finish");

    // Verify VpnClientSession was created.
    assert_vpn_session_exists(&context.pool, network.id, device.id).await;

    // Verify BidiStreamEvent::DesktopClientMfa(Success) was emitted.
    let location_id = expect_bidi_mfa_success(&mut context.bidi_events_rx).await;
    assert_eq!(location_id, network.id);

    context.finish().await.expect_server_finished().await;
}
