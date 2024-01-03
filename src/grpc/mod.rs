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
use tokio::sync::{
    broadcast::Sender,
    mpsc::{self, UnboundedSender},
};
use tokio_stream::{wrappers::UnboundedReceiverStream, StreamExt};
use tonic::transport::{Certificate, ClientTlsConfig, Endpoint, Identity, Server, ServerTlsConfig};
use uaparser::UserAgentParser;
use uuid::Uuid;

#[cfg(feature = "wireguard")]
use self::gateway::{gateway_service_server::GatewayServiceServer, GatewayServer};
use self::{
    auth::{auth_service_server::AuthServiceServer, AuthServer},
    enrollment::EnrollmentServer,
    proto::{
        enrollment_service_server::EnrollmentServiceServer,
        password_reset_service_server::PasswordResetServiceServer, proxy_request,
    },
};
#[cfg(feature = "worker")]
use self::{
    interceptor::JwtInterceptor,
    worker::{worker_service_server::WorkerServiceServer, WorkerServer},
};
use crate::{
    auth::failed_login::FailedLoginMap,
    config::DefGuardConfig,
    db::AppEvent,
    grpc::{enrollment::start_enrollment, password_reset::PasswordResetServer},
    handlers::mail::send_gateway_disconnected_email,
    mail::Mail,
    SERVER_CONFIG,
};
#[cfg(feature = "worker")]
use crate::{
    auth::ClaimsType,
    db::{DbPool, GatewayEvent},
};

mod auth;
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

use proto::{proxy_client::ProxyClient, proxy_response, ProxyRequest};

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
        info!("Removing gateway from network {network_id}");
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
        Ok(())
    }

    // change gateway status to connected
    // we assume that the gateway is already present in hashmap
    pub fn connect_gateway(
        &mut self,
        network_id: i64,
        hostname: &str,
    ) -> Result<(), GatewayMapError> {
        info!("Connecting gateway {hostname} in network {network_id}");
        if let Some(network_gateway_map) = self.0.get_mut(&network_id) {
            if let Some(state) = network_gateway_map.get_mut(hostname) {
                state.connected = true;
                state.disconnected_at = None;
                state.connected_at = Some(Utc::now().naive_utc());
            } else {
                error!("Gateway {hostname} not found in gateway map for network {network_id}");
                return Err(GatewayMapError::NotFound(network_id, hostname.into()));
            }
        } else {
            // no map for a given network exists yet
            error!("Network {network_id} not found in gateway map");
            return Err(GatewayMapError::NetworkNotFound(network_id));
        };
        Ok(())
    }

    // change gateway status to disconnected
    pub fn disconnect_gateway(
        &mut self,
        network_id: i64,
        hostname: String,
        pool: &DbPool,
    ) -> Result<(), GatewayMapError> {
        info!("Disconnecting gateway {hostname} in network {network_id}");
        if let Some(network_gateway_map) = self.0.get_mut(&network_id) {
            if let Some(state) = network_gateway_map.get_mut(&hostname) {
                state.connected = false;
                state.disconnected_at = Some(Utc::now().naive_utc());
                state.send_disconnect_notification(pool)?;
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
    fn send_disconnect_notification(&mut self, pool: &DbPool) -> Result<(), GatewayMapError> {
        // Clone here because self doesn't live long enough
        let name = self.name.clone();
        let mail_tx = self.mail_tx.clone();
        let pool = pool.clone();
        let hostname = self.hostname.clone();
        let network_name = self.network_name.clone();
        let send_email = if let Some(last_notification_time) = self.last_email_notification {
            Utc::now().naive_utc() - last_notification_time
                > ChronoDuration::from_std(
                    *SERVER_CONFIG
                        .get()
                        .ok_or(GatewayMapError::ConfigError)?
                        .gateway_disconnection_notification_timeout,
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
                }
            });
        } else {
            debug!(
                "Gateway {hostname} disconnected. Email notification not sent. Last notification was at {:?}",
                self.last_email_notification
            );
        };

        Ok(())
    }
}

