use std::net::SocketAddr;

use chrono::{TimeDelta, Utc};
use defguard_common::{db::setup_pool, messages::peer_stats_update::PeerStatsUpdate};
use defguard_session_manager::{
    SESSION_UPDATE_INTERVAL, events::SessionManagerEventType, run_session_manager_iteration,
};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use tokio::time::{Duration, interval};

use crate::common::{
    SessionManagerHarness, attach_device_to_network, create_device, create_gateway, create_network,
    create_user,
};

#[sqlx::test]
async fn test_session_manager_emits_connected_event(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let network = create_network(&pool).await;
    let user = create_user(&pool).await;
    let device = create_device(&pool, user.id).await;
    attach_device_to_network(&pool, network.id, device.id).await;
    let gateway = create_gateway(&pool, network.id, user.fullname()).await;

    let mut harness = SessionManagerHarness::new(pool);

    let endpoint: SocketAddr = "203.0.113.10:51820".parse().unwrap();
    let base_time = Utc::now().naive_utc();
    let update = PeerStatsUpdate {
        location_id: network.id,
        gateway_id: gateway.id,
        device_pubkey: device.wireguard_pubkey.clone(),
        collected_at: base_time,
        endpoint,
        upload: 100,
        download: 200,
        latest_handshake: base_time - TimeDelta::seconds(5),
    };

    harness.send_stats(update);

    let mut session_update_timer = interval(Duration::from_secs(SESSION_UPDATE_INTERVAL));
    let _ = run_session_manager_iteration(
        &mut harness.manager,
        &mut harness.stats_rx,
        &mut session_update_timer,
    )
    .await
    .expect("session manager iteration failed");

    let event = harness
        .event_rx
        .recv()
        .await
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
