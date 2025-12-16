use std::{
    collections::HashMap,
    net::IpAddr,
    sync::{Arc, Mutex},
};

use chrono::{DateTime, Utc};
use defguard_common::{
    config::server_config,
    db::{ChangeNotification, Id, NoId, TriggerOperation},
};
use defguard_mail::Mail;
use defguard_proto::{
    enterprise::firewall::FirewallConfig,
    gateway::{Configuration, CoreResponse, Peer, PeerStats, Update, core_response, update},
};
use sqlx::{PgPool, postgres::PgListener};
use tokio::{
    sync::{
        broadcast::{Receiver as BroadcastReceiver, Sender},
        mpsc::UnboundedSender,
    },
    task::{AbortHandle, JoinSet},
};
use tonic::{Code, Status};

use crate::{
    db::{
        GatewayEvent,
        models::{
            gateway::Gateway, wireguard::WireguardNetwork, wireguard_peer_stats::WireguardPeerStats,
        },
    },
    events::{GrpcEvent, GrpcRequestContext},
    grpc::gateway::{client_state::ClientMap, handler::GatewayHandler},
};

pub mod client_state;
pub(crate) mod handler;
pub mod map;
pub(crate) mod state;
#[cfg(test)]
mod tests;

#[cfg(test)]
pub(super) static TONIC_SOCKET: &str = "tonic.sock";

/// Sends given `GatewayEvent` to be handled by gateway GRPC server
///
/// If you want to use it inside the API context, use [`crate::AppState::send_wireguard_event`] instead
pub fn send_wireguard_event(event: GatewayEvent, wg_tx: &Sender<GatewayEvent>) {
    debug!("Sending the following WireGuard event to Defguard Gateway: {event:?}");
    if let Err(err) = wg_tx.send(event) {
        error!("Error sending WireGuard event {err}");
    }
}

/// Sends multiple events to be handled by gateway GRPC server
///
/// If you want to use it inside the API context, use [`crate::AppState::send_multiple_wireguard_events`] instead
pub fn send_multiple_wireguard_events(events: Vec<GatewayEvent>, wg_tx: &Sender<GatewayEvent>) {
    debug!("Sending {} WireGuard events", events.len());
    for event in events {
        send_wireguard_event(event, wg_tx);
    }
}

fn gen_config(
    network: &WireguardNetwork<Id>,
    peers: Vec<Peer>,
    maybe_firewall_config: Option<FirewallConfig>,
) -> Configuration {
    Configuration {
        name: network.name.clone(),
        port: network.port as u32,
        prvkey: network.prvkey.clone(),
        addresses: network.address.iter().map(ToString::to_string).collect(),
        peers,
        firewall_config: maybe_firewall_config,
    }
}

impl WireguardPeerStats {
    fn from_peer_stats(stats: PeerStats, network_id: Id, device_id: Id) -> Self {
        let endpoint = match stats.endpoint {
            endpoint if endpoint.is_empty() => None,
            _ => Some(stats.endpoint),
        };
        Self {
            id: NoId,
            network: network_id,
            endpoint,
            device_id,
            collected_at: Utc::now().naive_utc(),
            upload: stats.upload as i64,
            download: stats.download as i64,
            latest_handshake: DateTime::from_timestamp(stats.latest_handshake as i64, 0)
                .unwrap_or_default()
                .naive_utc(),
            allowed_ips: Some(stats.allowed_ips),
        }
    }
}

const GATEWAY_TABLE_TRIGGER: &str = "gateway_change";

