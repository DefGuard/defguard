use std::{collections::HashMap, net::IpAddr, time::Duration};

use chrono::DateTime;
use defguard_common::{
    db::{
        ChangeNotification, Id, TriggerOperation,
        models::{
            WireguardNetwork,
            gateway::Gateway,
            wireguard::{DEFAULT_WIREGUARD_MTU, ServiceLocationMode},
        },
    },
    messages::peer_stats_update::PeerStatsUpdate,
};
use defguard_mail::Mail;
use defguard_proto::{
    enterprise::firewall::FirewallConfig,
    gateway::{Configuration, CoreResponse, Peer, PeerStats, Update, core_response, update},
};
use sqlx::{PgExecutor, PgPool, postgres::PgListener, query};
use thiserror::Error;
use tokio::{
    sync::{
        broadcast::{Receiver as BroadcastReceiver, Sender},
        mpsc::{UnboundedSender, error::SendError},
    },
    task::{AbortHandle, JoinSet},
};
use tonic::{Code, Status};

use crate::{
    enterprise::{firewall::FirewallError, is_enterprise_license_active},
    events::GrpcEvent,
    grpc::gateway::{events::GatewayEvent, handler::GatewayHandler},
};

pub mod events;
pub(crate) mod handler;
// #[cfg(test)]
// mod tests;

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

/// Sends multiple events to be handled by gateway gRPC server.
///
/// If you want to use it inside the API context, use [`crate::AppState::send_multiple_wireguard_events`] instead
pub fn send_multiple_wireguard_events(events: Vec<GatewayEvent>, wg_tx: &Sender<GatewayEvent>) {
    debug!("Sending {} WireGuard events", events.len());
    for event in events {
        send_wireguard_event(event, wg_tx);
    }
}

