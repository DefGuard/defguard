use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    time::Duration,
};

use chrono::{NaiveDateTime, TimeDelta, Timelike, Utc};
use defguard_common::{
    db::{
        Id,
        models::{
            Device, DeviceType, User, WireguardNetwork,
            device::WireguardNetworkDevice,
            gateway::Gateway,
            vpn_client_session::{VpnClientMfaMethod, VpnClientSession},
            vpn_session_stats::VpnSessionStats,
            wireguard::{LocationMfaMode, ServiceLocationMode},
        },
    },
    messages::peer_stats_update::PeerStatsUpdate,
};
use defguard_session_manager::{
    IterationOutcome, SESSION_UPDATE_INTERVAL, SessionManager, events::SessionManagerEvent,
    run_session_manager_iteration,
};
use ipnetwork::IpNetwork;
use sqlx::{PgExecutor, query, query_scalar};
use tokio::{
    sync::{
        broadcast,
        mpsc::{self},
    },
    time::interval,
};

pub(crate) struct SessionManagerHarness {
    pub(crate) manager: SessionManager,
    stats_tx: mpsc::UnboundedSender<PeerStatsUpdate>,
    pub(crate) stats_rx: mpsc::UnboundedReceiver<PeerStatsUpdate>,
    pub(crate) event_rx: mpsc::UnboundedReceiver<SessionManagerEvent>,
    pub(crate) gateway_rx: broadcast::Receiver<defguard_core::grpc::GatewayEvent>,
}

pub(crate) fn assert_no_session_manager_events(harness: &mut SessionManagerHarness) {
    match harness.event_rx.try_recv() {
        Err(mpsc::error::TryRecvError::Empty) => {}
        Err(mpsc::error::TryRecvError::Disconnected) => {
            panic!("session manager event channel disconnected unexpectedly")
        }
        Ok(event) => panic!("unexpected session manager event: {event:?}"),
    }
}

pub(crate) fn assert_no_gateway_events(harness: &mut SessionManagerHarness) {
    match harness.gateway_rx.try_recv() {
        Err(broadcast::error::TryRecvError::Empty) => {}
        Err(broadcast::error::TryRecvError::Closed) => {
            panic!("gateway event channel closed unexpectedly")
        }
        Err(broadcast::error::TryRecvError::Lagged(skipped)) => {
            panic!("gateway event channel lagged and skipped {skipped} events")
        }
        Ok(event) => panic!("unexpected gateway event: {event:?}"),
    }
}

impl SessionManagerHarness {
    pub(crate) fn new(pool: sqlx::PgPool) -> Self {
        let (stats_tx, stats_rx) = mpsc::unbounded_channel();
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let (gateway_tx, gateway_rx) = broadcast::channel(16);
        let manager = SessionManager::new(pool, event_tx, gateway_tx);

        Self {
            manager,
            stats_tx,
            stats_rx,
            event_rx,
            gateway_rx,
        }
    }

    pub(crate) fn send_stats(&self, update: PeerStatsUpdate) {
        self.stats_tx
            .send(update)
            .expect("failed to send peer stats update");
    }

    pub(crate) fn close_event_channel(&mut self) {
        self.event_rx.close();
    }

    pub(crate) async fn run_iteration(&mut self) -> IterationOutcome {
        let mut session_update_timer = interval(Duration::from_secs(SESSION_UPDATE_INTERVAL));
        run_session_manager_iteration(
            &mut self.manager,
            &mut self.stats_rx,
            &mut session_update_timer,
        )
        .await
        .expect("session manager iteration failed")
    }

    pub(crate) async fn run_idle_iteration(&mut self) -> IterationOutcome {
        let mut session_update_timer = interval(Duration::from_millis(1));
        run_session_manager_iteration(
            &mut self.manager,
            &mut self.stats_rx,
            &mut session_update_timer,
        )
        .await
        .expect("session manager iteration failed")
    }
}

pub(crate) async fn create_location(pool: &sqlx::PgPool) -> WireguardNetwork<Id> {
    create_location_with_mfa_mode(pool, LocationMfaMode::Disabled).await
}

pub(crate) async fn create_location_with_mfa_mode(
    pool: &sqlx::PgPool,
    location_mfa_mode: LocationMfaMode,
) -> WireguardNetwork<Id> {
    WireguardNetwork::new(
        "TestNet".to_string(),
        51820,
        "10.0.0.1".to_string(),
        None,
        vec![IpNetwork::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0).unwrap()],
        true,
        false,
        false,
        location_mfa_mode,
        ServiceLocationMode::Disabled,
    )
    .set_address([IpNetwork::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)), 24).unwrap()])
    .unwrap()
    .save(pool)
    .await
    .expect("failed to create WireGuard location")
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
    create_device_with_pubkey(pool, user_id, "device-pubkey-test").await
}

pub(crate) async fn create_device_with_pubkey(
    pool: &sqlx::PgPool,
    user_id: Id,
    wireguard_pubkey: &str,
) -> Device<Id> {
    Device::new(
        "session-test-device".to_string(),
        wireguard_pubkey.to_string(),
        user_id,
        DeviceType::User,
        None,
        true,
    )
    .save(pool)
    .await
    .expect("failed to create device")
}

