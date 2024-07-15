use std::{
    collections::hash_map::HashMap,
    fs::read_to_string,
    time::{Duration, Instant},
};
#[cfg(any(feature = "wireguard", feature = "worker"))]
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::{Arc, Mutex},
};

use chrono::{Duration as ChronoDuration, NaiveDateTime, Utc};
use serde::Serialize;
use thiserror::Error;
use tokio::{
    sync::{
        broadcast::Sender,
        mpsc::{self, UnboundedSender},
    },
    time::sleep,
};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tonic::{
    transport::{Certificate, ClientTlsConfig, Endpoint, Identity, Server, ServerTlsConfig},
    Status,
};
use uaparser::UserAgentParser;
use uuid::Uuid;

#[cfg(feature = "wireguard")]
use self::gateway::{gateway_service_server::GatewayServiceServer, GatewayServer};
use self::{
    auth::{auth_service_server::AuthServiceServer, AuthServer},
    desktop_client_mfa::ClientMfaServer,
    enrollment::EnrollmentServer,
    password_reset::PasswordResetServer,
    proto::core_response,
};
#[cfg(feature = "worker")]
use self::{
    interceptor::JwtInterceptor,
    worker::{worker_service_server::WorkerServiceServer, WorkerServer},
};
use crate::{
    auth::failed_login::FailedLoginMap, db::AppEvent,
    handlers::mail::send_gateway_disconnected_email, mail::Mail, server_config,
};
#[cfg(feature = "worker")]
use crate::{
    auth::ClaimsType,
    db::{DbPool, GatewayEvent},
};

mod auth;
mod desktop_client_mfa;
pub mod enrollment;
#[cfg(feature = "wireguard")]
pub(crate) mod gateway;
#[cfg(any(feature = "wireguard", feature = "worker"))]
mod interceptor;
pub mod password_reset;
#[cfg(feature = "worker")]
pub mod worker;

pub(crate) mod proto {
    tonic::include_proto!("defguard.proxy");
}

use proto::{core_request, proxy_client::ProxyClient, CoreError, CoreResponse};

// Helper struct used to handle gateway state
// gateways are grouped by network
type NetworkId = i64;
type GatewayHostname = String;
#[derive(Debug)]
pub struct GatewayMap(HashMap<NetworkId, HashMap<GatewayHostname, GatewayState>>);

#[derive(Error, Debug)]
pub enum GatewayMapError {
    #[error("Gateway {1} for network {0} not found")]
    NotFound(i64, GatewayHostname),
    #[error("Network {0} not found")]
    NetworkNotFound(i64),
    #[error("Gateway with UID {0} not found")]
    UidNotFound(Uuid),
    #[error("Cannot remove. Gateway with UID {0} is still active")]
    RemoveActive(Uuid),
    #[error("Config missing")]
    ConfigError,
}

impl GatewayMap {
    #[must_use]
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    // add a new gateway to map
    // this method is meant to be called when a gateway requests a config
    // as a sort of "registration"
    pub fn add_gateway(
        &mut self,
        network_id: i64,
        network_name: &str,
        hostname: String,
        name: Option<String>,
        mail_tx: UnboundedSender<Mail>,
    ) {
        info!("Adding gateway {hostname} with to gateway map for network {network_id}",);
        let gateway_state = GatewayState::new(network_id, network_name, &hostname, name, mail_tx);

        if let Some(network_gateway_map) = self.0.get_mut(&network_id) {
            network_gateway_map.entry(hostname).or_insert(gateway_state);
        } else {
            // no map for a given network exists yet
            let mut network_gateway_map = HashMap::new();
            network_gateway_map.insert(hostname, gateway_state);
            self.0.insert(network_id, network_gateway_map);
        }
    }

