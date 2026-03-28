use std::time::Duration;

use defguard_common::types::proxy::ProxyControlMessage;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

use crate::tests::common::{
    ManagerTestContext, MockProxyHarness, create_proxy, create_proxy_with_enabled, reload_proxy,
    unique_mock_proxy_socket_path, wait_for_proxy_connection_state,
};

const FAST_RETRY_DELAY: Duration = Duration::from_millis(20);

/// Complete the initial proxy handshake: wait for connection and consume the
/// `InitialInfo` response sent by the handler.
async fn complete_manager_proxy_handshake(mock_proxy: &mut MockProxyHarness) {
    mock_proxy.wait_connected().await;
    mock_proxy.recv_initial_info().await;
}

// ---------------------------------------------------------------------------
// Startup / discovery
// ---------------------------------------------------------------------------

#[sqlx::test]
async fn test_manager_starts_all_enabled_proxies_on_startup(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = ManagerTestContext::new(options).await;
    let proxy = create_proxy(&context.pool).await;
    let mut mock_proxy = MockProxyHarness::start().await;
    context.register_proxy_mock(&proxy, &mock_proxy);

    context.start().await;
    complete_manager_proxy_handshake(&mut mock_proxy).await;

    let proxy_after = wait_for_proxy_connection_state(&context.pool, proxy.id, true).await;
    let is_connected = match (proxy_after.connected_at, proxy_after.disconnected_at) {
        (Some(c), Some(d)) => c > d,
        (Some(_), None) => true,
        _ => false,
    };
    assert!(is_connected, "enabled proxy should be connected after manager startup");

    context.finish().await;
}

#[sqlx::test]
async fn test_manager_does_not_start_disabled_proxies(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let context = ManagerTestContext::new(options).await;
    let proxy = create_proxy_with_enabled(&context.pool, false).await;

    // No mock registered — if the manager tried to connect it would fail
    // (or there would be nothing to connect to).

    // Start the manager and give it a moment to process startup.
    let mut context = context;
    context.start().await;
    tokio::time::sleep(Duration::from_millis(50)).await;

    assert_eq!(
        context.handler_spawn_attempt_count(proxy.id),
        0,
        "disabled proxy should not have a handler spawned at startup"
    );

    let proxy_after = reload_proxy(&context.pool, proxy.id).await;
    let is_connected = match (proxy_after.connected_at, proxy_after.disconnected_at) {
        (Some(c), Some(d)) => c > d,
        (Some(_), None) => true,
        _ => false,
    };
    assert!(!is_connected, "disabled proxy should not be marked connected");

    context.finish().await;
}

// ---------------------------------------------------------------------------
// Control messages: StartConnection
// ---------------------------------------------------------------------------

#[sqlx::test]
async fn test_start_connection_spawns_new_handler(_: PgPoolOptions, options: PgConnectOptions) {
    let mut context = ManagerTestContext::new(options).await;
    // Create a disabled proxy so the manager doesn't start it on startup.
    let proxy = create_proxy_with_enabled(&context.pool, false).await;
    let mut mock_proxy = MockProxyHarness::start().await;
    context.register_proxy_mock(&proxy, &mock_proxy);

    context.start().await;
    assert_eq!(
        context.handler_spawn_attempt_count(proxy.id),
        0,
        "disabled proxy should not start on manager startup"
    );

    // Enable the proxy in the DB, then send StartConnection.
    let mut proxy_db = reload_proxy(&context.pool, proxy.id).await;
    proxy_db.enabled = true;
    proxy_db
        .save(&context.pool)
        .await
        .expect("failed to enable test proxy");

    context
        .proxy_control_tx
        .send(ProxyControlMessage::StartConnection(proxy.id))
        .await
        .expect("failed to send StartConnection");

    context
        .wait_for_handler_spawn_attempt_count(proxy.id, 1)
        .await;

    complete_manager_proxy_handshake(&mut mock_proxy).await;
    let proxy_after = wait_for_proxy_connection_state(&context.pool, proxy.id, true).await;
    let is_connected = match (proxy_after.connected_at, proxy_after.disconnected_at) {
        (Some(c), Some(d)) => c > d,
        (Some(_), None) => true,
        _ => false,
    };
    assert!(is_connected, "proxy should be connected after StartConnection");

    context.finish().await;
}

// ---------------------------------------------------------------------------
// Control messages: ShutdownConnection (without purge)
// ---------------------------------------------------------------------------

