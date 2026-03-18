use std::{
    collections::HashMap,
    net::IpAddr,
    path::PathBuf,
    str::FromStr,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};

use chrono::DateTime;
use defguard_common::{
    VERSION,
    db::{
        Id,
        models::{Settings, WireguardNetwork, gateway::Gateway, wireguard::DEFAULT_WIREGUARD_MTU},
    },
    messages::peer_stats_update::PeerStatsUpdate,
};
use defguard_core::{
    enterprise::firewall::try_get_location_firewall_config, grpc::GatewayEvent,
    handlers::mail::send_gateway_disconnected_email,
    location_management::allowed_peers::get_location_allowed_peers,
};
use defguard_grpc_tls::{certs as tls_certs, connector::HttpsSchemeConnector};
use defguard_proto::{
    enterprise::firewall::FirewallConfig,
    gateway::{
        Configuration, CoreResponse, Peer, PeerStats, Update, core_request, core_response,
        gateway_client, update,
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
    time::sleep,
};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tonic::{Code, Status, transport::Endpoint};

use crate::{Client, GatewayManagerTestSupport, TEN_SECS, error::GatewayError};

#[derive(Debug, Default)]
struct GatewayTestTransport {
    socket_path: Option<PathBuf>,
}

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
pub(super) struct GatewayHandler {
    // Gateway server endpoint URL.
    url: Url,
    gateway: Gateway<Id>,
    message_id: AtomicU64,
    pool: PgPool,
    events_tx: Sender<GatewayEvent>,
    peer_stats_tx: UnboundedSender<PeerStatsUpdate>,
    certs_rx: watch::Receiver<Arc<HashMap<Id, String>>>,
    test_transport: GatewayTestTransport,
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
            test_transport: GatewayTestTransport::default(),
            test_support: None,
        })
    }

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

    pub(super) fn attach_test_support(&mut self, test_support: GatewayManagerTestSupport) {
        self.test_support = Some(test_support);
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
        let payload = Some(core_response::Payload::Config(gen_config(
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
        let settings = Settings::get_current_settings();
        if !settings.gateway_disconnect_notifications_enabled {
            return;
        }

        debug!("Sending gateway disconnect email notification");
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
                    send_gateway_disconnected_email(name, network.name, &url, &pool).await
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
        retry_delay: std::time::Duration,
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

        let channel = if let Some(socket_path) = self.test_transport.socket_path().cloned() {
            endpoint.connect_with_connector_lazy(tower::service_fn(
                move |_: tonic::transport::Uri| {
                    let socket_path = socket_path.clone();
                    async move {
                        Ok::<_, std::io::Error>(hyper_util::rt::TokioIo::new(
                            tokio::net::UnixStream::connect(socket_path).await?,
                        ))
                    }
                },
            ))
        } else {
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

        debug!("Connecting to Gateway {uri}");
        let interceptor = ClientVersionInterceptor::new(
            Version::parse(VERSION).expect("failed to parse self version"),
        );
        let mut client = gateway_client::GatewayClient::with_interceptor(channel, interceptor);
        if let Some(test_support) = &self.test_support {
            test_support.note_handler_connection_attempt(self.gateway.id);
        }
        let (tx, rx) = mpsc::unbounded_channel();
        let retry_delay = self
            .test_support
            .as_ref()
            .map_or(TEN_SECS, GatewayManagerTestSupport::handler_reconnect_delay);
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
                        Some(core_request::Payload::ConfigRequest(_config_request)) => {
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
                                    let _ = self.gateway.touch_connected(&self.pool).await;
                                    let mut updates_handler = GatewayUpdatesHandler::new(
                                        self.gateway.location_id,
                                        network,
                                        self.gateway.name.clone(),
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
    ) -> Result<(), GatewayError> {
        loop {
            self.handle_connection_iteration(Arc::clone(&clients), true)
                .await?;
        }
    }
}

pub(crate) struct TestGatewayHandler {
    inner: GatewayHandler,
}

impl TestGatewayHandler {
    pub(crate) fn new(
        gateway: Gateway<Id>,
        pool: PgPool,
        events_tx: Sender<GatewayEvent>,
        peer_stats_tx: UnboundedSender<PeerStatsUpdate>,
        certs_rx: watch::Receiver<Arc<HashMap<Id, String>>>,
        socket_path: PathBuf,
    ) -> anyhow::Result<Self> {
        let inner = GatewayHandler::new_with_test_socket(
            gateway,
            pool,
            events_tx,
            peer_stats_tx,
            certs_rx,
            socket_path,
        )?;
        Ok(Self { inner })
    }

    pub(crate) async fn handle_connection_once(&mut self) -> anyhow::Result<()> {
        let clients = Arc::<Mutex<HashMap<Id, Client>>>::default();
        self.inner
            .handle_connection_iteration(clients, false)
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
                        self.send_network_update(&network, Vec::new(), None, 0)
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
                        let result =
                            self.send_network_update(&network, peers, maybe_firewall_config, 1);
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
                        Some(network_info) => {
                            // FIXME: this shouldn't happen, since when the device is created
                            // it's impossible for MFA authorization to already be completed
                            if self.network.mfa_enabled() && !network_info.is_authorized {
                                debug!(
                                    "Created WireGuard device {} is not authorized to connect to \
                                    MFA enabled location {}",
                                    device.device.name, self.network.name
                                );
                                continue;
                            }
                            self.send_peer_update(
                                Peer {
                                    pubkey: device.device.wireguard_pubkey,
                                    allowed_ips: network_info
                                        .device_wireguard_ips
                                        .iter()
                                        .map(IpAddr::to_string)
                                        .collect(),
                                    preshared_key: network_info.preshared_key.clone(),
                                    keepalive_interval: Some(
                                        self.network.keepalive_interval.cast_unsigned(),
                                    ),
                                },
                                0,
                            )
                        }
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
                        Some(network_info) => {
                            if self.network.mfa_enabled() && !network_info.is_authorized {
                                debug!(
                                    "Modified WireGuard device {} is not authorized to connect to \
                                    MFA enabled location {}",
                                    device.device.name, self.network.name
                                );
                                continue;
                            }
                            self.send_peer_update(
                                Peer {
                                    pubkey: device.device.wireguard_pubkey,
                                    allowed_ips: network_info
                                        .device_wireguard_ips
                                        .iter()
                                        .map(IpAddr::to_string)
                                        .collect(),
                                    preshared_key: network_info.preshared_key.clone(),
                                    keepalive_interval: Some(
                                        self.network.keepalive_interval.cast_unsigned(),
                                    ),
                                },
                                1,
                            )
                        }
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
                GatewayEvent::MfaSessionAuthorized(location_id, device, network_device) => {
                    if location_id == self.network_id {
                        // validate that network info is for the correct location
                        if network_device.wireguard_network_id != location_id {
                            error!(
                                "Received MFA authorization success event for location {location_id} with invalid device config: {network_device:?}"
                            );
                            continue;
                        }

                        // FIXME: at this point the device authorization should already have been verified
                        if self.network.mfa_enabled() && !network_device.is_authorized {
                            debug!(
                                "Created WireGuard device {} is not authorized to connect to \
                                    MFA enabled location {}",
                                device.name, self.network.name
                            );
                            continue;
                        }

                        self.send_peer_update(
                            Peer {
                                pubkey: device.wireguard_pubkey,
                                allowed_ips: network_device
                                    .wireguard_ips
                                    .iter()
                                    .map(IpAddr::to_string)
                                    .collect(),
                                preshared_key: network_device.preshared_key.clone(),
                                keepalive_interval: Some(
                                    self.network.keepalive_interval.cast_unsigned(),
                                ),
                            },
                            0,
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
                    prvkey: network.prvkey.clone(),
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
                if update_type == 0 { "CREATE" } else { "MODIFY" },
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
                update_type: 2,
                update: Some(update::Update::Network(Configuration {
                    name: network_name.to_string(),
                    prvkey: String::new(),
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
                if update_type == 0 { "CREATE" } else { "MODIFY" },
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
                update_type: 2,
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
                update_type: 1,
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
                update_type: 2,
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

    let latest_handshake = DateTime::from_timestamp(proto_stats.latest_handshake as i64, 0)
        .unwrap_or_default()
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

fn gen_config<I>(
    network: &WireguardNetwork<I>,
    peers: Vec<Peer>,
    maybe_firewall_config: Option<FirewallConfig>,
) -> Configuration {
    Configuration {
        name: network.name.clone(),
        port: network.port.cast_unsigned(),
        prvkey: network.prvkey.clone(),
        addresses: network.address().iter().map(ToString::to_string).collect(),
        peers,
        firewall_config: maybe_firewall_config,
        mtu: network.mtu.cast_unsigned(),
        fwmark: network.fwmark as u32,
    }
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};
    use defguard_common::db::models::wireguard::{LocationMfaMode, ServiceLocationMode};

    use super::{
        FirewallConfig, Peer, PeerStats, WireguardNetwork, gen_config,
        try_protos_into_stats_message,
    };

    fn build_peer_stats(endpoint: &str) -> PeerStats {
        PeerStats {
            public_key: "peer-public-key".to_string(),
            endpoint: endpoint.to_string(),
            upload: 123,
            download: 456,
            keepalive_interval: 25,
            latest_handshake: 1_700_000_000,
            allowed_ips: "10.10.0.2/32".to_string(),
        }
    }

    fn build_network() -> WireguardNetwork {
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
        .expect("valid network addresses");
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
    fn try_protos_into_stats_message_falls_back_to_default_timestamp() {
        let stats = try_protos_into_stats_message(
            PeerStats {
                latest_handshake: i64::MAX as u64,
                ..build_peer_stats("203.0.113.10:51820")
            },
            11,
            22,
        )
        .expect("valid endpoint should still produce stats");

        assert_eq!(
            stats.latest_handshake,
            DateTime::<Utc>::default().naive_utc()
        );
    }

    #[test]
    fn gen_config_maps_network_fields() {
        let config = gen_config(
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
        assert_eq!(config.prvkey, "network-private-key");
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
        let config = gen_config(&build_network(), Vec::new(), None);

        assert!(config.peers.is_empty());
        assert!(config.firewall_config.is_none());
    }
}