/// Helper used to convert peer stats coming from gRPC client
/// into an internal representation
fn try_protos_into_stats_message(
    proto_stats: PeerStats,
    location_id: Id,
    gateway_id: Id,
) -> Option<PeerStatsUpdate> {
    // try to parse endpoint
    let endpoint = proto_stats.endpoint.parse().ok()?;

    let latest_handshake = DateTime::from_timestamp(proto_stats.latest_handshake as i64, 0)
        .unwrap_or_default()
        .naive_utc();

    Some(PeerStatsUpdate::new(
        location_id,
        gateway_id,
        proto_stats.public_key,
        endpoint,
        proto_stats.upload,
        proto_stats.download,
        latest_handshake,
    ))
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Error)]
pub enum GatewayError {
    #[error("Failed to acquire lock on VPN client state map")]
    ClientStateMutexError,
    #[error("gRPC event channel error: {0}")]
    GrpcEventChannelError(#[from] SendError<GrpcEvent>),
    #[error("Endpoint error: {0}")]
    EndpointError(String),
    #[error("gRPC communication error: {0}")]
    GrpcCommunicationError(#[from] tonic::Status),
    #[error(transparent)]
    CertificateError(#[from] defguard_certs::CertificateError),
    #[error("Configuration error: {0}")]
    ConfigurationError(String),
    #[error("Conversion error: {0}")]
    ConversionError(String),
    #[error(transparent)]
    SqlxError(#[from] sqlx::Error),
    #[error("Not found: {0}")]
    NotFound(String),
    // mpsc channel send/receive error
    #[error("Message channel error: {0}")]
    MessageChannelError(String),
    #[error(transparent)]
    FirewallError(#[from] FirewallError),
}

impl From<GatewayError> for Status {
    fn from(value: GatewayError) -> Self {
        Self::new(Code::Internal, value.to_string())
    }
}

/// If this location is marked as a service location, checks if all requirements are met for it to
/// function:
/// - Enterprise is enabled
#[must_use]
pub fn should_prevent_service_location_usage(location: &WireguardNetwork<Id>) -> bool {
    location.service_location_mode != ServiceLocationMode::Disabled
        && !is_enterprise_license_active()
}

/// Get a list of all allowed peers
///
/// Each device is marked as allowed or not allowed in a given network,
/// which enables enforcing peer disconnect in MFA-protected networks.
///
/// If the location is a service location, only returns peers if enterprise features are enabled.
///
/// XXX: should be implemented in defguard_core::db::models::wireguard::WireguardNetwork.
pub async fn get_peers<'e, E>(
    location: &WireguardNetwork<Id>,
    executor: E,
) -> Result<Vec<Peer>, sqlx::Error>
where
    E: PgExecutor<'e>,
{
    debug!("Fetching all peers for network {}", location.id);

    if should_prevent_service_location_usage(location) {
        warn!(
            "Tried to use service location {} with disabled enterprise features. No clients \
            will be allowed to connect.",
            location.name
        );
        return Ok(Vec::new());
    }

    // TODO: possible to not use ARRAY-unnest here?
    let rows = query!(
        "SELECT d.wireguard_pubkey pubkey, preshared_key, \
            ARRAY(
                SELECT host(ip)
                FROM unnest(wnd.wireguard_ips) AS ip
            ) \"allowed_ips!: Vec<String>\" \
        FROM wireguard_network_device wnd \
        JOIN device d ON wnd.device_id = d.id \
        JOIN \"user\" u ON d.user_id = u.id \
        WHERE wireguard_network_id = $1 AND (is_authorized = true OR NOT $2) \
        AND d.configured = true \
        AND u.is_active = true \
        ORDER BY d.id ASC",
        location.id,
        location.mfa_enabled()
    )
    .fetch_all(executor)
    .await?;

    // keepalive has to be added manually because Postgres
    // doesn't support unsigned integers
    let result = rows
        .into_iter()
        .map(|row| Peer {
            pubkey: row.pubkey,
            allowed_ips: row.allowed_ips,
            // Don't send preshared key if MFA is not enabled, it can't be used and may
            // cause issues with clients connecting if they expect no preshared key
            // e.g. when you disable MFA on a location
            preshared_key: if location.mfa_enabled() {
                row.preshared_key
            } else {
                None
            },
            keepalive_interval: Some(location.keepalive_interval as u32),
        })
        .collect();

    Ok(result)
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
        mtu: network.mtu as u32,
        fwmark: network.fwmark as u32,
    }
}

const GATEWAY_TABLE_TRIGGER: &str = "gateway_change";
const GATEWAY_RECONNECT_DELAY: Duration = Duration::from_secs(5);

/// Bi-directional gRPC stream for communication with Defguard Gateway.
pub async fn run_grpc_gateway_stream(
    pool: PgPool,
    events_tx: Sender<GatewayEvent>,
    mail_tx: UnboundedSender<Mail>,
    peer_stats_tx: UnboundedSender<PeerStatsUpdate>,
) -> Result<(), anyhow::Error> {
    let mut abort_handles = HashMap::new();

    let mut tasks = JoinSet::new();
    // Helper closure to launch `GatewayHandler`.
    let mut launch_gateway_handler = |gateway: Gateway<Id>| -> Result<AbortHandle, anyhow::Error> {
        let mut gateway_handler = GatewayHandler::new(
            gateway,
            pool.clone(),
            events_tx.clone(),
            mail_tx.clone(),
            peer_stats_tx.clone(),
        )?;
        let abort_handle = tasks.spawn(async move {
            loop {
                if let Err(err) = gateway_handler.handle_connection().await {
                    error!("Gateway connection error: {err}, retrying in 5 seconds...");
                    tokio::time::sleep(GATEWAY_RECONNECT_DELAY).await;
                }
            }
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
                        let result =
                            self.send_network_update(&network, peers, maybe_firewall_config, 1);
                        // update stored network data
                        self.network = network;
                        result
                    } else {
                        Ok(())
                    }
                }
                GatewayEvent::NetworkDeleted(network_id, network_name) => {
                    if network_id == self.network_id {
                        self.send_network_delete(&network_name)
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
                            // FIXME: this shouldn't happen, since when the device is created
                            // it's impossible for MFA authorization to already be completed
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
                        Some(_) => self.send_peer_delete(&device.device.wireguard_pubkey),
                        None => Ok(()),
                    }
                }
                GatewayEvent::FirewallConfigChanged(location_id, firewall_config) => {
                    if location_id == self.network_id {
                        self.send_firewall_update(firewall_config)
                    } else {
                        Ok(())
                    }
                }
                GatewayEvent::FirewallDisabled(location_id) => {
                    if location_id == self.network_id {
                        self.send_firewall_disable()
                    } else {
                        Ok(())
                    }
                }
                GatewayEvent::MfaSessionDisconnected(location_id, device) => {
                    if location_id == self.network_id {
                        self.send_peer_delete(&device.wireguard_pubkey)
                    } else {
                        Ok(())
                    }
                }
                GatewayEvent::MfaSessionAuthorized(location_id, device, network_device) => {
                    if location_id == self.network_id {
                        // validate that network info is for the correct location
                        if network_device.wireguard_network_id != location_id {
                            error!(
                                "Received MFA authorization success event for location {location_id} with invalid device config: {network_device:?}"
                            );
                            continue;
                        }

                        // FIXME: at this point the device authorization should already have been verified
                        if self.network.mfa_enabled() && !network_device.is_authorized {
                            debug!(
                                "Created WireGuard device {} is not authorized to connect to \
                                    MFA enabled location {}",
                                device.name, self.network.name
                            );
                            continue;
                        }

                        self.send_peer_update(
                            Peer {
                                pubkey: device.wireguard_pubkey,
                                allowed_ips: network_device
                                    .wireguard_ips
                                    .iter()
                                    .map(IpAddr::to_string)
                                    .collect(),
                                preshared_key: network_device.preshared_key.clone(),
                                keepalive_interval: Some(self.network.keepalive_interval as u32),
                            },
                            0,
                        )
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
    fn send_network_update(
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
                    mtu: network.mtu as u32,
                    fwmark: network.fwmark as u32,
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
    fn send_network_delete(&self, network_name: &str) -> Result<(), Status> {
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
                    mtu: DEFAULT_WIREGUARD_MTU as u32,
                    fwmark: 0,
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
    fn send_peer_update(&self, peer: Peer, update_type: i32) -> Result<(), Status> {
        debug!("Sending peer update for network {}", self.network);
        if let Err(err) = self.tx.send(CoreResponse {
            id: 0,
            payload: Some(core_response::Payload::Update(Update {
                update_type,
                update: Some(update::Update::Peer(peer)),
            })),
        }) {
            let msg = format!(
                "Failed to send peer update for network {}, update type: {update_type} ({}), \
                error: {err}",
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
    fn send_peer_delete(&self, peer_pubkey: &str) -> Result<(), Status> {
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
                "Failed to send peer update for network {}, peer {peer_pubkey}, update type: 2 \
                (DELETE), error: {err}",
                self.network,
            );
            error!(msg);
            return Err(Status::new(Code::Internal, msg));
        }
        debug!("Peer delete command sent for network {}", self.network);
        Ok(())
    }

    /// Send firewall config update command to gateway
    fn send_firewall_update(&self, firewall_config: FirewallConfig) -> Result<(), Status> {
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
    fn send_firewall_disable(&self) -> Result<(), Status> {
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

// #[tonic::async_trait]
// impl gateway_service_server::GatewayService for GatewayServer {
//     type UpdatesStream = GatewayUpdatesStream;

//     /// Retrieve stats from gateway and save it to database
//     async fn stats(
//         &self,
//         request: Request<tonic::Streaming<StatsUpdate>>,
//     ) -> Result<Response<()>, Status> {
//         let GatewayMetadata {
//             network_id,
//             hostname,
//             ..
//         } = Self::extract_metadata(request.metadata())?;
//         let mut stream = request.into_inner();
//         let mut disconnect_timer = interval(Duration::from_secs(PEER_DISCONNECT_INTERVAL));
//         // FIXME: tracing causes looping messages, like `INFO gateway_config:gateway_stats:...`.
//         // let span = tracing::info_span!("gateway_stats", component = %DefguardComponent::Gateway,
//         //     version = version.to_string(), info);
//         // let _guard = span.enter();
//         loop {
//             // Wait for a message or update client map at least once a mninute, if no messages are
//             // received.
//             let stats_update = tokio::select! {
//                 message = stream.message() => {
//                     match message? {
//                         Some(update) => update,
//                         None => break, // Stream ended
//                     }
//                 }
//                 _ = disconnect_timer.tick() => {
//                     debug!("No stats updates received in last {PEER_DISCONNECT_INTERVAL} seconds. \
//                         Updating disconnected VPN clients");
//                     // fetch location to get current peer disconnect threshold
//                     let location = self.fetch_location_from_db(network_id).await?;

//                     // perform client state operations in a dedicated block to drop mutex guard
//                     let disconnected_clients = {
//                         // acquire lock on client state map
//                         let mut client_map = self.get_client_state_guard()?;

//                         // disconnect inactive clients
//                         client_map.disconnect_inactive_vpn_clients_for_location(&location
//                         )?
//                     };

//                     // emit client disconnect events
//                     for (device, context) in disconnected_clients {
//                         self.emit_event(GrpcEvent::ClientDisconnected {
//                             context,
//                             location: location.clone(),
//                             device,
//                         })?;
//                     };
//                     continue;
//                 }
//             };

//             debug!("Received stats message: {stats_update:?}");
//             let Some(stats_update::Payload::PeerStats(peer_stats)) = stats_update.payload else {
//                 debug!("Received stats message is empty, skipping.");
//                 continue;
//             };
//             let public_key = peer_stats.public_key.clone();

//             // fetch device from DB
//             // TODO: fetch only when device has changed and use client state otherwise
//             let device = match self.fetch_device_from_db(&public_key).await? {
//                 Some(device) => device,
//                 None => {
//                     warn!(
//                         "Received stats update for a device which does not exist: {public_key}, skipping."
//                     );
//                     continue;
//                 }
//             };

//             // copy device ID for easier reference later
//             let device_id = device.id;

//             // fetch user and location from DB for activity log
//             // TODO: cache usernames since they don't change
//             let user = self.fetch_user_from_db(device.user_id, &public_key).await?;
//             let location = self.fetch_location_from_db(network_id).await?;

//             // convert stats to DB storage format
//             let stats = protos_into_internal_stats(peer_stats, network_id, device_id);

//             // only perform client state update if stats include an endpoint IP
//             // otherwise a peer was added to the gateway interface
//             // but has not connected yet
//             if let Some(endpoint) = &stats.endpoint {
//                 // parse client endpoint IP
//                 let socket_addr: SocketAddr = endpoint.clone().parse().map_err(|err| {
//                     error!("Failed to parse VPN client endpoint: {err}");
//                     Status::new(
//                         Code::Internal,
//                         format!("Failed to parse VPN client endpoint: {err}"),
//                     )
//                 })?;

//                 // perform client state operations in a dedicated block to drop mutex guard
//                 let disconnected_clients = {
//                     // acquire lock on client state map
//                     let mut client_map = self.get_client_state_guard()?;

//                     // update connected clients map
//                     match client_map.get_vpn_client(network_id, &public_key) {
//                         Some(client_state) => {
//                             // update connected client state
//                             client_state.update_client_state(
//                                 device,
//                                 socket_addr,
//                                 stats.latest_handshake,
//                                 stats.upload,
//                                 stats.download,
//                             );
//                         }
//                         None => {
//                             // don't mark inactive peers as connected
//                             if (Utc::now().naive_utc() - stats.latest_handshake)
//                                 < TimeDelta::seconds(location.peer_disconnect_threshold.into())
//                             {
//                                 // mark new VPN client as connected
//                                 client_map.connect_vpn_client(
//                                     network_id,
//                                     &hostname,
//                                     &public_key,
//                                     &device,
//                                     &user,
//                                     socket_addr,
//                                     &stats,
//                                 )?;

//                                 // emit connection event
//                                 let context = GrpcRequestContext::new(
//                                     user.id,
//                                     user.username.clone(),
//                                     socket_addr.ip(),
//                                     device.id,
//                                     device.name.clone(),
//                                     location.clone(),
//                                 );
//                                 self.emit_event(GrpcEvent::ClientConnected {
//                                     context,
//                                     location: location.clone(),
//                                     device: device.clone(),
//                                 })?;
//                             }
//                         }
//                     }

// convert stats to DB storage format
// match try_protos_into_stats_message(peer_stats.clone(), network_id, device_id) {
//     None => {
//         warn!(
//             "Failed to parse peer stats update. Skipping sending message to session manager."
//         )
//     }
//     Some(message) => {
//         self.peer_stats_tx.send(message).map_err(|err| {
//             error!("Failed to send peers stats update to session manager: {err}");
//             Status::new(
//                 Code::Internal,
//                 format!("Failed to send peers stats update to session manager: {err}"),
//             )
//         })?;
//     }
// };

// convert stats to DB storage format
// let stats = protos_into_internal_stats(peer_stats, network_id, device_id);

//                 // emit client disconnect events
//                 for (device, context) in disconnected_clients {
//                     self.emit_event(GrpcEvent::ClientDisconnected {
//                         context,
//                         location: location.clone(),
//                         device,
//                     })?;
//                 }
//             }

//             // Save stats to db
//             let stats = match stats.save(&self.pool).await {
//                 Ok(stats) => stats,
//                 Err(err) => {
//                     error!("Saving WireGuard peer stats to db failed: {err}");
//                     return Err(Status::new(
//                         Code::Internal,
//                         format!("Saving WireGuard peer stats to db failed: {err}"),
//                     ));
//                 }
//             };
//             info!("Saved WireGuard peer stats to db.");
//             debug!("WireGuard peer stats: {stats:?}");
//         }

//         Ok(Response::new(()))
//     }

//     async fn config(
//         &self,
//         request: Request<ConfigurationRequest>,
//     ) -> Result<Response<Configuration>, Status> {
//         debug!("Sending configuration to gateway client.");
//         let GatewayMetadata {
//             network_id,
//             hostname,
//             version,
//             ..
//             // info,
//         } = Self::extract_metadata(request.metadata())?;
//         // FIXME: tracing causes looping messages, like `INFO gateway_config:gateway_stats:...`.
//         // let span = tracing::info_span!("gateway_config", component = %DefguardComponent::Gateway,
//         //     version = version.to_string(), info);
//         // let _guard = span.enter();

//         let mut conn = self.pool.acquire().await.map_err(|e| {
//             error!("Failed to acquire DB connection: {e}");
//             Status::new(
//                 Code::Internal,
//                 "Failed to acquire DB connection".to_string(),
//             )
//         })?;

//         let mut network = WireguardNetwork::find_by_id(&mut *conn, network_id)
//             .await
//             .map_err(|e| {
//                 error!("Network {network_id} not found");
//                 Status::new(Code::Internal, format!("Failed to retrieve network: {e}"))
//             })?
//             .ok_or_else(|| {
//                 Status::new(
//                     Code::Internal,
//                     format!("Network with id {network_id} not found"),
//                 )
//             })?;

//         debug!("Sending configuration to gateway client, network {network}.");

//         // store connected gateway in memory
//         {
//             let mut state = self.gateway_state.lock().unwrap();
//             state.add_gateway(
//                 network_id,
//                 &network.name,
//                 hostname,
//                 request.into_inner().name,
//                 self.mail_tx.clone(),
//                 version,
//             );
//         }

//         network.connected_at = Some(Utc::now().naive_utc());
//         if let Err(err) = network.save(&mut *conn).await {
//             error!("Failed to save updated network {network_id} in the database, status: {err}");
//         }

//         let peers =
//             get_location_allowed_peers(&network, &mut *conn)
//                 .await
//                 .map_err(|error| {
//                     error!(
//                         "Failed to fetch peers from the database for network {network_id}: {error}",
//                     );
//                     Status::new(
//                         Code::Internal,
//                         format!(
//                             "Failed to retrieve peers from the database for network: {network_id}"
//                         ),
//                     )
//                 })?;
//         let maybe_firewall_config = try_get_location_firewall_config(&network, &mut conn)
//             .await
//             .map_err(|err| {
//                 error!("Failed to generate firewall config for network {network_id}: {err}");
//                 Status::new(
//                     Code::Internal,
//                     format!("Failed to generate firewall config for network: {network_id}"),
//                 )
//             })?;

//         info!("Configuration sent to gateway client, network {network}.");

//         Ok(Response::new(gen_config(
//             &network,
//             peers,
//             maybe_firewall_config,
//         )))
//     }

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
