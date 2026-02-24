use chrono::{TimeDelta, Utc};
use defguard_common::db::models::vpn_client_session::{VpnClientSession, VpnClientSessionState};
use defguard_common::db::setup_pool;
use defguard_common::messages::peer_stats_update::PeerStatsUpdate;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use tokio::time::{Duration, timeout};

use crate::common::{
    attach_device_to_network, create_device, create_gateway, create_network, create_user,
    start_session_manager,
};

const DB_WAIT_TIMEOUT: Duration = Duration::from_secs(2);

async fn wait_for_active_session(
    pool: &sqlx::PgPool,
    location_id: defguard_common::db::Id,
    device_id: defguard_common::db::Id,
) -> VpnClientSession<defguard_common::db::Id> {
    timeout(DB_WAIT_TIMEOUT, async {
        loop {
            if let Ok(Some(session)) =
                VpnClientSession::try_get_active_session(pool, location_id, device_id).await
            {
                return session;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .expect("timed out waiting for active session")
}

#[sqlx::test]
async fn test_session_manager_creates_active_session(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let network = create_network(&pool).await;
    let user = create_user(&pool).await;
    let device = create_device(&pool, user.id).await;
    attach_device_to_network(&pool, network.id, device.id).await;
    let gateway = create_gateway(&pool, network.id, user.id).await;

    let manager = start_session_manager(pool.clone());

    let base_time = Utc::now().naive_utc();
    let update = PeerStatsUpdate {
        location_id: network.id,
        gateway_id: gateway.id,
        device_pubkey: device.wireguard_pubkey.clone(),
        collected_at: base_time,
        endpoint: "203.0.113.10:51820".parse().unwrap(),
        upload: 100,
        download: 200,
        latest_handshake: base_time - TimeDelta::seconds(5),
    };

    manager.send_stats(update);

    let session = wait_for_active_session(&pool, network.id, device.id).await;
    assert_eq!(session.state, VpnClientSessionState::Connected);
}
