use std::time::Duration;

use defguard_common::db::{Id, models::gateway::Gateway};
use defguard_proto::gateway::core_response;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use tonic::Status;

use crate::tests::common::{
    ManagerTestContext, MockGatewayHarness, build_gateway_with_enabled, create_gateway,
    create_gateway_with_enabled, create_network, reload_gateway, unique_mock_gateway_socket_path,
    wait_for_gateway_connection_state,
};

const FAST_RETRY_DELAY: Duration = Duration::from_millis(20);

async fn complete_manager_handshake(
    context: &ManagerTestContext,
    gateway: &Gateway<Id>,
    mock_gateway: &mut MockGatewayHarness,
) {
    mock_gateway.wait_connected().await;
    mock_gateway.send_config_request();
    let outbound = mock_gateway.recv_outbound().await;
    assert!(matches!(
        outbound.payload,
        Some(core_response::Payload::Config(_))
    ));

    let gateway_after = wait_for_gateway_connection_state(&context.pool, gateway.id, true).await;
    assert!(gateway_after.is_connected());
}

#[sqlx::test]
async fn test_starts_existing_enabled_gateway_on_startup(
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

    context.finish().await;
}

#[sqlx::test]
async fn test_starts_gateway_after_enabled_update_notification(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = ManagerTestContext::new(options).await;
    let network = create_network(&context.pool).await;
    let mut gateway = create_gateway_with_enabled(&context.pool, network.id, false).await;
    let mut mock_gateway = MockGatewayHarness::start().await;
    context.register_gateway_mock(&gateway, &mock_gateway);

    context.start().await;
    assert_eq!(
        context.handler_spawn_attempt_count(gateway.id),
        0,
        "disabled gateway handler should not start during manager startup"
    );

    let gateway_before = reload_gateway(&context.pool, gateway.id).await;
    assert!(!gateway_before.is_connected());

    gateway.enabled = true;
    gateway
        .save(&context.pool)
        .await
        .expect("failed to enable test gateway");

    context
        .wait_for_gateway_notification_count(gateway.id, 1)
        .await;
    context
        .wait_for_handler_spawn_attempt_count(gateway.id, 1)
        .await;
    complete_manager_handshake(&context, &gateway, &mut mock_gateway).await;

    context.finish().await;
}

#[sqlx::test]
async fn test_noop_gateway_update_does_not_restart_handler(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = ManagerTestContext::new(options).await;
    let network = create_network(&context.pool).await;
    let mut gateway = create_gateway(&context.pool, network.id).await;
    let mut mock_gateway = MockGatewayHarness::start().await;
    context.register_gateway_mock(&gateway, &mock_gateway);

    context.start().await;
    complete_manager_handshake(&context, &gateway, &mut mock_gateway).await;

    gateway = reload_gateway(&context.pool, gateway.id).await;
    let initial_spawn_attempts = context.handler_spawn_attempt_count(gateway.id);
    let initial_notification_count = context.gateway_notification_count(gateway.id);
    let initial_connection_count = mock_gateway.connection_count();

    gateway.modified_by = "manager-noop-update".to_string();
    gateway
        .save(&context.pool)
        .await
        .expect("failed to save no-op gateway update");

    context
        .wait_for_gateway_notification_count(gateway.id, initial_notification_count + 1)
        .await;
    assert_eq!(
        context.handler_spawn_attempt_count(gateway.id),
        initial_spawn_attempts,
        "no-op gateway update should not restart the handler"
    );
    assert_eq!(
        mock_gateway.connection_count(),
        initial_connection_count,
        "no-op gateway update should not reconnect the handler"
    );

    let gateway_after = reload_gateway(&context.pool, gateway.id).await;
    assert!(gateway_after.is_connected());

    context.finish().await;
}

