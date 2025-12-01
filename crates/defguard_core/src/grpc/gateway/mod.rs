use std::{
    net::IpAddr,
    sync::{Arc, Mutex},
};

use chrono::{DateTime, Utc};
use client_state::ClientMap;
use defguard_common::db::{Id, NoId};
use defguard_mail::Mail;
use defguard_proto::{
    enterprise::firewall::FirewallConfig,
    gateway::{Configuration, CoreResponse, Peer, PeerStats, Update, core_response, update},
};
use defguard_version::version_info_from_metadata;
use semver::Version;
use sqlx::PgPool;
use thiserror::Error;
use tokio::sync::{
    broadcast::{Receiver as BroadcastReceiver, Sender},
    mpsc::{self, UnboundedSender, error::SendError},
};
use tonic::{Code, Status, metadata::MetadataMap};

use self::map::GatewayMap;
use crate::{
    db::{
        GatewayEvent,
        models::{wireguard::WireguardNetwork, wireguard_peer_stats::WireguardPeerStats},
    },
    events::{GrpcEvent, GrpcRequestContext},
};

pub mod client_state;
pub(crate) mod handler;
pub mod map;
pub(crate) mod state;
// #[cfg(test)]
// mod tests;

const PEER_DISCONNECT_INTERVAL: u64 = 60;
pub(super) static TONIC_SOCKET: &str = "tonic.sock";

/// Sends given `GatewayEvent` to be handled by gateway GRPC server
///
/// If you want to use it inside the API context, use [`crate::AppState::send_wireguard_event`] instead
pub fn send_wireguard_event(event: GatewayEvent, wg_tx: &Sender<GatewayEvent>) {
    debug!("Sending the following WireGuard event to the gateway: {event:?}");
    if let Err(err) = wg_tx.send(event) {
        error!("Error sending WireGuard event {err}");
    }
}

/// Sends multiple events to be handled by gateway GRPC server
///
/// If you want to use it inside the API context, use [`crate::AppState::send_multiple_wireguard_events`] instead
pub fn send_multiple_wireguard_events(events: Vec<GatewayEvent>, wg_tx: &Sender<GatewayEvent>) {
    debug!("Sending {} wireguard events", events.len());
    for event in events {
        send_wireguard_event(event, wg_tx);
    }
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Error)]
pub enum GatewayServerError {
    #[error("Failed to acquire lock on VPN client state map")]
    ClientStateMutexError,
    #[error("gRPC event channel error: {0}")]
    GrpcEventChannelError(#[from] SendError<GrpcEvent>),
}

impl From<GatewayServerError> for Status {
    fn from(value: GatewayServerError) -> Self {
        Self::new(Code::Internal, value.to_string())
    }
}

pub struct GatewayServer {
    pool: PgPool,
    gateway_state: Arc<Mutex<GatewayMap>>,
    client_state: Arc<Mutex<ClientMap>>,
    wireguard_tx: Sender<GatewayEvent>,
    mail_tx: UnboundedSender<Mail>,
    grpc_event_tx: UnboundedSender<GrpcEvent>,
}

/// Utility struct encapsulating commonly extracted metadata fields during gRPC communication.
struct GatewayMetadata {
    network_id: Id,
    hostname: String,
    version: Version,
    // info: String,
}

impl GatewayServer {
    /// Create new gateway server instance
    #[must_use]
    pub fn new(
        pool: PgPool,
        gateway_state: Arc<Mutex<GatewayMap>>,
        client_state: Arc<Mutex<ClientMap>>,
        wireguard_tx: Sender<GatewayEvent>,
        mail_tx: UnboundedSender<Mail>,
        grpc_event_tx: UnboundedSender<GrpcEvent>,
    ) -> Self {
        Self {
            pool,
            gateway_state,
            client_state,
            wireguard_tx,
            mail_tx,
            grpc_event_tx,
        }
    }

    fn get_network_id(metadata: &MetadataMap) -> Result<i64, Status> {
        match Self::get_network_id_from_metadata(metadata) {
            Some(m) => Ok(m),
            None => Err(Status::new(
                Code::Internal,
                "Network ID was not found in metadata",
            )),
        }
    }

    // parse network id from gateway request metadata from intercepted information from JWT token
    fn get_network_id_from_metadata(metadata: &MetadataMap) -> Option<Id> {
        if let Some(ascii_value) = metadata.get("gateway_network_id") {
            if let Ok(slice) = ascii_value.clone().to_str() {
                if let Ok(id) = slice.parse::<Id>() {
                    return Some(id);
                }
            }
        }
        None
    }

