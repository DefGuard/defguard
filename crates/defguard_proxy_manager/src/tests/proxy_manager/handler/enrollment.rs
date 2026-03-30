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

#[sqlx::test]
async fn test_new_device_creates_polling_token(
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

    let pubkey = "BxQhLjtIVWJvfImWo7C9ytfk8f4LGCUyP0xZZnOAjZo=";
    context.mock_proxy().send_request(CoreRequest {
        id: 2,
        device_info: Some(make_device_info()),
        payload: Some(core_request::Payload::NewDevice(NewDevice {
            name: "My Phone".to_string(),
            pubkey: pubkey.to_string(),
            token: Some(token.id.clone()),
        })),
    });

    let response = context.mock_proxy_mut().recv_outbound().await;
    let cfg = assert_device_config_response(&response);

    // The DeviceConfigResponse must contain the polling token.
    assert!(
        cfg.token.is_some(),
        "DeviceConfigResponse should include a polling token"
    );
    let polling_token_str = cfg.token.as_ref().unwrap();
    assert!(!polling_token_str.is_empty(), "polling token should not be empty");

    // Verify the polling token was persisted.
    let device = Device::find_by_pubkey(&context.pool, pubkey)
        .await
        .expect("DB query failed")
        .expect("device should exist after NewDevice");
    let db_token = PollingToken::find(&context.pool, polling_token_str)
        .await
        .expect("DB query failed");
    assert!(db_token.is_some(), "polling token should be in DB");
    assert_eq!(
        db_token.unwrap().device_id,
        device.id,
        "polling token should belong to the created device"
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
