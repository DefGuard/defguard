use defguard_proto::gateway::{
    CoreResponse, Update, UpdateType,
    core_response,
    update::{self},
};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use tonic::Status;

use super::support::{HandlerTestContext, build_peer_stats, reload_gateway};
use defguard_core::grpc::GatewayEvent;

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
async fn test_forwards_valid_peer_stats_after_config(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
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
async fn test_matching_location_network_event_produces_outbound_update(
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
async fn test_different_location_network_event_is_ignored(
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
