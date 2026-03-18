use defguard_common::db::{Id, models::gateway::Gateway};
use defguard_proto::gateway::core_response;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

use crate::common::{
    ManagerTestContext, MockGatewayHarness, build_gateway_with_enabled, create_gateway,
    create_gateway_with_enabled, create_network, reload_gateway,
    wait_for_gateway_connection_state,
};

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

    context.wait_for_gateway_notification_count(gateway.id, 1).await;
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
async fn test_gateway_address_change_restarts_handler(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
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

    context.wait_for_gateway_notification_count(gateway.id, 1).await;
    context.wait_for_handler_spawn_attempt_count(gateway.id, 1).await;
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
