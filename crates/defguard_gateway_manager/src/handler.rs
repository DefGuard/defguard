#[cfg(test)]
use std::path::PathBuf;
use std::{
    collections::HashMap,
    str::FromStr,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::Duration,
};

use chrono::DateTime;
use defguard_common::{
    VERSION,
    db::{
        Id,
        models::{Certificates, Settings, WireguardNetwork, gateway::Gateway},
    },
    messages::peer_stats_update::PeerStatsUpdate,
};
use defguard_core::{
    enterprise::firewall::try_get_location_firewall_config,
    events::GatewayConnectionEvent,
    handlers::mail::{send_gateway_disconnected_email, send_gateway_reconnected_email},
    location_management::allowed_peers::get_location_allowed_peers,
};
use defguard_grpc_tls::{certs as tls_certs, connector::HttpsSchemeConnector};
use defguard_proto::gateway::{
    Configuration, CoreResponse, PeerStats, core_request, core_response, gateway_client,
};
use defguard_version::client::ClientVersionInterceptor;
use hyper_rustls::HttpsConnectorBuilder;
use reqwest::Url;
use semver::Version;
use sqlx::PgPool;
use tokio::{
    sync::{
        mpsc::{self, UnboundedSender},
        watch,
    },
    task::JoinHandle,
    time::sleep,
};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tonic::transport::{Channel, Endpoint};

#[cfg(test)]
use crate::GatewayManagerTestSupport;
use crate::{
    Client, GatewayEventRouter, TEN_SECS, error::GatewayError, updates::GatewayUpdatesHandler,
};

/// One instance per connected Gateway.
pub(crate) struct GatewayHandler {
    // Gateway server endpoint URL.
    url: Url,
    gateway: Gateway<Id>,
    message_id: AtomicU64,
    pool: PgPool,
    event_router: GatewayEventRouter,
    connection_events_tx: UnboundedSender<GatewayConnectionEvent>,
    peer_stats_tx: UnboundedSender<PeerStatsUpdate>,
    certs_rx: watch::Receiver<Arc<HashMap<Id, String>>>,
    updates_handler_handle: Option<JoinHandle<()>>,
    /// Disconnect email notification waiting out the inactivity threshold. Aborted when the
    /// Gateway reconnects inside the window, so short outages produce no email at all.
    pending_disconnect_notification: Option<JoinHandle<()>>,
    /// Set by the pending task just before it sends the disconnect email. Guarantees a
    /// reconnect email is sent if and only if a disconnect email went out for that outage.
    disconnect_notification_sent: Arc<AtomicBool>,
    #[cfg(test)]
    test_socket_path: Option<PathBuf>,
    #[cfg(test)]
    test_support: Option<GatewayManagerTestSupport>,
}

impl GatewayHandler {
    pub fn new(
        gateway: Gateway<Id>,
        pool: PgPool,
        event_router: GatewayEventRouter,
        connection_events_tx: UnboundedSender<GatewayConnectionEvent>,
        peer_stats_tx: UnboundedSender<PeerStatsUpdate>,
        certs_rx: watch::Receiver<Arc<HashMap<Id, String>>>,
        disconnect_notification_sent: Arc<AtomicBool>,
    ) -> Result<Self, GatewayError> {
        let url = Url::from_str(&gateway.url()).map_err(|err| {
            GatewayError::EndpointError(format!(
                "Failed to parse Gateway URL {}: {err}",
                gateway.url()
            ))
        })?;

        Ok(Self {
            url,
            gateway,
            message_id: AtomicU64::new(0),
            pool,
            event_router,
            connection_events_tx,
            peer_stats_tx,
            certs_rx,
            updates_handler_handle: None,
            pending_disconnect_notification: None,
            disconnect_notification_sent,
            #[cfg(test)]
            test_socket_path: None,
            #[cfg(test)]
            test_support: None,
        })
    }

    #[cfg(not(test))]
    fn handler_retry_delay(&self) -> Duration {
        TEN_SECS
    }

    #[cfg(not(test))]
    fn disconnect_notification_delay(&self, configured_delay: Duration) -> Duration {
        configured_delay
    }

    #[cfg(not(test))]
    async fn connect_channel(&self, endpoint: &Endpoint) -> Result<Channel, GatewayError> {
        self.connect_tls_channel(endpoint).await
    }

