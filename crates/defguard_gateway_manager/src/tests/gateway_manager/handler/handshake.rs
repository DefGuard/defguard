#[sqlx::test]
async fn test_sends_configuration_on_first_config_request(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;

    context.mock_gateway().send_config_request();
    let outbound = context.mock_gateway_mut().recv_outbound().await;

    match outbound.payload {
        Some(core_response::Payload::Config(config)) => {
            assert_eq!(config.name, context.network.name);
            assert_eq!(config.port, context.network.port as u32);
            assert_eq!(config.peers, Vec::new());
        }
        _ => panic_unexpected("expected configuration response"),
    }

    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_does_not_send_configuration_before_gateway_requests_it(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;

    let gateway_before = context.reload_gateway().await;
    assert!(!gateway_before.is_connected());

    context.mock_gateway_mut().expect_no_outbound().await;

    let gateway_after = context.reload_gateway().await;
    assert!(!gateway_after.is_connected());

    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_ignores_repeated_config_request(_: PgPoolOptions, options: PgConnectOptions) {
    let mut context = HandlerTestContext::new(options).await;

    context.mock_gateway().send_config_request();
    let first_outbound = context.mock_gateway_mut().recv_outbound().await;
    assert!(matches!(
        first_outbound.payload,
        Some(core_response::Payload::Config(_))
    ));

    context.mock_gateway().send_config_request();
    context.mock_gateway_mut().expect_no_outbound().await;

    context.finish().await.expect_server_finished().await;
}
