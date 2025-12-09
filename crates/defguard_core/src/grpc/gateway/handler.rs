use std::{
    net::SocketAddr,
    str::FromStr,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};

use chrono::{TimeDelta, Utc};
use defguard_common::{VERSION, auth::claims::Claims, db::Id};
use defguard_mail::Mail;
use defguard_proto::gateway::{CoreResponse, core_request, core_response, gateway_client};
use defguard_version::{client::ClientVersionInterceptor, version_info_from_metadata};
use semver::Version;
use sqlx::PgPool;
use tokio::{
    sync::{
        broadcast::Sender,
        mpsc::{self, UnboundedSender},
    },
    time::sleep,
};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tonic::{
    Code, Status,
    metadata::MetadataMap,
    transport::{ClientTlsConfig, Endpoint},
};

use crate::{
    ClaimsType,
    db::{
        Device, GatewayEvent, User, WireguardNetwork,
        models::{gateway::Gateway, wireguard_peer_stats::WireguardPeerStats},
    },
    grpc::{ClientMap, GrpcEvent, TEN_SECS, gateway::GrpcRequestContext},
    handlers::mail::send_gateway_disconnected_email,
};

/// One instance per connected Gateway.
pub(crate) struct GatewayHandler {
    endpoint: Endpoint,
    gateway: Gateway<Id>,
    message_id: AtomicU64,
    pool: PgPool,
    client_state: Arc<Mutex<ClientMap>>,
    events_tx: Sender<GatewayEvent>,
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

impl GatewayHandler {
    pub(crate) fn new(
        gateway: Gateway<Id>,
        tls_config: Option<ClientTlsConfig>,
        pool: PgPool,
        client_state: Arc<Mutex<ClientMap>>,
        events_tx: Sender<GatewayEvent>,
        mail_tx: UnboundedSender<Mail>,
        grpc_event_tx: UnboundedSender<GrpcEvent>,
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
            client_state,
            events_tx,
            mail_tx,
            grpc_event_tx,
        })
    }

    // Parse network ID from Gateway request metadata from intercepted information from JWT token.
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

    // Extract Gateway hostname from request headers.
    fn get_gateway_hostname(metadata: &MetadataMap) -> Option<String> {
        match metadata.get("hostname") {
            Some(ascii_value) => {
                let Ok(hostname) = ascii_value.to_str() else {
                    error!("Failed to parse Gateway hostname from request metadata");
                    return None;
                };
                Some(hostname.into())
            }
            None => {
                error!("Gateway hostname not found in request metadata");
                None
            }
        }
    }

    /// Utility function extracting metadata fields during gRPC communication.
    fn extract_metadata(metadata: &MetadataMap) -> Option<GatewayMetadata> {
        let (version, _info) = version_info_from_metadata(metadata);
        Some(GatewayMetadata {
            network_id: 0, // FIXME: not needed; was Self::get_network_id_from_metadata(metadata)?,
            hostname: Self::get_gateway_hostname(metadata)?,
            version,
        })
    }

    /// Send network and VPN configuration to Gateway.
    async fn send_configuration(
        &self,
        tx: &UnboundedSender<CoreResponse>,
    ) -> Result<WireguardNetwork<Id>, Status> {
        debug!("Sending configuration to Gateway");
        let network_id = self.gateway.network_id;

        let mut conn = self.pool.acquire().await.map_err(|err| {
            error!("Failed to acquire DB connection: {err}");
            Status::new(
                Code::Internal,
                "Failed to acquire database connection".to_string(),
            )
        })?;

        let mut network = WireguardNetwork::find_by_id(&mut *conn, network_id)
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
        if let Err(err) = network.touch_connected(&mut *conn).await {
            error!(
                "Failed to update connection time for network {network_id} in the database, \
                status: {err}"
            );
        }

        let peers = network.get_peers(&self.pool).await.map_err(|error| {
            error!("Failed to fetch peers from the database for network {network_id}: {error}",);
            Status::new(
                Code::Internal,
                format!("Failed to retrieve peers from the database for network: {network_id}"),
            )
        })?;

        let maybe_firewall_config =
            network
                .try_get_firewall_config(&mut *conn)
                .await
                .map_err(|err| {
                    error!("Failed to generate firewall config for network {network_id}: {err}");
                    Status::new(
                        Code::Internal,
                        format!("Failed to generate firewall config for network: {network_id}"),
                    )
                })?;
        let payload = Some(core_response::Payload::Config(super::gen_config(
            &network,
            peers,
            maybe_firewall_config,
        )));
        let id = self.message_id.fetch_add(1, Ordering::Relaxed);
        let req = CoreResponse { id, payload };
        match tx.send(req) {
            Ok(()) => {
                info!("Configuration sent to {}, network {network}", self.gateway);
                Ok(network)
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
                    send_gateway_disconnected_email(hostname, network.name, &url, &mail_tx, &pool)
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

    /// Helper method to fetch `Device` info from DB by pubkey and return appropriate errors
    async fn fetch_device_from_db(&self, public_key: &str) -> Result<Option<Device<Id>>, Status> {
        let device = Device::find_by_pubkey(&self.pool, public_key)
            .await
            .map_err(|err| {
                error!("Failed to retrieve device with public key {public_key}: {err}",);
                Status::new(
                    Code::Internal,
                    format!("Failed to retrieve device with public key {public_key}: {err}",),
                )
            })?;

        Ok(device)
    }

    /// Helper method to fetch `WireguardNetwork` info from DB and return appropriate errors
    async fn fetch_location_from_db(
        &self,
        location_id: Id,
    ) -> Result<WireguardNetwork<Id>, Status> {
        let location = match WireguardNetwork::find_by_id(&self.pool, location_id).await {
            Ok(Some(location)) => location,
            Ok(None) => {
                error!("Location {location_id} not found");
                return Err(Status::new(
                    Code::Internal,
                    format!("Location {location_id} not found"),
                ));
            }
            Err(err) => {
                error!("Failed to retrieve location {location_id}: {err}",);
                return Err(Status::new(
                    Code::Internal,
                    format!("Failed to retrieve location {location_id}: {err}",),
                ));
            }
        };
        Ok(location)
    }

    /// Helper method to fetch `User` info from DB and return appropriate errors
    async fn fetch_user_from_db(&self, user_id: Id, public_key: &str) -> Result<User<Id>, Status> {
        let user = match User::find_by_id(&self.pool, user_id).await {
            Ok(Some(user)) => user,
            Ok(None) => {
                error!("User {user_id} assigned to device with public key {public_key} not found");
                return Err(Status::new(
                    Code::Internal,
                    format!("User assigned to device with public key {public_key} not found"),
                ));
            }
            Err(err) => {
                error!(
                    "Failed to retrieve user {user_id} for device with public key {public_key}: {err}",
                );
                return Err(Status::new(
                    Code::Internal,
                    format!(
                        "Failed to retrieve user for device with public key {public_key}: {err}",
                    ),
                ));
            }
        };

        Ok(user)
    }

    fn emit_event(&self, event: GrpcEvent) {
        if self.grpc_event_tx.send(event).is_err() {
            warn!("Failed to send gRPC event");
        }
    }

    /// Connect to Gateway and handle its messages through gRPC.
    pub(crate) async fn handle_connection(&mut self) -> ! {
        let uri = self.endpoint.uri();
        loop {
            #[cfg(not(test))]
            let channel = self.endpoint.connect_lazy();
            #[cfg(test)]
            let channel = self.endpoint.connect_with_connector_lazy(tower::service_fn(
                |_: tonic::transport::Uri| async {
                    Ok::<_, std::io::Error>(hyper_util::rt::TokioIo::new(
                        tokio::net::UnixStream::connect(super::TONIC_SOCKET).await?,
                    ))
                },
            ));

            debug!("Connecting to Gateway {uri}");
            let interceptor = ClientVersionInterceptor::new(
                Version::parse(VERSION).expect("failed to parse self version"),
            );
            let mut client = gateway_client::GatewayClient::with_interceptor(channel, interceptor);
            let (tx, rx) = mpsc::unbounded_channel();
            let response = match client.bidi(UnboundedReceiverStream::new(rx)).await {
                Ok(response) => response,
                Err(err) => {
                    error!("Failed to connect to Gateway {uri}, retrying: {err}");
                    sleep(TEN_SECS).await;
                    continue;
                }
            };

            info!("Connected to Defguard Gateway {uri}");
            // Metadata isn't needed in reversed communication. TODO: remove, but only check version.
            // let Some(GatewayMetadata {
            //     hostname,
            // }) = Self::extract_metadata(response.metadata()) else {
            //     error!("Failed to extract metadata");
            //     continue;
            // };

            let mut resp_stream = response.into_inner();
            let mut config_sent = false;

            'message: loop {
                match resp_stream.message().await {
                    Ok(None) => {
                        info!("Stream was closed by the sender.");
                        break 'message;
                    }
                    Ok(Some(received)) => {
                        info!("Received message from Gateway.");
                        debug!("Message from Gateway {uri}");

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
                                            warn!(
                                                "Authorization token received from {uri} has \
                                                `client_id` for a different network"
                                            );
                                            continue;
                                        }
                                    } else {
                                        warn!(
                                            "Authorization token received from {uri} has incorrect \
                                            `client_id`"
                                        );
                                        continue;
                                    }
                                } else {
                                    warn!("Invalid authorization token received from {uri}");
                                    continue;
                                }

                                // Send network configuration to Gateway.
                                match self.send_configuration(&tx).await {
                                    Ok(network) => {
                                        info!("Sent configuration to {}", self.gateway);
                                        config_sent = true;
                                        let _ = self
                                            .gateway
                                            .touch_connected(&self.pool, config_request.hostname)
                                            .await;
                                        let mut guh = super::GatewayUpdatesHandler::new(
                                            self.gateway.network_id,
                                            network,
                                            self.gateway
                                                .hostname
                                                .as_ref()
                                                .cloned()
                                                .unwrap_or_default()
                                                .clone(),
                                            self.events_tx.subscribe(),
                                            tx.clone(),
                                        );
                                        tokio::spawn(async move {
                                            guh.run().await;
                                        });
                                    }
                                    Err(err) => {
                                        error!(
                                            "Failed to send configuration to {}: {err}",
                                            self.gateway
                                        );
                                    }
                                }
                            }
                            Some(core_request::Payload::PeerStats(peer_stats)) => {
                                if !config_sent {
                                    warn!(
                                        "Ignoring peer statistics from {} because it hasn't \
                                        authorize itself",
                                        self.gateway
                                    );
                                    continue;
                                }

                                let public_key = peer_stats.public_key.clone();

                                // Fetch device from database.
                                // TODO: fetch only when device has changed and use client state
                                // otherwise
                                let Ok(Some(device)) = self.fetch_device_from_db(&public_key).await
                                else {
                                    warn!(
                                        "Received stats update for a device which does not \
                                        exist: {public_key}, skipping."
                                    );
                                    continue;
                                };

                                // copy device ID for easier reference later
                                let device_id = device.id;

                                // fetch user and location from DB for activity log
                                // TODO: cache usernames since they don't change
                                let Ok(user) =
                                    self.fetch_user_from_db(device.user_id, &public_key).await
                                else {
                                    continue;
                                };
                                let Ok(location) =
                                    self.fetch_location_from_db(self.gateway.network_id).await
                                else {
                                    continue;
                                };

                                // Convert stats to database storage format.
                                let stats = WireguardPeerStats::from_peer_stats(
                                    peer_stats,
                                    self.gateway.network_id,
                                    device_id,
                                );

                                // Only perform client state update if stats include an endpoint IP.
                                // Otherwise, a peer was added to the gateway interface, but hasn't
                                // connected yet.
                                if let Some(endpoint) = &stats.endpoint {
                                    // parse client endpoint IP
                                    let Ok(socket_addr) = endpoint.clone().parse::<SocketAddr>()
                                    else {
                                        error!("Failed to parse VPN client endpoint");
                                        continue;
                                    };

                                    // Perform client state operations in a dedicated block to drop
                                    // mutex guard.
                                    let disconnected_clients = {
                                        // acquire lock on client state map
                                        let mut client_map = self.client_state.lock().unwrap();

                                        // update connected clients map
                                        match client_map
                                            .get_vpn_client(self.gateway.network_id, &public_key)
                                        {
                                            Some(client_state) => {
                                                // update connected client state
                                                client_state.update_client_state(
                                                    device,
                                                    socket_addr,
                                                    stats.latest_handshake,
                                                    stats.upload,
                                                    stats.download,
                                                );
                                            }
                                            None => {
                                                // don't mark inactive peers as connected
                                                if (Utc::now().naive_utc() - stats.latest_handshake)
                                                    < TimeDelta::seconds(
                                                        location.peer_disconnect_threshold.into(),
                                                    )
                                                {
                                                    // mark new VPN client as connected
                                                    if client_map
                                                        .connect_vpn_client(
                                                            self.gateway.network_id,
                                                            // Hostname is for logging only.
                                                            &self
                                                                .gateway
                                                                .hostname
                                                                .as_ref()
                                                                .cloned()
                                                                .unwrap_or_default(),
                                                            &public_key,
                                                            &device,
                                                            &user,
                                                            socket_addr,
                                                            &stats,
                                                        )
                                                        .is_err()
                                                    {
                                                        // TODO: log message
                                                        continue;
                                                    }

                                                    // emit connection event
                                                    let context = GrpcRequestContext::new(
                                                        user.id,
                                                        user.username.clone(),
                                                        socket_addr.ip(),
                                                        device.id,
                                                        device.name.clone(),
                                                        location.clone(),
                                                    );
                                                    self.emit_event(GrpcEvent::ClientConnected {
                                                        context,
                                                        location: location.clone(),
                                                        device: device.clone(),
                                                    });
                                                }
                                            }
                                        }

                                        // disconnect inactive clients
                                        let Ok(clients) = client_map
                                            .disconnect_inactive_vpn_clients_for_location(
                                                &location,
                                            )
                                        else {
                                            // TODO: log message
                                            continue;
                                        };
                                        clients
                                    };

                                    // emit client disconnect events
                                    for (device, context) in disconnected_clients {
                                        self.emit_event(GrpcEvent::ClientDisconnected {
                                            context,
                                            location: location.clone(),
                                            device,
                                        });
                                    }
                                }

                                // Save stats to database.
                                let stats = match stats.save(&self.pool).await {
                                    Ok(stats) => stats,
                                    Err(err) => {
                                        error!(
                                            "Saving WireGuard peer stats to database failed: {err}"
                                        );
                                        continue;
                                    }
                                };
                                info!("Saved WireGuard peer stats to database.");
                                debug!("WireGuard peer stats: {stats:?}");
                            }
                            None => (),
                        };
                    }
                    Err(err) => {
                        error!("Disconnected from Gateway at {uri}, error: {err}");
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
