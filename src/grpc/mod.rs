#[cfg(feature = "worker")]
use crate::enterprise::grpc::worker::{worker_service_server::WorkerServiceServer, WorkerServer};
use crate::{
    auth::ClaimsType,
    db::{DbPool, GatewayEvent},
    enterprise::grpc::WorkerState,
    grpc::interceptor::JwtInterceptor,
};
use auth::{auth_service_server::AuthServiceServer, AuthServer};
#[cfg(feature = "wireguard")]
use gateway::{gateway_service_server::GatewayServiceServer, GatewayServer};
#[cfg(any(feature = "wireguard", feature = "worker"))]
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::{Arc, Mutex},
};
use tokio::sync::{mpsc::UnboundedReceiver, Mutex as AsyncMutex};
use tonic::transport::{Identity, Server, ServerTlsConfig};

mod auth;
#[cfg(feature = "wireguard")]
mod gateway;
#[cfg(any(feature = "wireguard", feature = "worker"))]
mod interceptor;

pub struct GatewayState {
    pub connected: bool,
    pub wireguard_rx: Arc<AsyncMutex<UnboundedReceiver<GatewayEvent>>>,
}

impl GatewayState {
    #[must_use]
    pub fn new(wireguard_rx: UnboundedReceiver<GatewayEvent>) -> Self {
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
) -> Result<(), Box<dyn std::error::Error>> {
    // Build gRPC services
    let auth_service = AuthServiceServer::new(AuthServer::new(pool.clone()));
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
