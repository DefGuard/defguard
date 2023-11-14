use std::{
    collections::hash_map::HashMap,
    time::{Duration, Instant},
};
#[cfg(any(feature = "wireguard", feature = "worker"))]
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::{Arc, Mutex},
};

use chrono::{NaiveDateTime, Utc};
use serde::Serialize;
use thiserror::Error;
use tokio::sync::{broadcast::Sender, mpsc::UnboundedSender};
use tonic::transport::{Identity, Server, ServerTlsConfig};
use uuid::Uuid;

#[cfg(feature = "wireguard")]
use self::gateway::{gateway_service_server::GatewayServiceServer, GatewayServer};
use self::{
    auth::{auth_service_server::AuthServiceServer, AuthServer},
    enrollment::{proto::enrollment_service_server::EnrollmentServiceServer, EnrollmentServer},
};
#[cfg(feature = "worker")]
use self::{
    interceptor::JwtInterceptor,
    worker::{worker_service_server::WorkerServiceServer, WorkerServer},
};
use crate::{auth::failed_login::FailedLoginMap, config::DefGuardConfig, db::AppEvent, mail::Mail};
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
#[cfg(feature = "worker")]
pub mod worker;

// Helper struct used to handle gateway state
// gateways are grouped by network
type NetworkId = i64;
type GatewayHostname = String;
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
}

impl GatewayMap {
    #[must_use]
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    // add a new gateway to map
    // this method is meant to be called when a gateway requests a config
    // as a sort of "registration"
    pub fn add_gateway(&mut self, network_id: i64, hostname: String, name: Option<String>) {
        info!("Adding gateway {hostname} with to gateway map for network {network_id}",);
        if let Some(network_gateway_map) = self.0.get_mut(&network_id) {
            network_gateway_map.insert(
                hostname.clone(),
                GatewayState::new(network_id, hostname, name),
            );
        } else {
            // no map for a given network exists yet
            let mut network_gateway_map = HashMap::new();
            network_gateway_map.insert(
                hostname.clone(),
                GatewayState::new(network_id, hostname, name),
            );
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
    ) -> Result<(), GatewayMapError> {
        info!("Disconnecting gateway {hostname} in network {network_id}");
        if let Some(network_gateway_map) = self.0.get_mut(&network_id) {
            if let Some(state) = network_gateway_map.get_mut(&hostname) {
                state.connected = false;
                state.disconnected_at = Some(Utc::now().naive_utc());
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
            Some(network_gateway_map) => network_gateway_map.get(hostname).unwrap().name.clone(),
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
    pub name: Option<String>,
    pub hostname: String,
    pub connected_at: Option<NaiveDateTime>,
    pub disconnected_at: Option<NaiveDateTime>,
}

impl GatewayState {
    #[must_use]
    pub fn new(network_id: i64, hostname: String, name: Option<String>) -> Self {
        Self {
            uid: Uuid::new_v4(),
            connected: false,
            network_id,
            name,
            hostname,
            connected_at: None,
            disconnected_at: None,
        }
    }
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
    failed_logins: Arc<Mutex<FailedLoginMap>>,
) -> Result<(), anyhow::Error> {
    // Build gRPC services
    let auth_service = AuthServiceServer::new(AuthServer::new(pool.clone(), failed_logins));
    let enrollment_service = EnrollmentServiceServer::new(EnrollmentServer::new(
        pool.clone(),
        wireguard_tx.clone(),
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
