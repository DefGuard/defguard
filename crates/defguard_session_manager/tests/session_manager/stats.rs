use chrono::{TimeDelta, Utc};
use defguard_common::db::models::vpn_session_stats::VpnSessionStats;
use defguard_common::db::setup_pool;
use defguard_common::messages::peer_stats_update::PeerStatsUpdate;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use tokio::time::{Duration, timeout};

use crate::common::{
    attach_device_to_network, create_device, create_gateway, create_network, create_user,
    start_session_manager,
};

const DB_WAIT_TIMEOUT: Duration = Duration::from_secs(2);

async fn wait_for_latest_stats(
    pool: &sqlx::PgPool,
    device_id: defguard_common::db::Id,
    location_id: defguard_common::db::Id,
    expected_upload: i64,
    expected_download: i64,
) -> VpnSessionStats<defguard_common::db::Id> {
    timeout(DB_WAIT_TIMEOUT, async {
        loop {
            if let Ok(Some(stats)) =
                VpnSessionStats::fetch_latest_for_device(pool, device_id, location_id).await
            {
                if stats.total_upload == expected_upload
                    && stats.total_download == expected_download
                {
                    return stats;
                }
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .expect("timed out waiting for latest stats")
}

#[sqlx::test]
async fn test_session_manager_updates_stats(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let network = create_network(&pool).await;
    let user = create_user(&pool).await;
    let device = create_device(&pool, user.id).await;
    attach_device_to_network(&pool, network.id, device.id).await;
    let gateway = create_gateway(&pool, network.id, user.id).await;

    let manager = start_session_manager(pool.clone());

    let endpoint: std::net::SocketAddr = "203.0.113.10:51820".parse().unwrap();
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

    manager.send_stats(first_update);

    let first_stats = wait_for_latest_stats(&pool, device.id, network.id, 100, 200).await;
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

    manager.send_stats(second_update);

    let second_stats = wait_for_latest_stats(&pool, device.id, network.id, 150, 260).await;
    assert_eq!(second_stats.upload_diff, 50);
    assert_eq!(second_stats.download_diff, 60);
}
