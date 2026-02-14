// FIXME: actua, Updatelly refactor errors instead
#![allow(clippy::result_large_err)]
use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::{Arc, Mutex},
    time::Duration,
};

use defguard_common::{
    auth::claims::ClaimsType,
    config::server_config,
    db::{ChangeNotification, Id, TriggerOperation, models::gateway::Gateway},
    messages::peer_stats_update::PeerStatsUpdate,
};
use defguard_core::{
    auth::failed_login::FailedLoginMap,
    grpc::{GatewayEvent, WorkerState, interceptor::JwtInterceptor, worker::WorkerServer},
};
use defguard_proto::{
    auth::auth_service_server::AuthServiceServer, gateway::gateway_client::GatewayClient,
    worker::worker_service_server::WorkerServiceServer,
};
use defguard_version::client::ClientVersionInterceptor;
use sqlx::{PgPool, postgres::PgListener};
use tokio::{
    sync::{broadcast::Sender, mpsc::UnboundedSender},
    task::{AbortHandle, JoinSet},
};
use tonic::{
    Request,
    service::interceptor::InterceptedService,
    transport::{Channel, Identity, Server, ServerTlsConfig, server::Router},
};

use crate::{auth::AuthServer, handler::GatewayHandler};

#[macro_use]
extern crate tracing;

mod auth;
mod certs;
mod error;
mod handler;
// #[cfg(test)]
// mod tests;

#[cfg(test)]
static TONIC_SOCKET: &str = "tonic.sock";
const GATEWAY_TABLE_TRIGGER: &str = "gateway_change";
const GATEWAY_RECONNECT_DELAY: Duration = Duration::from_secs(5);
const TEN_SECS: Duration = Duration::from_secs(10);

type Client = GatewayClient<InterceptedService<Channel, ClientVersionInterceptor>>;

pub struct GatewayManager {
    clients: Arc<Mutex<HashMap<Id, Client>>>,
}

impl GatewayManager {
    pub fn new() -> Self {
        Self {
            clients: Arc::default(),
        }
    }

    /// Bi-directional gRPC stream for communication with Defguard Gateway.
    pub async fn run_grpc_gateway_stream(
        &mut self,
        pool: PgPool,
        events_tx: Sender<GatewayEvent>,
        peer_stats_tx: UnboundedSender<PeerStatsUpdate>,
    ) -> Result<(), anyhow::Error> {
        let (certs_tx, certs_rx) = tokio::sync::watch::channel(Arc::new(HashMap::new()));
        certs::refresh_certs(&pool, &certs_tx).await;
        let refresh_pool = pool.clone();
        tokio::spawn(async move {
            loop {
                certs::refresh_certs(&refresh_pool, &certs_tx).await;
                tokio::time::sleep(TEN_SECS).await;
            }
        });
        let mut abort_handles = HashMap::new();

        let mut tasks = JoinSet::new();
        // Helper closure to launch `GatewayHandler`.
		// TODO(jck) store arguments in GatewayManager
		// TODO(jck) rewrite this to method
        let mut launch_gateway_handler = |gateway: Gateway<Id>,
                                          clients: Arc<Mutex<HashMap<Id, Client>>>|
         -> Result<AbortHandle, anyhow::Error> {
            let mut gateway_handler = GatewayHandler::new(
                gateway,
                pool.clone(),
                events_tx.clone(),
                peer_stats_tx.clone(),
                certs_rx.clone(),
            )?;
            let abort_handle = tasks.spawn(async move {
                loop {
                    if let Err(err) = gateway_handler
                        .handle_connection(Arc::clone(&clients))
                        .await
                    {
                        error!("Gateway connection error: {err}, retrying in 5 seconds...");
                        tokio::time::sleep(GATEWAY_RECONNECT_DELAY).await;
                    }
                }
            });
            Ok(abort_handle)
        };
        for gateway in Gateway::all(&pool).await? {
            let id = gateway.id;
            let abort_handle = launch_gateway_handler(gateway, Arc::clone(&self.clients))?;
            abort_handles.insert(id, abort_handle);
        }

        // Observe gateway URL changes.
        let mut listener = PgListener::connect_with(&pool).await?;
        listener.listen(GATEWAY_TABLE_TRIGGER).await?;
        while let Ok(notification) = listener.recv().await {
            let payload = notification.payload();
            match serde_json::from_str::<ChangeNotification<Gateway<Id>>>(payload) {
                Ok(gateway_notification) => match gateway_notification.operation {
                    TriggerOperation::Insert => {
                        if let Some(new) = gateway_notification.new {
                            let id = new.id;
                            let abort_handle =
                                launch_gateway_handler(new, Arc::clone(&self.clients))?;
                            abort_handles.insert(id, abort_handle);
                        }
                    }
                    TriggerOperation::Update => {
                        if let (Some(old), Some(new)) =
                            (gateway_notification.old, gateway_notification.new)
                        {
                            if old.url == new.url {
                                debug!(
                                    "Gateway URL didn't change. Keeping the current gateway handler"
                                );
                            } else if let Some(abort_handle) = abort_handles.remove(&old.id) {
                                info!(
                                    "Aborting connection to {old}, it has changed in the database"
                                );
                                abort_handle.abort();
                                let id = new.id;
                                let abort_handle =
                                    launch_gateway_handler(new, Arc::clone(&self.clients))?;
                                abort_handles.insert(id, abort_handle);
                            } else {
                                warn!("Cannot find {old} on the list of connected gateways");
                            }
                        }
                    }
                    TriggerOperation::Delete => {
						// TODO(jck) refactor
                        if let Some(old) = gateway_notification.old {
                            if let Some(mut client) = self
                                .clients
                                .lock()
                                .expect("Failed to lock GatewayManager::clients")
                                .remove(&old.id)
                            {
                                debug!("Sending purge request to gateway {old}");
                                if let Err(err) = client.purge(Request::new(())).await {
                                    error!("Error sending purge request to gateway {old}: {err}");
                                } else {
                                    info!("Sent purge request to gateway {old}");
                                }
                            } else {
                                warn!(
                                    "Cannot find gateway {old} on the list of connected gateways"
                                );
                            }
                            if let Some(abort_handle) = abort_handles.remove(&old.id) {
                                info!(
                                    "Aborting connection to gateway {old}, it has disappeard from the database"
                                );
                                abort_handle.abort();
                            } else {
                                warn!(
                                    "Cannot find gateway {old} on the list of connected gateways"
                                );
                            }
                        }
                    }
                },
                Err(err) => error!("Failed to de-serialize database notification object: {err}"),
            }
        }

        while let Some(Ok(_result)) = tasks.join_next().await {
            debug!("Gateway gRPC task has ended");
        }

        Ok(())
    }
}