    async fn connect_tls_channel(&self, endpoint: &Endpoint) -> Result<Channel, GatewayError> {
        let certs = Certificates::get_or_default(&self.pool)
            .await
            .map_err(|err| {
                GatewayError::EndpointError(format!("Failed to load certificates from DB: {err}"))
            })?;
        let Some(ca_cert_der) = certs.ca_cert_der else {
            return Err(GatewayError::EndpointError(
                "Core CA is not setup, can't create a Gateway endpoint".to_owned(),
            ));
        };
        let Some(core_client_cert_der) = self.gateway.core_client_cert_der.as_deref() else {
            return Err(GatewayError::EndpointError(format!(
                "Core client certificate not provisioned for gateway id={}",
                self.gateway.id
            )));
        };
        let Some(core_client_cert_key_der) = self.gateway.core_client_cert_key_der.as_deref()
        else {
            return Err(GatewayError::EndpointError(format!(
                "Core client certificate key not provisioned for gateway id={}",
                self.gateway.id
            )));
        };
        let tls_config = tls_certs::client_config(
            &ca_cert_der,
            self.certs_rx.clone(),
            self.gateway.id,
            core_client_cert_der,
            core_client_cert_key_der,
        )
        .map_err(|err| GatewayError::EndpointError(err.to_string()))?;
        let connector = HttpsConnectorBuilder::new()
            .with_tls_config(tls_config)
            .https_only()
            .enable_http2()
            .build();
        let connector = HttpsSchemeConnector::new(connector);

        Ok(endpoint.connect_with_connector_lazy(connector))
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
        let network_id = self.gateway.location_id;

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

        let peers = get_location_allowed_peers(&network, &mut conn).await?;

        let maybe_firewall_config = try_get_location_firewall_config(&network, &mut conn).await?;
        let payload = Some(core_response::Payload::Config(Configuration::new(
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

    /// Schedule a Gateway disconnected email notification.
    ///
    /// The email is delayed by the configured inactivity threshold instead of being sent
    /// straight away. A reconnect inside that window aborts the pending task, so a short
    /// outage produces no notification at all.
    fn schedule_disconnect_notification(&mut self) {
        let settings = Settings::get_current_settings();
        if !settings.gateway_disconnect_notifications_enabled {
            return;
        }

        if let Some(handle) = self.pending_disconnect_notification.take() {
            warn!(
                "Found a disconnect email notification already pending for {} while scheduling a \
                new one; aborting the old one",
                self.gateway
            );
            handle.abort();
        }

        // A threshold of 0 keeps the notification immediate, which is what the settings form
        // allows as its minimum value.
        let threshold = settings.gateway_disconnect_notifications_inactivity_threshold;
        let threshold_minutes = if let Ok(minutes) = u64::try_from(threshold) {
            minutes
        } else {
            warn!(
                "Gateway disconnect notifications inactivity threshold {threshold} is \
                negative; treating it as 0 (immediate)"
            );
            0
        };
        let delay = self.disconnect_notification_delay(Duration::from_secs(60 * threshold_minutes));

        let gateway_id = self.gateway.id;
        let location_id = self.gateway.location_id;
        let url = format!("{}:{}", self.gateway.address, self.gateway.port);
        let pool = self.pool.clone();
        let notification_sent = Arc::clone(&self.disconnect_notification_sent);
        #[cfg(test)]
        let test_support = self.test_support.clone();

        debug!(
            "Scheduling Gateway disconnect email notification for {} in {delay:?}",
            self.gateway
        );
        let handle = tokio::spawn(async move {
            sleep(delay).await;

            // Re-read the current state, since the Gateway may have been removed from the
            // database or reconnected without this task being aborted in time.
            let gateway = match Gateway::find_by_id(&pool, gateway_id).await {
                Ok(Some(gateway)) => gateway,
                Ok(None) => {
                    info!(
                        "Gateway id={gateway_id} is no longer in the database; disconnect email \
                        notification not sent"
                    );
                    return;
                }
                Err(err) => {
                    error!(
                        "Failed to fetch Gateway id={gateway_id} from database, disconnect email \
                        notification not sent: {err}"
                    );
                    return;
                }
            };
            if gateway.is_connected() {
                info!(
                    "{gateway} reconnected within the inactivity threshold; disconnect email \
                    notification not sent"
                );
                return;
            }

            let Ok(Some(network)) = WireguardNetwork::find_by_id(&pool, location_id).await else {
                error!("Failed to fetch network ID {location_id} from database");
                return;
            };

            // Record the notification as sent before awaiting the send, so an abort landing
            // mid-send cannot leave a later reconnect notification unpaired.
            notification_sent.store(true, Ordering::SeqCst);
            #[cfg(test)]
            note_disconnect_notification_for_tests(test_support.as_ref(), gateway_id);

            debug!("Sending Gateway disconnect email notification");
            // TODO: return result instead of logging.
            if let Err(err) =
                send_gateway_disconnected_email(gateway.name, network.name, &url, &pool).await
            {
                error!("Failed to send Gateway disconnect notification: {err}");
            } else {
                info!("Sent email notification about Gateway being disconnected");
            }
        });
        self.pending_disconnect_notification = Some(handle);
    }

    /// Returns true if a disconnect email went out for this outage, and atomically clears the
    /// flag so a later outage cannot inherit it.
    fn take_disconnect_notification_sent(&self) -> bool {
        self.disconnect_notification_sent
            .swap(false, Ordering::SeqCst)
    }

    /// Send a Gateway reconnected notification, but only when a matching disconnect notification
    /// actually went out for this outage.
    fn maybe_send_reconnect_notification(&self, network_name: String) {
        // Consume the flag even when reconnect notifications are turned off, so that a later
        // outage cannot inherit it.
        if !self.take_disconnect_notification_sent() {
            return;
        }

        let settings = Settings::get_current_settings();
        if !settings.gateway_disconnect_notifications_reconnect_notification_enabled {
            return;
        }

        debug!("Sending Gateway reconnect email notification");
        #[cfg(test)]
        self.note_reconnect_notification_for_tests();
        let gateway_id = self.gateway.id;
        let fallback_name = self.gateway.name.clone();
        let pool = self.pool.clone();
        let url = format!("{}:{}", self.gateway.address, self.gateway.port);

        tokio::spawn(async move {
            let gateway_name = match Gateway::find_by_id(&pool, gateway_id).await {
                Ok(Some(gateway)) => gateway.name,
                _ => fallback_name,
            };
            if let Err(err) =
                send_gateway_reconnected_email(gateway_name, network_name, &url, &pool).await
            {
                error!("Failed to send Gateway reconnect notification: {err}");
            } else {
                info!("Sent email notification about Gateway being reconnected");
            }
        });
    }

    async fn mark_disconnected(&mut self) -> bool {
        if let Err(err) = self.gateway.touch_disconnected(&self.pool).await {
            error!(
                "Failed to update disconnection time for {} in the database: {err}",
                self.gateway
            );
            return false;
        }
        true
    }

    async fn handle_disconnection_error(&mut self) {
        if !self.gateway.is_connected() {
            return;
        }

        // Mark the Gateway disconnected before scheduling the notification: the delayed task
        // re-reads the row when it fires and must not see a stale connected state. If the DB
        // write fails, skip the notification entirely rather than scheduling a task that would
        // re-read the still-connected row and never fire.
        if self.mark_disconnected().await {
            let _ = self
                .connection_events_tx
                .send(GatewayConnectionEvent::Disconnected {
                    gateway_id: self.gateway.id,
                    gateway_name: self.gateway.name.clone(),
                });

            self.schedule_disconnect_notification();
        }
    }

    async fn mark_connected_and_maybe_notify(&mut self, network_name: &str) {
        // A Gateway that came back should not be reported as down at all. Cancel any pending
        // disconnect email before touching the database, so a DB error cannot leave the task
        // alive to send a false "disconnected" alert for a gateway that is actually back.
        if let Some(handle) = self.pending_disconnect_notification.take() {
            debug!(
                "Cancelling pending disconnect email notification for {}",
                self.gateway
            );
            handle.abort();
        }

        let was_connected = self.gateway.is_connected();
        if let Err(err) = self.gateway.touch_connected(&self.pool).await {
            error!(
                "Failed to update connection time for {} in the database: {err}",
                self.gateway
            );
            return;
        }

        if !was_connected {
            let _ = self
                .connection_events_tx
                .send(GatewayConnectionEvent::Connected {
                    gateway_id: self.gateway.id,
                    gateway_name: self.gateway.name.clone(),
                });
        }

        self.maybe_send_reconnect_notification(network_name.to_owned());
    }

    fn remove_client(&self, clients: &Arc<Mutex<HashMap<Id, Client>>>) {
        clients
            .lock()
            .expect("GatewayHandler failed to lock clients")
            .remove(&self.gateway.id);
    }

    async fn handle_stream_disconnection(
        &mut self,
        clients: &Arc<Mutex<HashMap<Id, Client>>>,
        retry_on_connect_failure: bool,
        retry_delay: Duration,
    ) {
        self.remove_client(clients);
        self.handle_disconnection_error().await;

        if !retry_on_connect_failure {
            return;
        }

        debug!("Waiting {retry_delay:?} to re-establish the connection");
        sleep(retry_delay).await;
    }

    async fn handle_connection_iteration(
        &mut self,
        clients: Arc<Mutex<HashMap<Id, Client>>>,
        retry_on_connect_failure: bool,
    ) -> Result<(), GatewayError> {
        let endpoint = self.endpoint()?;
        let uri = endpoint.uri().to_string();

        let channel = self.connect_channel(&endpoint).await?;

        debug!("Connecting to Gateway {uri}");
        let interceptor = ClientVersionInterceptor::new(
            Version::parse(VERSION).expect("failed to parse self version"),
        );
        let mut client = gateway_client::GatewayClient::with_interceptor(channel, interceptor);

        #[cfg(test)]
        self.note_handler_connection_attempt_for_tests();

        let (tx, rx) = mpsc::unbounded_channel();
        let retry_delay = self.handler_retry_delay();
        let response = match client.bidi(UnboundedReceiverStream::new(rx)).await {
            Ok(response) => response,
            Err(err) => {
                error!("Failed to connect to Gateway {uri}, retrying: {err}");
                if retry_on_connect_failure {
                    sleep(retry_delay).await;
                    return Ok(());
                }

                return Err(err.into());
            }
        };
        let maybe_info = defguard_version::ComponentInfo::from_metadata(response.metadata());
        let (version, _info) = defguard_version::get_tracing_variables(&maybe_info);

        if let Some(mut gateway) = Gateway::find_by_id(&self.pool, self.gateway.id).await? {
            gateway.version = Some(version.to_string());
            gateway.save(&self.pool).await?;
        }

        clients
            .lock()
            .expect("GatewayHandler failed to lock clients")
            .insert(self.gateway.id, client.clone());
        info!("Connected to Defguard Gateway {uri}");

        let mut resp_stream = response.into_inner();
        let mut config_sent = false;

        loop {
            match resp_stream.message().await {
                Ok(None) => {
                    info!("Stream was closed by the sender.");
                    self.handle_stream_disconnection(
                        &clients,
                        retry_on_connect_failure,
                        retry_delay,
                    )
                    .await;
                    return Ok(());
                }
                Ok(Some(received)) => {
                    info!("Received message from Gateway.");
                    debug!("Message from Gateway {uri}");

                    match received.payload {
                        Some(core_request::Payload::ConfigRequest(())) => {
                            if config_sent {
                                warn!(
                                    "Ignoring repeated configuration request from {}",
                                    self.gateway
                                );
                                continue;
                            }

                            match self.send_configuration(&tx).await {
                                Ok(network) => {
                                    info!("Sent configuration to {}", self.gateway);
                                    config_sent = true;
                                    self.mark_connected_and_maybe_notify(&network.name).await;
                                    let (events_tx, events_rx) = mpsc::unbounded_channel();
                                    let mut updates_handler = GatewayUpdatesHandler::new(
                                        self.gateway.location_id,
                                        network,
                                        self.gateway.name.clone(),
                                        Some(self.pool.clone()),
                                        events_rx,
                                        tx.clone(),
                                    );
                                    let handle = tokio::spawn(async move {
                                        updates_handler.run().await;
                                    });
                                    self.event_router.register(self.gateway.id, events_tx);
                                    self.updates_handler_handle = Some(handle);
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

                            match try_protos_into_stats_message(
                                peer_stats.clone(),
                                self.gateway.location_id,
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
                    self.handle_stream_disconnection(
                        &clients,
                        retry_on_connect_failure,
                        retry_delay,
                    )
                    .await;
                    return Ok(());
                }
            }
        }
    }

    /// Connect to Gateway and handle its messages through gRPC.
    pub(super) async fn handle_connection(
        &mut self,
        clients: Arc<Mutex<HashMap<Id, Client>>>,
        reconnect_delay: Duration,
    ) -> Result<(), GatewayError> {
        loop {
            if let Err(err) = self
                .handle_connection_iteration(Arc::clone(&clients), true)
                .await
            {
                error!("Gateway connection error: {err}, retrying in {reconnect_delay:?}");
                sleep(reconnect_delay).await;
            }
        }
    }
}

impl Drop for GatewayHandler {
    fn drop(&mut self) {
        if let Some(handle) = self.updates_handler_handle.take() {
            handle.abort();
        }
        if let Some(handle) = self.pending_disconnect_notification.take() {
            handle.abort();
        }
    }
}

/// Records that a disconnect email notification was sent for the given Gateway.
/// A free function because the pending notification task only owns a clone of the test support,
/// not the handler itself.
#[cfg(test)]
fn note_disconnect_notification_for_tests(
    test_support: Option<&GatewayManagerTestSupport>,
    gateway_id: Id,
) {
    if let Some(test_support) = test_support {
        test_support.note_disconnect_notification_sent(gateway_id);
    }
}

#[cfg(test)]
impl GatewayHandler {
    // Bundles the socket path and reconnect-pairing flag on top of the already-long
    // `GatewayHandler::new` parameter list; splitting them out is not worth the churn.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new_with_test_socket(
        gateway: Gateway<Id>,
        pool: PgPool,
        event_router: GatewayEventRouter,
        connection_events_tx: UnboundedSender<GatewayConnectionEvent>,
        peer_stats_tx: UnboundedSender<PeerStatsUpdate>,
        certs_rx: watch::Receiver<Arc<HashMap<Id, String>>>,
        socket_path: PathBuf,
        disconnect_notification_sent: Arc<AtomicBool>,
    ) -> Result<Self, GatewayError> {
        let mut handler = Self::new(
            gateway,
            pool,
            event_router,
            connection_events_tx,
            peer_stats_tx,
            certs_rx,
            disconnect_notification_sent,
        )?;
        handler.test_socket_path = Some(socket_path);
        Ok(handler)
    }

    pub(crate) fn attach_test_support(&mut self, test_support: GatewayManagerTestSupport) {
        self.test_support = Some(test_support);
    }

    fn note_handler_connection_attempt_for_tests(&self) {
        if let Some(test_support) = &self.test_support {
            test_support.note_handler_connection_attempt(self.gateway.id);
        }
    }

    fn note_reconnect_notification_for_tests(&self) {
        if let Some(test_support) = &self.test_support {
            test_support.note_reconnect_notification_sent(self.gateway.id);
        }
    }

    fn handler_retry_delay(&self) -> Duration {
        self.test_support
            .as_ref()
            .map_or(TEN_SECS, GatewayManagerTestSupport::handler_reconnect_delay)
    }

    fn disconnect_notification_delay(&self, configured_delay: Duration) -> Duration {
        self.test_support
            .as_ref()
            .map_or(configured_delay, |test_support| {
                test_support.disconnect_notification_delay(configured_delay)
            })
    }

    async fn connect_channel(&self, endpoint: &Endpoint) -> Result<Channel, GatewayError> {
        if let Some(socket_path) = self.test_socket_path.clone() {
            Ok(endpoint.connect_with_connector_lazy(tower::service_fn(
                move |_: tonic::transport::Uri| {
                    let socket_path = socket_path.clone();
                    async move {
                        Ok::<_, std::io::Error>(hyper_util::rt::TokioIo::new(
                            tokio::net::UnixStream::connect(socket_path).await?,
                        ))
                    }
                },
            )))
        } else {
            self.connect_tls_channel(endpoint).await
        }
    }

    pub(crate) async fn handle_connection_once(&mut self) -> anyhow::Result<()> {
        let clients = Arc::<Mutex<HashMap<Id, Client>>>::default();
        self.handle_connection_iteration(clients, false)
            .await
            .map_err(anyhow::Error::from)
    }
}

/// Helper used to convert peer stats coming from gRPC client
/// into an internal representation
fn try_protos_into_stats_message(
    proto_stats: PeerStats,
    location_id: Id,
    gateway_id: Id,
) -> Option<PeerStatsUpdate> {
    let endpoint = proto_stats.endpoint.parse().ok()?;

    let latest_handshake = proto_stats
        .latest_handshake
        .and_then(|ts| DateTime::from_timestamp(ts.seconds, ts.nanos as u32))?
        .naive_utc();

    Some(PeerStatsUpdate::new(
        location_id,
        gateway_id,
        proto_stats.public_key,
        endpoint,
        proto_stats.upload,
        proto_stats.download,
        latest_handshake,
    ))
}

#[cfg(test)]
mod tests;
