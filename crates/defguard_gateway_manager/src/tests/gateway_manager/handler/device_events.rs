#[sqlx::test]
async fn test_device_created_for_network_produces_peer_create_update(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;
    let expected_keepalive_interval = expected_keepalive_interval(&context);

    let _ = context.complete_config_handshake().await;
    let device_info = create_device_info_for_current_network(
        &context,
        "created-peer-device",
        "LQKsT6/3HWKuJmMulH63R8iK+5sI8FyYEL6WDIi6lQU=",
        "10.10.0.10",
    )
    .await;

    assert_send_ok!(
        context
            .events_tx()
            .send(GatewayEvent::DeviceCreated(device_info)),
        "failed to broadcast created device event"
    );

    let outbound = context.mock_gateway_mut().recv_outbound().await;
    assert_peer_update(
        outbound,
        UpdateType::Create,
        "LQKsT6/3HWKuJmMulH63R8iK+5sI8FyYEL6WDIi6lQU=",
        &["10.10.0.10"],
        None,
        Some(expected_keepalive_interval),
    );
    context.mock_gateway_mut().expect_no_outbound().await;

    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_device_created_before_config_handshake_is_ignored(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    assert_device_event_is_ignored_before_config_handshake(
        options,
        "created-before-config-device",
        "tND8hJQhYnI8naBTo59He43zYldagfjlwmSxWEc01Cc=",
        "10.10.0.11",
        GatewayEvent::DeviceCreated,
    )
    .await;
}

#[sqlx::test]
async fn test_device_modified_for_network_produces_peer_modify_update(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;
    let expected_keepalive_interval = expected_keepalive_interval(&context);

    let _ = context.complete_config_handshake().await;
    let device = create_device_for_network(
        &context,
        context.network.id,
        "modified-peer-device",
        "TJgN9JzUF5zdZAPYD96G/Wys2M3TvaT5TIrErUl20nI=",
        "10.10.0.20",
    )
    .await;

    let mut network_device = WireguardNetworkDevice::find(&context.pool, device.id, context.network.id)
        .await
        .expect("failed to load device network info")
        .expect("expected device network info for modified device");
    network_device.wireguard_ips = vec![parse_test_ip("10.10.0.21")];
    network_device
        .update(&context.pool)
        .await
        .expect("failed to update device network info");

    let device_info = DeviceInfo::from_device(&context.pool, device)
        .await
        .expect("failed to load modified device info");

    assert_send_ok!(
        context
            .events_tx()
            .send(GatewayEvent::DeviceModified(device_info)),
        "failed to broadcast modified device event"
    );

    let outbound = context.mock_gateway_mut().recv_outbound().await;
    assert_peer_update(
        outbound,
        UpdateType::Modify,
        "TJgN9JzUF5zdZAPYD96G/Wys2M3TvaT5TIrErUl20nI=",
        &["10.10.0.21"],
        None,
        Some(expected_keepalive_interval),
    );
    context.mock_gateway_mut().expect_no_outbound().await;

    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_device_modified_before_config_handshake_is_ignored(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    assert_device_event_is_ignored_before_config_handshake(
        options,
        "modified-before-config-device",
        "wyFOHCec/Fi9s+cARikVO71JhyYtYMk0FrQx3fK2PTM=",
        "10.10.0.22",
        GatewayEvent::DeviceModified,
    )
    .await;
}

#[sqlx::test]
async fn test_device_deleted_for_network_produces_peer_delete_update(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;

    let _ = context.complete_config_handshake().await;
    let device_info = create_device_info_for_current_network(
        &context,
        "deleted-peer-device",
        "PKY3zg5/ecNyMjqLi6yJ3jwb4PvC/SGzjhJ3jrn2vVQ=",
        "10.10.0.30",
    )
    .await;

    assert_send_ok!(
        context
            .events_tx()
            .send(GatewayEvent::DeviceDeleted(device_info)),
        "failed to broadcast deleted device event"
    );

    let outbound = context.mock_gateway_mut().recv_outbound().await;
    assert_peer_update(
        outbound,
        UpdateType::Delete,
        "PKY3zg5/ecNyMjqLi6yJ3jwb4PvC/SGzjhJ3jrn2vVQ=",
        &[],
        None,
        None,
    );
    context.mock_gateway_mut().expect_no_outbound().await;

    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_device_deleted_before_config_handshake_is_ignored(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    assert_device_event_is_ignored_before_config_handshake(
        options,
        "deleted-before-config-device",
        "m84QJmDMkqdCj8AB2NTE8F55W7M/i3CaaD3eQbQdInY=",
        "10.10.0.31",
        GatewayEvent::DeviceDeleted,
    )
    .await;
}

#[sqlx::test]
async fn test_device_created_for_different_network_is_ignored(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    assert_device_event_for_different_network_is_ignored(
        options,
        "created-other-network-device",
        "W6wBmd8wgTwvCyGqDRXk6Hf4OMqDUbUn2XWKnG5wVVQ=",
        "10.11.0.10",
        GatewayEvent::DeviceCreated,
    )
    .await;
}

#[sqlx::test]
async fn test_device_modified_for_different_network_is_ignored(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    assert_device_event_for_different_network_is_ignored(
        options,
        "modified-other-network-device",
        "yjuzq0cLk3Ww5oQcqK6YkSKwXnqQ1V9OlSMFAEkr0lU=",
        "10.11.0.20",
        GatewayEvent::DeviceModified,
    )
    .await;
}

#[sqlx::test]
async fn test_device_deleted_for_different_network_is_ignored(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    assert_device_event_for_different_network_is_ignored(
        options,
        "deleted-other-network-device",
        "Jtp+K8xnFXuF4cae+tVGZNwoSM2fXjJbRl3sI6rdcAQ=",
        "10.11.0.30",
        GatewayEvent::DeviceDeleted,
    )
    .await;
}