pub(crate) async fn attach_device_to_location(pool: &sqlx::PgPool, location_id: Id, device_id: Id) {
    let network_device = WireguardNetworkDevice::new(
        location_id,
        device_id,
        [IpAddr::V4(Ipv4Addr::new(10, 0, 0, 10))],
    );
    network_device
        .insert(pool)
        .await
        .expect("failed to attach device to location");
}

pub(crate) async fn create_gateway(
    pool: &sqlx::PgPool,
    location_id: Id,
    modified_by: String,
) -> Gateway<Id> {
    create_gateway_named(pool, location_id, modified_by, "gateway-1").await
}

pub(crate) async fn create_gateway_named(
    pool: &sqlx::PgPool,
    location_id: Id,
    modified_by: String,
    name: &str,
) -> Gateway<Id> {
    Gateway::new(
        location_id,
        name.to_string(),
        "127.0.0.1".to_string(),
        51820,
        modified_by,
    )
    .save(pool)
    .await
    .expect("failed to create gateway")
}

pub(crate) async fn authorize_device_in_location(
    pool: &sqlx::PgPool,
    location_id: Id,
    device_id: Id,
    preshared_key: &str,
) {
    let mut network_device = WireguardNetworkDevice::find(pool, device_id, location_id)
        .await
        .expect("failed to load device network info")
        .expect("expected device network info");
    network_device.is_authorized = true;
    network_device.authorized_at = Some(chrono::Utc::now().naive_utc());
    network_device.preshared_key = Some(preshared_key.to_string());
    network_device
        .update(pool)
        .await
        .expect("failed to authorize device in location");
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn build_stats_update(
    location_id: Id,
    gateway_id: Id,
    device_pubkey: impl Into<String>,
    collected_at: NaiveDateTime,
    endpoint: SocketAddr,
    upload: u64,
    download: u64,
    latest_handshake: NaiveDateTime,
) -> PeerStatsUpdate {
    PeerStatsUpdate {
        location_id,
        gateway_id,
        device_pubkey: device_pubkey.into(),
        collected_at: truncate_timestamp(collected_at),
        endpoint,
        upload,
        download,
        latest_handshake: truncate_timestamp(latest_handshake),
    }
}

pub(crate) fn truncate_timestamp(timestamp: NaiveDateTime) -> NaiveDateTime {
    timestamp
        .with_nanosecond((timestamp.nanosecond() / 1_000) * 1_000)
        .expect("failed to truncate timestamp precision")
}

pub(crate) fn stale_session_timestamp(location: &WireguardNetwork<Id>) -> NaiveDateTime {
    let reference_time = Utc::now().naive_utc();
    reference_time
        .checked_sub_signed(TimeDelta::seconds(
            i64::from(location.peer_disconnect_threshold) + 1,
        ))
        .expect("reference timestamp should stay within range")
}

pub(crate) async fn create_session(
    pool: &sqlx::PgPool,
    location_id: Id,
    user_id: Id,
    device_id: Id,
    connected_at: Option<NaiveDateTime>,
    mfa_method: Option<VpnClientMfaMethod>,
) -> VpnClientSession<Id> {
    VpnClientSession::new(location_id, user_id, device_id, connected_at, mfa_method)
        .save(pool)
        .await
        .expect("failed to create vpn client session")
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn create_session_stats(
    pool: &sqlx::PgPool,
    session_id: Id,
    gateway_id: Id,
    collected_at: NaiveDateTime,
    latest_handshake: NaiveDateTime,
    endpoint: SocketAddr,
    total_upload: i64,
    total_download: i64,
    upload_diff: i64,
    download_diff: i64,
) -> VpnSessionStats<Id> {
    VpnSessionStats::new(
        session_id,
        gateway_id,
        collected_at,
        latest_handshake,
        endpoint.to_string(),
        total_upload,
        total_download,
        upload_diff,
        download_diff,
    )
    .save(pool)
    .await
    .expect("failed to create vpn session stats")
}

pub(crate) async fn set_session_created_at<'e, E: PgExecutor<'e>>(
    executor: E,
    session_id: Id,
    created_at: NaiveDateTime,
) {
    query("UPDATE vpn_client_session SET created_at = $1 WHERE id = $2")
        .bind(created_at)
        .bind(session_id)
        .execute(executor)
        .await
        .expect("failed to update session created_at");
}

pub(crate) async fn count_session_stats<'e, E: PgExecutor<'e>>(executor: E, session_id: Id) -> i64 {
    query_scalar("SELECT COUNT(*) FROM vpn_session_stats WHERE session_id = $1")
        .bind(session_id)
        .fetch_one(executor)
        .await
        .expect("failed to count vpn session stats")
}

pub(crate) async fn count_stats_for_device_location<'e, E: PgExecutor<'e>>(
    executor: E,
    device_id: Id,
    location_id: Id,
) -> i64 {
    query_scalar(
        "SELECT COUNT(*) \
         FROM vpn_session_stats stats \
         JOIN vpn_client_session session ON stats.session_id = session.id \
         WHERE session.device_id = $1 AND session.location_id = $2",
    )
    .bind(device_id)
    .bind(location_id)
    .fetch_one(executor)
    .await
    .expect("failed to count device session stats")
}
