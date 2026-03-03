use chrono::{TimeDelta, Utc};
use defguard_common::{
    db::{
        models::vpn_client_session::{VpnClientSession, VpnClientSessionState},
        setup_pool,
    },
    messages::peer_stats_update::PeerStatsUpdate,
};
use defguard_session_manager::{SESSION_UPDATE_INTERVAL, run_session_manager_iteration};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use tokio::time::{Duration, interval};

use crate::common::{
    SessionManagerHarness, attach_device_to_network, create_device, create_gateway, create_network,
    create_user,
};

#[sqlx::test]
async fn test_session_manager_creates_active_session(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let network = create_network(&pool).await;
    let user = create_user(&pool).await;
    let device = create_device(&pool, user.id).await;
    attach_device_to_network(&pool, network.id, device.id).await;
    let gateway = create_gateway(&pool, network.id, user.id).await;

    let mut harness = SessionManagerHarness::new(pool.clone());

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

    harness.send_stats(update);

    let mut session_update_timer = interval(Duration::from_secs(SESSION_UPDATE_INTERVAL));
    let _ = run_session_manager_iteration(
        &mut harness.manager,
        &mut harness.stats_rx,
        &mut session_update_timer,
    )
    .await
    .expect("session manager iteration failed");

    let session = VpnClientSession::try_get_active_session(&pool, network.id, device.id)
        .await
        .expect("failed to query active session")
        .expect("expected active session");
    assert_eq!(session.state, VpnClientSessionState::Connected);
}
