use std::time::Duration;

use defguard_common::types::proxy::ProxyControlMessage;
use defguard_proto::proxy::core_response;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

use crate::tests::common::{
    ManagerTestContext, MockProxyHarness, create_proxy, create_proxy_with_enabled,
    mock_proxy_socket_path, reload_proxy, wait_for_proxy_connection_state,
};

const FAST_RETRY_DELAY: Duration = Duration::from_millis(20);

/// Complete the initial proxy handshake: wait for connection and consume the
/// `InitialInfo` response sent by the handler.
async fn complete_manager_proxy_handshake(mock_proxy: &mut MockProxyHarness) {
    mock_proxy.wait_connected().await;
    mock_proxy.recv_initial_info().await;
}

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
    assert!(
        proxy_after.is_connected(),
        "enabled proxy should be connected after manager startup"
    );

    context.finish().await;
}

/// Two enabled proxies at startup — both complete their handshake and both
/// appear as connected in the DB.  Verifies that the manager spawns independent
/// handler tasks and that they do not interfere with each other.
#[sqlx::test]
async fn test_two_proxies_connect_independently(_: PgPoolOptions, options: PgConnectOptions) {
    let mut context = ManagerTestContext::new(options).await;

    let proxy_a = create_proxy(&context.pool).await;
    let mut mock_a = MockProxyHarness::start().await;
    context.register_proxy_mock(&proxy_a, &mock_a);

    let proxy_b = create_proxy(&context.pool).await;
    let mut mock_b = MockProxyHarness::start().await;
    context.register_proxy_mock(&proxy_b, &mock_b);

    context.start().await;

    // Both handshakes must complete — order is not guaranteed.
    complete_manager_proxy_handshake(&mut mock_a).await;
    complete_manager_proxy_handshake(&mut mock_b).await;

    // Both proxies must be recorded as connected in the DB.
    let after_a = wait_for_proxy_connection_state(&context.pool, proxy_a.id, true).await;
    assert!(
        after_a.is_connected(),
        "proxy A should be connected after startup"
    );

    let after_b = wait_for_proxy_connection_state(&context.pool, proxy_b.id, true).await;
    assert!(
        after_b.is_connected(),
        "proxy B should be connected after startup"
    );

    // Each mock must have received exactly one connection.
    assert_eq!(
        mock_a.connection_count(),
        1,
        "proxy A mock should have exactly one connection"
    );
    assert_eq!(
        mock_b.connection_count(),
        1,
        "proxy B mock should have exactly one connection"
    );

    context.finish().await;
}

/// Two proxies are connected at startup.  A third proxy exists in the DB but
/// is disabled.  When it is enabled and `StartConnection` is sent at runtime,
/// the manager spawns a new handler and all three proxies are connected.
#[sqlx::test]
async fn test_start_connection_adds_proxy_at_runtime(_: PgPoolOptions, options: PgConnectOptions) {
    let mut context = ManagerTestContext::new(options).await;

    // Two proxies that connect at startup.
    let proxy_a = create_proxy(&context.pool).await;
    let mut mock_a = MockProxyHarness::start().await;
    context.register_proxy_mock(&proxy_a, &mock_a);

    let proxy_b = create_proxy(&context.pool).await;
    let mut mock_b = MockProxyHarness::start().await;
    context.register_proxy_mock(&proxy_b, &mock_b);

    // Third proxy: disabled — manager must not start it.
    let proxy_c = create_proxy_with_enabled(&context.pool, false).await;
    let mut mock_c = MockProxyHarness::start().await;
    context.register_proxy_mock(&proxy_c, &mock_c);

    context.start().await;

    // Wait for the two startup proxies to connect.
    complete_manager_proxy_handshake(&mut mock_a).await;
    complete_manager_proxy_handshake(&mut mock_b).await;
    wait_for_proxy_connection_state(&context.pool, proxy_a.id, true).await;
    wait_for_proxy_connection_state(&context.pool, proxy_b.id, true).await;

    // The third proxy must still be inactive at this point.
    assert_eq!(
        context.handler_spawn_attempt_count(proxy_c.id),
        0,
        "disabled proxy C must not be started at startup"
    );

    // Enable proxy C in the DB and send StartConnection.
    let mut proxy_c_db = reload_proxy(&context.pool, proxy_c.id).await;
    proxy_c_db.enabled = true;
    proxy_c_db
        .save(&context.pool)
        .await
        .expect("failed to enable proxy C");

    context
        .proxy_control_tx
        .send(ProxyControlMessage::StartConnection(proxy_c.id))
        .await
        .expect("failed to send StartConnection for proxy C");

    context
        .wait_for_handler_spawn_attempt_count(proxy_c.id, 1)
        .await;
    complete_manager_proxy_handshake(&mut mock_c).await;

    let after_c = wait_for_proxy_connection_state(&context.pool, proxy_c.id, true).await;
    assert!(
        after_c.is_connected(),
        "proxy C should be connected after StartConnection at runtime"
    );

    // The original two proxies must still be connected.
    let still_a = wait_for_proxy_connection_state(&context.pool, proxy_a.id, true).await;
    assert!(
        still_a.is_connected(),
        "proxy A must still be connected after C joins"
    );

    let still_b = wait_for_proxy_connection_state(&context.pool, proxy_b.id, true).await;
    assert!(
        still_b.is_connected(),
        "proxy B must still be connected after C joins"
    );

    context.finish().await;
}

