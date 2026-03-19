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

    let pool = context.pool.clone();
    let gateway_id = context.gateway.id;
    let mock_gateway = context.finish().await;
    let disconnected_gateway = reload_gateway(&pool, gateway_id).await;
    assert!(!disconnected_gateway.is_connected());
    assert!(disconnected_gateway.disconnected_at.is_some());

    mock_gateway.expect_server_finished().await;
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
