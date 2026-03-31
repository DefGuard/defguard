/// New device enrollment and existing device network-info tests.
///
/// These tests use `HandlerTestContext` (single `run_once` handler) and inject
/// `CoreRequest` messages through the mock proxy harness, asserting on the
/// `CoreResponse` payloads that come back.

#[sqlx::test]
async fn test_new_device_creates_device_and_returns_configs(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;

    // Create a network so `add_to_all_networks` has somewhere to assign the device.
    let _network = create_network(&context.pool).await;

    // Create user and enrollment token.
    let user = create_user(&context.pool).await;
    // Pass Some(user.id) as admin_id so the enrollment welcome-page template
    // can render {{ admin_first_name }} etc. without failing.
    let token = create_enrollment_token(&context.pool, user.id, Some(user.id)).await;

    // Start the enrollment session so Token::used_at is set.
    start_enrollment_session(&mut context, &token.id).await;

    // Inject NewDevice request.
    let pubkey = "AA0aJzRBTltodYKPnKm2w9Dd6vcEER4rOEVSX2x5hpM=";
    context.mock_proxy().send_request(CoreRequest {
        id: 1,
        device_info: Some(make_device_info()),
        payload: Some(core_request::Payload::NewDevice(NewDevice {
            name: "My Laptop".to_string(),
            pubkey: pubkey.to_string(),
            token: Some(token.id.clone()),
        })),
    });

    // The handler should respond with a DeviceConfig.
    let response = context.mock_proxy_mut().recv_outbound().await;
    let cfg = assert_device_config_response(&response);
    assert!(cfg.device.is_some(), "DeviceConfigResponse should contain device");
    assert!(
        !cfg.configs.is_empty(),
        "DeviceConfigResponse should contain at least one network config"
    );

    // Verify the device was actually persisted in the DB.
    let devices = Device::find_by_pubkey(&context.pool, pubkey)
        .await
        .expect("DB query failed");
    assert!(devices.is_some(), "device should exist in DB after NewDevice");

    context.finish().await.expect_server_finished().await;
}

// ---------------------------------------------------------------------------
// ActivateUser tests
// ---------------------------------------------------------------------------

/// A valid password that satisfies `check_password_strength`:
/// ≥8 chars, digit, upper, lower, special character.
const STRONG_PASSWORD: &str = "Test1234!";

/// Happy path: submit a strong password through an active enrollment session →
/// handler returns `Empty`, the user's `enrollment_pending` flag is cleared, and
/// the user gains a password hash in the DB.  A `BidiStreamEvent::Enrollment`
/// `EnrollmentCompleted` event must also be emitted.
#[sqlx::test]
async fn test_activate_user_happy_path(_: PgPoolOptions, options: PgConnectOptions) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;

    let user = create_user(&context.pool).await;
    let token = create_enrollment_token(&context.pool, user.id, None).await;
    start_enrollment_session(&mut context, &token.id).await;

    let response = send_activate_user(&mut context, &token.id, STRONG_PASSWORD, None).await;

    // Must receive Empty on success.
    match &response.payload {
        Some(core_response::Payload::Empty(())) => {}
        _ => panic!("expected Empty response"),
    }

    // User must have a password hash in DB and enrollment_pending cleared.
    let updated = User::find_by_username(&context.pool, &user.username)
        .await
        .expect("db query failed")
        .expect("user not found");
    assert!(updated.has_password(), "user must have a password hash after activation");
    assert!(
        !updated.enrollment_pending,
        "enrollment_pending must be false after activation"
    );

    // A BidiStreamEvent::Enrollment(EnrollmentCompleted) must have been emitted.
    let event = tokio::time::timeout(TEST_TIMEOUT, context.bidi_events_rx.recv())
        .await
        .expect("timed out waiting for BidiStreamEvent")
        .expect("bidi_events_rx closed");
    match event.event {
        BidiStreamEventType::Enrollment(e) => match *e {
            EnrollmentEvent::EnrollmentCompleted => {}
            other => panic!("expected EnrollmentCompleted event, got: {other:?}"),
        },
        other => panic!("expected BidiStreamEventType::Enrollment, got: {other:?}"),
    }

    context.finish().await.expect_server_finished().await;
}

