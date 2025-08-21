use std::sync::{Arc, Mutex};

use defguard_core::{
    auth::failed_login::FailedLoginMap,
    db::{AppEvent, GatewayEvent, models::settings::initialize_current_settings},
    enterprise::license::{License, set_cached_license},
    events::GrpcEvent,
    grpc::{GatewayMap, WorkerState, build_grpc_service_router},
    mail::Mail,
};
use sqlx::PgPool;
use tokio::{
    io::DuplexStream,
    sync::{
        broadcast::{self, Sender},
        mpsc::{UnboundedReceiver, unbounded_channel},
    },
    task::JoinHandle,
};
use tonic::transport::{Server, server::Router};

use crate::common::{init_config, initialize_users};

pub mod mock_gateway;

pub struct TestGrpcServer {
    grpc_server_task_handle: JoinHandle<()>,
    grpc_event_rx: UnboundedReceiver<GrpcEvent>,
    // app_event_rx: UnboundedReceiver<AppEvent>,
    wireguard_tx: Sender<GatewayEvent>,
    // mail_rx: UnboundedReceiver<Mail>,
    worker_state: Arc<Mutex<WorkerState>>,
    gateway_state: Arc<Mutex<GatewayMap>>,
    failed_logins: Arc<Mutex<FailedLoginMap>>,
}

impl TestGrpcServer {
    pub async fn new(
        server_stream: DuplexStream,
        grpc_router: Router,
        grpc_event_rx: UnboundedReceiver<GrpcEvent>,
        wireguard_tx: Sender<GatewayEvent>,
        worker_state: Arc<Mutex<WorkerState>>,
        gateway_state: Arc<Mutex<GatewayMap>>,
        failed_logins: Arc<Mutex<FailedLoginMap>>,
    ) -> Self {
        // spawn test gRPC server
        let grpc_server_task_handle = tokio::spawn(async move {
            grpc_router
                .serve_with_incoming(tokio_stream::once(Ok::<_, std::io::Error>(server_stream)))
                .await
                .map_err(|err| eprintln!("Unexpected test gRPC server error: {err}"))
                .unwrap()
        });

        Self {
            grpc_server_task_handle,
            grpc_event_rx,
            wireguard_tx,
            worker_state,
            gateway_state,
            failed_logins,
        }
    }

    pub fn get_gateway_map(&self) -> std::sync::MutexGuard<'_, GatewayMap> {
        self.gateway_state
            .lock()
            .expect("failed to acquire lock on gateway state")
    }
}

impl Drop for TestGrpcServer {
    fn drop(&mut self) {
        // explicitly stop spawned gRPC server task
        self.grpc_server_task_handle.abort();
    }
}

pub(crate) async fn make_grpc_test_server(pool: &PgPool) -> (TestGrpcServer, DuplexStream) {
    // create communication channel for clients
    let (client_stream, server_stream) = tokio::io::duplex(1024);

    // setup helper structs
    let (grpc_event_tx, grpc_event_rx) = unbounded_channel::<GrpcEvent>();
    let (app_event_tx, app_event_rx) = unbounded_channel::<AppEvent>();
    let worker_state = Arc::new(Mutex::new(WorkerState::new(app_event_tx.clone())));
    let (wg_tx, wg_rx) = broadcast::channel::<GatewayEvent>(16);
    let (mail_tx, mail_rx) = unbounded_channel::<Mail>();
    let gateway_state = Arc::new(Mutex::new(GatewayMap::new()));

    let failed_logins = FailedLoginMap::new();
    let failed_logins = Arc::new(Mutex::new(failed_logins));

    let config = init_config(None);
    initialize_users(&pool, &config).await;
    initialize_current_settings(&pool)
        .await
        .expect("Could not initialize settings");

    let license = License::new(
        "test_customer".to_string(),
        false,
        // Permanent license
        None,
        None,
    );

    set_cached_license(Some(license));
    let server = Server::builder();

    let grpc_router = build_grpc_service_router(
        server,
        pool.clone(),
        worker_state.clone(),
        gateway_state.clone(),
        wg_tx.clone(),
        mail_tx,
        failed_logins.clone(),
        grpc_event_tx,
    )
    .await;

    (
        TestGrpcServer::new(
            server_stream,
            grpc_router,
            grpc_event_rx,
            wg_tx,
            worker_state,
            gateway_state,
            failed_logins,
        )
        .await,
        client_stream,
    )
}
