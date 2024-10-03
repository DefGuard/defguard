use std::{
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll},
};

use chrono::{DateTime, Utc};
use sqlx::{query, Error as SqlxError, PgExecutor, PgPool};
use tokio::{
    sync::{
        broadcast::{Receiver as BroadcastReceiver, Sender},
        mpsc::{self, Receiver, UnboundedSender},
    },
    task::JoinHandle,
};
use tokio_stream::Stream;
use tonic::{metadata::MetadataMap, Code, Request, Response, Status};

use super::GatewayMap;
use crate::{
    db::{
        models::wireguard::{WireguardNetwork, WireguardPeerStats},
        Device, GatewayEvent, Id, NoId,
    },
    mail::Mail,
};

tonic::include_proto!("gateway");

pub struct GatewayServer {
    pool: PgPool,
    state: Arc<Mutex<GatewayMap>>,
    wireguard_tx: Sender<GatewayEvent>,
    mail_tx: UnboundedSender<Mail>,
}

impl WireguardNetwork<Id> {
    /// Get a list of all allowed peers
    ///
    /// Each device is marked as allowed or not allowed in a given network,
    /// which enables enforcing peer disconnect in MFA-protected networks.
    pub async fn get_peers<'e, E>(&self, executor: E) -> Result<Vec<Peer>, SqlxError>
    where
        E: PgExecutor<'e>,
    {
        debug!("Fetching all peers for network {}", self.id);
        let rows = query!(
            "SELECT d.wireguard_pubkey pubkey, preshared_key, \
                array[host(wnd.wireguard_ip)] \"allowed_ips!: Vec<String>\" \
            FROM wireguard_network_device wnd \
            JOIN device d ON wnd.device_id = d.id \
            JOIN \"user\" u ON d.user_id = u.id \
            WHERE wireguard_network_id = $1 AND (is_authorized = true OR NOT $2) \
            AND u.is_active = true \
            ORDER BY d.id ASC",
            self.id,
            self.mfa_enabled
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
                preshared_key: row.preshared_key,
                keepalive_interval: Some(self.keepalive_interval as u32),
            })
            .collect();

        Ok(result)
    }
}

