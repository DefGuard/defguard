use super::GatewayMap;
use crate::{
    db::{
        models::wireguard::{WireguardNetwork, WireguardPeerStats},
        DbPool, Device, GatewayEvent,
    },
    handlers::mail::send_gateway_disconnected_email,
    mail::Mail,
};
use chrono::{NaiveDateTime, Utc};
use sqlx::{query_as, Error as SqlxError};
use std::{
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll},
};
use tokio::{
    sync::{
        broadcast::{Receiver as BroadcastReceiver, Sender},
        mpsc::{self, Receiver, UnboundedSender},
    },
    task::JoinHandle,
};
use tokio_stream::Stream;
use tonic::{metadata::MetadataMap, Code, Request, Response, Status};

tonic::include_proto!("gateway");

pub struct GatewayServer {
    pool: DbPool,
    state: Arc<Mutex<GatewayMap>>,
    mail_tx: UnboundedSender<Mail>,
    wireguard_tx: Sender<GatewayEvent>,
}

impl WireguardNetwork {
    /// Get a list of all peers
    pub async fn get_peers<'e, E>(&self, executor: E) -> Result<Vec<Peer>, SqlxError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        debug!("Fetching all peers for network {}", self.id.unwrap());
        let result = query_as!(
            Peer,
            r#"
            SELECT d.wireguard_pubkey as pubkey, array[host(wnd.wireguard_ip)] as "allowed_ips!: Vec<String>" FROM wireguard_network_device wnd
            JOIN device d
            ON wnd.device_id = d.id
            WHERE wireguard_network_id = $1
            ORDER BY d.id ASC
        "#,
            self.id
        )
        .fetch_all(executor)
        .await?;

        Ok(result)
    }
}

impl GatewayServer {
    /// Create new gateway server instance
    #[must_use]
    pub fn new(
        pool: DbPool,
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

fn gen_config(network: &WireguardNetwork, peers: Vec<Peer>) -> Configuration {
    Configuration {
        name: network.name.clone(),
        port: network.port as u32,
        prvkey: network.prvkey.clone(),
        address: network.address.to_string(),
        peers,
    }
}

impl WireguardPeerStats {
    fn from_peer_stats(stats: PeerStats, network_id: i64) -> Self {
        let endpoint = match stats.endpoint {
            endpoint if endpoint.is_empty() => None,
            _ => Some(stats.endpoint),
        };
        Self {
            id: None,
            network: network_id,
            endpoint,
            device_id: -1,
            collected_at: Utc::now().naive_utc(),
            upload: stats.upload,
            download: stats.download,
            latest_handshake: NaiveDateTime::from_timestamp_opt(stats.latest_handshake, 0)
                .unwrap_or_default(),
            allowed_ips: Some(stats.allowed_ips),
        }
    }
}

/// Helper struct for handling gateway events
struct GatewayUpdatesHandler {
    network_id: i64,
    network: WireguardNetwork,
    gateway_hostname: String,
    events_rx: BroadcastReceiver<GatewayEvent>,
    tx: mpsc::Sender<Result<Update, Status>>,
}

impl GatewayUpdatesHandler {
    pub fn new(
        network_id: i64,
        network: WireguardNetwork,
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
            debug!("Received wireguard update: {:?}", update);
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
                        self.send_network_update(&network, peers, 1).await
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
                            self.send_peer_update(
                                Peer {
                                    pubkey: device.device.wireguard_pubkey,
                                    allowed_ips: vec![network_info.device_wireguard_ip.to_string()],
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
                            self.send_peer_update(
                                Peer {
                                    pubkey: device.device.wireguard_pubkey,
                                    allowed_ips: vec![network_info.device_wireguard_ip.to_string()],
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
        network: &WireguardNetwork,
        peers: Vec<Peer>,
        update_type: i32,
    ) -> Result<(), Status> {
        debug!("Sending network update for network {}", network);
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
                "Failed to send network update, network {network}, update type: {update_type}, error: {err}",
            );
            error!("{msg}");
            return Err(Status::new(tonic::Code::Internal, msg));
        }
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
                "Failed to send network update, network {}, update type: {}, error: {}",
                self.network, 2, err,
            );
            error!("{}", msg);
            return Err(Status::new(tonic::Code::Internal, msg));
        }
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
                "Failed to send peer update for network {}, update type: {}, error: {}",
                self.network, update_type, err,
            );
            error!("{}", msg);
            return Err(Status::new(tonic::Code::Internal, msg));
        }
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
                })),
            }))
            .await
        {
            let msg = format!(
                "Failed to send peer update for network {}, peer {}, update type: 2, error: {}",
                self.network, peer_pubkey, err,
            );
            error!("{}", msg);
            return Err(Status::new(tonic::Code::Internal, msg));
        }
        Ok(())
    }
}

pub struct GatewayUpdatesStream {
    task_handle: JoinHandle<()>,
    rx: Receiver<Result<Update, Status>>,
    network_id: i64,
    gateway_hostname: String,
    gateway_state: Arc<Mutex<GatewayMap>>,
    mail_tx: UnboundedSender<Mail>,
    pool: DbPool,
}

