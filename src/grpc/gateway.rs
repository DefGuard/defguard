use crate::db::{
    models::wireguard::{WireguardNetwork, WireguardPeerStats},
    DbPool, Device, GatewayEvent,
};
use chrono::{NaiveDateTime, Utc};
use std::sync::Arc;
use tokio::sync::{Mutex,
    mpsc::{self, UnboundedReceiver},
};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};

use super::GatewayState;

tonic::include_proto!("gateway");

pub struct GatewayServer {
    pool: DbPool,
    wireguard_rx: Arc<Mutex<UnboundedReceiver<GatewayEvent>>>,
    state: Arc<Mutex<GatewayState>>,
}

impl GatewayServer {
    /// Create new gateway server instance
    #[must_use]
    pub fn new(
        wireguard_rx: UnboundedReceiver<GatewayEvent>,
        pool: DbPool,
        state: Arc<Mutex<GatewayState>>,
    ) -> Self {
        Self {
            wireguard_rx: Arc::new(Mutex::new(wireguard_rx)),
            pool,
            state,
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
        update_type: i32,
    ) -> Result<(), Status> {
        if let Err(err) = tx
            .send(Ok(Update {
                update_type,
                update: Some(update::Update::Peer(Peer {
                    pubkey: device.wireguard_pubkey.clone(),
                    allowed_ips: vec![device.wireguard_ip.clone()],
                })),
            }))
            .await
        {
            let msg = format!(
                "Failed to send network update, network {}, update type: {}, error: {}",
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

fn gen_config(network: &WireguardNetwork, devices: &[Device]) -> Configuration {
    let peers = devices
        .iter()
        .map(|d| Peer {
            pubkey: d.wireguard_pubkey.clone(),
            allowed_ips: vec![d.wireguard_ip.clone()],
        })
        .collect();

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

#[tonic::async_trait]
impl gateway_service_server::GatewayService for GatewayServer {
    type UpdatesStream = ReceiverStream<Result<Update, Status>>;
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

    async fn config(&self, _request: Request<()>) -> Result<Response<Configuration>, Status> {
        let pool = self.pool.clone();
        let mut network = WireguardNetwork::find_by_id(&pool, 1)
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
        Ok(Response::new(gen_config(&network, &devices)))
    }

    async fn updates(&self, _: Request<()>) -> Result<Response<Self::UpdatesStream>, Status> {
        let (tx, rx) = mpsc::channel(4);
        let events_rx = Arc::clone(&self.wireguard_rx);
        self.state.lock().await.clients.push("test".to_string());
        let state = Arc::clone(&self.state);
        tokio::spawn(async move {
            while let Some(update) = events_rx.lock().await.recv().await {
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
                    GatewayEvent::DeviceCreated(device) => {
                        Self::send_peer_update(&tx, &device, 0).await
                    }
                    GatewayEvent::DeviceModified(device) => {
                        Self::send_peer_update(&tx, &device, 1).await
                    }
                    GatewayEvent::DeviceDeleted(device_name) => {
                        Self::send_peer_delete(&tx, &device_name).await
                    }
                };
                if let Err(err) = result {
                    error!("Client stream disconnected: {}", err);
                    state.lock().await.clients.clear();
                    break;
                }
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }
}
