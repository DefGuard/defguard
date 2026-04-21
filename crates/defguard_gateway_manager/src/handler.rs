#[cfg(test)]
use std::path::PathBuf;
use std::{
    collections::HashMap,
    net::IpAddr,
    str::FromStr,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

use chrono::{DateTime, TimeDelta};
use defguard_common::{
    VERSION,
    db::{
        Id,
        models::{
            Certificates, DeviceNetworkInfo, Settings, WireguardNetwork, gateway::Gateway,
            wireguard::DEFAULT_WIREGUARD_MTU,
        },
    },
    messages::peer_stats_update::PeerStatsUpdate,
};
use defguard_core::{
    enterprise::firewall::try_get_location_firewall_config,
    grpc::GatewayEvent,
    handlers::mail::{send_gateway_disconnected_email, send_gateway_reconnected_email},
    location_management::allowed_peers::get_location_allowed_peers,
};
use defguard_grpc_tls::{certs as tls_certs, connector::HttpsSchemeConnector};
use defguard_proto::{
    enterprise::firewall::FirewallConfig,
    gateway::{
        Configuration, CoreResponse, Peer, PeerStats, Update, UpdateType, core_request,
        core_response, gateway_client, update,
    },
};
use defguard_version::client::ClientVersionInterceptor;
use hyper_rustls::HttpsConnectorBuilder;
use reqwest::Url;
use semver::Version;
use sqlx::PgPool;
use tokio::{
    sync::{
        broadcast::{self, Sender},
        mpsc::{self, UnboundedSender},
        watch,
    },
    task::JoinHandle,
    time::sleep,
};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tonic::{
    Code, Status,
    transport::{Channel, Endpoint},
};

#[cfg(test)]
use crate::GatewayManagerTestSupport;
use crate::{Client, TEN_SECS, error::GatewayError};

#[cfg(test)]
#[derive(Default)]
struct GatewayTestTransport {
    socket_path: Option<PathBuf>,
}

#[cfg(test)]
impl GatewayTestTransport {
    fn with_socket_path(socket_path: PathBuf) -> Self {
        Self {
            socket_path: Some(socket_path),
        }
    }

    fn socket_path(&self) -> Option<&PathBuf> {
        self.socket_path.as_ref()
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
    peer_stats_tx: UnboundedSender<PeerStatsUpdate>,
    certs_rx: watch::Receiver<Arc<HashMap<Id, String>>>,
    updates_handler_handle: Option<JoinHandle<()>>,
    #[cfg(test)]
    test_transport: GatewayTestTransport,
    #[cfg(test)]
    test_support: Option<GatewayManagerTestSupport>,
}

impl GatewayHandler {
    pub fn new(
        gateway: Gateway<Id>,
        pool: PgPool,
        events_tx: Sender<GatewayEvent>,
        peer_stats_tx: UnboundedSender<PeerStatsUpdate>,
        certs_rx: watch::Receiver<Arc<HashMap<Id, String>>>,
    ) -> Result<Self, GatewayError> {
        let url = Url::from_str(&gateway.url()).map_err(|err| {
            GatewayError::EndpointError(format!(
                "Failed to parse Gateway URL {}: {err}",
                &gateway.url()
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
            updates_handler_handle: None,
            #[cfg(test)]
            test_transport: GatewayTestTransport::default(),
            #[cfg(test)]
            test_support: None,
        })
    }

    #[cfg(not(test))]
    fn handler_retry_delay(&self) -> Duration {
        TEN_SECS
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
                "Core CA is not setup, can't create a Gateway endpoint".to_string(),
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

        let peers = get_location_allowed_peers(&network, &self.pool).await?;

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

    /// Send Gateway disconnected notification.
    /// Sends notification only if last notification time is bigger than specified in config.
    async fn send_disconnect_notification(&self) {
        let settings = Settings::get_current_settings();
        if !settings.gateway_disconnect_notifications_enabled {
            return;
        }

        // Send email only if disconnection time is before the connection time.
        if let (Some(connected_at), Some(disconnected_at)) =
            (self.gateway.connected_at, self.gateway.disconnected_at)
        {
            if disconnected_at > connected_at {
                info!("{} disconnected; email notification not sent", self.gateway);
                return;
            }
        }

        debug!("Sending Gateway disconnect email notification");
        let name = self.gateway.name.clone();
        let pool = self.pool.clone();
        let url = format!("{}:{}", self.gateway.address, self.gateway.port);

        let Ok(Some(network)) =
            WireguardNetwork::find_by_id(&self.pool, self.gateway.location_id).await
        else {
            error!(
                "Failed to fetch network ID {} from database",
                self.gateway.location_id
            );
            return;
        };

        // TODO: return result instead of logging.
        if let Err(err) = send_gateway_disconnected_email(name, network.name, &url, &pool).await {
            error!("Failed to send Gateway disconnect notification: {err}");
        } else {
            info!("Sent email notification about Gateway being disconnected");
        }
    }

    /// Send Gateway reconnected notification.
    fn send_reconnect_notification(&self, network_name: String) {
        let settings = Settings::get_current_settings();
        if !settings.gateway_disconnect_notifications_reconnect_notification_enabled {
            return;
        }

        let (Some(connected_at), Some(disconnected_at)) =
            (self.gateway.connected_at, self.gateway.disconnected_at)
        else {
            return;
        };
        let inactivity_threshold = TimeDelta::minutes(i64::from(
            settings.gateway_disconnect_notifications_inactivity_threshold,
        ));
        if connected_at - disconnected_at <= inactivity_threshold {
            return;
        }

        debug!("Sending Gateway reconnect email notification");
        let gateway_name = self.gateway.name.clone();
        let pool = self.pool.clone();
        let url = format!("{}:{}", self.gateway.address, self.gateway.port);

        tokio::spawn(async move {
            if let Err(err) =
                send_gateway_reconnected_email(gateway_name, network_name, &url, &pool).await
            {
                error!("Failed to send Gateway reconnect notification: {err}");
            } else {
                info!("Sent email notification about Gateway being reconnected");
            }
        });
    }

    async fn mark_disconnected(&mut self) {
        if let Err(err) = self.gateway.touch_disconnected(&self.pool).await {
            error!(
                "Failed to update disconnection time for {} in the database: {err}",
                self.gateway
            );
        }
    }

    async fn handle_disconnection_error(&mut self) {
        if self.gateway.is_connected() {
            self.send_disconnect_notification().await;
        }

        self.mark_disconnected().await;
    }

    async fn mark_connected_and_maybe_notify(&mut self, network_name: &str) {
        if let Err(err) = self.gateway.touch_connected(&self.pool).await {
            error!(
                "Failed to update connection time for {} in the database: {err}",
                self.gateway
            );
            return;
        }

        self.send_reconnect_notification(network_name.to_owned());
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
                                    let mut updates_handler = GatewayUpdatesHandler::new(
                                        self.gateway.location_id,
                                        network,
                                        self.gateway.name.clone(),
                                        self.events_tx.subscribe(),
                                        tx.clone(),
                                    );
                                    let handle = tokio::spawn(async move {
                                        updates_handler.run().await;
                                    });
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
    }
}

#[cfg(test)]
impl GatewayHandler {
    pub(crate) fn new_with_test_socket(
        gateway: Gateway<Id>,
        pool: PgPool,
        events_tx: Sender<GatewayEvent>,
        peer_stats_tx: UnboundedSender<PeerStatsUpdate>,
        certs_rx: watch::Receiver<Arc<HashMap<Id, String>>>,
        socket_path: PathBuf,
    ) -> Result<Self, GatewayError> {
        let mut handler = Self::new(gateway, pool, events_tx, peer_stats_tx, certs_rx)?;
        handler.test_transport = GatewayTestTransport::with_socket_path(socket_path);
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

    fn handler_retry_delay(&self) -> Duration {
        self.test_support
            .as_ref()
            .map_or(TEN_SECS, GatewayManagerTestSupport::handler_reconnect_delay)
    }

    async fn connect_channel(&self, endpoint: &Endpoint) -> Result<Channel, GatewayError> {
        if let Some(socket_path) = self.test_transport.socket_path().cloned() {
            return Ok(endpoint.connect_with_connector_lazy(tower::service_fn(
                move |_: tonic::transport::Uri| {
                    let socket_path = socket_path.clone();
                    async move {
                        Ok::<_, std::io::Error>(hyper_util::rt::TokioIo::new(
                            tokio::net::UnixStream::connect(socket_path).await?,
                        ))
                    }
                },
            )));
        }

        self.connect_tls_channel(endpoint).await
    }

    pub(crate) async fn handle_connection_once(&mut self) -> anyhow::Result<()> {
        let clients = Arc::<Mutex<HashMap<Id, Client>>>::default();
        self.handle_connection_iteration(clients, false)
            .await
            .map_err(anyhow::Error::from)
    }
}

/// Helper struct for handling gateway events.
struct GatewayUpdatesHandler {
    network_id: Id,
    network: WireguardNetwork<Id>,
    gateway_name: String,
    events_rx: broadcast::Receiver<GatewayEvent>,
    tx: UnboundedSender<CoreResponse>,
}

impl GatewayUpdatesHandler {
    #[must_use]
    fn new(
        network_id: Id,
        network: WireguardNetwork<Id>,
        gateway_name: String,
        events_rx: broadcast::Receiver<GatewayEvent>,
        tx: UnboundedSender<CoreResponse>,
    ) -> Self {
        Self {
            network_id,
            network,
            gateway_name,
            events_rx,
            tx,
        }
    }

    #[must_use]
    fn runtime_peer_update(
        &self,
        peer_label: &str,
        peer_pubkey: String,
        allowed_ips: Vec<String>,
        is_authorized: bool,
        preshared_key: Option<String>,
    ) -> Option<Peer> {
        if !self.network.mfa_enabled() {
            return Some(Peer {
                pubkey: peer_pubkey,
                allowed_ips,
                preshared_key: None,
                keepalive_interval: Some(self.network.keepalive_interval.cast_unsigned()),
            });
        }

        if !is_authorized {
            debug!(
                "Skipping gateway peer update for WireGuard device {peer_label} in MFA enabled location {} because there is no active MFA session",
                self.network.name
            );
            return None;
        }

        let Some(preshared_key) = preshared_key else {
            debug!(
                "Skipping gateway peer update for WireGuard device {peer_label} in location {} because the runtime preshared key is missing",
                self.network.name
            );
            return None;
        };

        Some(Peer {
            pubkey: peer_pubkey,
            allowed_ips,
            preshared_key: Some(preshared_key),
            keepalive_interval: Some(self.network.keepalive_interval.cast_unsigned()),
        })
    }

    fn send_runtime_device_update(
        &self,
        peer_label: &str,
        peer_pubkey: String,
        network_info: &DeviceNetworkInfo,
        update_type: i32,
    ) -> Result<(), Status> {
        let allowed_ips = network_info
            .device_wireguard_ips
            .iter()
            .map(IpAddr::to_string)
            .collect();

        let Some(peer) = self.runtime_peer_update(
            peer_label,
            peer_pubkey,
            allowed_ips,
            network_info.is_authorized,
            network_info.preshared_key.clone(),
        ) else {
            return Ok(());
        };

        self.send_peer_update(peer, update_type)
    }

    /// Process incoming Gateway events
    ///
    /// Main gRPC server uses a shared channel for broadcasting all gateway events
    /// so the handler must determine if an event is relevant for the network being serviced
    async fn run(&mut self) {
        info!(
            "Starting update stream to gateway: {}, network {}",
            self.gateway_name, self.network
        );
        while let Ok(update) = self.events_rx.recv().await {
            debug!("Received WireGuard update: {update:?}");
            let result = match update {
                GatewayEvent::NetworkCreated(network_id, network) => {
                    if network_id == self.network_id {
                        self.send_network_update(
                            &network,
                            Vec::new(),
                            None,
                            UpdateType::Create as i32,
                        )
                    } else {
                        Ok(())
                    }
                }
                GatewayEvent::NetworkModified(
                    network_id,
                    network,
                    peers,
                    maybe_firewall_config,
                ) => {
                    if network_id == self.network_id {
                        let result = self.send_network_update(
                            &network,
                            peers,
                            maybe_firewall_config,
                            UpdateType::Modify as i32,
                        );
                        // update stored network data
                        self.network = network;
                        result
                    } else {
                        Ok(())
                    }
                }
                GatewayEvent::NetworkDeleted(network_id, network_name) => {
                    if network_id == self.network_id {
                        self.send_network_delete(&network_name)
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
                        Some(network_info) => self.send_runtime_device_update(
                            &device.device.name,
                            device.device.wireguard_pubkey,
                            network_info,
                            UpdateType::Create as i32,
                        ),
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
                        Some(network_info) => self.send_runtime_device_update(
                            &device.device.name,
                            device.device.wireguard_pubkey,
                            network_info,
                            UpdateType::Modify as i32,
                        ),
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
                        Some(_) => self.send_peer_delete(&device.device.wireguard_pubkey),
                        None => Ok(()),
                    }
                }
                GatewayEvent::FirewallConfigChanged(location_id, firewall_config) => {
                    if location_id == self.network_id {
                        self.send_firewall_update(firewall_config)
                    } else {
                        Ok(())
                    }
                }
                GatewayEvent::FirewallDisabled(location_id) => {
                    if location_id == self.network_id {
                        self.send_firewall_disable()
                    } else {
                        Ok(())
                    }
                }
                GatewayEvent::MfaSessionDisconnected(location_id, device) => {
                    if location_id == self.network_id {
                        self.send_peer_delete(&device.wireguard_pubkey)
                    } else {
                        Ok(())
                    }
                }
                GatewayEvent::MfaSessionAuthorized(location_id, device, network_info) => {
                    if location_id == self.network_id {
                        if network_info.network_id != location_id {
                            error!(
                                "Received MFA authorization success event for location {location_id} with invalid runtime network info: {network_info:?}"
                            );
                            continue;
                        }

                        self.send_runtime_device_update(
                            &device.name,
                            device.wireguard_pubkey,
                            &network_info,
                            UpdateType::Create as i32,
                        )
                    } else {
                        Ok(())
                    }
                }
            };
            if result.is_err() {
                error!(
                    "Closing update steam to gateway: {}, network {}",
                    self.gateway_name, self.network
                );
                break;
            }
        }
    }

    /// Sends updated network configuration
    fn send_network_update(
        &self,
        network: &WireguardNetwork<Id>,
        peers: Vec<Peer>,
        firewall_config: Option<FirewallConfig>,
        update_type: i32,
    ) -> Result<(), Status> {
        debug!("Sending network update for network {network}");
        if let Err(err) = self.tx.send(CoreResponse {
            id: 0,
            payload: Some(core_response::Payload::Update(Update {
                update_type,
                update: Some(update::Update::Network(Configuration {
                    name: network.name.clone(),
                    private_key: network.prvkey.clone(),
                    addresses: network.address().iter().map(ToString::to_string).collect(),
                    port: network.port.cast_unsigned(),
                    peers,
                    firewall_config,
                    mtu: network.mtu.cast_unsigned(),
                    fwmark: network.fwmark as u32,
                })),
            })),
        }) {
            let msg = format!(
                "Failed to send network update, network {network}, update type: {update_type} \
                ({}), error: {err}",
                if update_type == UpdateType::Create as i32 {
                    "CREATE"
                } else {
                    "MODIFY"
                },
            );
            error!(msg);
            return Err(Status::new(Code::Internal, msg));
        }
        debug!("Network update sent for network {network}");
        Ok(())
    }

    /// Sends delete network command to gateway
    fn send_network_delete(&self, network_name: &str) -> Result<(), Status> {
        debug!(
            "Sending network delete command for network {}",
            self.network
        );
        if let Err(err) = self.tx.send(CoreResponse {
            id: 0,
            payload: Some(core_response::Payload::Update(Update {
                update_type: UpdateType::Delete as i32,
                update: Some(update::Update::Network(Configuration {
                    name: network_name.to_string(),
                    private_key: String::new(),
                    addresses: Vec::new(),
                    port: 0,
                    peers: Vec::new(),
                    firewall_config: None,
                    mtu: DEFAULT_WIREGUARD_MTU.cast_unsigned(),
                    fwmark: 0,
                })),
            })),
        }) {
            let msg = format!(
                "Failed to send network update, network {}, update type: 2 (DELETE), error: {err}",
                self.network,
            );
            error!(msg);
            return Err(Status::new(Code::Internal, msg));
        }
        debug!("Network delete command sent for network {}", self.network);
        Ok(())
    }

    /// Send update peer command to gateway
    fn send_peer_update(&self, peer: Peer, update_type: i32) -> Result<(), Status> {
        debug!("Sending peer update for network {}", self.network);
        if let Err(err) = self.tx.send(CoreResponse {
            id: 0,
            payload: Some(core_response::Payload::Update(Update {
                update_type,
                update: Some(update::Update::Peer(peer)),
            })),
        }) {
            let msg = format!(
                "Failed to send peer update for network {}, update type: {update_type} ({}), \
                error: {err}",
                self.network,
                if update_type == UpdateType::Create as i32 {
                    "CREATE"
                } else {
                    "MODIFY"
                },
            );
            error!(msg);
            return Err(Status::new(Code::Internal, msg));
        }
        debug!("Peer update sent for network {}", self.network);
        Ok(())
    }

    /// Send delete peer command to gateway
    fn send_peer_delete(&self, peer_pubkey: &str) -> Result<(), Status> {
        debug!("Sending peer delete for network {}", self.network);
        if let Err(err) = self.tx.send(CoreResponse {
            id: 0,
            payload: Some(core_response::Payload::Update(Update {
                update_type: UpdateType::Delete as i32,
                update: Some(update::Update::Peer(Peer {
                    pubkey: peer_pubkey.into(),
                    allowed_ips: Vec::new(),
                    preshared_key: None,
                    keepalive_interval: None,
                })),
            })),
        }) {
            let msg = format!(
                "Failed to send peer update for network {}, peer {peer_pubkey}, update type: 2 \
                (DELETE), error: {err}",
                self.network,
            );
            error!(msg);
            return Err(Status::new(Code::Internal, msg));
        }
        debug!("Peer delete command sent for network {}", self.network);
        Ok(())
    }

    /// Send firewall config update command to gateway
    fn send_firewall_update(&self, firewall_config: FirewallConfig) -> Result<(), Status> {
        debug!(
            "Sending firewall config update for network {} with config {firewall_config:?}",
            self.network
        );
        if let Err(err) = self.tx.send(CoreResponse {
            id: 0,
            payload: Some(core_response::Payload::Update(Update {
                update_type: UpdateType::Modify as i32,
                update: Some(update::Update::FirewallConfig(firewall_config)),
            })),
        }) {
            let msg = format!(
                "Failed to send firewall config update for network {}, error: {err}",
                self.network,
            );
            error!(msg);
            return Err(Status::new(Code::Internal, msg));
        }
        debug!("Firewall config update sent for network {}", self.network);
        Ok(())
    }

    /// Send firewall disable command to gateway
    fn send_firewall_disable(&self) -> Result<(), Status> {
        debug!(
            "Sending firewall disable command for network {}",
            self.network
        );
        if let Err(err) = self.tx.send(CoreResponse {
            id: 0,
            payload: Some(core_response::Payload::Update(Update {
                update_type: UpdateType::Delete as i32,
                update: Some(update::Update::DisableFirewall(())),
            })),
        }) {
            let msg = format!(
                "Failed to send firewall disable command for network {}, error: {err}",
                self.network,
            );
            error!(msg);
            return Err(Status::new(Code::Internal, msg));
        }
        debug!("Firewall disable command sent for network {}", self.network);
        Ok(())
    }
}

/// Helper used to convert peer stats coming from gRPC client
/// into an internal representation
fn try_protos_into_stats_message(
    proto_stats: PeerStats,
    location_id: Id,
    gateway_id: Id,
) -> Option<PeerStatsUpdate> {
    // try to parse endpoint
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
mod tests {
    use std::{collections::HashMap, net::IpAddr, str::FromStr, sync::Arc};

    use chrono::{DateTime, Utc};
    use defguard_common::db::{
        Id,
        models::{
            Device, DeviceType, User,
            device::WireguardNetworkDevice,
            gateway::Gateway,
            vpn_client_session::VpnClientSession,
            wireguard::{LocationMfaMode, ServiceLocationMode, WireguardNetwork},
        },
        setup_pool,
    };
    use defguard_core::grpc::GatewayEvent;
    use defguard_proto::gateway::{Configuration, Peer, PeerStats, core_response};
    use prost_types::Timestamp;
    use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
    use tokio::sync::{broadcast, mpsc::unbounded_channel, watch};

    use super::{
        FirewallConfig, GatewayHandler, GatewayUpdatesHandler, try_protos_into_stats_message,
    };

    fn test_network(location_mfa_mode: LocationMfaMode) -> WireguardNetwork<Id> {
        WireguardNetwork::new(
            "test-network".into(),
            51820,
            "127.0.0.1".into(),
            None,
            Vec::new(),
            true,
            false,
            false,
            location_mfa_mode,
            ServiceLocationMode::Disabled,
        )
        .with_id(1)
    }

    fn build_peer_stats(endpoint: &str) -> PeerStats {
        PeerStats {
            public_key: "peer-public-key".to_string(),
            endpoint: endpoint.to_string(),
            upload: 123,
            download: 456,
            keepalive_interval: 25,
            latest_handshake: Some(prost_types::Timestamp {
                seconds: 1_700_000_000,
                nanos: 0,
            }),
            allowed_ips: "10.10.0.2/32".to_string(),
        }
    }

    fn build_network() -> WireguardNetwork<Id> {
        let mut network = WireguardNetwork::new(
            "test-network".to_string(),
            51820,
            "198.51.100.10".to_string(),
            Some("1.1.1.1".to_string()),
            ["0.0.0.0/0".parse().expect("valid allowed IP network")],
            false,
            false,
            false,
            LocationMfaMode::default(),
            ServiceLocationMode::default(),
        )
        .set_address([
            "10.10.0.1/24".parse().expect("valid IPv4 network"),
            "fd00::1/64".parse().expect("valid IPv6 network"),
        ])
        .expect("valid network addresses")
        .with_id(1);
        network.pubkey = "network-public-key".to_string();
        network.prvkey = "network-private-key".to_string();
        network.mtu = 1420;
        network.fwmark = 4321;
        network.keepalive_interval = 25;
        network.peer_disconnect_threshold = 180;
        network
    }

    #[test]
    fn try_protos_into_stats_message_maps_valid_peer_stats() {
        let stats = try_protos_into_stats_message(build_peer_stats("203.0.113.10:51820"), 11, 22)
            .expect("valid peer stats should be converted");

        assert_eq!(stats.location_id, 11);
        assert_eq!(stats.gateway_id, 22);
        assert_eq!(stats.device_pubkey, "peer-public-key");
        assert_eq!(stats.endpoint.to_string(), "203.0.113.10:51820");
        assert_eq!(stats.upload, 123);
        assert_eq!(stats.download, 456);
        assert_eq!(
            stats.latest_handshake,
            DateTime::from_timestamp(1_700_000_000, 0)
                .expect("valid handshake timestamp")
                .naive_utc()
        );
    }

    #[test]
    fn try_protos_into_stats_message_rejects_invalid_endpoint() {
        let stats = try_protos_into_stats_message(build_peer_stats("not-a-socket-address"), 11, 22);

        assert!(stats.is_none());
    }

    #[test]
    fn try_protos_into_stats_message_returns_none_for_missing_handshake() {
        let stats = try_protos_into_stats_message(
            PeerStats {
                latest_handshake: None,
                ..build_peer_stats("203.0.113.10:51820")
            },
            11,
            22,
        );

        assert!(stats.is_none());
    }

    #[test]
    fn try_protos_into_stats_message_returns_none_for_invalid_timestamp() {
        let stats = try_protos_into_stats_message(
            PeerStats {
                latest_handshake: Some(Timestamp {
                    seconds: i64::MAX,
                    nanos: 0,
                }),
                ..build_peer_stats("203.0.113.10:51820")
            },
            11,
            22,
        );

        assert!(stats.is_none());
    }

    #[test]
    fn gen_config_maps_network_fields() {
        let config = Configuration::new(
            &build_network(),
            vec![Peer {
                pubkey: "peer-public-key".to_string(),
                allowed_ips: vec!["10.10.0.2/32".to_string()],
                preshared_key: Some("peer-preshared-key".to_string()),
                keepalive_interval: Some(25),
            }],
            Some(FirewallConfig {
                default_policy: 0,
                rules: Vec::new(),
                snat_bindings: Vec::new(),
            }),
        );

        assert_eq!(config.name, "test-network");
        assert_eq!(config.port, 51820);
        assert_eq!(config.private_key, "network-private-key");
        assert_eq!(config.addresses, vec!["10.10.0.1/24", "fd00::1/64"]);
        assert_eq!(config.mtu, 1420);
        assert_eq!(config.fwmark, 4321);

        let peer = config
            .peers
            .first()
            .expect("generated config should include peer");
        assert_eq!(peer.pubkey, "peer-public-key");
        assert_eq!(peer.allowed_ips, vec!["10.10.0.2/32"]);
        assert_eq!(peer.preshared_key.as_deref(), Some("peer-preshared-key"));
        assert_eq!(peer.keepalive_interval, Some(25));

        let firewall_config = config
            .firewall_config
            .expect("generated config should include firewall config");
        assert_eq!(firewall_config.default_policy, 0);
        assert!(firewall_config.rules.is_empty());
        assert!(firewall_config.snat_bindings.is_empty());
    }

    #[test]
    fn gen_config_preserves_absent_firewall_config_and_empty_peers() {
        let config = Configuration::new(&build_network(), Vec::new(), None);

        assert!(config.peers.is_empty());
        assert!(config.firewall_config.is_none());
    }

    fn test_handler(location_mfa_mode: LocationMfaMode) -> GatewayUpdatesHandler {
        let network = test_network(location_mfa_mode);
        let (events_tx, events_rx) = broadcast::channel(1);
        let (tx, _rx) = unbounded_channel();
        drop(events_tx);

        GatewayUpdatesHandler::new(network.id, network, "gateway".into(), events_rx, tx)
    }

    #[test]
    fn test_runtime_peer_update_strips_preshared_key_for_non_mfa_locations() {
        let handler = test_handler(LocationMfaMode::Disabled);

        let peer = handler
            .runtime_peer_update(
                "device",
                "device-pubkey".into(),
                vec!["10.1.1.2".into()],
                true,
                Some("legacy-psk".into()),
            )
            .unwrap();

        assert_eq!(peer.pubkey, "device-pubkey");
        assert_eq!(peer.allowed_ips, ["10.1.1.2"]);
        assert_eq!(peer.preshared_key, None);
        assert_eq!(peer.keepalive_interval, Some(25));
    }

    #[test]
    fn test_runtime_peer_update_skips_authorized_mfa_peer_without_session_preshared_key() {
        let handler = test_handler(LocationMfaMode::Internal);

        let peer = handler.runtime_peer_update(
            "device",
            "device-pubkey".into(),
            vec!["10.1.1.2".into()],
            true,
            None,
        );

        assert_eq!(peer, None);
    }

    #[test]
    fn test_runtime_peer_update_preserves_session_preshared_key_for_authorized_mfa_peer() {
        let handler = test_handler(LocationMfaMode::Internal);

        let peer = handler
            .runtime_peer_update(
                "device",
                "device-pubkey".into(),
                vec!["10.1.1.2".into()],
                true,
                Some("session-psk".into()),
            )
            .unwrap();

        assert_eq!(peer.preshared_key, Some("session-psk".into()));
    }

    #[sqlx::test]
    async fn test_send_configuration_includes_mfa_peers_with_session_preshared_key(
        _: PgPoolOptions,
        options: PgConnectOptions,
    ) {
        let pool = setup_pool(options).await;

        let user = User::new(
            "testuser",
            Some("password123"),
            "Test",
            "User",
            "test@example.com",
            None,
        )
        .save(&pool)
        .await
        .unwrap();

        let new_device = Device::new(
            "device-new".into(),
            "pubkey-new".into(),
            user.id,
            DeviceType::User,
            None,
            true,
        )
        .save(&pool)
        .await
        .unwrap();

        let connected_device = Device::new(
            "device-connected".into(),
            "pubkey-connected".into(),
            user.id,
            DeviceType::User,
            None,
            true,
        )
        .save(&pool)
        .await
        .unwrap();

        let mut network = WireguardNetwork::default()
            .try_set_address("10.7.1.1/24")
            .unwrap();
        network.name = "mfa-full-config-location".to_string();
        network.location_mfa_mode = LocationMfaMode::Internal;
        network.service_location_mode = ServiceLocationMode::Disabled;
        let network = network.save(&pool).await.unwrap();

        WireguardNetworkDevice::new(
            network.id,
            new_device.id,
            vec![IpAddr::from_str("10.7.1.2").unwrap()],
        )
        .insert(&pool)
        .await
        .unwrap();

        WireguardNetworkDevice::new(
            network.id,
            connected_device.id,
            vec![IpAddr::from_str("10.7.1.3").unwrap()],
        )
        .insert(&pool)
        .await
        .unwrap();

        let mut new_session = VpnClientSession::new(network.id, user.id, new_device.id, None, None);
        new_session.preshared_key = Some("new-session-psk".into());
        new_session.save(&pool).await.unwrap();

        let mut connected_session = VpnClientSession::new(
            network.id,
            user.id,
            connected_device.id,
            Some(Utc::now().naive_utc()),
            None,
        );
        connected_session.preshared_key = Some("connected-session-psk".into());
        connected_session.save(&pool).await.unwrap();

        let gateway = Gateway::new(network.id, "gateway", "127.0.0.1", 50051, "test")
            .save(&pool)
            .await
            .unwrap();
        let (events_tx, _events_rx) = broadcast::channel::<GatewayEvent>(1);
        let (peer_stats_tx, _peer_stats_rx) = unbounded_channel();
        let (_certs_tx, certs_rx) = watch::channel(Arc::new(HashMap::<Id, String>::new()));
        let handler =
            GatewayHandler::new(gateway, pool.clone(), events_tx, peer_stats_tx, certs_rx).unwrap();
        let (tx, mut rx) = unbounded_channel();

        handler.send_configuration(&tx).await.unwrap();

        let response = rx.recv().await.unwrap();
        let Some(core_response::Payload::Config(configuration)) = response.payload else {
            panic!("expected gateway config payload");
        };

        assert_eq!(configuration.peers.len(), 2);
        assert_eq!(
            configuration
                .peers
                .iter()
                .map(|peer| (peer.pubkey.as_str(), peer.preshared_key.as_deref()))
                .collect::<Vec<_>>(),
            [
                ("pubkey-new", Some("new-session-psk")),
                ("pubkey-connected", Some("connected-session-psk")),
            ]
        );
    }
}
