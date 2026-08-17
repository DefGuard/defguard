//! Tests for the Gateway disconnect/reconnect email notification pairing.
//!
//! The disconnect email is delayed by the configured inactivity threshold, so these tests need
//! a Gateway that can actually go down and come back. That makes them manager tests rather than
//! handler tests: `HandlerTestContext` only handles a single connection attempt.
//!
//! Mail delivery itself is fire-and-forget, so the assertions count notification decisions
//! recorded through `GatewayManagerTestSupport` instead of intercepting SMTP traffic.

use std::time::Duration;

use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use tokio::time::sleep;

use crate::tests::common::{
    ManagerTestContext, MockGatewayHarness, complete_manager_handshake,
    configure_gateway_notifications, create_gateway, create_network,
    wait_for_gateway_connection_state,
};

/// Stands in for the real inactivity threshold, which is configured in whole minutes.
const FAST_NOTIFICATION_DELAY: Duration = Duration::from_millis(50);
/// Longer than any of these tests can run, so a pending notification can only disappear by
/// being cancelled.
const NEVER_ELAPSING_NOTIFICATION_DELAY: Duration = Duration::from_secs(600);
/// How long to wait before concluding that no notification is going to be sent.
const NO_NOTIFICATION_GRACE_PERIOD: Duration = Duration::from_millis(200);

#[sqlx::test]
async fn test_reconnect_inside_inactivity_threshold_sends_no_notifications(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = ManagerTestContext::new(options).await;
    configure_gateway_notifications(true, true, 5);
    context.set_disconnect_notification_delay(NEVER_ELAPSING_NOTIFICATION_DELAY);

    let network = create_network(&context.pool).await;
    let gateway = create_gateway(&context.pool, network.id).await;
    let mut mock_gateway = MockGatewayHarness::start().await;
    context.register_gateway_mock(&gateway, &mock_gateway);

    context.start().await;
    complete_manager_handshake(&context, &gateway, &mut mock_gateway).await;

    let reconnect_socket_path = mock_gateway.socket_path();
    mock_gateway.close_stream();
    let disconnected_gateway =
        wait_for_gateway_connection_state(&context.pool, gateway.id, false).await;
    assert!(disconnected_gateway.disconnected_at.is_some());
    mock_gateway.expect_server_finished().await;

    // The Gateway comes back well inside the inactivity threshold.
    let mut replacement_mock_gateway = MockGatewayHarness::start_at(reconnect_socket_path).await;
    replacement_mock_gateway.wait_for_connection_count(1).await;
    complete_manager_handshake(&context, &gateway, &mut replacement_mock_gateway).await;

    sleep(NO_NOTIFICATION_GRACE_PERIOD).await;
    assert_eq!(
        context.disconnect_notification_count(gateway.id),
        0,
        "a Gateway blip should not produce a disconnect email"
    );
    assert_eq!(
        context.reconnect_notification_count(gateway.id),
        0,
        "a Gateway blip should not produce a reconnect email"
    );

    context.finish().await;
}

#[sqlx::test]
async fn test_outage_past_inactivity_threshold_sends_disconnect_then_reconnect_notification(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = ManagerTestContext::new(options).await;
    configure_gateway_notifications(true, true, 5);
    context.set_disconnect_notification_delay(FAST_NOTIFICATION_DELAY);

    let network = create_network(&context.pool).await;
    let gateway = create_gateway(&context.pool, network.id).await;
    let mut mock_gateway = MockGatewayHarness::start().await;
    context.register_gateway_mock(&gateway, &mock_gateway);

    context.start().await;
    complete_manager_handshake(&context, &gateway, &mut mock_gateway).await;

    let reconnect_socket_path = mock_gateway.socket_path();
    mock_gateway.close_stream();
    let disconnected_gateway =
        wait_for_gateway_connection_state(&context.pool, gateway.id, false).await;
    assert!(disconnected_gateway.disconnected_at.is_some());
    mock_gateway.expect_server_finished().await;

    // Nothing is listening on the socket yet, so the Gateway stays down past the threshold and
    // the disconnect email goes out.
    context
        .wait_for_disconnect_notification_count(gateway.id, 1)
        .await;
    assert_eq!(
        context.reconnect_notification_count(gateway.id),
        0,
        "reconnect email must not precede the Gateway coming back"
    );

    let mut replacement_mock_gateway = MockGatewayHarness::start_at(reconnect_socket_path).await;
    replacement_mock_gateway.wait_for_connection_count(1).await;
    complete_manager_handshake(&context, &gateway, &mut replacement_mock_gateway).await;

    context
        .wait_for_reconnect_notification_count(gateway.id, 1)
        .await;
    assert_eq!(
        context.disconnect_notification_count(gateway.id),
        1,
        "the outage should have produced exactly one disconnect email"
    );

    context.finish().await;
}

