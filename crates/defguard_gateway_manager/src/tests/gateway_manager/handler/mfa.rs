#[sqlx::test]
async fn test_matching_location_mfa_session_authorized_produces_peer_create(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;
    let expected_keepalive_interval = expected_keepalive_interval(&context);
    enable_internal_mfa_for_network(&context.pool, &mut context.network).await;

    let _ = context.complete_config_handshake().await;
    let (device, network_device) = create_authorized_mfa_device_for_current_network(
        &context,
        "mfa-authorized-device",
        "4v9K9Q4HEdmlX0Mb4uxDLPq3nKjvU8fNnJ9fKjzh4ko=",
        "10.10.0.40",
        Some("mfa-authorized-preshared-key"),
    )
    .await;

    assert_send_ok!(
        context.events_tx().send(GatewayEvent::MfaSessionAuthorized(
            context.network.id,
            device,
            network_device,
        )),
        "failed to broadcast MFA session authorized event"
    );

    let outbound = context.mock_gateway_mut().recv_outbound().await;
    assert_peer_update(
        outbound,
        UpdateType::Create,
        "4v9K9Q4HEdmlX0Mb4uxDLPq3nKjvU8fNnJ9fKjzh4ko=",
        &["10.10.0.40"],
        Some("mfa-authorized-preshared-key"),
        Some(expected_keepalive_interval),
    );
    context.mock_gateway_mut().expect_no_outbound().await;

    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_mfa_session_authorized_with_mismatched_network_id_is_ignored(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;
    enable_internal_mfa_for_network(&context.pool, &mut context.network).await;

    let mut other_network = context.create_other_network().await;
    enable_internal_mfa_for_network(&context.pool, &mut other_network).await;
    assert_ne!(other_network.id, context.network.id);

    let _ = context.complete_config_handshake().await;
    let (device, network_device) = create_authorized_mfa_device_for_network(
        &context,
        other_network.id,
        "mfa-mismatched-network-device",
        "Z2UuIvYJvU5fTOp8i3tHfLm4xZ0R8ExY6E3S3l+rqT8=",
        "10.11.0.40",
        Some("mfa-mismatched-network-preshared-key"),
    )
    .await;

    assert_send_ok!(
        context.events_tx().send(GatewayEvent::MfaSessionAuthorized(
            context.network.id,
            device,
            network_device,
        )),
        "failed to broadcast mismatched MFA session authorized event"
    );

    context.mock_gateway_mut().expect_no_outbound().await;

    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_matching_location_mfa_session_disconnected_produces_peer_delete(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;
    enable_internal_mfa_for_network(&context.pool, &mut context.network).await;

    let _ = context.complete_config_handshake().await;
    let (device, _) = create_authorized_mfa_device_for_current_network(
        &context,
        "mfa-disconnected-device",
        "2+n8hQ1yA2sPp1z2i6m8lP4VtY7M8W6hYqS3n4uL7qg=",
        "10.10.0.41",
        Some("mfa-disconnected-preshared-key"),
    )
    .await;

    assert_send_ok!(
        context
            .events_tx()
            .send(GatewayEvent::MfaSessionDisconnected(
                context.network.id,
                device,
            )),
        "failed to broadcast MFA session disconnected event"
    );

    let outbound = context.mock_gateway_mut().recv_outbound().await;
    assert_peer_update(
        outbound,
        UpdateType::Delete,
        "2+n8hQ1yA2sPp1z2i6m8lP4VtY7M8W6hYqS3n4uL7qg=",
        &[],
        None,
        None,
    );
    context.mock_gateway_mut().expect_no_outbound().await;

    context.finish().await.expect_server_finished().await;
}