impl GatewayUpdatesStream {
    #[must_use]
    pub fn new(
        task_handle: JoinHandle<()>,
        rx: Receiver<Result<Update, Status>>,
        network_id: i64,
        gateway_hostname: String,
        gateway_state: Arc<Mutex<GatewayMap>>,
        mail_tx: UnboundedSender<Mail>,
        pool: DbPool,
    ) -> Self {
        Self {
            task_handle,
            rx,
            network_id,
            gateway_hostname,
            gateway_state,
            mail_tx,
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
        let mut gateway_state = self.gateway_state.lock().unwrap();
        let gateway_name =
            gateway_state.get_network_gateway_name(self.network_id, &self.gateway_hostname);
        gateway_state
            .disconnect_gateway(self.network_id, self.gateway_hostname.clone())
            .expect("Unable to disconnect gateway.");
        // Clone objects here to avoid '`self` is a reference that is only valid in the method body' error
        let hostname = self.gateway_hostname.clone();
        let network_id = self.network_id;
        let mail_tx = self.mail_tx.clone();
        let pool = self.pool.clone();
        tokio::spawn(async move {
            if let Err(e) =
                send_gateway_disconnected_email(gateway_name, hostname, network_id, &mail_tx, pool)
                    .await
            {
                error!("Sending gateway disconnected notification failed: {}", e);
            }
        });
    }
}

#[tonic::async_trait]
impl gateway_service_server::GatewayService for GatewayServer {
    type UpdatesStream = GatewayUpdatesStream;

    /// Retrieve stats from gateway and save it to database
    async fn stats(
        &self,
        request: Request<tonic::Streaming<PeerStats>>,
    ) -> Result<Response<()>, Status> {
        let network_id = Self::get_network_id(request.metadata())?;
        let mut stream = request.into_inner();
        while let Some(peer_stats) = stream.message().await? {
            let public_key = peer_stats.public_key.clone();
            let mut stats = WireguardPeerStats::from_peer_stats(peer_stats, network_id);
            // Get device by public key and fill in stats.device_id
            // FIXME: keep an in-memory device map to avoid repeated DB requests
            stats.device_id = match Device::find_by_pubkey(&self.pool, &public_key).await {
                Ok(Some(device)) => device
                    .id
                    .ok_or_else(|| Status::new(tonic::Code::Internal, "Device has no id"))?,
                Ok(None) => {
                    error!("Device with public key {} not found", &public_key);
                    return Err(Status::new(
                        tonic::Code::Internal,
                        format!("Device with public key {} not found", &public_key),
                    ));
                }
                Err(err) => {
                    error!(
                        "Failed to retrieve device with public key {}: {err}",
                        &public_key
                    );
                    return Err(Status::new(
                        tonic::Code::Internal,
                        format!(
                            "Failed to retrieve device with public key {}: {err}",
                            &public_key
                        ),
                    ));
                }
            };
            // Save stats to db
            if let Err(err) = stats.save(&self.pool).await {
                error!("Saving WireGuard peer stats to db failed: {err}");
                return Err(Status::new(
                    tonic::Code::Internal,
                    format!("Saving WireGuard peer stats to db failed: {err}"),
                ));
            }
            info!("Saved WireGuard peer stats to db: {stats:?}");
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

        let pool = self.pool.clone();
        let mut network = WireguardNetwork::find_by_id(&pool, network_id)
            .await
            .map_err(|e| {
                error!("Network {} not found", network_id);
                Status::new(
                    tonic::Code::Internal,
                    format!("Failed to retrieve network: {e}"),
                )
            })?
            .ok_or_else(|| Status::new(tonic::Code::Internal, "Network not found"))?;

        info!("Sending configuration to gateway client, network {network}.");

        {
            let mut state = self.state.lock().unwrap();
            state.add_gateway(network_id, hostname, request.into_inner().name);
        }

        network.connected_at = Some(Utc::now().naive_utc());
        if let Err(err) = network.save(&pool).await {
            error!("Failed to update network {network_id} status: {err}");
        }

        let peers = network.get_peers(&pool).await.map_err(|error| {
            error!("Failed to fetch peers for network {network_id}: {error}",);
            Status::new(
                tonic::Code::Internal,
                format!("Failed to retrieve peers for network: {network_id}"),
            )
        })?;

        Ok(Response::new(gen_config(&network, peers)))
    }

    async fn updates(&self, request: Request<()>) -> Result<Response<Self::UpdatesStream>, Status> {
        let gateway_network_id = Self::get_network_id(request.metadata())?;
        let hostname = Self::get_gateway_hostname(request.metadata())?;

        let Some(network) = WireguardNetwork::find_by_id(&self.pool, gateway_network_id)
            .await
            .map_err(|_| {
                error!("Failed to fetch network {gateway_network_id}");
                Status::new(
                    tonic::Code::Internal,
                    format!("Failed to retrieve network {gateway_network_id}"),
                )
            })?
        else {
            return Err(Status::new(Code::Internal, "Network not found"));
        };

        info!("New client connected to updates stream: {hostname}, network {network}",);

        let (tx, rx) = mpsc::channel(4);
        let events_rx = self.wireguard_tx.subscribe();
        let mut state = self.state.lock().unwrap();
        state
            .connect_gateway(gateway_network_id, &hostname)
            .map_err(|err| {
                error!("Failed to connect gateway: {err}");
                Status::new(tonic::Code::Internal, "Failed to connect gateway ")
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
            self.mail_tx.clone(),
            self.pool.clone(),
        )))
    }
}