    // remove gateway from map
    pub fn remove_gateway(&mut self, network_id: i64, uid: Uuid) -> Result<(), GatewayMapError> {
        debug!("Removing gateway from network {network_id}");
        if let Some(network_gateway_map) = self.0.get_mut(&network_id) {
            // find gateway by uuid
            let hostname = match network_gateway_map
                .iter()
                .find(|(_address, state)| state.uid == uid)
            {
                None => {
                    error!("Failed to find gateway with UID {uid}");
                    return Err(GatewayMapError::UidNotFound(uid));
                }
                Some((hostname, state)) => {
                    if state.connected {
                        error!("Cannot remove. Gateway with UID {uid} is still active");
                        return Err(GatewayMapError::RemoveActive(uid));
                    }
                    hostname.clone()
                }
            };
            // remove matching gateway
            network_gateway_map.remove(&hostname)
        } else {
            // no map for a given network exists yet
            error!("Network {network_id} not found in gateway map");
            return Err(GatewayMapError::NetworkNotFound(network_id));
        };
        info!("Gateway with UID {uid} removed from network {network_id}");
        Ok(())
    }

    // change gateway status to connected
    // we assume that the gateway is already present in hashmap
    pub fn connect_gateway(
        &mut self,
        network_id: i64,
        hostname: &str,
    ) -> Result<(), GatewayMapError> {
        debug!("Connecting gateway {hostname} in network {network_id}");
        if let Some(network_gateway_map) = self.0.get_mut(&network_id) {
            if let Some(state) = network_gateway_map.get_mut(hostname) {
                state.connected = true;
                state.disconnected_at = None;
                state.connected_at = Some(Utc::now().naive_utc());
                debug!(
                    "Gateway {hostname} found in gateway map, current state: {:#?}",
                    state
                );
            } else {
                error!("Gateway {hostname} not found in gateway map for network {network_id}");
                return Err(GatewayMapError::NotFound(network_id, hostname.into()));
            }
        } else {
            // no map for a given network exists yet
            error!("Network {network_id} not found in gateway map");
            return Err(GatewayMapError::NetworkNotFound(network_id));
        };
        info!("Gateway {hostname} connected in network {network_id}");
        Ok(())
    }

    // change gateway status to disconnected
    pub fn disconnect_gateway(
        &mut self,
        network_id: i64,
        hostname: String,
        pool: &DbPool,
    ) -> Result<(), GatewayMapError> {
        debug!("Disconnecting gateway {hostname} in network {network_id}");
        if let Some(network_gateway_map) = self.0.get_mut(&network_id) {
            if let Some(state) = network_gateway_map.get_mut(&hostname) {
                state.connected = false;
                state.disconnected_at = Some(Utc::now().naive_utc());
                state.send_disconnect_notification(pool);
                debug!("Gateway {hostname} found in gateway map, current state: {state:#?}");
                info!("Gateway {hostname} disconnected in network {network_id}");
                return Ok(());
            };
        };
        let err = GatewayMapError::NotFound(network_id, hostname);
        error!("Gateway disconnect failed: {err}");
        Err(err)
    }

    // return `true` if at least one gateway in a given network is connected
    #[must_use]
    pub fn connected(&self, network_id: i64) -> bool {
        match self.0.get(&network_id) {
            Some(network_gateway_map) => network_gateway_map
                .values()
                .any(|gateway| gateway.connected),
            None => false,
        }
    }

    // return a list af aff statuses af all gateways in a given network
    #[must_use]
    pub fn get_network_gateway_status(&self, network_id: i64) -> Vec<GatewayState> {
        match self.0.get(&network_id) {
            Some(network_gateway_map) => network_gateway_map.clone().into_values().collect(),
            None => Vec::new(),
        }
    }

    // return gateway name
    #[must_use]
    pub fn get_network_gateway_name(&self, network_id: i64, hostname: &str) -> Option<String> {
        match self.0.get(&network_id) {
            Some(network_gateway_map) => {
                if let Some(state) = network_gateway_map.get(hostname) {
                    state.name.clone()
                } else {
                    None
                }
            }
            None => None,
        }
    }
}

impl Default for GatewayMap {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Serialize, Clone, Debug)]
pub struct GatewayState {
    pub uid: Uuid,
    pub connected: bool,
    pub network_id: i64,
    pub network_name: String,
    pub name: Option<String>,
    pub hostname: String,
    pub connected_at: Option<NaiveDateTime>,
    pub disconnected_at: Option<NaiveDateTime>,
    #[serde(skip)]
    pub mail_tx: UnboundedSender<Mail>,
    #[serde(skip)]
    pub last_email_notification: Option<NaiveDateTime>,
}

