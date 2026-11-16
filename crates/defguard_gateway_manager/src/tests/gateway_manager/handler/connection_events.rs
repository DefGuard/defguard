use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

#[sqlx::test]
async fn test_stores_connection_event_after_config(_: PgPoolOptions, options: PgConnectOptions) {
    let mut context = HandlerTestContext::new(options).await;
    // Source of the outbound test event.
    let device = create_device_for_network(
        &context,
        context.network.id,
        "laptop",
        "laptop-pubkey",
        "10.10.0.2",
    )
    .await;

    // Sent before the configuration handshake, so it must be ignored.
    context
        .mock_gateway()
        .send_connection_events(vec![build_connection_event()]);

    context.mock_gateway().send_config_request();
    let _ = context.mock_gateway_mut().recv_outbound().await;
    context
        .mock_gateway()
        .send_connection_events(vec![build_connection_event()]);

    let events = wait_for_connection_events(&context.pool, 1).await;
    let event = &events[0];
    assert_eq!(event.location_id, context.network.id);
    assert_eq!(event.gateway_id, context.gateway.id);
    assert_eq!(event.timestamp.and_utc().timestamp(), 1_700_000_000);
    assert_eq!(event.direction, ConnectionDirection::Outbound);
    assert_eq!(event.status, ConnectionStatus::Blocked);
    assert_eq!(event.protocol, 6);
    assert_eq!(event.source_ip, IpAddr::V4(Ipv4Addr::new(10, 10, 0, 2)));
    assert_eq!(event.source_port, Some(54321));
    assert_eq!(
        event.destination_ip,
        IpAddr::V6(Ipv6Addr::new(0x2001, 0x0db8, 0, 0, 0, 0, 0, 1))
    );
    assert_eq!(event.destination_port, Some(443));
    assert_eq!(event.rule_id, Some(7));
    assert_eq!(event.device_id, Some(device.id));
    assert_eq!(event.upload_bytes, 1024);
    assert_eq!(event.download_bytes, 2048);
    assert_eq!(event.count, 1);

    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_drops_malformed_connection_events(_: PgPoolOptions, options: PgConnectOptions) {
    let mut context = HandlerTestContext::new(options).await;

    context.mock_gateway().send_config_request();
    let _ = context.mock_gateway_mut().recv_outbound().await;

    context.mock_gateway().send_connection_events(vec![
        monitor::ConnectionEvent {
            source_ip_addr: vec![1, 2, 3],
            ..build_connection_event()
        },
        monitor::ConnectionEvent {
            destination_port: Some(70_000),
            ..build_connection_event()
        },
        monitor::ConnectionEvent {
            timestamp: None,
            ..build_connection_event()
        },
        monitor::ConnectionEvent {
            direction: monitor::Direction::Unspecified.into(),
            ..build_connection_event()
        },
        // Unknown enum values are treated as unspecified.
        monitor::ConnectionEvent {
            status: 7,
            ..build_connection_event()
        },
        // Valid events.
        monitor::ConnectionEvent {
            source_port: None,
            ..build_connection_event()
        },
        monitor::ConnectionEvent {
            count: 2,
            ..build_connection_event()
        },
    ]);

    // Stored in one statement, so both arrive together.
    let mut events = wait_for_connection_events(&context.pool, 2).await;
    events.sort_by_key(|event| event.count);
    assert_eq!(events[0].source_port, None);
    assert_eq!(events[1].count, 2);

    context.finish().await.expect_server_finished().await;
}