/// One proxy's stream closes and reconnects to a replacement mock server at the
/// same socket path.  The other proxy must remain connected throughout — the
/// reconnect must be fully isolated to the affected handler task.
#[sqlx::test]
async fn test_one_proxy_reconnects_while_other_stays_connected(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = ManagerTestContext::new(options).await;
    context.set_retry_delay(FAST_RETRY_DELAY);

    // Proxy A: reconnects — use a fixed socket path so we can start a replacement.
    let proxy_a = create_proxy(&context.pool).await;
    let socket_a = mock_proxy_socket_path();
    context.register_proxy_socket_path(
        format!("http://{}:{}/", proxy_a.address, proxy_a.port),
        socket_a.clone(),
    );

    // Proxy B: stable — standard mock.
    let proxy_b = create_proxy(&context.pool).await;
    let mut mock_b = MockProxyHarness::start().await;
    context.register_proxy_mock(&proxy_b, &mock_b);

    context.start().await;
    context
        .wait_for_handler_spawn_attempt_count(proxy_a.id, 1)
        .await;

    // First mock for proxy A — will be closed to trigger a reconnect.
    let mut mock_a1 = MockProxyHarness::start_at(socket_a.clone()).await;
    mock_a1.wait_for_connection_count(1).await;
    complete_manager_proxy_handshake(&mut mock_a1).await;

    complete_manager_proxy_handshake(&mut mock_b).await;

    wait_for_proxy_connection_state(&context.pool, proxy_a.id, true).await;
    wait_for_proxy_connection_state(&context.pool, proxy_b.id, true).await;

    let initial_spawn_count_a = context.handler_spawn_attempt_count(proxy_a.id);

    // Close proxy A's stream — triggers internal retry loop in handler A.
    mock_a1.close_stream();
    wait_for_proxy_connection_state(&context.pool, proxy_a.id, false).await;

    // Start replacement mock for proxy A at the same socket path.
    let mut mock_a2 = MockProxyHarness::start_at(socket_a).await;
    mock_a2.wait_for_connection_count(1).await;
    complete_manager_proxy_handshake(&mut mock_a2).await;
    wait_for_proxy_connection_state(&context.pool, proxy_a.id, true).await;

    // Handler A reused its existing task — no new supervisor spawned.
    assert_eq!(
        context.handler_spawn_attempt_count(proxy_a.id),
        initial_spawn_count_a,
        "proxy A reconnect must not spawn a new handler task"
    );

    // Proxy B must have remained connected throughout the reconnect of A.
    let after_b = wait_for_proxy_connection_state(&context.pool, proxy_b.id, true).await;
    assert!(
        after_b.is_connected(),
        "proxy B must remain connected while proxy A reconnects"
    );
    assert_eq!(
        mock_b.connection_count(),
        1,
        "proxy B mock must have received exactly one connection (no reconnect)"
    );

    context.finish().await;
}

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
    assert!(
        proxy_after.is_connected(),
        "proxy should be connected after StartConnection"
    );

    context.finish().await;
}

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
    assert!(
        !proxy_after.is_connected(),
        "proxy should be disconnected after ShutdownConnection"
    );

    // Verify purge() was NOT called.
    assert_eq!(
        mock_proxy.connection_count(),
        1,
        "only one connection should have occurred"
    );

    context.finish().await;
}

