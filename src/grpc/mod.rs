#[cfg(feature = "worker")]
use crate::enterprise::grpc::worker::{
    token_interceptor, worker_service_server::WorkerServiceServer, WorkerServer,
};
use crate::{
    db::{DbPool, GatewayEvent},
    enterprise::grpc::WorkerState,
};
use auth::{auth_service_server::AuthServiceServer, AuthServer};
#[cfg(feature = "wireguard")]
use gateway::{gateway_service_server::GatewayServiceServer, GatewayServer};
#[cfg(any(feature = "wireguard", feature = "worker"))]
use interceptor::jwt_auth_interceptor;
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::{Arc, Mutex},
};
use tokio::sync::mpsc::UnboundedReceiver;
use tonic::transport::{Identity, Server, ServerTlsConfig};
mod auth;
#[cfg(feature = "wireguard")]
mod gateway;
#[cfg(any(feature = "wireguard", feature = "worker"))]
mod interceptor;

/// Runs gRPC server with core services.
pub async fn run_grpc_server(
    grpc_port: u16,
    worker_state: Arc<Mutex<WorkerState>>,
    wireguard_rx: UnboundedReceiver<GatewayEvent>,
    pool: DbPool,
    grpc_cert: Option<String>,
    grpc_key: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Build gRPC services
    let auth_service = AuthServiceServer::new(AuthServer::new(pool.clone()));
    #[cfg(feature = "worker")]
    let worker_service = WorkerServiceServer::with_interceptor(
        WorkerServer::new(pool.clone(), worker_state),
        token_interceptor,
    );
    #[cfg(feature = "wireguard")]
    let gateway_service = GatewayServiceServer::with_interceptor(
        GatewayServer::new(wireguard_rx, pool),
        jwt_auth_interceptor,
    );
    // Run gRPC server
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), grpc_port);
    info!("Started gRPC services");
    let mut builder = if let (Some(cert), Some(key)) = (grpc_cert, grpc_key) {
        let identity = Identity::from_pem(&cert, &key);
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