impl GatewayState {
    #[must_use]
    pub fn new<S: Into<String>>(
        network_id: i64,
        network_name: S,
        hostname: S,
        name: Option<String>,
        mail_tx: UnboundedSender<Mail>,
    ) -> Self {
        Self {
            uid: Uuid::new_v4(),
            connected: false,
            network_id,
            network_name: network_name.into(),
            name,
            hostname: hostname.into(),
            connected_at: None,
            disconnected_at: None,
            mail_tx,
            last_email_notification: None,
        }
    }

    /// Send gateway disconnected notification
    /// Sends notification only if last notification time is bigger than specified in config
    fn send_disconnect_notification(&mut self, pool: &DbPool) {
        debug!("Sending gateway disconnect email notification");
        // Clone here because self doesn't live long enough
        let name = self.name.clone();
        let mail_tx = self.mail_tx.clone();
        let pool = pool.clone();
        let hostname = self.hostname.clone();
        let network_name = self.network_name.clone();
        let send_email = if let Some(last_notification_time) = self.last_email_notification {
            Utc::now().naive_utc() - last_notification_time
                > ChronoDuration::from_std(
                    *server_config().gateway_disconnection_notification_timeout,
                )
                .expect("Failed to parse duration")
        } else {
            true
        };
        if send_email {
            self.last_email_notification = Some(Utc::now().naive_utc());
            // FIXME: Try to get rid of spawn and use something like block_on
            // To return result instead of logging
            tokio::spawn(async move {
                if let Err(e) =
                    send_gateway_disconnected_email(name, network_name, &hostname, &mail_tx, &pool)
                        .await
                {
                    error!("Failed to send gateway disconnect notification: {e}");
                } else {
                    info!("Gateway {hostname} disconnected. Email notification sent",);
                }
            });
        } else {
            info!(
                "Gateway {hostname} disconnected. Email notification not sent. Last notification was at {:?}",
                self.last_email_notification
            );
        };
    }
}

const TEN_SECS: Duration = Duration::from_secs(10);

impl From<Status> for CoreError {
    fn from(status: Status) -> Self {
        Self {
            status_code: status.code().into(),
            message: status.message().into(),
        }
    }
}

