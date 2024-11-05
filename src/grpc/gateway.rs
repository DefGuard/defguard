use std::{
    str::FromStr,
    sync::atomic::{AtomicU64, Ordering},
};

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use tokio::{
    sync::{
        broadcast::{Receiver, Sender},
        mpsc::{self, UnboundedSender},
    },
    time::sleep,
};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tonic::{
    transport::{ClientTlsConfig, Endpoint},
    Code, Status,
};

use super::TEN_SECS;
use crate::{
    auth::{Claims, ClaimsType},
    db::{
        models::{
            device::Device,
            gateway::Gateway,
            wireguard::{ChangeEvent, WireguardNetwork, WireguardPeerStats},
        },
        Id, NoId,
    },
    handlers::mail::send_gateway_disconnected_email,
    mail::Mail,
};

#[cfg(test)]
mod tests;

tonic::include_proto!("gateway");

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

/// One instance per connected gateway.
pub(super) struct GatewayHandler {
    endpoint: Endpoint,
    gateway: Gateway<Id>,
    message_id: AtomicU64,
    pool: PgPool,
    events_tx: Sender<ChangeEvent>,
    mail_tx: UnboundedSender<Mail>,
}

impl GatewayHandler {
    pub(super) fn new(
        gateway: Gateway<Id>,
        tls_config: Option<ClientTlsConfig>,
        pool: PgPool,
        events_tx: Sender<ChangeEvent>,
        mail_tx: UnboundedSender<Mail>,
    ) -> Result<Self, tonic::transport::Error> {
        let endpoint = Endpoint::from_shared(gateway.url.to_string())?
            .http2_keep_alive_interval(TEN_SECS)
            .tcp_keepalive(Some(TEN_SECS))
            .keep_alive_while_idle(true);
        let endpoint = if let Some(tls) = tls_config {
            endpoint.tls_config(tls)?
        } else {
            endpoint
        };

        Ok(Self {
            endpoint,
            gateway,
            message_id: AtomicU64::new(0),
            pool,
            events_tx,
            mail_tx,
        })
    }

    /// Send network and VPN configuration to gateway.
    async fn send_configuration(&self, tx: &UnboundedSender<CoreResponse>) -> Result<(), Status> {
        debug!("Sending configuration to gateway");
        let network_id = self.gateway.network_id;
        // let hostname = Self::get_gateway_hostname(request.metadata())?;

        let mut network = WireguardNetwork::find_by_id(&self.pool, network_id)
            .await
            .map_err(|err| {
                error!("Network {network_id} not found");
                Status::new(Code::Internal, format!("Failed to retrieve network: {err}"))
            })?
            .ok_or_else(|| {
                Status::new(
                    Code::Internal,
                    format!("Network with id {network_id} not found"),
                )
            })?;

        debug!(
            "Sending configuration to {}, network {network}",
            self.gateway
        );
        if let Err(err) = network.touch_connected(&self.pool).await {
            error!("Failed to update connected at for network {network_id} in the database, status: {err}");
        }

        let peers = network.get_peers(&self.pool).await.map_err(|error| {
            error!("Failed to fetch peers from the database for network {network_id}: {error}",);
            Status::new(
                Code::Internal,
                format!("Failed to retrieve peers from the database for network: {network_id}"),
            )
        })?;

        let payload = Some(core_response::Payload::Config(gen_config(&network, peers)));
        let id = self.message_id.fetch_add(1, Ordering::Relaxed);
        let req = CoreResponse { id, payload };
        match tx.send(req) {
            Ok(()) => {
                info!("Configuration sent to {}, network {network}", self.gateway);
                Ok(())
            }
            Err(err) => {
                error!("Failed to send configuration sent to {}", self.gateway);
                Err(Status::new(
                    Code::Internal,
                    format!("Configuration not sent to {}, error {err}", self.gateway),
                ))
            }
        }
    }

    /// Send gateway disconnected notification.
    /// Sends notification only if last notification time is bigger than specified in config.
    async fn send_disconnect_notification(&self) {
        debug!("Sending gateway disconnect email notification");
        let hostname = self.gateway.hostname.clone();
        let mail_tx = self.mail_tx.clone();
        let pool = self.pool.clone();
        let url = self.gateway.url.clone();

        let Ok(Some(network)) =
            WireguardNetwork::find_by_id(&self.pool, self.gateway.network_id).await
        else {
            error!(
                "Failed to fetch network ID {} from database",
                self.gateway.network_id
            );
            return;
        };

        // Send email only if disconnection time is before the connection time.
        let send_email = if let (Some(connected_at), Some(disconnected_at)) =
            (self.gateway.connected_at, self.gateway.disconnected_at)
        {
            disconnected_at <= connected_at
        } else {
            true
        };
        if send_email {
            // FIXME: Try to get rid of spawn and use something like block_on
            // To return result instead of logging
            tokio::spawn(async move {
                if let Err(err) =
                    send_gateway_disconnected_email(hostname, &network.name, &url, &mail_tx, &pool)
                        .await
                {
                    error!("Failed to send gateway disconnect notification: {err}");
                } else {
                    info!("Email notification sent about gateway being disconnected");
                }
            });
        } else {
            info!(
                "{} disconnected. Email notification not sent.",
                self.gateway
            );
        };
    }