// TODO(jck) move this to core/grpc/lib
/// Runs gRPC server with core services.
#[instrument(skip_all)]
pub async fn run_grpc_server(
    worker_state: Arc<Mutex<WorkerState>>,
    pool: PgPool,
    grpc_cert: Option<String>,
    grpc_key: Option<String>,
    failed_logins: Arc<Mutex<FailedLoginMap>>,
) -> Result<(), anyhow::Error> {
    // Build gRPC services
    let server = if let (Some(cert), Some(key)) = (grpc_cert, grpc_key) {
        let identity = Identity::from_pem(cert, key);
        Server::builder().tls_config(ServerTlsConfig::new().identity(identity))?
    } else {
        Server::builder()
    };

    let router = build_grpc_service_router(server, pool, worker_state, failed_logins).await?;

    // Run gRPC server
    let addr = SocketAddr::new(
        server_config()
            .grpc_bind_address
            .unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED)),
        server_config().grpc_port,
    );
    debug!("Starting gRPC services");
    router.serve(addr).await?;
    info!("gRPC server started on {addr}");
    Ok(())
}

// TODO(jck) move this to core/grpc/lib
pub(crate) async fn build_grpc_service_router(
    server: Server,
    pool: PgPool,
    worker_state: Arc<Mutex<WorkerState>>,
    failed_logins: Arc<Mutex<FailedLoginMap>>,
    // incompatible_components: Arc<RwLock<IncompatibleComponents>>,
) -> Result<Router, anyhow::Error> {
    let auth_service = AuthServiceServer::new(AuthServer::new(pool.clone(), failed_logins));

    let worker_service = WorkerServiceServer::with_interceptor(
        WorkerServer::new(pool.clone(), worker_state),
        JwtInterceptor::new(ClaimsType::YubiBridge),
    );

    let (health_reporter, health_service) = tonic_health::server::health_reporter();
    health_reporter
        .set_serving::<AuthServiceServer<AuthServer>>()
        .await;

    let router = server
        .http2_keepalive_interval(Some(TEN_SECS))
        .tcp_keepalive(Some(TEN_SECS))
        .add_service(health_service)
        .add_service(auth_service);
    let router = router.add_service(worker_service);

    Ok(router)
}
