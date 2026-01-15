use std::{
    net::SocketAddr,
    str::FromStr,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};

use chrono::{DateTime, TimeDelta, Utc};
use defguard_certs::{Csr, der_to_pem};
use defguard_common::{
    VERSION,
    db::{
        Id, NoId,
        models::{
            Device, Settings, User, WireguardNetwork, gateway::Gateway,
            wireguard_peer_stats::WireguardPeerStats,
        },
    },
};
use defguard_mail::Mail;
use defguard_proto::gateway::{
    CoreResponse, DerPayload, InitialSetupInfo, PeerStats, core_request, core_response,
    gateway_client, gateway_setup_client,
};
use defguard_version::client::ClientVersionInterceptor;
use reqwest::Url;
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
use tonic::transport::{Certificate, ClientTlsConfig, Endpoint};

use crate::{
    enterprise::firewall::try_get_location_firewall_config,
    events::GrpcRequestContext,
    grpc::{
        ClientMap, GrpcEvent, TEN_SECS,
        gateway::{GatewayError, GrpcRequestContext, events::GatewayEvent, get_peers},
    },
    handlers::mail::send_gateway_disconnected_email,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Scheme {
    Http,
    Https,
}

impl Scheme {
    #[must_use]
    pub const fn as_str(&self) -> &str {
        match self {
            Self::Http => "http",
            Self::Https => "https",
        }
    }
}

fn peer_stats_from_proto(stats: PeerStats, network_id: Id, device_id: Id) -> WireguardPeerStats {
    let endpoint = match stats.endpoint {
        endpoint if endpoint.is_empty() => None,
        _ => Some(stats.endpoint),
    };
    WireguardPeerStats {
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

/// One instance per connected Gateway.
pub(crate) struct GatewayHandler {
    // Gateway server endpoint URL.
    url: Url,
    gateway: Gateway<Id>,
    message_id: AtomicU64,
    pool: PgPool,
    client_state: Arc<Mutex<ClientMap>>,
    events_tx: Sender<GatewayEvent>,
    mail_tx: UnboundedSender<Mail>,
    grpc_event_tx: UnboundedSender<GrpcEvent>,
}

impl GatewayHandler {
    pub(crate) fn new(
        gateway: Gateway<Id>,
        pool: PgPool,
        client_state: Arc<Mutex<ClientMap>>,
        events_tx: Sender<GatewayEvent>,
        mail_tx: UnboundedSender<Mail>,
        grpc_event_tx: UnboundedSender<GrpcEvent>,
    ) -> Result<Self, GatewayError> {
        let url = Url::from_str(&gateway.url).map_err(|err| {
            GatewayError::EndpointError(format!(
                "Failed to parse Gateway URL {}: {}",
                &gateway.url, err
            ))
        })?;

        Ok(Self {
            url,
            gateway,
            message_id: AtomicU64::new(0),
            pool,
            client_state,
            events_tx,
            mail_tx,
            grpc_event_tx,
        })
    }

    pub const fn has_certificate(&self) -> bool {
        self.gateway.has_certificate
    }

    fn endpoint(&self, scheme: Scheme) -> Result<Endpoint, GatewayError> {
        let mut url = self.url.clone();

        if let Err(()) = url.set_scheme(scheme.as_str()) {
            return Err(GatewayError::EndpointError(format!(
                "Failed to set scheme {} for Gateway URL {:?}",
                scheme.as_str(),
                self.url
            )));
        }

        let endpoint = Endpoint::from_shared(url.to_string())
            .map_err(|err| {
                GatewayError::EndpointError(format!(
                    "Failed to create endpoint for Gateway URL {url:?}: {err}",
                ))
            })?
            .http2_keep_alive_interval(TEN_SECS)
            .tcp_keepalive(Some(TEN_SECS))
            .keep_alive_while_idle(true);

        if scheme == Scheme::Https {
            let settings = Settings::get_current_settings();
            let Some(ca_cert_der) = settings.ca_cert_der else {
                return Err(GatewayError::EndpointError(
                    "Core CA is not setup, can't create a Gateway endpoint.".to_string(),
                ));
            };

            let cert_pem = der_to_pem(&ca_cert_der, defguard_certs::PemLabel::Certificate)
                .map_err(|err| {
                    GatewayError::EndpointError(format!(
                        "Failed to convert CA certificate DER to PEM for Gateway URL {url:?}: {err}",
                    ))
                })?;
            let tls = ClientTlsConfig::new().ca_certificate(Certificate::from_pem(&cert_pem));

            Ok(endpoint.tls_config(tls).map_err(|err| {
                GatewayError::EndpointError(format!(
                    "Failed to set TLS config for Gateway URL {url:?}: {err}",
                ))
            })?)
        } else {
            Ok(endpoint)
        }
    }

    /// Send network and VPN configuration to Gateway.
    async fn send_configuration(
        &self,
        tx: &UnboundedSender<CoreResponse>,
    ) -> Result<WireguardNetwork<Id>, GatewayError> {
        debug!("Sending configuration to Gateway");
        let network_id = self.gateway.network_id;

        let mut conn = self.pool.acquire().await?;

        let mut network = WireguardNetwork::find_by_id(&mut *conn, network_id)
            .await?
            .ok_or_else(|| {
                GatewayError::NotFound(format!("Network with id {network_id} not found"))
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

        let peers = get_peers(&network, &self.pool).await?;

        let maybe_firewall_config = try_get_location_firewall_config(&network, &mut conn).await?;
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
                Err(GatewayError::MessageChannelError(format!(
                    "Configuration not sent to {}, error {err}",
                    self.gateway
                )))
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
        }
    }

    /// Helper method to fetch `Device` info from DB by pubkey and return appropriate errors
    async fn fetch_device_from_db(
        &self,
        public_key: &str,
    ) -> Result<Option<Device<Id>>, GatewayError> {
        let device = Device::find_by_pubkey(&self.pool, public_key).await?;
        Ok(device)
    }

    /// Helper method to fetch `WireguardNetwork` info from DB and return appropriate errors
    async fn fetch_location_from_db(
        &self,
        location_id: Id,
    ) -> Result<WireguardNetwork<Id>, GatewayError> {
        let location = match WireguardNetwork::find_by_id(&self.pool, location_id).await? {
            Some(location) => location,
            None => {
                error!("Location {location_id} not found");
                return Err(GatewayError::NotFound(format!(
                    "Location {location_id} not found"
                )));
            }
        };
        Ok(location)
    }

    /// Helper method to fetch `User` info from DB and return appropriate errors
    async fn fetch_user_from_db(
        &self,
        user_id: Id,
        public_key: &str,
    ) -> Result<User<Id>, GatewayError> {
        let user = match User::find_by_id(&self.pool, user_id).await? {
            Some(user) => user,
            None => {
                error!("User {user_id} assigned to device with public key {public_key} not found");
                return Err(GatewayError::NotFound(format!(
                    "User assigned to device with public key {public_key} not found"
                )));
            }
        };

        Ok(user)
    }

    fn emit_event(&self, event: GrpcEvent) {
        if self.grpc_event_tx.send(event).is_err() {
            warn!("Failed to send gRPC event");
        }
    }

    pub(crate) async fn handle_setup(&mut self) -> Result<(), GatewayError> {
        debug!("Handling initial setup for Gateway {}", self.gateway);
        let endpoint = self.endpoint(Scheme::Http)?;
        let uri = endpoint.uri().to_string();

        let hostname = self
            .url
            .host_str()
            .ok_or_else(|| {
                error!("Failed to get hostname from Gateway URL {}", self.url);
                GatewayError::EndpointError(format!(
                    "Failed to get hostname from Gateway URL {}",
                    self.url
                ))
            })?
            .to_string();

        #[cfg(not(test))]
        let channel = endpoint.connect_lazy();
        #[cfg(test)]
        let channel = endpoint.connect_with_connector_lazy(tower::service_fn(
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
        let mut client =
            gateway_setup_client::GatewaySetupClient::with_interceptor(channel, interceptor);

        let request = InitialSetupInfo {
            cert_hostname: hostname,
        };

        let response = client.start(request).await?;
        let response = response.into_inner();

        let csr = Csr::from_der(&response.der_data)?;

        let settings = Settings::get_current_settings();

        let ca_cert_der = settings.ca_cert_der.ok_or_else(|| {
            GatewayError::ConfigurationError(
                "CA certificate DER not found in settings for Gateway setup".to_string(),
            )
        })?;
        let ca_key_pair = settings.ca_key_der.ok_or_else(|| {
            GatewayError::ConfigurationError(
                "CA key pairs DER not found in settings for Gateway setup".to_string(),
            )
        })?;

        let ca = defguard_certs::CertificateAuthority::from_cert_der_key_pair(
            &ca_cert_der,
            &ca_key_pair,
        )?;

        match ca.sign_csr(&csr) {
            Ok(cert) => {
                let req = DerPayload {
                    der_data: cert.der().to_vec(),
                };

                client.send_cert(req).await?;

                let expiry = defguard_certs::get_certificate_expiry(&cert)?;

                self.gateway.has_certificate = true;
                self.gateway.certificate_expiry = Some(
                    chrono::DateTime::from_timestamp(expiry.unix_timestamp(), 0)
                        .ok_or_else(|| {
                            GatewayError::ConversionError(format!(
                                "Failed to convert certificate expiry timestamp {} to DateTime",
                                expiry.unix_timestamp()
                            ))
                        })?
                        .naive_utc(),
                );
                self.gateway.save(&self.pool).await?;
            }
            Err(err) => {
                error!("Failed to sign CSR: {err}");
            }
        }

        debug!(
            "Saving information about issued certificate to the database for Gateway {}",
            self.gateway
        );

        Ok(())
    }

    /// Connect to Gateway and handle its messages through gRPC.
    pub(crate) async fn handle_connection(&mut self) -> Result<(), GatewayError> {
        let endpoint = self.endpoint(Scheme::Https)?;
        let uri = endpoint.uri().to_string();
        loop {
            #[cfg(not(test))]
            let channel = endpoint.connect_lazy();
            #[cfg(test)]
            let channel = endpoint.connect_with_connector_lazy(tower::service_fn(
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

                                // Send network configuration to Gateway.
                                match self.send_configuration(&tx).await {
                                    Ok(network) => {
                                        info!("Sent configuration to {}", self.gateway);
                                        config_sent = true;
                                        let _ = self
                                            .gateway
                                            .touch_connected(&self.pool, config_request.hostname)
                                            .await;
                                        let mut updates_handler = super::GatewayUpdatesHandler::new(
                                            self.gateway.network_id,
                                            network,
                                            self.gateway
                                                .hostname
                                                .clone()
                                                .unwrap_or_default()
                                                .clone(),
                                            self.events_tx.subscribe(),
                                            tx.clone(),
                                        );
                                        tokio::spawn(async move {
                                            updates_handler.run().await;
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
                                let stats = peer_stats_from_proto(
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
                                                                .clone()
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
                        }
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