    /// Connect to Gateway and handle its messages through gRPC.
    pub(super) async fn handle_connection(&mut self) -> ! {
        let uri = self.endpoint.uri();
        loop {
            #[cfg(not(test))]
            let channel = self.endpoint.connect_lazy();
            #[cfg(test)]
            let channel = self.endpoint.connect_with_connector_lazy(tower::service_fn(
                |_: tonic::transport::Uri| async {
                    Ok::<_, std::io::Error>(hyper_util::rt::TokioIo::new(
                        tokio::net::UnixStream::connect(tests::TONIC_SOCKET).await?,
                    ))
                },
            ));

            debug!("Connecting to gateway {uri}");
            let mut client = gateway_client::GatewayClient::new(channel);
            let (tx, rx) = mpsc::unbounded_channel();
            let response = match client.bidi(UnboundedReceiverStream::new(rx)).await {
                Ok(response) => response,
                Err(err) => {
                    error!("Failed to connect to gateway {uri}, retrying: {err}");
                    sleep(TEN_SECS).await;
                    continue;
                }
            };

            info!("Connected to gateway {uri}");
            let mut resp_stream = response.into_inner();
            let mut config_sent = false;

            'message: loop {
                match resp_stream.message().await {
                    Ok(None) => {
                        info!("stream was closed by the sender");
                        break 'message;
                    }
                    Ok(Some(received)) => {
                        info!("Received message from gateway.");
                        debug!("Message from gateway {uri}: {received:?}");
                        match received.payload {
                            Some(core_request::Payload::ConfigRequest(config_request)) => {
                                if config_sent {
                                    warn!(
                                        "Ignoring repeated configuration request from {}",
                                        self.gateway
                                    );
                                    continue;
                                }
                                // Validate authorization token.
                                if let Ok(claims) = Claims::from_jwt(
                                    ClaimsType::Gateway,
                                    &config_request.auth_token,
                                ) {
                                    if let Ok(client_id) = Id::from_str(&claims.client_id) {
                                        if client_id == self.gateway.network_id {
                                            debug!(
                                                "Authorization token is correct for {}",
                                                self.gateway
                                            );
                                        } else {
                                            warn!("Authorization token received from {uri} has `client_id` for a different network");
                                            continue;
                                        }
                                    } else {
                                        warn!("Authorization token received from {uri} has incorrect `client_id`");
                                        continue;
                                    }
                                } else {
                                    warn!("Invalid authorization token received from {uri}");
                                    continue;
                                }

                                // Send network configuration to gateway.
                                match self.send_configuration(&tx).await {
                                    Ok(()) => {
                                        info!("Sent configuration to {}", self.gateway);
                                        config_sent = true;
                                        let _ = self
                                            .gateway
                                            .touch_connected(&self.pool, config_request.hostname)
                                            .await;
                                    }
                                    Err(err) => {
                                        error!(
                                            "Failed to send configuration to {}: {err}",
                                            self.gateway
                                        );
                                    }
                                }

                                // Start observing configuration changes.
                                let Ok(Some(network)) = WireguardNetwork::find_by_id(
                                    &self.pool,
                                    self.gateway.network_id,
                                )
                                .await
                                else {
                                    error!(
                                        "Failed to fetch network ID {} from the database",
                                        self.gateway.network_id
                                    );
                                    continue;
                                };
                                tokio::spawn(handle_events(
                                    network,
                                    tx.clone(),
                                    self.events_tx.subscribe(),
                                ));
                            }
                            Some(core_request::Payload::PeerStats(peer_stats)) => {
                                if !config_sent {
                                    warn!("Ignoring peer statistics from {} because it didn't authorize itself", self.gateway);
                                    continue;
                                }

                                let public_key = peer_stats.public_key.clone();
                                let mut stats = WireguardPeerStats::from_peer_stats(
                                    peer_stats,
                                    self.gateway.network_id,
                                );
                                // Get device by public key and fill in stats.device_id
                                match Device::find_by_pubkey(&self.pool, &public_key).await {
                                    Ok(Some(device)) => {
                                        stats.device_id = device.id;
                                        match stats.save(&self.pool).await {
                                            Ok(_) => info!("Saved WireGuard peer stats to database."),
                                            Err(err) => error!("Failed to save WireGuard peer stats to database: {err}"),
                                        }
                                    }
                                    Ok(None) => {
                                        error!("Device with public key {public_key} not found");
                                    }
                                    Err(err) => {
                                        error!("Failed to retrieve device with public key {public_key}: {err}",);
                                    }
                                };
                            }
                            None => (),
                        };
                    }
                    Err(err) => {
                        error!("Disconnected from gateway at {uri}, error: {err}");
                        // Important: call this funtion before setting disconnection time.
                        self.send_disconnect_notification().await;
                        let _ = self.gateway.touch_disconnected(&self.pool).await;
                        debug!("Waiting 10s to re-establish the connection");
                        sleep(TEN_SECS).await;
                        break 'message;
                    }
                }
            }
        }
    }
}

