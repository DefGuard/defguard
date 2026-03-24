#[sqlx::test]
async fn test_matching_location_network_deleted_event_produces_delete_update(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;

    let _ = context.complete_config_handshake().await;

    assert_send_ok!(
        context.events_tx().send(GatewayEvent::NetworkDeleted(
            context.network.id,
            context.network.name.clone(),
        )),
        "failed to broadcast gateway event"
    );

    let outbound = context.mock_gateway_mut().recv_outbound().await;
    assert_network_delete_update(outbound, &context.network.name);

    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_matching_location_network_modified_event_produces_modify_update(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;

    let _ = context.complete_config_handshake().await;

    let mut modified_network = context
        .network
        .clone()
        .set_address([
            "10.20.0.1/24"
                .parse()
                .expect("failed to parse modified network address"),
        ])
        .expect("failed to set modified network address");
    modified_network.name = format!("{}-modified", context.network.name);
    modified_network.port = 51821;
    modified_network.mtu = 1380;
    modified_network.fwmark = 42;

    assert_send_ok!(
        context.events_tx().send(GatewayEvent::NetworkModified(
            context.network.id,
            modified_network,
            Vec::new(),
            None,
        )),
        "failed to broadcast modified gateway event"
    );

    let outbound = context.mock_gateway_mut().recv_outbound().await;
    assert_network_modify_update(
        outbound,
        &format!("{}-modified", context.network.name),
        "10.20.0.1/24",
        51821,
        1380,
        42,
    );
    context.mock_gateway_mut().expect_no_outbound().await;

    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_matching_location_network_created_event_produces_create_update(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;

    let _ = context.complete_config_handshake().await;

    let mut created_network = context
        .network
        .clone()
        .set_address([
            "10.40.0.1/24"
                .parse()
                .expect("failed to parse created network address"),
        ])
        .expect("failed to set created network address");
    created_network.name = format!("{}-created", context.network.name);
    created_network.port = 51841;
    created_network.mtu = 1410;
    created_network.fwmark = 17;

    assert_send_ok!(
        context.events_tx().send(GatewayEvent::NetworkCreated(
            context.network.id,
            created_network,
        )),
        "failed to broadcast created gateway event"
    );

    let outbound = context.mock_gateway_mut().recv_outbound().await;
    assert_network_create_update(
        outbound,
        &format!("{}-created", context.network.name),
        "10.40.0.1/24",
        51841,
        1410,
        17,
    );
    context.mock_gateway_mut().expect_no_outbound().await;

    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_only_matching_handler_receives_network_modified_update(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let (events_tx, _) = tokio::sync::broadcast::channel(16);
    let mut matching_context =
        HandlerTestContext::new_with_events_tx(options.clone(), events_tx.clone()).await;
    let mut unrelated_context = HandlerTestContext::new_with_events_tx(options, events_tx).await;

    assert_ne!(matching_context.network.id, unrelated_context.network.id);

    let _ = matching_context.complete_config_handshake().await;
    let _ = unrelated_context.complete_config_handshake().await;

    let mut modified_network = matching_context
        .network
        .clone()
        .set_address([
            "10.30.0.1/24"
                .parse()
                .expect("failed to parse modified network address"),
        ])
        .expect("failed to set modified network address");
    modified_network.name = format!("{}-modified", matching_context.network.name);
    modified_network.port = 51831;
    modified_network.mtu = 1400;
    modified_network.fwmark = 7;

    assert_send_ok!(
        matching_context
            .events_tx()
            .send(GatewayEvent::NetworkModified(
                matching_context.network.id,
                modified_network,
                Vec::new(),
                None,
            )),
        "failed to broadcast modified gateway event"
    );

    let outbound = matching_context.mock_gateway_mut().recv_outbound().await;
    assert_network_modify_update(
        outbound,
        &format!("{}-modified", matching_context.network.name),
        "10.30.0.1/24",
        51831,
        1400,
        7,
    );
    matching_context.mock_gateway_mut().expect_no_outbound().await;
    unrelated_context.mock_gateway_mut().expect_no_outbound().await;

    matching_context
        .finish()
        .await
        .expect_server_finished()
        .await;
    unrelated_context
        .finish()
        .await
        .expect_server_finished()
        .await;
}

#[sqlx::test]
async fn test_different_location_network_created_event_is_ignored(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;
    let other_network = context.create_other_network().await;
    assert_ne!(other_network.id, context.network.id);

    let _ = context.complete_config_handshake().await;
    assert_send_ok!(
        context.events_tx().send(GatewayEvent::NetworkCreated(
            other_network.id,
            other_network,
        )),
        "failed to broadcast unrelated created gateway event"
    );

    context.mock_gateway_mut().expect_no_outbound().await;

    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_different_location_network_deleted_event_is_ignored(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;
    let other_network = context.create_other_network().await;
    assert_ne!(other_network.id, context.network.id);

    let _ = context.complete_config_handshake().await;
    assert_send_ok!(
        context.events_tx().send(GatewayEvent::NetworkDeleted(
            other_network.id,
            other_network.name.clone(),
        )),
        "failed to broadcast unrelated gateway event"
    );

    context.mock_gateway_mut().expect_no_outbound().await;

    assert_send_ok!(
        context.events_tx().send(GatewayEvent::NetworkDeleted(
            context.network.id,
            context.network.name.clone(),
        )),
        "failed to broadcast owned gateway event"
    );

    let outbound = context.mock_gateway_mut().recv_outbound().await;
    assert_network_delete_update(outbound, &context.network.name);

    context.finish().await.expect_server_finished().await;
}