#[sqlx::test]
async fn test_purge_control_message_calls_purge_rpc(_: PgPoolOptions, options: PgConnectOptions) {
    let mut context = ManagerTestContext::new(options).await;

    // Two proxies connected at startup.
    let proxy_a = create_proxy(&context.pool).await;
    let mut mock_a = MockProxyHarness::start().await;
    context.register_proxy_mock(&proxy_a, &mock_a);

    let proxy_b = create_proxy(&context.pool).await;
    let mut mock_b = MockProxyHarness::start().await;
    context.register_proxy_mock(&proxy_b, &mock_b);

    context.start().await;
    complete_manager_proxy_handshake(&mut mock_a).await;
    complete_manager_proxy_handshake(&mut mock_b).await;
    wait_for_proxy_connection_state(&context.pool, proxy_a.id, true).await;
    wait_for_proxy_connection_state(&context.pool, proxy_b.id, true).await;

    // Send Purge targeting proxy A only — purge() RPC MUST be called on A.
    context
        .proxy_control_tx
        .send(ProxyControlMessage::Purge(proxy_a.id))
        .await
        .expect("failed to send Purge");

    mock_a.wait_purged().await;

    let proxy_a_after = wait_for_proxy_connection_state(&context.pool, proxy_a.id, false).await;
    assert!(
        !proxy_a_after.is_connected(),
        "proxy A should be disconnected after Purge"
    );

    // Proxy B must be completely unaffected — not purged, still connected.
    assert_eq!(
        mock_b.purge_count(),
        0,
        "proxy B must not be purged by a Purge message targeting proxy A"
    );
    let proxy_b_after = wait_for_proxy_connection_state(&context.pool, proxy_b.id, true).await;
    assert!(
        proxy_b_after.is_connected(),
        "proxy B must remain connected after proxy A is purged"
    );

    context.finish().await;
}