const TEN_SECS: Duration = Duration::from_secs(10);

// TODO: re-connect loop
pub async fn run_grpc_stream(pool: DbPool) -> Result<(), anyhow::Error> {
    let config = SERVER_CONFIG.get().unwrap();

    let endpoint = Endpoint::from_shared(config.proxy_url.clone())?;
    let endpoint = endpoint.http2_keep_alive_interval(TEN_SECS);
    let endpoint = endpoint.tcp_keepalive(Some(TEN_SECS));
    let endpoint = if let Some(ca) = config.grpc_cert.clone() {
        let ca = read_to_string(ca)?;
        let tls = ClientTlsConfig::new().ca_certificate(Certificate::from_pem(ca));
        endpoint.tls_config(tls)?
    } else {
        endpoint
    };

    let mut client = ProxyClient::new(endpoint.connect_lazy());
    let (tx, rx) = mpsc::unbounded_channel();
    let response = client.bidi(UnboundedReceiverStream::new(rx)).await.unwrap();
    let mut resp_stream = response.into_inner();
    while let Some(received) = resp_stream.next().await {
        let received = received.unwrap();
        info!("received message");
        if let Some(payload) = received.payload {
            info!("request {payload:?}");
            match payload {
                proxy_response::Payload::EnrollmentStart(request) => {
                    // TODO: get rid of unwrap() - send errors?
                    let response_payload = start_enrollment(&pool, &config, request).await.unwrap();
                    let req = ProxyRequest {
                        id: received.id,
                        payload: Some(proxy_request::Payload::EnrollmentStart(response_payload)),
                    };
                    tx.send(req).unwrap();
                }
            }
        }
    }

    Ok(())
}

/// Runs gRPC server with core services.
pub async fn run_grpc_server(
    config: &DefGuardConfig,
    worker_state: Arc<Mutex<WorkerState>>,
    pool: DbPool,
    gateway_state: Arc<Mutex<GatewayMap>>,
    wireguard_tx: Sender<GatewayEvent>,
    mail_tx: UnboundedSender<Mail>,
    grpc_cert: Option<String>,
    grpc_key: Option<String>,
    user_agent_parser: Arc<UserAgentParser>,
    failed_logins: Arc<Mutex<FailedLoginMap>>,
) -> Result<(), anyhow::Error> {
    // Build gRPC services
    let auth_service = AuthServiceServer::new(AuthServer::new(pool.clone(), failed_logins));
    let enrollment_service = EnrollmentServiceServer::new(EnrollmentServer::new(
        pool.clone(),
        wireguard_tx.clone(),
        mail_tx.clone(),
        user_agent_parser,
        config.clone(),
    ));
    let password_reset_service = PasswordResetServiceServer::new(PasswordResetServer::new(
        pool.clone(),
        mail_tx.clone(),
        config.clone(),
    ));
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
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), config.grpc_port);
    info!("Started gRPC services");
    let builder = if let (Some(cert), Some(key)) = (grpc_cert, grpc_key) {
        let identity = Identity::from_pem(cert, key);
        Server::builder().tls_config(ServerTlsConfig::new().identity(identity))?
    } else {
        Server::builder()
    };
    let builder = builder.http2_keepalive_interval(Some(Duration::from_secs(10)));
    let mut builder = builder.tcp_keepalive(Some(Duration::from_secs(10)));
    let router = builder.add_service(auth_service);
    let router = router.add_service(enrollment_service);
    let router = router.add_service(password_reset_service);
    #[cfg(feature = "wireguard")]
    let router = router.add_service(gateway_service);
    #[cfg(feature = "worker")]
    let router = router.add_service(worker_service);
    router.serve(addr).await?;
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
    pgp_key: String,
    pgp_cert_id: String,
    ssh_key: String,
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
