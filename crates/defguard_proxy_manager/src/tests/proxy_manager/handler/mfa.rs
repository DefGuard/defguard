// Phase 4: MFA handler tests
//
// Covers:
//  1. test_mfa_start_fails_for_disabled_location
//  2. test_mfa_start_fails_for_unknown_location
//  3. test_mfa_start_fails_for_unknown_device
//  4. test_mfa_start_fails_when_email_mfa_not_enabled
//  5. test_mfa_start_returns_token_for_email_mfa
//  6. test_mfa_finish_succeeds_and_creates_session
//  7. test_mfa_token_valid_before_finish_invalid_after
//  8. test_mfa_finish_fails_with_wrong_code
//  9. test_mfa_oidc_start_requires_license
// 10. test_mfa_await_remote_receives_psk_after_finish

// ---------------------------------------------------------------------------
// 1. MFA start fails when the location has MFA disabled
// ---------------------------------------------------------------------------

#[sqlx::test]
async fn test_mfa_start_fails_for_disabled_location(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;

    // create a network with MFA *disabled* (the default)
    let network = create_network(&context.pool).await;
    let (_, device) = create_user_with_device(&context.pool).await;

    context.mock_proxy().send_request(CoreRequest {
        id: 1,
        device_info: None,
        payload: Some(core_request::Payload::ClientMfaStart(
            ClientMfaStartRequest {
                location_id: network.id,
                pubkey: device.wireguard_pubkey.clone(),
                method: MfaMethod::Email as i32,
            },
        )),
    });

    let response = context.mock_proxy_mut().recv_outbound().await;
    let code = assert_error_response(&response);
    assert_eq!(code, tonic::Code::InvalidArgument);

    context.finish().await.expect_server_finished().await;
}

// ---------------------------------------------------------------------------
// 2. MFA start fails for an unknown location
// ---------------------------------------------------------------------------

#[sqlx::test]
async fn test_mfa_start_fails_for_unknown_location(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;

    let (_, device) = create_user_with_device(&context.pool).await;

    context.mock_proxy().send_request(CoreRequest {
        id: 1,
        device_info: None,
        payload: Some(core_request::Payload::ClientMfaStart(
            ClientMfaStartRequest {
                location_id: 99999,
                pubkey: device.wireguard_pubkey.clone(),
                method: MfaMethod::Email as i32,
            },
        )),
    });

    let response = context.mock_proxy_mut().recv_outbound().await;
    let code = assert_error_response(&response);
    assert_eq!(code, tonic::Code::InvalidArgument);

    context.finish().await.expect_server_finished().await;
}

// ---------------------------------------------------------------------------
// 3. MFA start fails when the device is unknown
// ---------------------------------------------------------------------------

#[sqlx::test]
async fn test_mfa_start_fails_for_unknown_device(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;

    let network = create_mfa_network(&context.pool).await;

    context.mock_proxy().send_request(CoreRequest {
        id: 1,
        device_info: None,
        payload: Some(core_request::Payload::ClientMfaStart(
            ClientMfaStartRequest {
                location_id: network.id,
                pubkey: "no-such-pubkey".to_string(),
                method: MfaMethod::Email as i32,
            },
        )),
    });

    let response = context.mock_proxy_mut().recv_outbound().await;
    let code = assert_error_response(&response);
    assert_eq!(code, tonic::Code::InvalidArgument);

    context.finish().await.expect_server_finished().await;
}

// ---------------------------------------------------------------------------
// 4. MFA start fails when the user has not enabled email MFA
// ---------------------------------------------------------------------------

#[sqlx::test]
async fn test_mfa_start_fails_when_email_mfa_not_enabled(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;

    let network = create_mfa_network(&context.pool).await;
    // device is created after the network so add_to_all_networks picks it up
    let (_, device) = create_user_with_device(&context.pool).await;
    // user.email_mfa_enabled is false by default — no setup call

    context.mock_proxy().send_request(CoreRequest {
        id: 1,
        device_info: None,
        payload: Some(core_request::Payload::ClientMfaStart(
            ClientMfaStartRequest {
                location_id: network.id,
                pubkey: device.wireguard_pubkey.clone(),
                method: MfaMethod::Email as i32,
            },
        )),
    });

    let response = context.mock_proxy_mut().recv_outbound().await;
    let code = assert_error_response(&response);
    assert_eq!(code, tonic::Code::InvalidArgument);

    context.finish().await.expect_server_finished().await;
}

// ---------------------------------------------------------------------------
// 5. MFA start returns a JWT token for a properly configured email-MFA user
// ---------------------------------------------------------------------------