/// Bi-directional gRPC stream for comminication with Defguard proxy.
pub async fn run_grpc_bidi_stream(
    pool: DbPool,
    wireguard_tx: Sender<GatewayEvent>,
    mail_tx: UnboundedSender<Mail>,
    user_agent_parser: Arc<UserAgentParser>,
) -> Result<(), anyhow::Error> {
    let config = server_config();

    // TODO: merge the two
    let enrollment_server = EnrollmentServer::new(
        pool.clone(),
        wireguard_tx.clone(),
        mail_tx.clone(),
        user_agent_parser,
    );
    let password_reset_server = PasswordResetServer::new(pool.clone(), mail_tx.clone());
    let mut client_mfa_server = ClientMfaServer::new(pool, mail_tx, wireguard_tx);

    let endpoint = Endpoint::from_shared(config.proxy_url.as_deref().unwrap())?;
    let endpoint = endpoint
        .http2_keep_alive_interval(TEN_SECS)
        .tcp_keepalive(Some(TEN_SECS))
        .keep_alive_while_idle(true);
    let endpoint = if let Some(ca) = &config.proxy_grpc_ca {
        let ca = read_to_string(ca)?;
        let tls = ClientTlsConfig::new().ca_certificate(Certificate::from_pem(ca));
        endpoint.tls_config(tls)?
    } else {
        endpoint
    };

    loop {
        debug!("Connecting to proxy at {}", endpoint.uri());
        let mut client = ProxyClient::new(endpoint.connect_lazy());
        let (tx, rx) = mpsc::unbounded_channel();
        let Ok(response) = client.bidi(UnboundedReceiverStream::new(rx)).await else {
            error!("Failed to connect to proxy, retrying in 10s");
            sleep(TEN_SECS).await;
            continue;
        };
        info!("Connected to proxy at {}", endpoint.uri());
        let mut resp_stream = response.into_inner();
        'message: loop {
            match resp_stream.message().await {
                Ok(None) => {
                    info!("stream was closed by the sender");
                    break 'message;
                }
                Ok(Some(received)) => {
                    info!("Received message from proxy");
                    let payload = match received.payload {
                        // rpc StartEnrollment (EnrollmentStartRequest) returns (EnrollmentStartResponse)
                        Some(core_request::Payload::EnrollmentStart(request)) => {
                            match enrollment_server.start_enrollment(request).await {
                                Ok(response_payload) => {
                                    Some(core_response::Payload::EnrollmentStart(response_payload))
                                }
                                Err(err) => {
                                    error!("start enrollment error {err}");
                                    Some(core_response::Payload::CoreError(err.into()))
                                }
                            }
                        }
                        // rpc ActivateUser (ActivateUserRequest) returns (google.protobuf.Empty)
                        Some(core_request::Payload::ActivateUser(request)) => {
                            match enrollment_server
                                .activate_user(request, received.device_info)
                                .await
                            {
                                Ok(()) => Some(core_response::Payload::Empty(())),
                                Err(err) => {
                                    error!("activate user error {err}");
                                    Some(core_response::Payload::CoreError(err.into()))
                                }
                            }
                        }
                        // rpc CreateDevice (NewDevice) returns (DeviceConfigResponse)
                        Some(core_request::Payload::NewDevice(request)) => {
                            match enrollment_server
                                .create_device(request, received.device_info)
                                .await
                            {
                                Ok(response_payload) => {
                                    Some(core_response::Payload::DeviceConfig(response_payload))
                                }
                                Err(err) => {
                                    error!("create device error {err}");
                                    Some(core_response::Payload::CoreError(err.into()))
                                }
                            }
                        }
                        // rpc GetNetworkInfo (ExistingDevice) returns (DeviceConfigResponse)
                        Some(core_request::Payload::ExistingDevice(request)) => {
                            match enrollment_server.get_network_info(request).await {
                                Ok(response_payload) => {
                                    Some(core_response::Payload::DeviceConfig(response_payload))
                                }
                                Err(err) => {
                                    error!("get network info error {err}");
                                    Some(core_response::Payload::CoreError(err.into()))
                                }
                            }
                        }
                        // rpc RequestPasswordReset (PasswordResetInitializeRequest) returns (google.protobuf.Empty)
                        Some(core_request::Payload::PasswordResetInit(request)) => {
                            match password_reset_server
                                .request_password_reset(request, received.device_info)
                                .await
                            {
                                Ok(()) => Some(core_response::Payload::Empty(())),
                                Err(err) => {
                                    error!("password reset init error {err}");
                                    Some(core_response::Payload::CoreError(err.into()))
                                }
                            }
                        }
                        // rpc StartPasswordReset (PasswordResetStartRequest) returns (PasswordResetStartResponse)
                        Some(core_request::Payload::PasswordResetStart(request)) => {
                            match password_reset_server.start_password_reset(request).await {
                                Ok(response_payload) => Some(
                                    core_response::Payload::PasswordResetStart(response_payload),
                                ),
                                Err(err) => {
                                    error!("password reset start error {err}");
                                    Some(core_response::Payload::CoreError(err.into()))
                                }
                            }
                        }
                        // rpc ResetPassword (PasswordResetRequest) returns (google.protobuf.Empty)
                        Some(core_request::Payload::PasswordReset(request)) => {
                            match password_reset_server
                                .reset_password(request, received.device_info)
                                .await
                            {
                                Ok(()) => Some(core_response::Payload::Empty(())),
                                Err(err) => {
                                    error!("password reset error {err}");
                                    Some(core_response::Payload::CoreError(err.into()))
                                }
                            }
                        }
                        // rpc ClientMfaStart (ClientMfaStartRequest) returns (ClientMfaStartResponse)
                        Some(core_request::Payload::ClientMfaStart(request)) => {
                            match client_mfa_server.start_client_mfa_login(request).await {
                                Ok(response_payload) => {
                                    Some(core_response::Payload::ClientMfaStart(response_payload))
                                }
                                Err(err) => {
                                    error!("client MFA start error {err}");
                                    Some(core_response::Payload::CoreError(err.into()))
                                }
                            }
                        }
                        // rpc ClientMfaFinish (ClientMfaFinishRequest) returns (ClientMfaFinishResponse)
                        Some(core_request::Payload::ClientMfaFinish(request)) => {
                            match client_mfa_server.finish_client_mfa_login(request).await {
                                Ok(response_payload) => {
                                    Some(core_response::Payload::ClientMfaFinish(response_payload))
                                }
                                Err(err) => {
                                    error!("client MFA start error {err}");
                                    Some(core_response::Payload::CoreError(err.into()))
                                }
                            }
                        }
                        // Reply without payload.
                        None => None,
                    };
                    let req = CoreResponse {
                        id: received.id,
                        payload,
                    };
                    tx.send(req).unwrap();
                }
                Err(err) => {
                    error!("stream error: {err}");
                    debug!("waiting 10s to re-establish the connection");
                    sleep(TEN_SECS).await;
                    break 'message;
                }
            }
        }
    }
}

