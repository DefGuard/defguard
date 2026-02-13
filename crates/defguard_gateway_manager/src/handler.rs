use std::{
    collections::HashMap,
    str::FromStr,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

use defguard_common::{
    VERSION,
    db::{
        Id,
        models::{Settings, WireguardNetwork, gateway::Gateway},
    },
    messages::peer_stats_update::PeerStatsUpdate,
};
use defguard_grpc_tls::{certs as tls_certs, connector::HttpsSchemeConnector};
use defguard_proto::gateway::{CoreResponse, core_request, core_response, gateway_client};
use defguard_version::client::ClientVersionInterceptor;
use hyper_rustls::HttpsConnectorBuilder;
use reqwest::Url;
use semver::Version;
use sqlx::PgPool;
use tokio::{
    sync::{
        broadcast::Sender,
        mpsc::{self, UnboundedSender},
        watch,
    },
    time::sleep,
};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tonic::transport::Endpoint;

use defguard_core::{
    enterprise::firewall::try_get_location_firewall_config, grpc::GatewayEvent,
    handlers::mail::send_gateway_disconnected_email,
    location_management::allowed_peers::get_location_allowed_peers,
};

use crate::{GatewayError, TEN_SECS, try_protos_into_stats_message};

type ShutdownReceiver = tokio::sync::oneshot::Receiver<bool>;

/// One instance per connected Gateway.
pub(crate) struct GatewayHandler {
    // Gateway server endpoint URL.
    url: Url,
    gateway: Gateway<Id>,
    message_id: AtomicU64,
    pool: PgPool,
    events_tx: Sender<GatewayEvent>,
    peer_stats_tx: UnboundedSender<PeerStatsUpdate>,
    certs_rx: watch::Receiver<Arc<HashMap<Id, String>>>,
}

impl GatewayHandler {
    pub(crate) fn new(
        gateway: Gateway<Id>,
        pool: PgPool,
        events_tx: Sender<GatewayEvent>,
        peer_stats_tx: UnboundedSender<PeerStatsUpdate>,
        certs_rx: watch::Receiver<Arc<HashMap<Id, String>>>,
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
            peer_stats_tx,
            certs_rx,
        })
    }

    fn endpoint(&self) -> Result<Endpoint, GatewayError> {
        let mut url = self.url.clone();

        if let Err(()) = url.set_scheme("http") {
            return Err(GatewayError::EndpointError(format!(
                "Failed to set http scheme for Gateway URL {:?}",
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

        Ok(endpoint)
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

        let peers = get_location_allowed_peers(&network, &self.pool).await?;

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
                    send_gateway_disconnected_email(hostname, network.name, &url, &pool).await
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

    /// Connect to Gateway and handle its messages through gRPC.
    pub(crate) async fn handle_connection(&mut self) -> Result<(), GatewayError> {
        let endpoint = self.endpoint()?;
        let uri = endpoint.uri().to_string();
        loop {
            // TODO(jck) how does proxy do this? can this be moved to lib?
            #[cfg(not(test))]
            let channel = {
                let settings = Settings::get_current_settings();
                let Some(ca_cert_der) = settings.ca_cert_der else {
                    return Err(GatewayError::EndpointError(
                        "Core CA is not setup, can't create a Gateway endpoint.".to_string(),
                    ));
                };
                let tls_config =
                    tls_certs::client_config(&ca_cert_der, self.certs_rx.clone(), self.gateway.id)
                        .map_err(|err| GatewayError::EndpointError(err.to_string()))?;
                let connector = HttpsConnectorBuilder::new()
                    .with_tls_config(tls_config)
                    .https_only()
                    .enable_http2()
                    .build();
                let connector = HttpsSchemeConnector::new(connector);
                endpoint.connect_with_connector_lazy(connector)
            };
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

            let maybe_info = defguard_version::ComponentInfo::from_metadata(response.metadata());
            let (version, _info) = defguard_version::get_tracing_variables(&maybe_info);

            if let Some(mut gateway) = Gateway::find_by_id(&self.pool, self.gateway.id).await? {
                gateway.version = Some(version.to_string());
                gateway.save(&self.pool).await?;
            }

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

                                // convert stats to DB storage format
                                match try_protos_into_stats_message(
                                    peer_stats.clone(),
                                    self.gateway.network_id,
                                    self.gateway.id,
                                ) {
                                    None => {
                                        warn!(
                                            "Failed to parse peer stats update. Skipping sending \
                                            message to session manager."
                                        );
                                    }
                                    Some(message) => {
                                        if let Err(err) = self.peer_stats_tx.send(message) {
                                            error!(
                                                "Failed to send peers stats update to session manager: {err}"
                                            );
                                        }
                                    }
                                }
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