#[sqlx::test]
async fn test_mfa_start_returns_token_for_email_mfa(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;

    let network = create_mfa_network(&context.pool).await;
    let (mut user, device) = create_user_with_device(&context.pool).await;
    setup_user_email_mfa(&context.pool, &mut user).await;

    context.mock_proxy().send_request(CoreRequest {
        id: 1,
        device_info: None,
        payload: Some(core_request::Payload::ClientMfaStart(
            ClientMfaStartRequest {
                location_id: network.id,
                pubkey: device.wireguard_pubkey.clone(),
                method: MfaMethod::Email as i32,
            },
        )),
    });

    let response = context.mock_proxy_mut().recv_outbound().await;
    match &response.payload {
        Some(core_response::Payload::ClientMfaStart(r)) => {
            assert!(!r.token.is_empty(), "token must not be empty");
        }
        other => panic!(
            "expected ClientMfaStart response, got: {:?}",
            other.as_ref().map(|p| std::mem::discriminant(p))
        ),
    }

    context.finish().await.expect_server_finished().await;
}

// ---------------------------------------------------------------------------
// 6. MFA finish succeeds, persists VpnClientSession, emits gateway & bidi events
// ---------------------------------------------------------------------------

#[sqlx::test]
async fn test_mfa_finish_succeeds_and_creates_session(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;

    let network = create_mfa_network(&context.pool).await;
    let (mut user, device) = create_user_with_device(&context.pool).await;
    // Setup email MFA — the code is the same one that start_client_mfa_login
    // will regenerate internally, so we can generate it once here.
    let code = setup_user_email_mfa(&context.pool, &mut user).await;

    let (_, token) = send_mfa_start(
        &mut context,
        network.id,
        &device.wireguard_pubkey,
        MfaMethod::Email,
    )
    .await;

    // Subscribe to the gateway broadcast BEFORE calling finish, so that the
    // handler's wireguard_tx.send() has at least one active receiver (without
    // one the send would fail with SendError and return Internal).
    let mut gateway_rx = context.wireguard_tx.subscribe();

    // The start handler has already called generate_email_mfa_code internally
    // and the in-memory secret is still the same, so regenerating here gives
    // the same code.
    let _ = code; // keep binding so the setup_user_email_mfa call is not dead
    // Regenerate for the finish call (same secret → same code while within window)
    let finish_code = user.generate_email_mfa_code().expect("generate email code");

    let (_, psk) = send_mfa_finish(&mut context, &token, Some(&finish_code)).await;
    assert!(!psk.is_empty(), "preshared key must not be empty");

    // Verify VpnClientSession was persisted
    let session = assert_vpn_session_exists(&context.pool, network.id, device.id).await;
    assert!(session.preshared_key.is_some());

    // Verify GatewayEvent::MfaSessionAuthorized was broadcast
    let event = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        gateway_rx.recv(),
    )
    .await
    .expect("timed out waiting for GatewayEvent::MfaSessionAuthorized")
    .expect("gateway event channel closed");
    let loc_id = match event {
        GatewayEvent::MfaSessionAuthorized(loc_id, _, _) => loc_id,
        other => panic!("expected MfaSessionAuthorized, got: {other:?}"),
    };
    assert_eq!(loc_id, network.id);

    // Verify BidiStreamEvent::DesktopClientMfa(Success) was sent
    let event_loc_id = expect_bidi_mfa_success(&mut context.bidi_events_rx).await;
    assert_eq!(event_loc_id, network.id);

    context.finish().await.expect_server_finished().await;
}

// ---------------------------------------------------------------------------
// 7. Token is valid after start, invalid after finish
// ---------------------------------------------------------------------------

#[sqlx::test]
async fn test_mfa_token_valid_before_finish_invalid_after(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;

    let network = create_mfa_network(&context.pool).await;
    let (mut user, device) = create_user_with_device(&context.pool).await;
    setup_user_email_mfa(&context.pool, &mut user).await;

    let (_, token) = send_mfa_start(
        &mut context,
        network.id,
        &device.wireguard_pubkey,
        MfaMethod::Email,
    )
    .await;

    // Token should be valid while session is in-progress
    let valid = send_token_validation(&mut context, &token).await;
    assert!(valid, "token must be valid after start");

    // Subscribe before finish so the handler's wireguard_tx.send() has a receiver
    let _gateway_rx = context.wireguard_tx.subscribe();

    let code = user.generate_email_mfa_code().expect("generate email code");
    send_mfa_finish(&mut context, &token, Some(&code)).await;

    // After finish the session is removed, so token is no longer valid
    let valid_after = send_token_validation(&mut context, &token).await;
    assert!(!valid_after, "token must be invalid after finish");

    context.finish().await.expect_server_finished().await;
}

// ---------------------------------------------------------------------------
// 8. MFA finish fails with a wrong code
// ---------------------------------------------------------------------------

