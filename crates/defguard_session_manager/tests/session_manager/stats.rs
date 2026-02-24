use std::net::SocketAddr;

use chrono::{TimeDelta, Utc};
use defguard_common::db::models::vpn_session_stats::VpnSessionStats;
use defguard_common::db::setup_pool;
use defguard_common::messages::peer_stats_update::PeerStatsUpdate;
use defguard_session_manager::{SESSION_UPDATE_INTERVAL, run_session_manager_iteration};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use tokio::time::{Duration, interval};

use crate::common::{
    SessionManagerHarness, attach_device_to_network, create_device, create_gateway, create_network,
    create_user,
};

#[sqlx::test]
async fn test_session_manager_updates_stats(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let network = create_network(&pool).await;
    let user = create_user(&pool).await;
    let device = create_device(&pool, user.id).await;
    attach_device_to_network(&pool, network.id, device.id).await;
    let gateway = create_gateway(&pool, network.id, user.id).await;

    let mut harness = SessionManagerHarness::new(pool.clone());

    let endpoint: SocketAddr = "203.0.113.10:51820".parse().unwrap();
    let base_time = Utc::now().naive_utc();
    let first_update = PeerStatsUpdate {
        location_id: network.id,
        gateway_id: gateway.id,
        device_pubkey: device.wireguard_pubkey.clone(),
        collected_at: base_time,
        endpoint,
        upload: 100,
        download: 200,
        latest_handshake: base_time - TimeDelta::seconds(5),
    };

    harness.send_stats(first_update);

    let mut session_update_timer = interval(Duration::from_secs(SESSION_UPDATE_INTERVAL));
    let _ = run_session_manager_iteration(
        &mut harness.manager,
        &mut harness.stats_rx,
        &mut session_update_timer,
    )
    .await
    .expect("session manager iteration failed");

    let first_stats = VpnSessionStats::fetch_latest_for_device(&pool, device.id, network.id)
        .await
        .expect("failed to query session stats")
        .expect("expected session stats");
    assert_eq!(first_stats.upload_diff, 0);
    assert_eq!(first_stats.download_diff, 0);

    let second_update = PeerStatsUpdate {
        location_id: network.id,
        gateway_id: gateway.id,
        device_pubkey: device.wireguard_pubkey.clone(),
        collected_at: base_time + TimeDelta::seconds(10),
        endpoint,
        upload: 150,
        download: 260,
        latest_handshake: base_time + TimeDelta::seconds(10),
    };

    harness.send_stats(second_update);

    let _ = run_session_manager_iteration(
        &mut harness.manager,
        &mut harness.stats_rx,
        &mut session_update_timer,
    )
    .await
    .expect("session manager iteration failed");

    let second_stats = VpnSessionStats::fetch_latest_for_device(&pool, device.id, network.id)
        .await
        .expect("failed to query session stats")
        .expect("expected session stats");
    assert_eq!(second_stats.upload_diff, 50);
    assert_eq!(second_stats.download_diff, 60);
}
