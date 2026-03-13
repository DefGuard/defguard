use std::net::{IpAddr, Ipv4Addr};

use defguard_common::{
    db::{
        Id,
        models::{
            Device, DeviceType, User, WireguardNetwork,
            device::WireguardNetworkDevice,
            gateway::Gateway,
            wireguard::{LocationMfaMode, ServiceLocationMode},
        },
    },
    messages::peer_stats_update::PeerStatsUpdate,
};
use defguard_session_manager::{SessionManager, events::SessionManagerEvent};
use ipnetwork::IpNetwork;
use tokio::sync::{broadcast, mpsc};

pub(crate) struct SessionManagerHarness {
    pub(crate) manager: SessionManager,
    stats_tx: mpsc::UnboundedSender<PeerStatsUpdate>,
    pub(crate) stats_rx: mpsc::UnboundedReceiver<PeerStatsUpdate>,
    pub(crate) event_rx: mpsc::UnboundedReceiver<SessionManagerEvent>,
}

impl SessionManagerHarness {
    pub(crate) fn new(pool: sqlx::PgPool) -> Self {
        let (stats_tx, stats_rx) = mpsc::unbounded_channel();
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let (gateway_tx, _gateway_rx) = broadcast::channel(16);
        let manager = SessionManager::new(pool, event_tx, gateway_tx);

        Self {
            manager,
            stats_tx,
            stats_rx,
            event_rx,
        }
    }

    pub(crate) fn send_stats(&self, update: PeerStatsUpdate) {
        self.stats_tx
            .send(update)
            .expect("failed to send peer stats update");
    }
}

pub(crate) async fn create_network(pool: &sqlx::PgPool) -> WireguardNetwork<Id> {
    WireguardNetwork::new(
        "TestNet".to_string(),
        vec![IpNetwork::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 0)), 24).unwrap()],
        51820,
        "10.0.0.1".to_string(),
        None,
        1420,
        0,
        vec![IpNetwork::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0).unwrap()],
        true,
        25,
        300,
        false,
        false,
        LocationMfaMode::Disabled,
        ServiceLocationMode::Disabled,
    )
    .save(pool)
    .await
    .expect("failed to create Wireguard network")
}

pub(crate) async fn create_user(pool: &sqlx::PgPool) -> User<Id> {
    User::new(
        "session-test",
        Some("pass123"),
        "Tester",
        "Session",
        "session-test@example.com",
        None,
    )
    .save(pool)
    .await
    .expect("failed to create user")
}

pub(crate) async fn create_device(pool: &sqlx::PgPool, user_id: Id) -> Device<Id> {
    Device::new(
        "session-test-device".to_string(),
        "device-pubkey-test".to_string(),
        user_id,
        DeviceType::User,
        None,
        true,
    )
    .save(pool)
    .await
    .expect("failed to create device")
}

pub(crate) async fn attach_device_to_network(pool: &sqlx::PgPool, network_id: Id, device_id: Id) {
    let network_device = WireguardNetworkDevice::new(
        network_id,
        device_id,
        vec![IpAddr::V4(Ipv4Addr::new(10, 0, 0, 10))],
    );
    network_device
        .insert(pool)
        .await
        .expect("failed to attach device to network");
}

pub(crate) async fn create_gateway(
    pool: &sqlx::PgPool,
    network_id: Id,
    modified_by: String,
) -> Gateway<Id> {
    Gateway::new(
        network_id,
        "gateway-1".to_string(),
        "127.0.0.1".to_string(),
        51820,
        modified_by,
    )
    .save(pool)
    .await
    .expect("failed to create gateway")
}
