use std::{
    str::FromStr,
    sync::atomic::{AtomicU64, Ordering},
};

use defguard_common::{auth::claims::Claims, db::Id};
use defguard_mail::Mail;
use defguard_proto::gateway::{CoreResponse, core_request, core_response, gateway_client};
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
    transport::{ClientTlsConfig, Endpoint},
};

use crate::{
    ClaimsType,
    db::{
        Device, GatewayEvent, WireguardNetwork,
        models::{gateway::Gateway, wireguard_peer_stats::WireguardPeerStats},
    },
    grpc::TEN_SECS,
    handlers::mail::send_gateway_disconnected_email,
};

/// One instance per connected Gateway.
pub(crate) struct GatewayHandler {
    endpoint: Endpoint,
    gateway: Gateway<Id>,
    message_id: AtomicU64,
    pool: PgPool,
    events_tx: Sender<GatewayEvent>,
    mail_tx: UnboundedSender<Mail>,
}

impl GatewayHandler {
    pub(crate) fn new(
        gateway: Gateway<Id>,
        tls_config: Option<ClientTlsConfig>,
        pool: PgPool,
        events_tx: Sender<GatewayEvent>,
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

    /// Send network and VPN configuration to Gateway.
    async fn send_configuration(&self, tx: &UnboundedSender<CoreResponse>) -> Result<(), Status> {
        debug!("Sending configuration to Gateway");
        let network_id = self.gateway.network_id;
        // let hostname = Self::get_gateway_hostname(request.metadata())?;

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

            info!("Connected to Defguard Gateway {uri}");
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
                                // tokio::spawn(super::handle_events(
                                //     network,
                                //     tx.clone(),
                                //     self.events_tx.subscribe(),
                                // ));
                            }
                            Some(core_request::Payload::PeerStats(peer_stats)) => {
                                if !config_sent {
                                    warn!(
                                        "Ignoring peer statistics from {} because it didn't \
                                        authorize itself",
                                        self.gateway
                                    );
                                    continue;
                                }

                                //     let public_key = peer_stats.public_key.clone();
                                //     let mut stats = WireguardPeerStats::from_peer_stats(
                                //         peer_stats,
                                //         self.gateway.network_id,

                                //     );
                                //     // Get device by public key and fill in stats.device_id
                                //     match Device::find_by_pubkey(&self.pool, &public_key).await {
                                //         Ok(Some(device)) => {
                                //             stats.device_id = device.id;
                                //             match stats.save(&self.pool).await {
                                //                 Ok(_) => {
                                //                     info!("Saved WireGuard peer stats to database.")
                                //                 }
                                //                 Err(err) => error!(
                                //                     "Failed to save WireGuard peer stats to database: \
                                //                     {err}"
                                //                 ),
                                //             }
                                //         }
                                //         Ok(None) => {
                                //             error!("Device with public key {public_key} not found");
                                //         }
                                //         Err(err) => {
                                //             error!(
                                //                 "Failed to retrieve device with public key \
                                //                 {public_key}: {err}",
                                //             );
                                //         }
                                //     };
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
