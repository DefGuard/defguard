use defguard_core::grpc::GatewayEvent;
use defguard_proto::gateway::{
    CoreResponse, Update, UpdateType, core_response,
    update::{self},
};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use tonic::Status;

use crate::common::{HandlerTestContext, build_peer_stats, reload_gateway};

macro_rules! assert_send_ok {
    ($result:expr, $message:literal) => {
        match $result {
            Ok(value) => value,
            Err(_) => panic!($message),
        }
    };
}

macro_rules! panic_unexpected {
    ($message:literal) => {
        panic!($message)
    };
}

#[sqlx::test]
async fn test_sends_configuration_on_first_config_request(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;

    context.mock_gateway().send_config_request();
    let outbound = context.mock_gateway_mut().recv_outbound().await;

    match outbound.payload {
        Some(core_response::Payload::Config(config)) => {
            assert_eq!(config.name, context.network.name);
            assert_eq!(config.port, context.network.port as u32);
            assert_eq!(config.peers, Vec::new());
        }
        _ => panic_unexpected!("expected configuration response"),
    }

    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_does_not_send_configuration_before_gateway_requests_it(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;

    let gateway_before = context.reload_gateway().await;
    assert!(!gateway_before.is_connected());

    context.mock_gateway_mut().expect_no_outbound().await;

    let gateway_after = context.reload_gateway().await;
    assert!(!gateway_after.is_connected());

    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_ignores_repeated_config_request(_: PgPoolOptions, options: PgConnectOptions) {
    let mut context = HandlerTestContext::new(options).await;

    context.mock_gateway().send_config_request();
    let first_outbound = context.mock_gateway_mut().recv_outbound().await;
    assert!(matches!(
        first_outbound.payload,
        Some(core_response::Payload::Config(_))
    ));

    context.mock_gateway().send_config_request();
    context.mock_gateway_mut().expect_no_outbound().await;

    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_ignores_peer_stats_before_config_handshake(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;

    context
        .mock_gateway()
        .send_peer_stats(build_peer_stats("203.0.113.10:51820"));

    context.expect_no_peer_stats().await;
    context.mock_gateway_mut().expect_no_outbound().await;

    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_forwards_valid_peer_stats_after_config(_: PgPoolOptions, options: PgConnectOptions) {
    let mut context = HandlerTestContext::new(options).await;

    context.mock_gateway().send_config_request();
    let _ = context.mock_gateway_mut().recv_outbound().await;
    context
        .mock_gateway()
        .send_peer_stats(build_peer_stats("203.0.113.10:51820"));

    let forwarded = context.recv_peer_stats().await;
    assert_eq!(forwarded.location_id, context.network.id);
    assert_eq!(forwarded.gateway_id, context.gateway.id);
    assert_eq!(forwarded.device_pubkey, "peer-public-key");
    assert_eq!(forwarded.endpoint.to_string(), "203.0.113.10:51820");
    assert_eq!(forwarded.upload, 123);
    assert_eq!(forwarded.download, 456);

    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_drops_malformed_or_missing_endpoint_peer_stats(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;

    context.mock_gateway().send_config_request();
    let _ = context.mock_gateway_mut().recv_outbound().await;

    context.mock_gateway().send_peer_stats(build_peer_stats(""));
    context.expect_no_peer_stats().await;

    context
        .mock_gateway()
        .send_peer_stats(build_peer_stats("not-a-socket-address"));
    context.expect_no_peer_stats().await;

    context.finish().await.expect_server_finished().await;
}

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

    let mut modified_network = context.network.clone();
    modified_network.name = format!("{}-modified", context.network.name);
    modified_network.address = vec![
        "10.20.0.1/24"
            .parse()
            .expect("failed to parse modified network address"),
    ];
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

    let mut created_network = context.network.clone();
    created_network.name = format!("{}-created", context.network.name);
    created_network.address = vec![
        "10.40.0.1/24"
            .parse()
            .expect("failed to parse created network address"),
    ];
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

    let mut modified_network = matching_context.network.clone();
    modified_network.name = format!("{}-modified", matching_context.network.name);
    modified_network.address = vec![
        "10.30.0.1/24"
            .parse()
            .expect("failed to parse modified network address"),
    ];
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
    matching_context
        .mock_gateway_mut()
        .expect_no_outbound()
        .await;
    unrelated_context
        .mock_gateway_mut()
        .expect_no_outbound()
        .await;

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

#[sqlx::test]
async fn test_gateway_is_marked_connected_after_successful_config_handshake(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;

    let gateway_before = context.reload_gateway().await;
    assert!(!gateway_before.is_connected());

    let gateway_after = context.complete_config_handshake().await;
    assert!(gateway_after.is_connected());
    assert!(gateway_after.connected_at.is_some());

    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_gateway_is_marked_disconnected_when_stream_closes(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;

    let connected_gateway = context.complete_config_handshake().await;
    assert!(connected_gateway.is_connected());

    let pool = context.pool.clone();
    let gateway_id = context.gateway.id;
    let mock_gateway = context.finish().await;
    let disconnected_gateway = reload_gateway(&pool, gateway_id).await;
    assert!(!disconnected_gateway.is_connected());
    assert!(disconnected_gateway.disconnected_at.is_some());

    mock_gateway.expect_server_finished().await;
}

#[sqlx::test]
async fn test_gateway_is_marked_disconnected_when_stream_errors(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;

    let _ = context.complete_config_handshake().await;

    context
        .mock_gateway()
        .send_stream_error(Status::internal("mock gateway stream failure"));

    let pool = context.pool.clone();
    let gateway_id = context.gateway.id;
    let mock_gateway = context.finish_after_error().await;
    let disconnected_gateway = reload_gateway(&pool, gateway_id).await;
    assert!(!disconnected_gateway.is_connected());
    assert!(disconnected_gateway.disconnected_at.is_some());

    mock_gateway.expect_server_finished().await;
}

fn assert_network_delete_update(outbound: CoreResponse, expected_network_name: &str) {
    match outbound.payload {
        Some(core_response::Payload::Update(Update {
            update_type,
            update: Some(update::Update::Network(network)),
        })) => {
            assert_eq!(update_type, UpdateType::Delete as i32);
            assert_eq!(network.name, expected_network_name);
        }
        _ => panic_unexpected!("expected network delete update"),
    }
}

fn assert_network_create_update(
    outbound: CoreResponse,
    expected_network_name: &str,
    expected_address: &str,
    expected_port: u32,
    expected_mtu: u32,
    expected_fwmark: u32,
) {
    match outbound.payload {
        Some(core_response::Payload::Update(Update {
            update_type,
            update: Some(update::Update::Network(network)),
        })) => {
            assert_eq!(update_type, UpdateType::Create as i32);
            assert_eq!(network.name, expected_network_name);
            assert_eq!(network.addresses, vec![expected_address.to_string()]);
            assert_eq!(network.port, expected_port);
            assert_eq!(network.peers, Vec::new());
            assert_eq!(network.firewall_config, None);
            assert_eq!(network.mtu, expected_mtu);
            assert_eq!(network.fwmark, expected_fwmark);
        }
        _ => panic_unexpected!("expected network create update"),
    }
}

fn assert_network_modify_update(
    outbound: CoreResponse,
    expected_network_name: &str,
    expected_address: &str,
    expected_port: u32,
    expected_mtu: u32,
    expected_fwmark: u32,
) {
    match outbound.payload {
        Some(core_response::Payload::Update(Update {
            update_type,
            update: Some(update::Update::Network(network)),
        })) => {
            assert_eq!(update_type, UpdateType::Modify as i32);
            assert_eq!(network.name, expected_network_name);
            assert_eq!(network.addresses, vec![expected_address.to_string()]);
            assert_eq!(network.port, expected_port);
            assert_eq!(network.peers, Vec::new());
            assert_eq!(network.firewall_config, None);
            assert_eq!(network.mtu, expected_mtu);
            assert_eq!(network.fwmark, expected_fwmark);
        }
        _ => panic_unexpected!("expected network modify update"),
    }
}