/// A weak password (too short, missing required character classes) must be
/// rejected with `InvalidArgument`.
#[sqlx::test]
async fn test_activate_user_weak_password_returns_error(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;

    let user = create_user(&context.pool).await;
    let token = create_enrollment_token(&context.pool, user.id, None).await;
    start_enrollment_session(&mut context, &token.id).await;

    let response = send_activate_user(&mut context, &token.id, "weak", None).await;

    let code = assert_error_response(&response);
    assert_eq!(
        code,
        tonic::Code::InvalidArgument,
        "weak password must return InvalidArgument"
    );

    context.finish().await.expect_server_finished().await;
}

/// Calling `ActivateUser` twice on the same account must fail the second time
/// with `InvalidArgument` because the user already has a password hash.
#[sqlx::test]
async fn test_activate_user_already_activated_returns_error(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;

    let user = create_user(&context.pool).await;
    let token = create_enrollment_token(&context.pool, user.id, None).await;
    start_enrollment_session(&mut context, &token.id).await;

    // First activation — must succeed.
    let first = send_activate_user(&mut context, &token.id, STRONG_PASSWORD, None).await;
    match &first.payload {
        Some(core_response::Payload::Empty(())) => {}
        _ => panic!("expected Empty on first activation"),
    }
    // Consume the EnrollmentCompleted bidi event so the channel doesn't fill.
    let _ = tokio::time::timeout(TEST_TIMEOUT, context.bidi_events_rx.recv()).await;

    // Create a fresh enrollment token (old one is now used), start a new session.
    let token2 = create_enrollment_token(&context.pool, user.id, None).await;
    start_enrollment_session(&mut context, &token2.id).await;

    // Second activation — must fail with InvalidArgument.
    let second = send_activate_user(&mut context, &token2.id, STRONG_PASSWORD, None).await;
    let code = assert_error_response(&second);
    assert_eq!(
        code,
        tonic::Code::InvalidArgument,
        "activating an already-activated user must return InvalidArgument"
    );

    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_new_device_sends_gateway_device_created_event(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;

    let _network = create_network(&context.pool).await;
    let user = create_user(&context.pool).await;
    let token = create_enrollment_token(&context.pool, user.id, Some(user.id)).await;

    // Start the enrollment session so Token::used_at is set.
    start_enrollment_session(&mut context, &token.id).await;

    // Subscribe to gateway events BEFORE sending the request.
    let mut gateway_rx = context.wireguard_tx.subscribe();

    let pubkey = "DhsoNUJPXGl2g5CdqrfE0d7r+AUSHyw5RlNgbXqHlKE=";
    context.mock_proxy().send_request(CoreRequest {
        id: 3,
        device_info: Some(make_device_info()),
        payload: Some(core_request::Payload::NewDevice(NewDevice {
            name: "My Tablet".to_string(),
            pubkey: pubkey.to_string(),
            token: Some(token.id.clone()),
        })),
    });

    // Wait for response first to ensure the handler has processed the request.
    let _response = context.mock_proxy_mut().recv_outbound().await;

    // Check that a DeviceCreated event was broadcast.
    let event = tokio::time::timeout(TEST_TIMEOUT, gateway_rx.recv())
        .await
        .expect("timed out waiting for GatewayEvent::DeviceCreated")
        .expect("gateway event channel closed");

    assert!(
        matches!(event, GatewayEvent::DeviceCreated(_)),
        "expected DeviceCreated gateway event, got: {event:?}"
    );

    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_new_device_invalid_token_returns_error(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;

    // Send a NewDevice request with a token that doesn't exist.
    context.mock_proxy().send_request(CoreRequest {
        id: 4,
        device_info: None,
        payload: Some(core_request::Payload::NewDevice(NewDevice {
            name: "Ghost Device".to_string(),
            pubkey: "FSIvPElWY3B9ipeksb7L2OXy/wwZJjNATVpndIGOm6g=".to_string(),
            token: Some("nonexistent-token-0000000000000000".to_string()),
        })),
    });

    let response = context.mock_proxy_mut().recv_outbound().await;
    // The handler should return an error response.
    let code = assert_error_response(&response);
    assert_ne!(code, tonic::Code::Ok, "expected error code for invalid token");

    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_existing_device_returns_config_and_rotates_polling_token(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;

    // Set up: user, device with a known public key, network, and an initial polling token.
    let _network = create_network(&context.pool).await;
    let (user, device) = create_user_with_device(&context.pool).await;
    let old_token = create_polling_token(&context.pool, device.id).await;

    // Create a valid enrollment token for the same user.
    let enrollment_token = create_enrollment_token(&context.pool, user.id, Some(user.id)).await;

    // Start the enrollment session so Token::used_at is set.
    start_enrollment_session(&mut context, &enrollment_token.id).await;

    context.mock_proxy().send_request(CoreRequest {
        id: 5,
        device_info: None,
        payload: Some(core_request::Payload::ExistingDevice(ExistingDevice {
            pubkey: device.wireguard_pubkey.clone(),
            token: Some(enrollment_token.id.clone()),
        })),
    });

    let response = context.mock_proxy_mut().recv_outbound().await;
    let cfg = assert_device_config_response(&response);

    // The response must contain a new polling token.
    let new_token_str = cfg
        .token
        .as_ref()
        .expect("DeviceConfigResponse should include a polling token");
    assert_ne!(
        new_token_str, &old_token,
        "ExistingDevice should rotate the polling token"
    );

    // The old token should no longer exist in the DB.
    let old_in_db = PollingToken::find(&context.pool, &old_token)
        .await
        .expect("DB query failed");
    assert!(old_in_db.is_none(), "old polling token should be deleted after rotation");

    // The new token should exist.
    let new_in_db = PollingToken::find(&context.pool, new_token_str)
        .await
        .expect("DB query failed");
    assert!(new_in_db.is_some(), "new polling token should be in DB");

    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_existing_device_wrong_user_returns_error(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;

    // device owned by user A; enrollment token for user B.
    let (user_a, device) = create_user_with_device(&context.pool).await;
    let user_b = create_user(&context.pool).await;
    let _ = user_a; // suppress unused warning

    // Enrollment token belonging to user_b, NOT user_a (device owner).
    // No admin needed — this test only checks that an error is returned;
    // the session validation will fail before the welcome-page template renders.
    let wrong_token = create_enrollment_token(&context.pool, user_b.id, None).await;

    context.mock_proxy().send_request(CoreRequest {
        id: 6,
        device_info: None,
        payload: Some(core_request::Payload::ExistingDevice(ExistingDevice {
            pubkey: device.wireguard_pubkey.clone(),
            token: Some(wrong_token.id.clone()),
        })),
    });

    let response = context.mock_proxy_mut().recv_outbound().await;
    let code = assert_error_response(&response);
    assert_ne!(code, tonic::Code::Ok, "expected error when token owner ≠ device owner");

    context.finish().await.expect_server_finished().await;
}

// ---------------------------------------------------------------------------
// CodeMfaSetupStart / CodeMfaSetupFinish tests
// ---------------------------------------------------------------------------

/// `CodeMfaSetupStart` with `MfaMethod::Totp` must return a non-empty
/// base32-encoded TOTP secret in the response.
#[sqlx::test]
async fn test_code_mfa_setup_start_totp_returns_secret(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;

    let user = create_user(&context.pool).await;
    let token = create_enrollment_token(&context.pool, user.id, None).await;
    start_enrollment_session(&mut context, &token.id).await;

    let response = send_code_mfa_setup_start(&mut context, &token.id, MfaMethod::Totp).await;

    match &response.payload {
        Some(core_response::Payload::CodeMfaSetupStartResponse(r)) => {
            let secret = r.totp_secret.as_deref().expect("TOTP start must include a secret");
            assert!(!secret.is_empty(), "TOTP secret must be non-empty");
            // Must be valid base32 (decodable).
            assert!(
                base32::decode(base32::Alphabet::Rfc4648 { padding: false }, secret).is_some(),
                "TOTP secret must be valid RFC 4648 base32"
            );
        }
        _ => panic!("expected CodeMfaSetupStartResponse"),
    }

    context.finish().await.expect_server_finished().await;
}

/// After `CodeMfaSetupStart(Totp)` returns the secret, submitting the correct
/// TOTP code in `CodeMfaSetupFinish` must return non-empty recovery codes and
/// enable TOTP + MFA on the user account in the DB.
#[sqlx::test]
async fn test_code_mfa_setup_finish_totp_returns_recovery_codes(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;

    let user = create_user(&context.pool).await;
    let token = create_enrollment_token(&context.pool, user.id, None).await;
    start_enrollment_session(&mut context, &token.id).await;

    // Start: get the base32 TOTP secret.
    let start_resp = send_code_mfa_setup_start(&mut context, &token.id, MfaMethod::Totp).await;
    let totp_secret_b32 = match &start_resp.payload {
        Some(core_response::Payload::CodeMfaSetupStartResponse(r)) => r
            .totp_secret
            .clone()
            .expect("TOTP start must include a secret"),
        _ => panic!("expected CodeMfaSetupStartResponse"),
    };

    // Generate a valid code from the returned secret.
    let code = totp_code_from_base32_secret(&totp_secret_b32);

    // Finish: submit the code.
    let finish_resp =
        send_code_mfa_setup_finish(&mut context, &token.id, MfaMethod::Totp, &code).await;

    match &finish_resp.payload {
        Some(core_response::Payload::CodeMfaSetupFinishResponse(r)) => {
            assert!(
                !r.recovery_codes.is_empty(),
                "finish must return at least one recovery code"
            );
        }
        _ => {
            // Show the error code if it came back as CoreError.
            if let Some(core_response::Payload::CoreError(e)) = &finish_resp.payload {
                panic!("expected CodeMfaSetupFinishResponse, got CoreError: {:?}", e.message);
            }
            panic!("expected CodeMfaSetupFinishResponse");
        }
    }

    // DB: user must now have totp_enabled = true and mfa_enabled = true.
    let updated = User::find_by_username(&context.pool, &user.username)
        .await
        .expect("db query failed")
        .expect("user not found");
    assert!(updated.totp_enabled, "totp_enabled must be true after CodeMfaSetupFinish");
    assert!(updated.mfa_enabled, "mfa_enabled must be true after CodeMfaSetupFinish");

    context.finish().await.expect_server_finished().await;
}

/// `CodeMfaSetupStart` with `MfaMethod::Email` must return a response with no
/// TOTP secret (email flow does not expose a secret to the client).
///
/// Note: SMTP is not configured in tests, so the activation mail will fail and
/// the handler returns `Internal("SMTP not configured")`.  We assert that error
/// code to confirm the Email path was entered, not the TOTP path.
#[sqlx::test]
async fn test_code_mfa_setup_start_email_enters_email_path(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;

    let user = create_user(&context.pool).await;
    let token = create_enrollment_token(&context.pool, user.id, None).await;
    start_enrollment_session(&mut context, &token.id).await;

    let response = send_code_mfa_setup_start(&mut context, &token.id, MfaMethod::Email).await;

    // Without SMTP configured the handler fails with Internal (not InvalidArgument),
    // which proves the Email branch was entered (TOTP would succeed).
    let code = assert_error_response(&response);
    assert_eq!(
        code,
        tonic::Code::Internal,
        "Email MFA start without SMTP must return Internal"
    );

    context.finish().await.expect_server_finished().await;
}

/// Submitting a wrong TOTP code in `CodeMfaSetupFinish` must return
/// `InvalidArgument`.
#[sqlx::test]
async fn test_code_mfa_setup_finish_wrong_totp_code_returns_error(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;

    let user = create_user(&context.pool).await;
    let token = create_enrollment_token(&context.pool, user.id, None).await;
    start_enrollment_session(&mut context, &token.id).await;

    // Start to generate the secret (must succeed first).
    let _ = send_code_mfa_setup_start(&mut context, &token.id, MfaMethod::Totp).await;

    // Finish with a deliberately wrong code.
    let response =
        send_code_mfa_setup_finish(&mut context, &token.id, MfaMethod::Totp, "000000").await;

    let code = assert_error_response(&response);
    assert_eq!(
        code,
        tonic::Code::InvalidArgument,
        "wrong TOTP code must return InvalidArgument"
    );

    context.finish().await.expect_server_finished().await;
}

/// Requesting `CodeMfaSetupStart` with an unsupported method (e.g. `Oidc`)
/// must be rejected with `InvalidArgument`.
#[sqlx::test]
async fn test_code_mfa_setup_unsupported_method_returns_error(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;

    let user = create_user(&context.pool).await;
    let token = create_enrollment_token(&context.pool, user.id, None).await;
    start_enrollment_session(&mut context, &token.id).await;

    let response = send_code_mfa_setup_start(&mut context, &token.id, MfaMethod::Oidc).await;

    let code = assert_error_response(&response);
    assert_eq!(
        code,
        tonic::Code::InvalidArgument,
        "unsupported MFA method must return InvalidArgument"
    );

    context.finish().await.expect_server_finished().await;
}