/// Bi-directional gRPC stream for comminication with Defguard Gateway.
pub async fn run_grpc_gateway_stream(
    pool: PgPool,
    client_state: Arc<Mutex<ClientMap>>,
    events_tx: Sender<GatewayEvent>,
    mail_tx: UnboundedSender<Mail>,
    grpc_event_tx: UnboundedSender<GrpcEvent>,
) -> Result<(), anyhow::Error> {
    let config = server_config();
    let tls_config = config.grpc_client_tls_config()?;

    let mut abort_handles = HashMap::new();

    let mut tasks = JoinSet::new();
    // Helper closure to launch `GatewayHandler`.
    let mut launch_gateway_handler =
        |gateway: Gateway<Id>| -> Result<AbortHandle, tonic::transport::Error> {
            let mut gateway_handler = GatewayHandler::new(
                gateway,
                tls_config.clone(),
                pool.clone(),
                Arc::clone(&client_state),
                events_tx.clone(),
                mail_tx.clone(),
                grpc_event_tx.clone(),
            )?;
            let abort_handle = tasks.spawn(async move {
                gateway_handler.handle_connection().await;
            });
            Ok(abort_handle)
        };

    for gateway in Gateway::all(&pool).await? {
        let id = gateway.id;
        let abort_handle = launch_gateway_handler(gateway)?;
        abort_handles.insert(id, abort_handle);
    }

    // Observe gateway URL changes.
    let mut listener = PgListener::connect_with(&pool).await?;
    listener.listen(GATEWAY_TABLE_TRIGGER).await?;
    while let Ok(notification) = listener.recv().await {
        let payload = notification.payload();
        match serde_json::from_str::<ChangeNotification<Gateway<Id>>>(payload) {
            Ok(gateway_notification) => match gateway_notification.operation {
                TriggerOperation::Insert => {
                    if let Some(new) = gateway_notification.new {
                        let id = new.id;
                        let abort_handle = launch_gateway_handler(new)?;
                        abort_handles.insert(id, abort_handle);
                    }
                }
                TriggerOperation::Update => {
                    if let (Some(old), Some(new)) =
                        (gateway_notification.old, gateway_notification.new)
                    {
                        if old.url == new.url {
                            debug!(
                                "Gateway URL didn't change. Keeping the current gateway handler"
                            );
                        } else if let Some(abort_handle) = abort_handles.remove(&old.id) {
                            info!("Aborting connection to {old}, it has changed in the database");
                            abort_handle.abort();
                            let id = new.id;
                            let abort_handle = launch_gateway_handler(new)?;
                            abort_handles.insert(id, abort_handle);
                        } else {
                            warn!("Cannot find {old} on the list of connected gateways");
                        }
                    }
                }
                TriggerOperation::Delete => {
                    if let Some(old) = gateway_notification.old {
                        if let Some(abort_handle) = abort_handles.remove(&old.id) {
                            info!(
                                "Aborting connection to {old}, it has disappeard from the database"
                            );
                            abort_handle.abort();
                        } else {
                            warn!("Cannot find {old} on the list of connected gateways");
                        }
                    }
                }
            },
            Err(err) => error!("Failed to de-serialize database notification object: {err}"),
        }
    }

    while let Some(Ok(_result)) = tasks.join_next().await {
        debug!("Gateway gRPC task has ended");
    }

    Ok(())
}

/// Helper struct for handling gateway events.
struct GatewayUpdatesHandler {
    network_id: Id,
    network: WireguardNetwork<Id>,
    gateway_hostname: String,
    events_rx: BroadcastReceiver<GatewayEvent>,
    tx: UnboundedSender<CoreResponse>,
}

impl GatewayUpdatesHandler {
    pub fn new(
        network_id: Id,
        network: WireguardNetwork<Id>,
        gateway_hostname: String,
        events_rx: BroadcastReceiver<GatewayEvent>,
        tx: UnboundedSender<CoreResponse>,
    ) -> Self {
        Self {
            network_id,
            network,
            gateway_hostname,
            events_rx,
            tx,
        }
    }

