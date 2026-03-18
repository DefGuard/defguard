use defguard_common::db::models::gateway::Gateway;
use defguard_proto::gateway::core_response;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

use crate::common::{
    ManagerTestContext, MockGatewayHarness, create_gateway, create_gateway_with_enabled,
    create_network, reload_gateway, wait_for_gateway_connection_state,
};

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
    mock_gateway.wait_connected().await;

    mock_gateway.send_config_request();
    let outbound = mock_gateway.recv_outbound().await;
    assert!(matches!(
        outbound.payload,
        Some(core_response::Payload::Config(_))
    ));

    let gateway_after = wait_for_gateway_connection_state(&context.pool, gateway.id, true).await;
    assert!(gateway_after.is_connected());

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
        .wait_for_handler_spawn_attempt_count(gateway.id, 1)
        .await;
    mock_gateway.wait_connected().await;
    mock_gateway.send_config_request();
    let outbound = mock_gateway.recv_outbound().await;
    assert!(matches!(
        outbound.payload,
        Some(core_response::Payload::Config(_))
    ));

    let gateway_after = wait_for_gateway_connection_state(&context.pool, gateway.id, true).await;
    assert!(gateway_after.is_connected());

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
    mock_gateway.wait_connected().await;

    mock_gateway.send_config_request();
    let outbound = mock_gateway.recv_outbound().await;
    assert!(matches!(
        outbound.payload,
        Some(core_response::Payload::Config(_))
    ));
    let gateway_after = wait_for_gateway_connection_state(&context.pool, gateway.id, true).await;
    assert!(gateway_after.is_connected());

    Gateway::delete_by_id(&context.pool, gateway.id)
        .await
        .expect("failed to delete test gateway");

    mock_gateway.wait_purged().await;
    mock_gateway.expect_server_finished().await;

    context.finish().await;
}
