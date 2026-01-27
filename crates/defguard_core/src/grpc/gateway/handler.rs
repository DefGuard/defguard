use std::{
    str::FromStr,
    sync::atomic::{AtomicU64, Ordering},
};

use chrono::{DateTime, TimeDelta, Utc};
use defguard_certs::der_to_pem;
use defguard_common::{
    VERSION,
    db::{
        Id, NoId,
        models::{
            Device, Settings, WireguardNetwork, gateway::Gateway,
            wireguard_peer_stats::WireguardPeerStats,
        },
    },
    messages::peer_stats_update::PeerStatsUpdate,
};
use defguard_mail::Mail;
use defguard_proto::gateway::{
    CoreResponse, PeerStats, core_request, core_response, gateway_client,
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
    grpc::{
        TEN_SECS,
        gateway::{GatewayError, events::GatewayEvent, get_peers, try_protos_into_stats_message},
    },
    handlers::mail::send_gateway_disconnected_email,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Scheme {
    #[allow(dead_code)]
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
    events_tx: Sender<GatewayEvent>,
    mail_tx: UnboundedSender<Mail>,
    peer_stats_tx: UnboundedSender<PeerStatsUpdate>,
}

impl GatewayHandler {
    pub(crate) fn new(
        gateway: Gateway<Id>,
        pool: PgPool,
        events_tx: Sender<GatewayEvent>,
        mail_tx: UnboundedSender<Mail>,
        peer_stats_tx: UnboundedSender<PeerStatsUpdate>,
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
            events_tx,
            mail_tx,
            peer_stats_tx,
        })
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
                                        authorized itself",
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

                                // Convert stats to database storage format.
                                // FIXME: remove once legacy table is removed
                                let stats = peer_stats_from_proto(
                                    peer_stats.clone(),
                                    self.gateway.network_id,
                                    device_id,
                                );

                                // convert stats to DB storage format
                                match try_protos_into_stats_message(
                                    peer_stats.clone(),
                                    self.gateway.network_id,
                                    self.gateway.id,
                                    device_id,
                                ) {
                                    None => {
                                        warn!(
                                            "Failed to parse peer stats update. Skipping sending message to session manager."
                                        )
                                    }
                                    Some(message) => {
                                        if let Err(err) = self.peer_stats_tx.send(message) {
                                            error!(
                                                "Failed to send peers stats update to session manager: {err}"
                                            );
                                            continue;
                                        };
                                    }
                                };

                                // Save stats to database.
                                // FIXME: remove once legacy table is removed
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