    /// Process incoming Gateway events
    ///
    /// Main gRPC server uses a shared channel for broadcasting all gateway events
    /// so the handler must determine if an event is relevant for the network being serviced
    pub async fn run(&mut self) {
        info!(
            "Starting update stream to gateway: {}, network {}",
            self.gateway_hostname, self.network
        );
        while let Ok(update) = self.events_rx.recv().await {
            debug!("Received WireGuard update: {update:?}");
            let result = match update {
                GatewayEvent::NetworkCreated(network_id, network) => {
                    if network_id == self.network_id {
                        self.send_network_update(&network, Vec::new(), None, 0)
                            .await
                    } else {
                        Ok(())
                    }
                }
                GatewayEvent::NetworkModified(
                    network_id,
                    network,
                    peers,
                    maybe_firewall_config,
                ) => {
                    if network_id == self.network_id {
                        let result = self
                            .send_network_update(&network, peers, maybe_firewall_config, 1)
                            .await;
                        // update stored network data
                        self.network = network;
                        result
                    } else {
                        Ok(())
                    }
                }
                GatewayEvent::NetworkDeleted(network_id, network_name) => {
                    if network_id == self.network_id {
                        self.send_network_delete(&network_name).await
                    } else {
                        Ok(())
                    }
                }
                GatewayEvent::DeviceCreated(device) => {
                    // check if a peer has to be added in the current network
                    match device
                        .network_info
                        .iter()
                        .find(|info| info.network_id == self.network_id)
                    {
                        Some(network_info) => {
                            if self.network.mfa_enabled() && !network_info.is_authorized {
                                debug!(
                                    "Created WireGuard device {} is not authorized to connect to \
                                    MFA enabled location {}",
                                    device.device.name, self.network.name
                                );
                                continue;
                            }
                            self.send_peer_update(
                                Peer {
                                    pubkey: device.device.wireguard_pubkey,
                                    allowed_ips: network_info
                                        .device_wireguard_ips
                                        .iter()
                                        .map(IpAddr::to_string)
                                        .collect(),
                                    preshared_key: network_info.preshared_key.clone(),
                                    keepalive_interval: Some(
                                        self.network.keepalive_interval as u32,
                                    ),
                                },
                                0,
                            )
                            .await
                        }
                        None => Ok(()),
                    }
                }
                GatewayEvent::DeviceModified(device) => {
                    // check if a peer has to be updated in the current network
                    match device
                        .network_info
                        .iter()
                        .find(|info| info.network_id == self.network_id)
                    {
                        Some(network_info) => {
                            if self.network.mfa_enabled() && !network_info.is_authorized {
                                debug!(
                                    "Modified WireGuard device {} is not authorized to connect to \
                                    MFA enabled location {}",
                                    device.device.name, self.network.name
                                );
                                continue;
                            }
                            self.send_peer_update(
                                Peer {
                                    pubkey: device.device.wireguard_pubkey,
                                    allowed_ips: network_info
                                        .device_wireguard_ips
                                        .iter()
                                        .map(IpAddr::to_string)
                                        .collect(),
                                    preshared_key: network_info.preshared_key.clone(),
                                    keepalive_interval: Some(
                                        self.network.keepalive_interval as u32,
                                    ),
                                },
                                1,
                            )
                            .await
                        }
                        None => Ok(()),
                    }
                }
                GatewayEvent::DeviceDeleted(device) => {
                    // check if a peer has to be updated in the current network
                    match device
                        .network_info
                        .iter()
                        .find(|info| info.network_id == self.network_id)
                    {
                        Some(_) => self.send_peer_delete(&device.device.wireguard_pubkey).await,
                        None => Ok(()),
                    }
                }
                GatewayEvent::FirewallConfigChanged(location_id, firewall_config) => {
                    if location_id == self.network_id {
                        self.send_firewall_update(firewall_config).await
                    } else {
                        Ok(())
                    }
                }
                GatewayEvent::FirewallDisabled(location_id) => {
                    if location_id == self.network_id {
                        self.send_firewall_disable().await
                    } else {
                        Ok(())
                    }
                }
            };
            if result.is_err() {
                error!(
                    "Closing update steam to gateway: {}, network {}",
                    self.gateway_hostname, self.network
                );
                break;
            }
        }
    }

    /// Sends updated network configuration
    async fn send_network_update(
        &self,
        network: &WireguardNetwork<Id>,
        peers: Vec<Peer>,
        firewall_config: Option<FirewallConfig>,
        update_type: i32,
    ) -> Result<(), Status> {
        debug!("Sending network update for network {network}");
        if let Err(err) = self.tx.send(CoreResponse {
            id: 0,
            payload: Some(core_response::Payload::Update(Update {
                update_type,
                update: Some(update::Update::Network(Configuration {
                    name: network.name.clone(),
                    prvkey: network.prvkey.clone(),
                    addresses: network.address.iter().map(ToString::to_string).collect(),
                    port: network.port as u32,
                    peers,
                    firewall_config,
                })),
            })),
        }) {
            let msg = format!(
                "Failed to send network update, network {network}, update type: {update_type} \
                ({}), error: {err}",
                if update_type == 0 { "CREATE" } else { "MODIFY" },
            );
            error!(msg);
            return Err(Status::new(Code::Internal, msg));
        }
        debug!("Network update sent for network {network}");
        Ok(())
    }