#[sqlx::test]
async fn test_manager_retries_after_stream_close_single_supervisor(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = ManagerTestContext::new(options).await;
    context.set_retry_delay(FAST_RETRY_DELAY);

    let proxy = create_proxy(&context.pool).await;
    let socket_path = mock_proxy_socket_path();
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

/// Simulates what `trim_gateways_and_edges` does when a license expires with
/// two proxies connected:
///
/// 1. Start the manager with two enabled proxies (both mocked and connected).
/// 2. Verify both report as connected in the DB.
/// 3. Send `ShutdownConnection` for the second proxy — exactly what
///    `trim_gateways_and_edges` would send after calling
///    `Proxy::leave_one_enabled`.
/// 4. Assert the second proxy is now disconnected in the DB.
/// 5. Assert the first proxy is still connected (was not affected).
#[sqlx::test]
async fn test_license_expiry_shuts_down_excess_proxy_only(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = ManagerTestContext::new(options).await;

    // Create two enabled proxies and register a distinct mock for each.
    let proxy_keep = create_proxy(&context.pool).await;
    let mut mock_keep = MockProxyHarness::start().await;
    context.register_proxy_mock(&proxy_keep, &mock_keep);

    let proxy_shutdown = create_proxy(&context.pool).await;
    let mut mock_shutdown = MockProxyHarness::start().await;
    context.register_proxy_mock(&proxy_shutdown, &mock_shutdown);

    context.start().await;

    // Wait for both proxies to complete their handshake with the manager.
    complete_manager_proxy_handshake(&mut mock_keep).await;
    complete_manager_proxy_handshake(&mut mock_shutdown).await;

    // Both must be connected before we trigger the simulated license expiry.
    wait_for_proxy_connection_state(&context.pool, proxy_keep.id, true).await;
    wait_for_proxy_connection_state(&context.pool, proxy_shutdown.id, true).await;

    // Simulate trim_gateways_and_edges: send ShutdownConnection for the
    // excess proxy (the one that would be returned by Proxy::leave_one_enabled).
    context
        .proxy_control_tx
        .send(ProxyControlMessage::ShutdownConnection(proxy_shutdown.id))
        .await
        .expect("failed to send ShutdownConnection for excess proxy");

    // The excess proxy must become disconnected.
    let after_shutdown =
        wait_for_proxy_connection_state(&context.pool, proxy_shutdown.id, false).await;
    assert!(
        !after_shutdown.is_connected(),
        "proxy targeted by ShutdownConnection should be disconnected after license expiry"
    );

    // Verify purge() was NOT called on the excess proxy (ShutdownConnection ≠ Purge).
    assert_eq!(
        mock_shutdown.connection_count(),
        1,
        "ShutdownConnection must not trigger a purge RPC on the excess proxy"
    );

    // The retained proxy must still be connected — license expiry must not
    // affect proxies that are allowed to remain.
    let after_keep = wait_for_proxy_connection_state(&context.pool, proxy_keep.id, true).await;
    assert!(
        after_keep.is_connected(),
        "proxy not targeted by ShutdownConnection must remain connected after license expiry"
    );

    context.finish().await;
}

/// `ProxyControlMessage::BroadcastHttpsCerts` must deliver an `HttpsCerts`
/// `CoreResponse` to every proxy handler that is currently registered in
/// `handler_tx_map` (i.e. every handler whose bidi stream is live).
///
/// Setup: two enabled proxies, both connected and past handshake.  Send the
/// control message and assert that both mock proxies receive the matching
/// `HttpsCerts` response.
#[sqlx::test]
async fn test_broadcast_https_certs_reaches_proxy(_: PgPoolOptions, options: PgConnectOptions) {
    let mut context = ManagerTestContext::new(options).await;

    let proxy_a = create_proxy(&context.pool).await;
    let mut mock_a = MockProxyHarness::start().await;
    context.register_proxy_mock(&proxy_a, &mock_a);

    let proxy_b = create_proxy(&context.pool).await;
    let mut mock_b = MockProxyHarness::start().await;
    context.register_proxy_mock(&proxy_b, &mock_b);

    context.start().await;
    complete_manager_proxy_handshake(&mut mock_a).await;
    complete_manager_proxy_handshake(&mut mock_b).await;

    // Ensure both handlers are connected (and therefore registered in handler_tx_map).
    wait_for_proxy_connection_state(&context.pool, proxy_a.id, true).await;
    wait_for_proxy_connection_state(&context.pool, proxy_b.id, true).await;

    let cert_pem = "-----BEGIN CERTIFICATE-----\nTESTCERT\n-----END CERTIFICATE-----\n".to_string();
    let key_pem = "-----BEGIN PRIVATE KEY-----\nTESTKEY\n-----END PRIVATE KEY-----\n".to_string();

    context
        .proxy_control_tx
        .send(ProxyControlMessage::BroadcastHttpsCerts {
            cert_pem: cert_pem.clone(),
            key_pem: key_pem.clone(),
        })
        .await
        .expect("failed to send BroadcastHttpsCerts control message");

    // Both mock proxies must receive an HttpsCerts response.
    for (label, mock) in [("proxy A", &mut mock_a), ("proxy B", &mut mock_b)] {
        let response = mock.recv_outbound().await;
        match response.payload {
            Some(core_response::Payload::HttpsCerts(h)) => {
                assert_eq!(
                    h.cert_pem, cert_pem,
                    "{label}: broadcast cert_pem must match the value sent in BroadcastHttpsCerts"
                );
                assert_eq!(
                    h.key_pem, key_pem,
                    "{label}: broadcast key_pem must match the value sent in BroadcastHttpsCerts"
                );
            }
            other => panic!(
                "{label}: expected HttpsCerts response, got: {:?}",
                other.as_ref().map(std::mem::discriminant)
            ),
        }
    }

    context.finish().await;
}