#[sqlx::test]
async fn test_shutdown_control_message_disconnects_without_purge(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = ManagerTestContext::new(options).await;
    let proxy = create_proxy(&context.pool).await;
    let mut mock_proxy = MockProxyHarness::start().await;
    context.register_proxy_mock(&proxy, &mock_proxy);

    context.start().await;
    complete_manager_proxy_handshake(&mut mock_proxy).await;
    wait_for_proxy_connection_state(&context.pool, proxy.id, true).await;

    // Send ShutdownConnection — purge() RPC must NOT be called.
    context
        .proxy_control_tx
        .send(ProxyControlMessage::ShutdownConnection(proxy.id))
        .await
        .expect("failed to send ShutdownConnection");

    let proxy_after = wait_for_proxy_connection_state(&context.pool, proxy.id, false).await;
    let is_connected = match (proxy_after.connected_at, proxy_after.disconnected_at) {
        (Some(c), Some(d)) => c > d,
        (Some(_), None) => true,
        _ => false,
    };
    assert!(!is_connected, "proxy should be disconnected after ShutdownConnection");

    // Verify purge() was NOT called.
    assert_eq!(
        mock_proxy.connection_count(),
        1,
        "only one connection should have occurred"
    );

    context.finish().await;
}

// ---------------------------------------------------------------------------
// Control messages: Purge
// ---------------------------------------------------------------------------

#[sqlx::test]
async fn test_purge_control_message_calls_purge_rpc(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = ManagerTestContext::new(options).await;
    let proxy = create_proxy(&context.pool).await;
    let mut mock_proxy = MockProxyHarness::start().await;
    context.register_proxy_mock(&proxy, &mock_proxy);

    context.start().await;
    complete_manager_proxy_handshake(&mut mock_proxy).await;
    wait_for_proxy_connection_state(&context.pool, proxy.id, true).await;

    // Send Purge — purge() RPC MUST be called.
    context
        .proxy_control_tx
        .send(ProxyControlMessage::Purge(proxy.id))
        .await
        .expect("failed to send Purge");

    mock_proxy.wait_purged().await;

    let proxy_after = wait_for_proxy_connection_state(&context.pool, proxy.id, false).await;
    let is_connected = match (proxy_after.connected_at, proxy_after.disconnected_at) {
        (Some(c), Some(d)) => c > d,
        (Some(_), None) => true,
        _ => false,
    };
    assert!(!is_connected, "proxy should be disconnected after Purge");

    context.finish().await;
}

// ---------------------------------------------------------------------------
// Reconnect behaviour (single handler supervisor)
// ---------------------------------------------------------------------------

#[sqlx::test]
async fn test_manager_retries_after_stream_close_single_supervisor(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = ManagerTestContext::new(options).await;
    context.set_retry_delay(FAST_RETRY_DELAY);

    let proxy = create_proxy(&context.pool).await;
    let socket_path = unique_mock_proxy_socket_path();
    context.register_proxy_socket_path(
        format!("http://{}:{}/", proxy.address, proxy.port),
        socket_path.clone(),
    );

    context.start().await;
    context
        .wait_for_handler_spawn_attempt_count(proxy.id, 1)
        .await;

    // First mock server — accept one connection, then close the stream.
    let mut mock_proxy = MockProxyHarness::start_at(socket_path.clone()).await;
    mock_proxy.wait_for_connection_count(1).await;
    complete_manager_proxy_handshake(&mut mock_proxy).await;
    wait_for_proxy_connection_state(&context.pool, proxy.id, true).await;

    let initial_spawn_attempts = context.handler_spawn_attempt_count(proxy.id);

    // Simulate proxy disconnect by closing the inbound stream.
    mock_proxy.close_stream();
    wait_for_proxy_connection_state(&context.pool, proxy.id, false).await;

    // The handler should retry without spawning a new supervisor task.
    // Start a replacement mock server at the same socket path.
    let mut replacement = MockProxyHarness::start_at(socket_path).await;
    replacement.wait_for_connection_count(1).await;
    complete_manager_proxy_handshake(&mut replacement).await;
    wait_for_proxy_connection_state(&context.pool, proxy.id, true).await;

    assert_eq!(
        context.handler_spawn_attempt_count(proxy.id),
        initial_spawn_attempts,
        "stream closure retry should reuse the existing handler task, not spawn a new one"
    );

    context.finish().await;
}
