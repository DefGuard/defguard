#[sqlx::test]
async fn test_matching_location_firewall_config_changed_event_produces_update(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;
    let expected_firewall_config = build_test_firewall_config();

    let _ = context.complete_config_handshake().await;

    assert_send_ok!(
        context
            .events_tx()
            .send(GatewayEvent::FirewallConfigChanged(
                context.network.id,
                expected_firewall_config.clone(),
            )),
        "failed to broadcast firewall config changed event"
    );

    let outbound = context.mock_gateway_mut().recv_outbound().await;
    assert_firewall_modify_update(outbound, &expected_firewall_config);
    context.mock_gateway_mut().expect_no_outbound().await;

    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_matching_location_firewall_disabled_event_produces_disable_update(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;

    let _ = context.complete_config_handshake().await;

    assert_send_ok!(
        context
            .events_tx()
            .send(GatewayEvent::FirewallDisabled(context.network.id)),
        "failed to broadcast firewall disabled event"
    );

    let outbound = context.mock_gateway_mut().recv_outbound().await;
    assert_firewall_disable_update(outbound);
    context.mock_gateway_mut().expect_no_outbound().await;

    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_different_location_firewall_config_changed_event_is_ignored(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let expected_firewall_config = build_test_firewall_config();

    assert_firewall_event_for_different_network_is_ignored(options, move |other_network_id| {
        GatewayEvent::FirewallConfigChanged(other_network_id, expected_firewall_config)
    })
    .await;
}

#[sqlx::test]
async fn test_different_location_firewall_disabled_event_is_ignored(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    assert_firewall_event_for_different_network_is_ignored(options, |other_network_id| {
        GatewayEvent::FirewallDisabled(other_network_id)
    })
    .await;
}
