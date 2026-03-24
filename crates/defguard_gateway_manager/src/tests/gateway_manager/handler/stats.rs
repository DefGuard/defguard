#[sqlx::test]
async fn test_ignores_peer_stats_before_config_handshake(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;

    context
        .mock_gateway()
        .send_peer_stats(build_peer_stats("203.0.113.10:51820"));

    context.expect_no_peer_stats().await;
    context.mock_gateway_mut().expect_no_outbound().await;

    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_forwards_valid_peer_stats_after_config(_: PgPoolOptions, options: PgConnectOptions) {
    let mut context = HandlerTestContext::new(options).await;

    context.mock_gateway().send_config_request();
    let _ = context.mock_gateway_mut().recv_outbound().await;
    context
        .mock_gateway()
        .send_peer_stats(build_peer_stats("203.0.113.10:51820"));

    let forwarded = context.recv_peer_stats().await;
    assert_eq!(forwarded.location_id, context.network.id);
    assert_eq!(forwarded.gateway_id, context.gateway.id);
    assert_eq!(forwarded.device_pubkey, "peer-public-key");
    assert_eq!(forwarded.endpoint.to_string(), "203.0.113.10:51820");
    assert_eq!(forwarded.upload, 123);
    assert_eq!(forwarded.download, 456);

    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_drops_malformed_or_missing_endpoint_peer_stats(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;

    context.mock_gateway().send_config_request();
    let _ = context.mock_gateway_mut().recv_outbound().await;

    context.mock_gateway().send_peer_stats(build_peer_stats(""));
    context.expect_no_peer_stats().await;

    context
        .mock_gateway()
        .send_peer_stats(build_peer_stats("not-a-socket-address"));
    context.expect_no_peer_stats().await;

    context.finish().await.expect_server_finished().await;
}
