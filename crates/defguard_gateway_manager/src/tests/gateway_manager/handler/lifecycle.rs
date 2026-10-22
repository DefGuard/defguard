use crate::tests::common::{
    ManagerTestContext, MockGatewayHarness, complete_manager_handshake, create_gateway,
    create_network,
};

#[sqlx::test]
async fn test_gateway_is_marked_connected_after_successful_config_handshake(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;

    let gateway_before = context.reload_gateway().await;
    assert!(!gateway_before.is_connected());

    let gateway_after = context.complete_config_handshake().await;
    assert!(gateway_after.is_connected());
    assert!(gateway_after.connected_at.is_some());
    let gateway_id = context.gateway.id;
    let gateway_name = context.gateway.name.clone();
    let mut connection_events_rx = context.take_connection_events_rx();
    assert_eq!(
        connection_events_rx.recv().await,
        Some(GatewayConnectionEvent::Connected {
            gateway_id,
            gateway_name,
        })
    );

    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_gateway_is_marked_disconnected_when_stream_closes(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;

    let connected_gateway = context.complete_config_handshake().await;
    assert!(connected_gateway.is_connected());
    let gateway_id = context.gateway.id;
    let gateway_name = context.gateway.name.clone();
    let mut connection_events_rx = context.take_connection_events_rx();

    let pool = context.pool.clone();
    let mut mock_gateway = context.finish().await;
    let disconnected_gateway = reload_gateway(&pool, gateway_id).await;
    assert!(!disconnected_gateway.is_connected());
    assert!(disconnected_gateway.disconnected_at.is_some());
    assert_eq!(
        connection_events_rx.recv().await,
        Some(GatewayConnectionEvent::Connected {
            gateway_id,
            gateway_name: gateway_name.clone(),
        })
    );
    assert_eq!(
        connection_events_rx.recv().await,
        Some(GatewayConnectionEvent::Disconnected {
            gateway_id,
            gateway_name,
        })
    );

    mock_gateway.wait_for_core_stream_closed().await;
    mock_gateway.expect_server_finished().await;
}

#[sqlx::test]
async fn test_gateway_manager_shutdown_closes_stream_with_updates_handler_running(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = ManagerTestContext::new(options).await;
    let network = create_network(&context.pool).await;
    let gateway = create_gateway(&context.pool, network.id).await;
    let mut mock_gateway = MockGatewayHarness::start().await;
    context.register_gateway_mock(&gateway, &mock_gateway);

    context.start().await;
    complete_manager_handshake(&context, &gateway, &mut mock_gateway).await;

    let mut updated_network = network.clone();
    updated_network.name.push_str("-shutdown-test");
    context.send_event(GatewayCommand::NetworkModified(
        updated_network,
        Vec::new(),
        None,
    ));
    let outbound = mock_gateway.recv_outbound().await;
    assert!(matches!(
        outbound.payload,
        Some(core_response::Payload::Update(_))
    ));

    context.request_shutdown().await;

    let stopped_gateway = reload_gateway(&context.pool, gateway.id).await;
    assert!(!stopped_gateway.is_connected());
    let connected_at = stopped_gateway
        .connected_at
        .expect("connected_at should be set after handshake");
    let disconnected_at = stopped_gateway
        .disconnected_at
        .expect("disconnected_at should be set after manager shutdown");
    assert!(
        disconnected_at > connected_at,
        "disconnected_at must be later than connected_at after shutdown"
    );

    mock_gateway.wait_for_core_stream_closed().await;
    context.finish().await;
}

#[sqlx::test]
async fn test_gateway_is_marked_disconnected_when_stream_errors(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;

    let _ = context.complete_config_handshake().await;

    context
        .mock_gateway()
        .send_stream_error(Status::internal("mock gateway stream failure"));

    let pool = context.pool.clone();
    let gateway_id = context.gateway.id;
    let mock_gateway = context.finish_after_error().await;
    let disconnected_gateway = reload_gateway(&pool, gateway_id).await;
    assert!(!disconnected_gateway.is_connected());
    assert!(disconnected_gateway.disconnected_at.is_some());

    mock_gateway.expect_server_finished().await;
}
