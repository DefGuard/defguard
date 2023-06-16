use crate::{
    db::{
        models::{
            device::DeviceNetworkInfo,
            wireguard::{WireguardNetwork, WireguardPeerStats},
        },
        DbPool, Device, GatewayEvent,
    },
    grpc::GatewayMap,
};
use chrono::{NaiveDateTime, Utc};
use std::{
    net::SocketAddr,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll},
};
use tokio::{
    sync::{
        broadcast::Sender,
        mpsc::{self, Receiver},
    },
    task::JoinHandle,
};
use tokio_stream::Stream;
use tonic::{metadata::MetadataMap, Code, Request, Response, Status};

tonic::include_proto!("gateway");

pub struct GatewayServer {
    pool: DbPool,
    state: Arc<Mutex<GatewayMap>>,
    wireguard_tx: Sender<GatewayEvent>,
}

impl GatewayServer {
    /// Create new gateway server instance
    #[must_use]
    pub fn new(
        pool: DbPool,
        state: Arc<Mutex<GatewayMap>>,
        wireguard_tx: Sender<GatewayEvent>,
    ) -> Self {
        Self {
            pool,
            state,
            wireguard_tx,
        }
    }
    /// Sends updated network configuration
    async fn send_network_update(
        tx: &mpsc::Sender<Result<Update, Status>>,
        network: &WireguardNetwork,
        update_type: i32,
    ) -> Result<(), Status> {
        if let Err(err) = tx
            .send(Ok(Update {
                update_type,
                update: Some(update::Update::Network(Configuration {
                    name: network.name.clone(),
                    prvkey: network.prvkey.clone(),
                    address: network.address.to_string(),
                    port: network.port as u32,
                    peers: Vec::new(),
                })),
            }))
            .await
        {
            let msg = format!(
                "Failed to send network update, network {}, update type: {}, error: {}",
                network.name, update_type, err,
            );
            error!("{}", msg);
            return Err(Status::new(tonic::Code::Internal, msg));
        }
        Ok(())
    }
    /// Sends delete network command to gateway
    async fn send_network_delete(
        tx: &mpsc::Sender<Result<Update, Status>>,
        network_name: &str,
    ) -> Result<(), Status> {
        if let Err(err) = tx
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
                network_name, 2, err,
            );
            error!("{}", msg);
            return Err(Status::new(tonic::Code::Internal, msg));
        }
        Ok(())
    }
    /// Send update peer command to gateway
    async fn send_peer_update(
        tx: &mpsc::Sender<Result<Update, Status>>,
        device: &Device,
        device_network_info: &DeviceNetworkInfo,
        update_type: i32,
    ) -> Result<(), Status> {
        if let Err(err) = tx
            .send(Ok(Update {
                update_type,
                update: Some(update::Update::Peer(Peer {
                    pubkey: device.wireguard_pubkey.clone(),
                    allowed_ips: vec![device_network_info.wireguard_ip.clone()],
                })),
            }))
            .await
        {
            let msg = format!(
                "Failed to send network update, device {}, update type: {}, error: {}",
                device.name, update_type, err,
            );
            error!("{}", msg);
            return Err(Status::new(tonic::Code::Internal, msg));
        }
        Ok(())
    }
    /// Send delete peer command to gateway
    async fn send_peer_delete(
        tx: &mpsc::Sender<Result<Update, Status>>,
        peer_pubkey: &str,
    ) -> Result<(), Status> {
        if let Err(err) = tx
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
                "Failed to send peer update, peer {}, update type: 2, error: {}",
                peer_pubkey, err,
            );
            error!("{}", msg);
            return Err(Status::new(tonic::Code::Internal, msg));
        }
        Ok(())
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