#[sqlx::test]
async fn test_gateway_address_change_restarts_handler(_: PgPoolOptions, options: PgConnectOptions) {
    let mut context = ManagerTestContext::new(options).await;
    let network = create_network(&context.pool).await;
    let mut gateway = create_gateway(&context.pool, network.id).await;
    let mut original_mock_gateway = MockGatewayHarness::start().await;
    context.register_gateway_mock(&gateway, &original_mock_gateway);

    context.start().await;
    complete_manager_handshake(&context, &gateway, &mut original_mock_gateway).await;

    gateway = reload_gateway(&context.pool, gateway.id).await;

    let replacement_mock_url = {
        gateway.address = "127.0.0.2".to_string();
        gateway.modified_by = "manager-address-update".to_string();
        gateway.url()
    };
    let mut replacement_mock_gateway = MockGatewayHarness::start().await;
    context.register_gateway_url(replacement_mock_url, &replacement_mock_gateway);

    let initial_spawn_attempts = context.handler_spawn_attempt_count(gateway.id);
    let initial_notification_count = context.gateway_notification_count(gateway.id);

    gateway
        .save(&context.pool)
        .await
        .expect("failed to save gateway address update");

    context
        .wait_for_gateway_notification_count(gateway.id, initial_notification_count + 1)
        .await;
    context
        .wait_for_handler_spawn_attempt_count(gateway.id, initial_spawn_attempts + 1)
        .await;
    replacement_mock_gateway.wait_for_connection_count(1).await;
    complete_manager_handshake(&context, &gateway, &mut replacement_mock_gateway).await;

    let gateway_after = reload_gateway(&context.pool, gateway.id).await;
    assert_eq!(gateway_after.address, "127.0.0.2");
    assert!(gateway_after.is_connected());
    original_mock_gateway.expect_server_finished().await;

    context.finish().await;
}

#[sqlx::test]
async fn test_enabled_gateway_update_to_disabled_stops_handler(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = ManagerTestContext::new(options).await;
    let network = create_network(&context.pool).await;
    let mut gateway = create_gateway(&context.pool, network.id).await;
    let mut mock_gateway = MockGatewayHarness::start().await;
    context.register_gateway_mock(&gateway, &mock_gateway);

    context.start().await;
    complete_manager_handshake(&context, &gateway, &mut mock_gateway).await;

    gateway = reload_gateway(&context.pool, gateway.id).await;
    let initial_spawn_attempts = context.handler_spawn_attempt_count(gateway.id);
    let initial_notification_count = context.gateway_notification_count(gateway.id);
    let initial_connection_count = mock_gateway.connection_count();

    gateway.enabled = false;
    gateway.modified_by = "manager-disable-update".to_string();
    gateway
        .save(&context.pool)
        .await
        .expect("failed to save gateway disable update");

    context
        .wait_for_gateway_notification_count(gateway.id, initial_notification_count + 1)
        .await;
    let gateway_after = wait_for_gateway_connection_state(&context.pool, gateway.id, false).await;
    assert!(!gateway_after.is_connected());
    assert!(gateway_after.disconnected_at.is_some());
    assert_eq!(
        context.handler_spawn_attempt_count(gateway.id),
        initial_spawn_attempts,
        "disabling the gateway should stop the existing handler without spawning a replacement"
    );
    assert_eq!(
        mock_gateway.connection_count(),
        initial_connection_count,
        "disabling the gateway should not create a new gateway connection"
    );
    mock_gateway.expect_server_finished().await;

    context.finish().await;
}

#[sqlx::test]
async fn test_insert_notification_starts_handler_for_enabled_gateway(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = ManagerTestContext::new(options).await;
    let network = create_network(&context.pool).await;
    let gateway = build_gateway_with_enabled(network.id, true);
    let gateway_url = gateway.url();
    let mut mock_gateway = MockGatewayHarness::start().await;
    context.register_gateway_url(gateway_url, &mock_gateway);

    context.start().await;

    let gateway = gateway
        .save(&context.pool)
        .await
        .expect("failed to insert enabled test gateway");

    context
        .wait_for_gateway_notification_count(gateway.id, 1)
        .await;
    context
        .wait_for_handler_spawn_attempt_count(gateway.id, 1)
        .await;
    complete_manager_handshake(&context, &gateway, &mut mock_gateway).await;

    context.finish().await;
}

#[sqlx::test]
async fn test_delete_notification_purges_and_aborts_gateway_connection(
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

    Gateway::delete_by_id(&context.pool, gateway.id)
        .await
        .expect("failed to delete test gateway");

    mock_gateway.wait_purged().await;
    mock_gateway.expect_server_finished().await;

    context.finish().await;
}

