use std::net::SocketAddr;

use chrono::{TimeDelta, Utc};
use defguard_common::db::setup_pool;
use defguard_common::messages::peer_stats_update::PeerStatsUpdate;
use defguard_session_manager::events::SessionManagerEventType;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

use crate::common::{
    attach_device_to_network, create_device, create_gateway, create_network, create_user,
    start_session_manager,
};

#[sqlx::test]
async fn test_session_manager_emits_connected_event(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let network = create_network(&pool).await;
    let user = create_user(&pool).await;
    let device = create_device(&pool, user.id).await;
    attach_device_to_network(&pool, network.id, device.id).await;
    let gateway = create_gateway(&pool, network.id, user.id).await;

    let mut manager = start_session_manager(pool);

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

    manager.send_stats(update);

    let event = manager.recv_event().await;

    assert!(matches!(
        event.event,
        SessionManagerEventType::ClientConnected
    ));
    assert_eq!(event.context.location.id, network.id);
    assert_eq!(event.context.user.id, user.id);
    assert_eq!(event.context.device.id, device.id);
    assert_eq!(event.context.public_ip, endpoint.ip());
}
