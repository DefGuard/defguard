use std::net::SocketAddr;

use chrono::{TimeDelta, Utc};
use defguard_common::db::{models::vpn_client_session::VpnClientSession, setup_pool};
use defguard_session_manager::events::SessionManagerEventType;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use tokio::time::{Duration, timeout};

use crate::common::{
    SessionManagerHarness, attach_device_to_network, build_stats_update, create_device,
    create_gateway, create_network, create_session, create_session_stats, create_user,
};

const RECEIVE_TIMEOUT: Duration = Duration::from_secs(1);

#[sqlx::test]
async fn test_session_manager_emits_connected_event_for_first_stats(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let network = create_network(&pool).await;
    let user = create_user(&pool).await;
    let device = create_device(&pool, user.id).await;
    attach_device_to_network(&pool, network.id, device.id).await;
    let gateway = create_gateway(&pool, network.id, user.fullname()).await;

    let mut harness = SessionManagerHarness::new(pool);

    let endpoint: SocketAddr = "203.0.113.10:51820".parse().unwrap();
    let handshake = Utc::now().naive_utc() - TimeDelta::seconds(5);
    harness.send_stats(build_stats_update(
        network.id,
        gateway.id,
        &device.wireguard_pubkey,
        handshake,
        endpoint,
        100,
        200,
        handshake,
    ));

    let _ = harness.run_iteration().await;

    let event = timeout(RECEIVE_TIMEOUT, harness.event_rx.recv())
        .await
        .expect("timed out waiting for ClientConnected event")
        .expect("session manager event channel closed");

    assert!(matches!(
        event.event,
        SessionManagerEventType::ClientConnected
    ));
    assert_eq!(event.context.location.id, network.id);
    assert_eq!(event.context.user.id, user.id);
    assert_eq!(event.context.device.id, device.id);
    assert_eq!(event.context.public_ip, endpoint.ip());
}

#[sqlx::test]
async fn test_reusing_existing_connected_session_does_not_emit_duplicate_connected_event(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let network = create_network(&pool).await;
    let user = create_user(&pool).await;
    let device = create_device(&pool, user.id).await;
    attach_device_to_network(&pool, network.id, device.id).await;
    let gateway = create_gateway(&pool, network.id, user.fullname()).await;
    let mut harness = SessionManagerHarness::new(pool.clone());

    let connected_at = Utc::now().naive_utc() - TimeDelta::seconds(5);
    let _session = create_session(
        &pool,
        network.id,
        user.id,
        device.id,
        Some(connected_at),
        None,
    )
    .await;

    let endpoint: SocketAddr = "203.0.113.10:51820".parse().unwrap();
    harness.send_stats(build_stats_update(
        network.id,
        gateway.id,
        &device.wireguard_pubkey,
        connected_at,
        endpoint,
        100,
        200,
        connected_at,
    ));

    let _ = harness.run_iteration().await;

    assert!(harness.event_rx.try_recv().is_err());
}

#[sqlx::test]
async fn test_session_manager_emits_disconnect_event_for_inactive_standard_session(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let network = create_network(&pool).await;
    let user = create_user(&pool).await;
    let device = create_device(&pool, user.id).await;
    attach_device_to_network(&pool, network.id, device.id).await;
    let gateway = create_gateway(&pool, network.id, user.fullname()).await;
    let mut harness = SessionManagerHarness::new(pool.clone());

    let stale_handshake = Utc::now().naive_utc() - TimeDelta::seconds(301);
    let session = create_session(
        &pool,
        network.id,
        user.id,
        device.id,
        Some(stale_handshake),
        None,
    )
    .await;
    create_session_stats(
        &pool,
        session.id,
        gateway.id,
        stale_handshake,
        stale_handshake,
        "203.0.113.10:51820".parse().unwrap(),
        100,
        200,
        0,
        0,
    )
    .await;

    let _ = harness.run_idle_iteration().await;

    let event = timeout(RECEIVE_TIMEOUT, harness.event_rx.recv())
        .await
        .expect("timed out waiting for ClientDisconnected event")
        .expect("session manager event channel closed");
    assert!(matches!(
        event.event,
        SessionManagerEventType::ClientDisconnected
    ));
    assert_eq!(event.context.location.id, network.id);
    assert_eq!(event.context.user.id, user.id);
    assert_eq!(event.context.device.id, device.id);

    let disconnected_session = VpnClientSession::find_by_id(&pool, session.id)
        .await
        .expect("failed to query session")
        .expect("expected session");
    assert!(disconnected_session.disconnected_at.is_some());
}