    /// Sends delete network command to gateway
    async fn send_network_delete(&self, network_name: &str) -> Result<(), Status> {
        debug!(
            "Sending network delete command for network {}",
            self.network
        );
        if let Err(err) = self.tx.send(CoreResponse {
            id: 0,
            payload: Some(core_response::Payload::Update(Update {
                update_type: 2,
                update: Some(update::Update::Network(Configuration {
                    name: network_name.to_string(),
                    prvkey: String::new(),
                    addresses: Vec::new(),
                    port: 0,
                    peers: Vec::new(),
                    firewall_config: None,
                })),
            })),
        }) {
            let msg = format!(
                "Failed to send network update, network {}, update type: 2 (DELETE), error: {err}",
                self.network,
            );
            error!(msg);
            return Err(Status::new(Code::Internal, msg));
        }
        debug!("Network delete command sent for network {}", self.network);
        Ok(())
    }

    /// Send update peer command to gateway
    async fn send_peer_update(&self, peer: Peer, update_type: i32) -> Result<(), Status> {
        debug!("Sending peer update for network {}", self.network);
        if let Err(err) = self.tx.send(CoreResponse {
            id: 0,
            payload: Some(core_response::Payload::Update(Update {
                update_type,
                update: Some(update::Update::Peer(peer)),
            })),
        }) {
            let msg = format!(
                "Failed to send peer update for network {}, update type: {update_type} ({}), error: {err}",
                self.network,
                if update_type == 0 { "CREATE" } else { "MODIFY" },
            );
            error!(msg);
            return Err(Status::new(Code::Internal, msg));
        }
        debug!("Peer update sent for network {}", self.network);
        Ok(())
    }

    /// Send delete peer command to gateway
    async fn send_peer_delete(&self, peer_pubkey: &str) -> Result<(), Status> {
        debug!("Sending peer delete for network {}", self.network);
        if let Err(err) = self.tx.send(CoreResponse {
            id: 0,
            payload: Some(core_response::Payload::Update(Update {
                update_type: 2,
                update: Some(update::Update::Peer(Peer {
                    pubkey: peer_pubkey.into(),
                    allowed_ips: Vec::new(),
                    preshared_key: None,
                    keepalive_interval: None,
                })),
            })),
        }) {
            let msg = format!(
                "Failed to send peer update for network {}, peer {peer_pubkey}, update type: 2 (DELETE), error: {err}",
                self.network,
            );
            error!(msg);
            return Err(Status::new(Code::Internal, msg));
        }
        debug!("Peer delete command sent for network {}", self.network);
        Ok(())
    }

    /// Send firewall config update command to gateway
    async fn send_firewall_update(&self, firewall_config: FirewallConfig) -> Result<(), Status> {
        debug!(
            "Sending firewall config update for network {} with config {firewall_config:?}",
            self.network
        );
        if let Err(err) = self.tx.send(CoreResponse {
            id: 0,
            payload: Some(core_response::Payload::Update(Update {
                update_type: 1,
                update: Some(update::Update::FirewallConfig(firewall_config)),
            })),
        }) {
            let msg = format!(
                "Failed to send firewall config update for network {}, error: {err}",
                self.network,
            );
            error!(msg);
            return Err(Status::new(Code::Internal, msg));
        }
        debug!("Firewall config update sent for network {}", self.network);
        Ok(())
    }

    /// Send firewall disable command to gateway
    async fn send_firewall_disable(&self) -> Result<(), Status> {
        debug!(
            "Sending firewall disable command for network {}",
            self.network
        );
        if let Err(err) = self.tx.send(CoreResponse {
            id: 0,
            payload: Some(core_response::Payload::Update(Update {
                update_type: 2,
                update: Some(update::Update::DisableFirewall(())),
            })),
        }) {
            let msg = format!(
                "Failed to send firewall disable command for network {}, error: {err}",
                self.network,
            );
            error!(msg);
            return Err(Status::new(Code::Internal, msg));
        }
        debug!("Firewall disable command sent for network {}", self.network);
        Ok(())
    }
}