/// Runs gRPC server with core services.
pub async fn run_grpc_server(
    worker_state: Arc<Mutex<WorkerState>>,
    pool: DbPool,
    gateway_state: Arc<Mutex<GatewayMap>>,
    wireguard_tx: Sender<GatewayEvent>,
    mail_tx: UnboundedSender<Mail>,
    grpc_cert: Option<String>,
    grpc_key: Option<String>,
    failed_logins: Arc<Mutex<FailedLoginMap>>,
) -> Result<(), anyhow::Error> {
    // Build gRPC services
    let auth_service = AuthServiceServer::new(AuthServer::new(pool.clone(), failed_logins));
    #[cfg(feature = "worker")]
    let worker_service = WorkerServiceServer::with_interceptor(
        WorkerServer::new(pool.clone(), worker_state),
        JwtInterceptor::new(ClaimsType::YubiBridge),
    );
    #[cfg(feature = "wireguard")]
    let gateway_service = GatewayServiceServer::with_interceptor(
        GatewayServer::new(pool, gateway_state, wireguard_tx, mail_tx),
        JwtInterceptor::new(ClaimsType::Gateway),
    );
    // Run gRPC server
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), server_config().grpc_port);
    debug!("Starting gRPC services");
    let builder = if let (Some(cert), Some(key)) = (grpc_cert, grpc_key) {
        let identity = Identity::from_pem(cert, key);
        Server::builder().tls_config(ServerTlsConfig::new().identity(identity))?
    } else {
        Server::builder()
    };
    let router = builder
        .http2_keepalive_interval(Some(TEN_SECS))
        .tcp_keepalive(Some(TEN_SECS))
        .add_service(auth_service);
    #[cfg(feature = "wireguard")]
    let router = router.add_service(gateway_service);
    #[cfg(feature = "worker")]
    let router = router.add_service(worker_service);
    router.serve(addr).await?;
    info!("gRPC server started on {addr}");
    Ok(())
}

#[cfg(feature = "worker")]
pub struct Job {
    id: u32,
    first_name: String,
    last_name: String,
    email: String,
    username: String,
}

#[cfg(feature = "worker")]
#[derive(Serialize)]
pub struct JobResponse {
    pub success: bool,
    pub serial: String,
    pub error: String,
    #[serde(skip)]
    pub username: String,
}

#[cfg(feature = "worker")]
pub struct WorkerInfo {
    last_seen: Instant,
    ip: IpAddr,
    jobs: Vec<Job>,
}

#[cfg(feature = "worker")]
pub struct WorkerState {
    current_job_id: u32,
    workers: HashMap<String, WorkerInfo>,
    job_status: HashMap<u32, JobResponse>,
    webhook_tx: UnboundedSender<AppEvent>,
}

#[cfg(feature = "worker")]
#[derive(Deserialize, Serialize)]
pub struct WorkerDetail {
    id: String,
    ip: IpAddr,
    connected: bool,
}