#[sqlx::test]
async fn test_retries_failed_connection_without_notification_or_duplicate_handler(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = ManagerTestContext::new(options).await;
    context.set_retry_delay(FAST_RETRY_DELAY);

    let network = create_network(&context.pool).await;
    let gateway = create_gateway(&context.pool, network.id).await;
    let socket_path = unique_mock_gateway_socket_path();
    context.register_gateway_socket_path(gateway.url(), socket_path.clone());

    context.start().await;
    context
        .wait_for_handler_spawn_attempt_count(gateway.id, 1)
        .await;
    context
        .wait_for_handler_connection_attempt_count(gateway.id, 2)
        .await;

    assert_eq!(
        context.gateway_notification_count(gateway.id),
        0,
        "manager reconnect retries should not depend on gateway table notifications"
    );
    assert_eq!(
        context.handler_spawn_attempt_count(gateway.id),
        1,
        "manager reconnect retries should reuse the existing handler task"
    );

    let mut mock_gateway = MockGatewayHarness::start_at(socket_path).await;
    mock_gateway.wait_for_connection_count(1).await;
    complete_manager_handshake(&context, &gateway, &mut mock_gateway).await;

    assert_eq!(
        context.handler_spawn_attempt_count(gateway.id),
        1,
        "reconnect success should not create a second concurrent handler"
    );

    context.finish().await;
}

#[sqlx::test]
async fn test_retries_after_stream_close_with_single_handler_supervisor(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = ManagerTestContext::new(options).await;
    context.set_retry_delay(FAST_RETRY_DELAY);

    let network = create_network(&context.pool).await;
    let gateway = create_gateway(&context.pool, network.id).await;
    let mut mock_gateway = MockGatewayHarness::start().await;
    context.register_gateway_mock(&gateway, &mock_gateway);

    context.start().await;
    complete_manager_handshake(&context, &gateway, &mut mock_gateway).await;

    let initial_spawn_attempts = context.handler_spawn_attempt_count(gateway.id);
    let initial_connection_attempts = context.handler_connection_attempt_count(gateway.id);
    let reconnect_socket_path = mock_gateway.socket_path();

    mock_gateway.close_stream();

    let gateway_after_disconnect =
        wait_for_gateway_connection_state(&context.pool, gateway.id, false).await;
    assert!(!gateway_after_disconnect.is_connected());
    assert!(gateway_after_disconnect.disconnected_at.is_some());

    mock_gateway.expect_server_finished().await;

    context
        .wait_for_handler_connection_attempt_count(gateway.id, initial_connection_attempts + 1)
        .await;
    assert_eq!(
        context.handler_spawn_attempt_count(gateway.id),
        initial_spawn_attempts,
        "stream closure retries should keep a single handler supervisor"
    );

    let mut replacement_mock_gateway = MockGatewayHarness::start_at(reconnect_socket_path).await;
    replacement_mock_gateway.wait_for_connection_count(1).await;
    complete_manager_handshake(&context, &gateway, &mut replacement_mock_gateway).await;

    assert_eq!(
        context.handler_spawn_attempt_count(gateway.id),
        initial_spawn_attempts,
        "successful reconnect after stream closure should not create a duplicate handler"
    );

    context.finish().await;
}

#[sqlx::test]
async fn test_retries_after_stream_error_with_single_handler_supervisor(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = ManagerTestContext::new(options).await;
    context.set_retry_delay(FAST_RETRY_DELAY);

    let network = create_network(&context.pool).await;
    let gateway = create_gateway(&context.pool, network.id).await;
    let mut mock_gateway = MockGatewayHarness::start().await;
    context.register_gateway_mock(&gateway, &mock_gateway);

    context.start().await;
    complete_manager_handshake(&context, &gateway, &mut mock_gateway).await;

    let initial_spawn_attempts = context.handler_spawn_attempt_count(gateway.id);
    let initial_connection_attempts = context.handler_connection_attempt_count(gateway.id);
    let reconnect_socket_path = mock_gateway.socket_path();

    mock_gateway.send_stream_error(Status::internal("mock gateway stream failure"));

    let gateway_after_disconnect =
        wait_for_gateway_connection_state(&context.pool, gateway.id, false).await;
    assert!(!gateway_after_disconnect.is_connected());
    assert!(gateway_after_disconnect.disconnected_at.is_some());

    mock_gateway.expect_server_finished().await;

    context
        .wait_for_handler_connection_attempt_count(gateway.id, initial_connection_attempts + 1)
        .await;
    assert_eq!(
        context.handler_spawn_attempt_count(gateway.id),
        initial_spawn_attempts,
        "stream failure retries should keep a single handler supervisor"
    );

    let mut replacement_mock_gateway = MockGatewayHarness::start_at(reconnect_socket_path).await;
    replacement_mock_gateway.wait_for_connection_count(1).await;
    complete_manager_handshake(&context, &gateway, &mut replacement_mock_gateway).await;

    assert_eq!(
        context.handler_spawn_attempt_count(gateway.id),
        initial_spawn_attempts,
        "successful reconnect after stream failure should not create a duplicate handler"
    );

    context.finish().await;
}