/// Process incoming gateway events
///
/// Main gRPC server uses a shared channel for broadcasting all gateway events
/// so the handler must determine if an event is relevant for the network being serviced.
async fn handle_events(
    mut current_network: WireguardNetwork<Id>,
    tx: UnboundedSender<CoreResponse>,
    mut events_rx: Receiver<ChangeEvent>,
) {
    info!("Starting update stream network {}", current_network);
    while let Ok(event) = events_rx.recv().await {
        debug!("Received networking state update event: {event:?}");
        let (update_type, update) = match event {
            ChangeEvent::NetworkCreated(network) => {
                if network.id != current_network.id {
                    continue;
                }
                (
                    UpdateType::Create,
                    update::Update::Network(Configuration {
                        name: network.name.clone(),
                        prvkey: network.prvkey.clone(),
                        address: network.address.to_string(),
                        port: network.port as u32,
                        peers: Vec::new(),
                    }),
                )
            }
            ChangeEvent::NetworkModified(network, peers) => {
                if network.id != current_network.id {
                    continue;
                }
                // update stored network data
                current_network = network.clone();
                (
                    UpdateType::Modify,
                    update::Update::Network(Configuration {
                        name: network.name,
                        prvkey: network.prvkey,
                        address: network.address.to_string(),
                        port: network.port as u32,
                        peers,
                    }),
                )
            }
            ChangeEvent::NetworkDeleted(network_id, network_name) => {
                if network_id != current_network.id {
                    continue;
                }
                (
                    UpdateType::Delete,
                    update::Update::Network(Configuration {
                        name: network_name.to_string(),
                        prvkey: String::new(),
                        address: String::new(),
                        port: 0,
                        peers: Vec::new(),
                    }),
                )
            }
            ChangeEvent::DeviceCreated(device) => {
                // check if a peer has to be added in the current network
                match device
                    .network_info
                    .iter()
                    .find(|info| info.network_id == current_network.id)
                {
                    Some(network_info) => {
                        if current_network.mfa_enabled && !network_info.is_authorized {
                            debug!("Created WireGuard device {} is not authorized to connect to MFA enabled location {}",
                                device.device.name, current_network.name
                            );
                            continue;
                        };
                        let peer = Peer {
                            pubkey: device.device.wireguard_pubkey,
                            allowed_ips: vec![network_info.device_wireguard_ip.to_string()],
                            preshared_key: network_info.preshared_key.clone(),
                            keepalive_interval: Some(current_network.keepalive_interval as u32),
                        };
                        (UpdateType::Create, update::Update::Peer(peer))
                    }
                    None => continue,
                }
            }
            ChangeEvent::DeviceModified(device) => {
                // check if a peer has to be updated in the current network
                match device
                    .network_info
                    .iter()
                    .find(|info| info.network_id == current_network.id)
                {
                    Some(network_info) => {
                        if current_network.mfa_enabled && !network_info.is_authorized {
                            debug!("Modified WireGuard device {} is not authorized to connect to MFA enabled location {}",
                                device.device.name, current_network.name
                            );
                            continue;
                        };
                        let peer = Peer {
                            pubkey: device.device.wireguard_pubkey,
                            allowed_ips: vec![network_info.device_wireguard_ip.to_string()],
                            preshared_key: network_info.preshared_key.clone(),
                            keepalive_interval: Some(current_network.keepalive_interval as u32),
                        };
                        (UpdateType::Modify, update::Update::Peer(peer))
                    }
                    None => continue,
                }
            }
            ChangeEvent::DeviceDeleted(device) => {
                // check if a peer has to be updated in the current network
                match device
                    .network_info
                    .iter()
                    .find(|info| info.network_id == current_network.id)
                {
                    Some(_) => (
                        UpdateType::Delete,
                        update::Update::Peer(Peer {
                            pubkey: device.device.wireguard_pubkey,
                            allowed_ips: Vec::new(),
                            preshared_key: None,
                            keepalive_interval: None,
                        }),
                    ),
                    None => continue,
                }
            }
        };

        let req = CoreResponse {
            id: 0,
            payload: Some(core_response::Payload::Update(Update {
                update_type: update_type as i32,
                update: Some(update),
            })),
        };
        if let Err(err) = tx.send(req) {
            error!(
                "Failed to send network update, network {}, update type: {}, error: {err}",
                current_network,
                update_type.as_str_name()
            );
            break;
        }
        debug!(
            "Network update sent for network {}, update type: {}",
            current_network,
            update_type.as_str_name()
        );
    }
}