impl From<PeerStats> for WireguardPeerStats {
    fn from(stats: PeerStats) -> Self {
        let endpoint = match stats.endpoint {
            endpoint if endpoint.is_empty() => None,
            _ => Some(stats.endpoint),
        };
        Self {
            id: None,
            // FIXME: hard-coded network id
            network: 1,
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

pub struct GatewayUpdatesStream {
    task_handle: JoinHandle<()>,
    rx: Receiver<Result<Update, Status>>,
    gateway_addr: SocketAddr,
    gateway_state: Arc<Mutex<GatewayMap>>,
}

impl GatewayUpdatesStream {
    #[must_use]
    pub fn new(
        task_handle: JoinHandle<()>,
        rx: Receiver<Result<Update, Status>>,
        gateway_addr: SocketAddr,
        gateway_state: Arc<Mutex<GatewayMap>>,
    ) -> Self {
        Self {
            task_handle,
            rx,
            gateway_addr,
            gateway_state,
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
        self.gateway_state
            .lock()
            .unwrap()
            .disconnect_gateway(self.gateway_addr);
        // terminate update task
        self.task_handle.abort();
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
        let mut stream = request.into_inner();
        while let Some(peer_stats) = stream.message().await? {
            let public_key = peer_stats.public_key.clone();
            let mut stats = WireguardPeerStats::from(peer_stats);
            // Get device by public key and fill in stats.device_id
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
                        "Failed to retrieve device with public key {}: {}",
                        &public_key, err
                    );
                    return Err(Status::new(
                        tonic::Code::Internal,
                        format!(
                            "Failed to retrieve device with public key {}: {}",
                            &public_key, err
                        ),
                    ));
                }
            };
            // Save stats to db
            if let Err(err) = stats.save(&self.pool).await {
                error!("Saving WireGuard peer stats to db failed: {}", err);
                return Err(Status::new(
                    tonic::Code::Internal,
                    format!("Saving WireGuard peer stats to db failed: {}", err),
                ));
            }
            debug!("Saved WireGuard peer stats to db: {:?}", stats);
        }
        Ok(Response::new(()))
    }

    async fn config(&self, request: Request<()>) -> Result<Response<Configuration>, Status> {
        info!("Sending configuration to gateway client.");
        let network_id = match get_network_id_from_metadata(request.metadata()) {
            Some(m) => m,
            None => {
                return Err(Status::new(
                    Code::Unauthenticated,
                    "Network ID was not found in metadata",
                ));
            }
        };
        let pool = self.pool.clone();
        let mut network = WireguardNetwork::find_by_id(&pool, network_id)
            .await
            .map_err(|e| {
                Status::new(
                    tonic::Code::FailedPrecondition,
                    format!("Failed to retrieve network: {}", e),
                )
            })?
            .ok_or_else(|| Status::new(tonic::Code::FailedPrecondition, "Network not found"))?;
        network.connected_at = Some(Utc::now().naive_utc());
        if let Err(err) = network.save(&pool).await {
            error!("Failed to save network: {}", err);
        }
        let devices = Device::all(&pool).await.unwrap_or_default();
        let network_devices_info = Device::find_by_network(&pool, network_id).await;
        let peers: Vec<Peer> = match network_devices_info {
            Ok(devices_option) => match devices_option {
                Some(network_devices_info) => network_devices_info
                    .iter()
                    .map(|(device, info)| Peer {
                        pubkey: device.wireguard_pubkey,
                        allowed_ips: vec![info.wireguard_ip],
                    })
                    .collect::<Vec<Peer>>(),
                None => [].into(),
            },
            Err(_) => [].into(),
        };
        Ok(Response::new(gen_config(&network, peers)))
    }

    async fn updates(&self, request: Request<()>) -> Result<Response<Self::UpdatesStream>, Status> {
        let network_id = match get_network_id_from_metadata(request.metadata()) {
            Some(m) => m,
            None => {
                return Err(Status::new(
                    Code::Unauthenticated,
                    "Network ID was not found in metadata",
                ));
            }
        };
        let address = request.remote_addr().expect("Unable to get peer address.");
        info!("New client connected to updates stream: {}", address);
        let (tx, rx) = mpsc::channel(4);

        let mut events_rx = self.wireguard_tx.subscribe();
        let mut state = self.state.lock().unwrap();
        state.connect_gateway(address, network_id);

        let handle = tokio::spawn(async move {
            info!("Starting update stream to gateway: {}", address);
            while let Ok(update) = events_rx.recv().await {
                let result = match update {
                    GatewayEvent::NetworkCreated(network) => {
                        Self::send_network_update(&tx, &network, 0).await
                    }
                    GatewayEvent::NetworkModified(network) => {
                        Self::send_network_update(&tx, &network, 1).await
                    }
                    GatewayEvent::NetworkDeleted(network_name) => {
                        Self::send_network_delete(&tx, &network_name).await
                    }
                    GatewayEvent::DeviceCreated(device, device_network_info) => {
                        Self::send_peer_update(&tx, &device, &device_network_info, 0).await
                    }
                    GatewayEvent::DeviceModified(device, device_network_info) => {
                        Self::send_peer_update(&tx, &device, &device_network_info, 1).await
                    }
                    GatewayEvent::DeviceDeleted(device_name) => {
                        Self::send_peer_delete(&tx, &device_name).await
                    }
                };
                if result.is_err() {
                    error!("Closing update steam to gateway: {}", address);
                    break;
                }
            }
        });
        Ok(Response::new(GatewayUpdatesStream::new(
            handle,
            rx,
            address,
            Arc::clone(&self.state),
        )))
    }
}

// prase network id from gateway request metadata from intecepted information from JWT token
fn get_network_id_from_metadata(metadata: &MetadataMap) -> Option<i64> {
    if let Some(ascii_value) = metadata.get("gateway_network_id") {
        match ascii_value.clone().to_str() {
            Ok(slice) => {
                if let Ok(id) = i64::from_str_radix(slice, 10) {
                    return Some(id);
                }
            }
            _ => (),
        }
    }
    None
}
