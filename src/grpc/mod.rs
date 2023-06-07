#[cfg(feature = "worker")]
use crate::{
    auth::ClaimsType,
    db::{DbPool, GatewayEvent},
    grpc::{
        interceptor::JwtInterceptor,
        worker::{worker_service_server::WorkerServiceServer, WorkerServer},
    },
};
use auth::{auth_service_server::AuthServiceServer, AuthServer};
#[cfg(feature = "wireguard")]
use gateway::{gateway_service_server::GatewayServiceServer, GatewayServer};
#[cfg(any(feature = "wireguard", feature = "worker"))]
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::{Arc, Mutex},
};
use tokio::sync::{broadcast::Receiver, mpsc::UnboundedSender, Mutex as AsyncMutex};
use tonic::transport::{Identity, Server, ServerTlsConfig};

use crate::auth::failed_login::FailedLoginMap;
use crate::db::AppEvent;
use serde::Serialize;
use std::{collections::hash_map::HashMap, time::Instant};

mod auth;
#[cfg(feature = "wireguard")]
mod gateway;
#[cfg(any(feature = "wireguard", feature = "worker"))]
mod interceptor;
#[cfg(feature = "worker")]
pub mod worker;

pub struct GatewayState {
    pub connected: bool,
    pub wireguard_rx: Arc<AsyncMutex<Receiver<GatewayEvent>>>,
}

impl GatewayState {
    #[must_use]
    pub fn new(wireguard_rx: Receiver<GatewayEvent>) -> Self {
        Self {
            connected: false,
            wireguard_rx: Arc::new(AsyncMutex::new(wireguard_rx)),
        }
    }
}

/// Runs gRPC server with core services.
pub async fn run_grpc_server(
    grpc_port: u16,
    worker_state: Arc<Mutex<WorkerState>>,
    pool: DbPool,
    gateway_state: Arc<Mutex<GatewayState>>,
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
        GatewayServer::new(pool, gateway_state),
        JwtInterceptor::new(ClaimsType::Gateway),
    );
    // Run gRPC server
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), grpc_port);
    info!("Started gRPC services");
    let mut builder = if let (Some(cert), Some(key)) = (grpc_cert, grpc_key) {
        let identity = Identity::from_pem(cert, key);
        Server::builder().tls_config(ServerTlsConfig::new().identity(identity))?
    } else {
        Server::builder()
    };
    let router = builder.add_service(auth_service);
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