#[sqlx::test]
async fn test_mfa_finish_fails_with_wrong_code(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;

    let network = create_mfa_network(&context.pool).await;
    let (mut user, device) = create_user_with_device(&context.pool).await;
    setup_user_email_mfa(&context.pool, &mut user).await;

    let (_, token) = send_mfa_start(
        &mut context,
        network.id,
        &device.wireguard_pubkey,
        MfaMethod::Email,
    )
    .await;

    // Send a clearly wrong code
    let id = 9990u64;
    context.mock_proxy().send_request(CoreRequest {
        id,
        device_info: Some(make_device_info()),
        payload: Some(core_request::Payload::ClientMfaFinish(
            ClientMfaFinishRequest {
                token: token.clone(),
                code: Some("000000".to_string()),
                auth_pub_key: None,
            },
        )),
    });

    let response = context.mock_proxy_mut().recv_outbound().await;
    let code = assert_error_response(&response);
    // invalid code → InvalidArgument or Unauthenticated
    assert!(
        matches!(
            code,
            tonic::Code::InvalidArgument | tonic::Code::Unauthenticated
        ),
        "expected InvalidArgument or Unauthenticated, got: {code:?}"
    );

    context.finish().await.expect_server_finished().await;
}

// ---------------------------------------------------------------------------
// 9. MFA start with OIDC method fails without a business license
// ---------------------------------------------------------------------------

#[sqlx::test]
async fn test_mfa_oidc_start_requires_license(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;

    clear_test_license();

    // External MFA location + OIDC method but no business license
    let network = create_external_mfa_network(&context.pool).await;
    let (mut user, device) = create_user_with_device(&context.pool).await;
    // email MFA is irrelevant for OIDC path but user still needs to exist
    setup_user_email_mfa(&context.pool, &mut user).await;

    context.mock_proxy().send_request(CoreRequest {
        id: 1,
        device_info: None,
        payload: Some(core_request::Payload::ClientMfaStart(
            ClientMfaStartRequest {
                location_id: network.id,
                pubkey: device.wireguard_pubkey.clone(),
                method: MfaMethod::Oidc as i32,
            },
        )),
    });

    let response = context.mock_proxy_mut().recv_outbound().await;
    let code = assert_error_response(&response);
    assert_eq!(code, tonic::Code::InvalidArgument);

    context.finish().await.expect_server_finished().await;
}

// ---------------------------------------------------------------------------
// 10. AwaitRemoteMfaFinish receives the PSK after ClientMfaFinish completes
// ---------------------------------------------------------------------------

#[sqlx::test]
async fn test_mfa_await_remote_receives_psk_after_finish(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;

    let network = create_mfa_network(&context.pool).await;
    let (mut user, device) = create_user_with_device(&context.pool).await;
    setup_user_email_mfa(&context.pool, &mut user).await;

    let (_, token) = send_mfa_start(
        &mut context,
        network.id,
        &device.wireguard_pubkey,
        MfaMethod::Email,
    )
    .await;

    // Send AwaitRemoteMfaFinish first — no immediate response expected
    let await_id = 8000u64;
    context.mock_proxy().send_request(CoreRequest {
        id: await_id,
        device_info: None,
        payload: Some(core_request::Payload::AwaitRemoteMfaFinish(
            AwaitRemoteMfaFinishRequest { token: token.clone() },
        )),
    });

    // Give the handler a moment to register the oneshot receiver before we
    // proceed with the finish call.
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // Subscribe before finish so the handler's wireguard_tx.send() has a receiver
    let _gateway_rx = context.wireguard_tx.subscribe();

    // Now finish the MFA login with the correct code
    let code = user.generate_email_mfa_code().expect("generate email code");
    let finish_id = 8001u64;
    context.mock_proxy().send_request(CoreRequest {
        id: finish_id,
        device_info: Some(make_device_info()),
        payload: Some(core_request::Payload::ClientMfaFinish(
            ClientMfaFinishRequest {
                token: token.clone(),
                code: Some(code),
                auth_pub_key: None,
            },
        )),
    });

    // Two responses should arrive: one ClientMfaFinish and one
    // AwaitRemoteMfaFinish — order is not guaranteed.
    let r1 = context.mock_proxy_mut().recv_outbound().await;
    let r2 = context.mock_proxy_mut().recv_outbound().await;

    let mut got_finish = false;
    let mut got_await = false;
    for r in [&r1, &r2] {
        match &r.payload {
            Some(core_response::Payload::ClientMfaFinish(fr)) => {
                assert!(!fr.preshared_key.is_empty());
                got_finish = true;
            }
            Some(core_response::Payload::AwaitRemoteMfaFinish(ar)) => {
                assert!(!ar.preshared_key.is_empty());
                got_await = true;
            }
            other => panic!(
                "unexpected response payload: {:?}",
                other.as_ref().map(|p| std::mem::discriminant(p))
            ),
        }
    }
    assert!(got_finish, "missing ClientMfaFinish response");
    assert!(got_await, "missing AwaitRemoteMfaFinish response");

    context.finish().await.expect_server_finished().await;
}