    // extract gateway hostname from request headers
    fn get_gateway_hostname(metadata: &MetadataMap) -> Result<String, Status> {
        match metadata.get("hostname") {
            Some(ascii_value) => {
                let hostname = ascii_value.to_str().map_err(|_| {
                    Status::new(
                        Code::Internal,
                        "Failed to parse gateway hostname from request metadata",
                    )
                })?;
                Ok(hostname.into())
            }
            None => Err(Status::new(
                Code::Internal,
                "Gateway hostname not found in request metadata",
            )),
        }
    }

    /// Utility function extracting metadata fields during gRPC communication.
    fn extract_metadata(metadata: &MetadataMap) -> Result<GatewayMetadata, Status> {
        let (version, _info) = version_info_from_metadata(metadata);
        Ok(GatewayMetadata {
            network_id: Self::get_network_id(metadata)?,
            hostname: Self::get_gateway_hostname(metadata)?,
            version,
        })
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

/// Process received Gateway events
///
/// Main gRPC server uses a shared channel for broadcasting all Gateway events,
/// so the handler must determine if an event is relevant for the network being serviced.
async fn handle_events(
    mut current_network: WireguardNetwork<Id>,
    // gateway_hostname: String,
    tx: UnboundedSender<CoreResponse>,
    mut events_rx: BroadcastReceiver<GatewayEvent>,
) {
    info!("Starting update stream network {current_network}");
    //     while let Some(event) = events_rx.recv().await {
    //         debug!("Received networking state update event: {event:?}");
    //         let (update_type, update) = match event {
    //             GatewayEvent::NetworkCreated(network, _fixme) => {
    //                 if network.id != current_network.id {
    //                     continue;
    //                 }
    //                 (
    //                     UpdateType::Create,
    //                     update::Update::Network(Configuration {
    //                         name: network.name.clone(),
    //                         prvkey: network.prvkey.clone(),
    //                         addresses: network.address.to_string(),
    //                         port: network.port as u32,
    //                         peers: Vec::new(),
    //                     }),
    //                 )
    //             }
    //             GatewayEvent::NetworkModified(network, peers, _fixme) => {
    //                 if network.id != current_network.id {
    //                     continue;
    //                 }
    //                 // update stored network data
    //                 current_network = network.clone();
    //                 (
    //                     UpdateType::Modify,
    //                     update::Update::Network(Configuration {
    //                         name: network.name,
    //                         prvkey: network.prvkey,
    //                         addresses: network.address.to_string(),
    //                         port: network.port as u32,
    //                         peers,
    //                     }),
    //                 )
    //             }
    //             GatewayEvent::NetworkDeleted(network_id, network_name) => {
    //                 if network_id != current_network.id {
    //                     continue;
    //                 }
    //                 (
    //                     UpdateType::Delete,
    //                     update::Update::Network(Configuration {
    //                         name: network_name.to_string(),
    //                         prvkey: String::new(),
    //                         addresses: Vec::new(),
    //                         port: 0,
    //                         peers: Vec::new(),
    //                         firewall_config: None,
    //                     }),
    //                 )
    //             }
    //             GatewayEvent::DeviceCreated(device) => {
    //                 // check if a peer has to be added in the current network
    //                 match device
    //                     .network_info
    //                     .iter()
    //                     .find(|info| info.network_id == current_network.id)
    //                 {
    //                     Some(network_info) => {
    //                         if current_network.mfa_enabled && !network_info.is_authorized {
    //                             debug!(
    //                                 "Created WireGuard device {} is not authorized to connect to MFA enabled location {}",
    //                                 device.device.name, current_network.name
    //                             );
    //                             continue;
    //                         };
    //                         let peer = Peer {
    //                             pubkey: device.device.wireguard_pubkey,
    //                             allowed_ips: vec![network_info.device_wireguard_ip.to_string()],
    //                             preshared_key: network_info.preshared_key.clone(),
    //                             keepalive_interval: Some(current_network.keepalive_interval as u32),
    //                         };
    //                         (UpdateType::Create, update::Update::Peer(peer))
    //                     }
    //                     None => continue,
    //                 }
    //             }
    //             GatewayEvent::DeviceModified(device) => {
    //                 // check if a peer has to be updated in the current network
    //                 match device
    //                     .network_info
    //                     .iter()
    //                     .find(|info| info.network_id == current_network.id)
    //                 {
    //                     Some(network_info) => {
    //                         if current_network.mfa_enabled && !network_info.is_authorized {
    //                             debug!(
    //                                 "Modified WireGuard device {} is not authorized to connect to MFA enabled location {}",
    //                                 device.device.name, current_network.name
    //                             );
    //                             continue;
    //                         };
    //                         let peer = Peer {
    //                             pubkey: device.device.wireguard_pubkey,
    //                             allowed_ips: vec![network_info.device_wireguard_ip.to_string()],
    //                             preshared_key: network_info.preshared_key.clone(),
    //                             keepalive_interval: Some(current_network.keepalive_interval as u32),
    //                         };
    //                         (UpdateType::Modify, update::Update::Peer(peer))
    //                     }
    //                     None => continue,
    //                 }
    //             }
    //             GatewayEvent::DeviceDeleted(device) => {
    //                 // check if a peer has to be updated in the current network
    //                 match device
    //                     .network_info
    //                     .iter()
    //                     .find(|info| info.network_id == current_network.id)
    //                 {
    //                     Some(_) => (
    //                         UpdateType::Delete,
    //                         update::Update::Peer(Peer {
    //                             pubkey: device.device.wireguard_pubkey,
    //                             allowed_ips: Vec::new(),
    //                             preshared_key: None,
    //                             keepalive_interval: None,
    //                         }),
    //                     ),
    //                     None => continue,
    //                 }
    //             }
    //             GatewayEvent::FirewallConfigChanged(_fixme, _) => (),
    //             GatewayEvent::FirewallDisabled(_id) => (),
    //         };

    //         let req = CoreResponse {
    //             id: 0,
    //             payload: Some(core_response::Payload::Update(Update {
    //                 update_type: update_type as i32,
    //                 update: Some(update),
    //             })),
    //         };
    //         if let Err(err) = tx.send(req) {
    //             error!(
    //                 "Failed to send network update, network {current_network}, update type: {}, error: \
    //                 {err}",
    //                 update_type.as_str_name()
    //             );
    //             break;
    //         }
    //         debug!(
    //             "Network update sent for network {current_network}, update type: {}",
    //             update_type.as_str_name()
    //         );
    //     }
}

/// Helper struct for handling gateway events
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
                                    "Created WireGuard device {} is not authorized to connect to MFA enabled location {}",
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
                                    "Modified WireGuard device {} is not authorized to connect to MFA enabled location {}",
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
                "Failed to send network update, network {network}, update type: {update_type} ({}), error: {err}",
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

// pub struct GatewayUpdatesStream {
//     task_handle: JoinHandle<()>,
//     rx: Receiver<Result<Update, Status>>,
//     network_id: Id,
//     gateway_hostname: String,
//     gateway_state: Arc<Mutex<GatewayMap>>,
//     pool: PgPool,
// }

// #[tonic::async_trait]
// impl gateway_service_server::GatewayService for GatewayServer {
//     type UpdatesStream = GatewayUpdatesStream;
//
//     async fn updates(&self, request: Request<()>) -> Result<Response<Self::UpdatesStream>, Status> {
//         let GatewayMetadata {
//             network_id,
//             hostname,
//             ..
//             // info,
//         } = Self::extract_metadata(request.metadata())?;
//         // FIXME: tracing causes looping messages, like `INFO gateway_config:gateway_stats:...`.
//         // let span = tracing::info_span!("gateway_updates", component = %DefguardComponent::Gateway,
//         //     version = version.to_string(), info);
//         // let _guard = span.enter();

//         let Some(network) = WireguardNetwork::find_by_id(&self.pool, network_id)
//             .await
//             .map_err(|_| {
//                 error!("Failed to fetch network {network_id} from the database");
//                 Status::new(
//                     Code::Internal,
//                     format!("Failed to retrieve network {network_id} from the database"),
//                 )
//             })?
//         else {
//             return Err(Status::new(
//                 Code::Internal,
//                 format!("Network with id {network_id} not found"),
//             ));
//         };

//         info!("New client connected to updates stream: {hostname}, network {network}",);

//         let (tx, rx) = mpsc::channel(4);
//         let events_rx = self.wireguard_tx.subscribe();
//         let mut state = self.gateway_state.lock().unwrap();
//         state
//             .connect_gateway(network_id, &hostname, &self.pool)
//             .map_err(|err| {
//                 error!("Failed to connect gateway on network {network_id}: {err}");
//                 Status::new(
//                     Code::Internal,
//                     format!("Failed to connect gateway on network {network_id}"),
//                 )
//             })?;

//         // clone here before moving into a closure
//         let gateway_hostname = hostname.clone();
//         let handle = tokio::spawn(async move {
//             let mut update_handler =
//                 GatewayUpdatesHandler::new(network_id, network, gateway_hostname, events_rx, tx);
//             update_handler.run().await;
//         });

//         Ok(Response::new(GatewayUpdatesStream::new(
//             handle,
//             rx,
//             network_id,
//             hostname,
//             Arc::clone(&self.gateway_state),
//             self.pool.clone(),
//         )))
//     }
// }