#[sqlx::test]
async fn test_disabled_notifications_send_nothing_across_a_full_outage(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = ManagerTestContext::new(options).await;
    configure_gateway_notifications(false, false, 5);
    // Same short delay as the outage test, so a scheduled notification would have fired.
    context.set_disconnect_notification_delay(FAST_NOTIFICATION_DELAY);

    let network = create_network(&context.pool).await;
    let gateway = create_gateway(&context.pool, network.id).await;
    let mut mock_gateway = MockGatewayHarness::start().await;
    context.register_gateway_mock(&gateway, &mock_gateway);

    context.start().await;
    complete_manager_handshake(&context, &gateway, &mut mock_gateway).await;

    let reconnect_socket_path = mock_gateway.socket_path();
    let connection_attempts_before = context.handler_connection_attempt_count(gateway.id);
    mock_gateway.close_stream();
    wait_for_gateway_connection_state(&context.pool, gateway.id, false).await;
    mock_gateway.expect_server_finished().await;

    // Wait out a failed reconnect attempt plus the notification delay, so a notification would
    // have had every chance to fire.
    context
        .wait_for_handler_connection_attempt_count(gateway.id, connection_attempts_before + 1)
        .await;
    sleep(NO_NOTIFICATION_GRACE_PERIOD).await;
    assert_eq!(
        context.disconnect_notification_count(gateway.id),
        0,
        "disabled notifications should not schedule a disconnect email"
    );

    let mut replacement_mock_gateway = MockGatewayHarness::start_at(reconnect_socket_path).await;
    replacement_mock_gateway.wait_for_connection_count(1).await;
    complete_manager_handshake(&context, &gateway, &mut replacement_mock_gateway).await;

    sleep(NO_NOTIFICATION_GRACE_PERIOD).await;
    assert_eq!(
        context.reconnect_notification_count(gateway.id),
        0,
        "disabled notifications should not produce a reconnect email"
    );

    context.finish().await;
}

#[sqlx::test]
async fn test_reconnect_notification_disabled_sends_only_disconnect_email(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = ManagerTestContext::new(options).await;
    configure_gateway_notifications(true, false, 5);
    context.set_disconnect_notification_delay(FAST_NOTIFICATION_DELAY);

    let network = create_network(&context.pool).await;
    let gateway = create_gateway(&context.pool, network.id).await;
    let mut mock_gateway = MockGatewayHarness::start().await;
    context.register_gateway_mock(&gateway, &mock_gateway);

    context.start().await;
    complete_manager_handshake(&context, &gateway, &mut mock_gateway).await;

    let reconnect_socket_path = mock_gateway.socket_path();
    mock_gateway.close_stream();
    let disconnected_gateway =
        wait_for_gateway_connection_state(&context.pool, gateway.id, false).await;
    assert!(disconnected_gateway.disconnected_at.is_some());
    mock_gateway.expect_server_finished().await;

    context
        .wait_for_disconnect_notification_count(gateway.id, 1)
        .await;

    let mut replacement_mock_gateway = MockGatewayHarness::start_at(reconnect_socket_path).await;
    replacement_mock_gateway.wait_for_connection_count(1).await;
    complete_manager_handshake(&context, &gateway, &mut replacement_mock_gateway).await;

    sleep(NO_NOTIFICATION_GRACE_PERIOD).await;
    assert_eq!(
        context.disconnect_notification_count(gateway.id),
        1,
        "the outage should have produced exactly one disconnect email"
    );
    assert_eq!(
        context.reconnect_notification_count(gateway.id),
        0,
        "a reconnect email must not be sent when reconnect notifications are disabled"
    );

    context.finish().await;
}