impl GatewayServer {
    /// Create new gateway server instance
    #[must_use]
    pub fn new(
        pool: PgPool,
        state: Arc<Mutex<GatewayMap>>,
        wireguard_tx: Sender<GatewayEvent>,
        mail_tx: UnboundedSender<Mail>,
    ) -> Self {
        Self {
            pool,
            state,
            wireguard_tx,
            mail_tx,
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
    fn get_network_id_from_metadata(metadata: &MetadataMap) -> Option<i64> {
        if let Some(ascii_value) = metadata.get("gateway_network_id") {
            if let Ok(slice) = ascii_value.clone().to_str() {
                if let Ok(id) = slice.parse::<i64>() {
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
}

fn gen_config(network: &WireguardNetwork<Id>, peers: Vec<Peer>) -> Configuration {
    Configuration {
        name: network.name.clone(),
        port: network.port as u32,
        prvkey: network.prvkey.clone(),
        address: network.address.to_string(),
        peers,
    }
}

impl WireguardPeerStats {
    fn from_peer_stats(stats: PeerStats, network_id: Id) -> Self {
        let endpoint = match stats.endpoint {
            endpoint if endpoint.is_empty() => None,
            _ => Some(stats.endpoint),
        };
        Self {
            id: NoId,
            network: network_id,
            endpoint,
            device_id: -1,
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

/// Helper struct for handling gateway events
struct GatewayUpdatesHandler {
    network_id: Id,
    network: WireguardNetwork<Id>,
    gateway_hostname: String,
    events_rx: BroadcastReceiver<GatewayEvent>,
    tx: mpsc::Sender<Result<Update, Status>>,
}

impl GatewayUpdatesHandler {
    pub fn new(
        network_id: Id,
        network: WireguardNetwork<Id>,
        gateway_hostname: String,
        events_rx: BroadcastReceiver<GatewayEvent>,
        tx: mpsc::Sender<Result<Update, Status>>,
    ) -> Self {
        Self {
            network_id,
            network,
            gateway_hostname,
            events_rx,
            tx,
        }
    }

    /// Process incoming gateway events
    ///
    /// Main gRPC server uses a shared channel for broadcasting all gateway events
    /// so the handler must determine if an event is relevant for the network being services
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
                        self.send_network_update(&network, Vec::new(), 0).await
                    } else {
                        Ok(())
                    }
                }
                GatewayEvent::NetworkModified(network_id, network, peers) => {
                    if network_id == self.network_id {
                        let result = self.send_network_update(&network, peers, 1).await;
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
                            if self.network.mfa_enabled && !network_info.is_authorized {
                                debug!("Created WireGuard device {} is not authorized to connect to MFA enabled location {}",
                                device.device.name, self.network.name
                            );
                                continue;
                            };
                            self.send_peer_update(
                                Peer {
                                    pubkey: device.device.wireguard_pubkey,
                                    allowed_ips: vec![network_info.device_wireguard_ip.to_string()],
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
                            if self.network.mfa_enabled && !network_info.is_authorized {
                                debug!("Modified WireGuard device {} is not authorized to connect to MFA enabled location {}",
                                device.device.name, self.network.name
                            );
                                continue;
                            };
                            self.send_peer_update(
                                Peer {
                                    pubkey: device.device.wireguard_pubkey,
                                    allowed_ips: vec![network_info.device_wireguard_ip.to_string()],
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
        update_type: i32,
    ) -> Result<(), Status> {
        debug!("Sending network update for network {network}");
        if let Err(err) = self
            .tx
            .send(Ok(Update {
                update_type,
                update: Some(update::Update::Network(Configuration {
                    name: network.name.clone(),
                    prvkey: network.prvkey.clone(),
                    address: network.address.to_string(),
                    port: network.port as u32,
                    peers,
                })),
            }))
            .await
        {
            let msg = format!(
                "Failed to send network update, network {network}, update type: {update_type} ({}), error: {err}",
                if update_type == 0 {
                    "CREATE"
                } else {
                    "MODIFY"
                },
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
        if let Err(err) = self
            .tx
            .send(Ok(Update {
                update_type: 2,
                update: Some(update::Update::Network(Configuration {
                    name: network_name.to_string(),
                    prvkey: String::new(),
                    address: String::new(),
                    port: 0,
                    peers: Vec::new(),
                })),
            }))
            .await
        {
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
        if let Err(err) = self
            .tx
            .send(Ok(Update {
                update_type,
                update: Some(update::Update::Peer(peer)),
            }))
            .await
        {
            let msg = format!(
                "Failed to send peer update for network {}, update type: {update_type} ({}), error: {err}",
                self.network,
                if update_type == 0 {
                    "CREATE"
                } else {
                    "MODIFY"
                },
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
        if let Err(err) = self
            .tx
            .send(Ok(Update {
                update_type: 2,
                update: Some(update::Update::Peer(Peer {
                    pubkey: peer_pubkey.into(),
                    allowed_ips: Vec::new(),
                    preshared_key: None,
                    keepalive_interval: None,
                })),
            }))
            .await
        {
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
}

pub struct GatewayUpdatesStream {
    task_handle: JoinHandle<()>,
    rx: Receiver<Result<Update, Status>>,
    network_id: Id,
    gateway_hostname: String,
    gateway_state: Arc<Mutex<GatewayMap>>,
    pool: PgPool,
}

impl GatewayUpdatesStream {
    #[must_use]
    pub fn new(
        task_handle: JoinHandle<()>,
        rx: Receiver<Result<Update, Status>>,
        network_id: Id,
        gateway_hostname: String,
        gateway_state: Arc<Mutex<GatewayMap>>,
        pool: PgPool,
    ) -> Self {
        Self {
            task_handle,
            rx,
            network_id,
            gateway_hostname,
            gateway_state,
            pool,
        }
    }
}

impl Stream for GatewayUpdatesStream {
    type Item = Result<Update, Status>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.rx).poll_recv(cx)
    }
}

impl Drop for GatewayUpdatesStream {
    fn drop(&mut self) {
        info!("Client disconnected");
        // terminate update task
        self.task_handle.abort();
        // update gateway state
        self.gateway_state
            .lock()
            .unwrap()
            .disconnect_gateway(self.network_id, self.gateway_hostname.clone(), &self.pool)
            .expect("Unable to disconnect gateway.");
    }
}

#[tonic::async_trait]
impl gateway_service_server::GatewayService for GatewayServer {
    type UpdatesStream = GatewayUpdatesStream;

    /// Retrieve stats from gateway and save it to database
    async fn stats(
        &self,
        request: Request<tonic::Streaming<StatsUpdate>>,
    ) -> Result<Response<()>, Status> {
        let network_id = Self::get_network_id(request.metadata())?;
        let mut stream = request.into_inner();
        while let Some(stats_update) = stream.message().await? {
            debug!("Received stats message: {stats_update:?}");
            let Some(stats_update::Payload::PeerStats(peer_stats)) = stats_update.payload else {
                debug!("Received stats message is empty, skipping.");
                continue;
            };
            let public_key = peer_stats.public_key.clone();
            let mut stats = WireguardPeerStats::from_peer_stats(peer_stats, network_id);
            // Get device by public key and fill in stats.device_id
            // FIXME: keep an in-memory device map to avoid repeated DB requests
            stats.device_id = match Device::find_by_pubkey(&self.pool, &public_key).await {
                Ok(Some(device)) => device.id,
                Ok(None) => {
                    error!("Device with public key {public_key} not found");
                    return Err(Status::new(
                        Code::Internal,
                        format!("Device with public key {public_key} not found"),
                    ));
                }
                Err(err) => {
                    error!("Failed to retrieve device with public key {public_key}: {err}",);
                    return Err(Status::new(
                        Code::Internal,
                        format!("Failed to retrieve device with public key {public_key}: {err}",),
                    ));
                }
            };
            // Save stats to db
            let stats = match stats.save(&self.pool).await {
                Ok(stats) => stats,
                Err(err) => {
                    error!("Saving WireGuard peer stats to db failed: {err}");
                    return Err(Status::new(
                        Code::Internal,
                        format!("Saving WireGuard peer stats to db failed: {err}"),
                    ));
                }
            };
            info!("Saved WireGuard peer stats to db.");
            debug!("WireGuard peer stats: {stats:?}");
        }

        Ok(Response::new(()))
    }

    async fn config(
        &self,
        request: Request<ConfigurationRequest>,
    ) -> Result<Response<Configuration>, Status> {
        debug!("Sending configuration to gateway client.");
        let network_id = Self::get_network_id(request.metadata())?;
        let hostname = Self::get_gateway_hostname(request.metadata())?;

        let mut network = WireguardNetwork::find_by_id(&self.pool, network_id)
            .await
            .map_err(|e| {
                error!("Network {network_id} not found");
                Status::new(Code::Internal, format!("Failed to retrieve network: {e}"))
            })?
            .ok_or_else(|| {
                Status::new(
                    Code::Internal,
                    format!("Network with id {network_id} not found"),
                )
            })?;

        debug!("Sending configuration to gateway client, network {network}.");

        // store connected gateway in memory
        {
            let mut state = self.state.lock().unwrap();
            state.add_gateway(
                network_id,
                &network.name,
                hostname,
                request.into_inner().name,
                self.mail_tx.clone(),
            );
        }

        network.connected_at = Some(Utc::now().naive_utc());
        if let Err(err) = network.save(&self.pool).await {
            error!("Failed to save updated network {network_id} in the database, status: {err}");
        }

        let peers = network.get_peers(&self.pool).await.map_err(|error| {
            error!("Failed to fetch peers from the database for network {network_id}: {error}",);
            Status::new(
                Code::Internal,
                format!("Failed to retrieve peers from the database for network: {network_id}"),
            )
        })?;

        info!("Configuration sent to gateway client, network {network}.");

        Ok(Response::new(gen_config(&network, peers)))
    }

    async fn updates(&self, request: Request<()>) -> Result<Response<Self::UpdatesStream>, Status> {
        let gateway_network_id = Self::get_network_id(request.metadata())?;
        let hostname = Self::get_gateway_hostname(request.metadata())?;

        let Some(network) = WireguardNetwork::find_by_id(&self.pool, gateway_network_id)
            .await
            .map_err(|_| {
                error!("Failed to fetch network {gateway_network_id} from the database");
                Status::new(
                    Code::Internal,
                    format!("Failed to retrieve network {gateway_network_id} from the database"),
                )
            })?
        else {
            return Err(Status::new(
                Code::Internal,
                format!("Network with id {gateway_network_id} not found"),
            ));
        };

        info!("New client connected to updates stream: {hostname}, network {network}",);

        let (tx, rx) = mpsc::channel(4);
        let events_rx = self.wireguard_tx.subscribe();
        let mut state = self.state.lock().unwrap();
        state
            .connect_gateway(gateway_network_id, &hostname)
            .map_err(|err| {
                error!("Failed to connect gateway on network {gateway_network_id}: {err}");
                Status::new(
                    Code::Internal,
                    "Failed to connect gateway on network {gateway_network_id}",
                )
            })?;

        // clone here before moving into a closure
        let gateway_hostname = hostname.clone();
        let handle = tokio::spawn(async move {
            let mut update_handler = GatewayUpdatesHandler::new(
                gateway_network_id,
                network,
                gateway_hostname,
                events_rx,
                tx,
            );
            update_handler.run().await;
        });

        Ok(Response::new(GatewayUpdatesStream::new(
            handle,
            rx,
            gateway_network_id,
            hostname,
            Arc::clone(&self.state),
            self.pool.clone(),
        )))
    }
}
