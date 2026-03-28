/// Handler-level lifecycle tests.
///
/// These test connection establishment and disconnection from the perspective
/// of a single `ProxyHandler` instance (using `HandlerTestContext` and
/// `run_once`). Reconnect retry and control-message tests live in `manager.rs`
/// because they require the full `ProxyManager` supervision loop.

#[sqlx::test]
async fn test_proxy_marked_connected_after_handshake(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;

    let proxy_before = context.reload_proxy().await;
    // Proxy not yet connected: connected_at must be None or older than disconnected_at.
    let is_connected_before = match (proxy_before.connected_at, proxy_before.disconnected_at) {
        (Some(c), Some(d)) => c > d,
        (Some(_), None) => true,
        _ => false,
    };
    assert!(!is_connected_before, "proxy should not be connected before handshake");

    complete_proxy_handshake(&mut context).await;

    let proxy_after = context.reload_proxy().await;
    let is_connected_after = match (proxy_after.connected_at, proxy_after.disconnected_at) {
        (Some(c), Some(d)) => c > d,
        (Some(_), None) => true,
        _ => false,
    };
    assert!(is_connected_after, "proxy should be connected after handshake");
    assert!(proxy_after.connected_at.is_some(), "connected_at should be set");

    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_proxy_marked_disconnected_when_stream_closes(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;

    complete_proxy_handshake(&mut context).await;

    let proxy_id = context.proxy.id;
    let pool = context.pool.clone();
    let mock_proxy = context.finish().await;

    let proxy_after = reload_proxy(&pool, proxy_id).await;
    let is_connected_after = match (proxy_after.connected_at, proxy_after.disconnected_at) {
        (Some(c), Some(d)) => c > d,
        (Some(_), None) => true,
        _ => false,
    };
    assert!(!is_connected_after, "proxy should be disconnected after stream closes");
    assert!(
        proxy_after.disconnected_at.is_some(),
        "disconnected_at should be set after stream close"
    );

    mock_proxy.expect_server_finished().await;
}

#[sqlx::test]
async fn test_proxy_marked_disconnected_when_stream_errors(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;

    complete_proxy_handshake(&mut context).await;

    context
        .mock_proxy_mut()
        .send_stream_error(tonic::Status::internal("mock proxy stream failure"));

    let proxy_id = context.proxy.id;
    let pool = context.pool.clone();
    let mock_proxy = context.finish_after_error().await;

    let proxy_after = reload_proxy(&pool, proxy_id).await;
    let is_connected_after = match (proxy_after.connected_at, proxy_after.disconnected_at) {
        (Some(c), Some(d)) => c > d,
        (Some(_), None) => true,
        _ => false,
    };
    assert!(!is_connected_after, "proxy should be disconnected after stream error");
    assert!(
        proxy_after.disconnected_at.is_some(),
        "disconnected_at should be set after stream error"
    );

    mock_proxy.expect_server_finished().await;
}
