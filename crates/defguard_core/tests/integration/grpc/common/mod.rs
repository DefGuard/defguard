use std::sync::{Arc, Mutex};

use axum::http::Uri;
use defguard_common::{
    db::models::settings::initialize_current_settings, messages::peer_stats_update::PeerStatsUpdate,
};
use defguard_core::{
    auth::failed_login::FailedLoginMap,
    db::AppEvent,
    enterprise::license::{License, LicenseTier, set_cached_license},
    events::GrpcEvent,
    grpc::{
        WorkerState, build_grpc_service_router,
        gateway::{client_state::ClientMap, events::GatewayEvent, map::GatewayMap},
    },
};
use defguard_mail::Mail;
use hyper_util::rt::TokioIo;
use sqlx::PgPool;
use tokio::{
    io::DuplexStream,
    sync::{
        broadcast::{self, Sender},
        mpsc::{UnboundedReceiver, unbounded_channel},
    },
    task::JoinHandle,
};
use tonic::transport::{Channel, Endpoint, Server, server::Router};
use tower::service_fn;

use crate::common::{init_config, initialize_users};

pub mod mock_gateway;

pub struct TestGrpcServer {
    grpc_server_task_handle: JoinHandle<()>,
    pub grpc_event_rx: UnboundedReceiver<GrpcEvent>,
    wireguard_tx: Sender<GatewayEvent>,
    client_state: Arc<Mutex<ClientMap>>,
    pub client_channel: Channel,
    #[allow(dead_code)]
    peer_stats_rx: UnboundedReceiver<PeerStatsUpdate>,
}

impl TestGrpcServer {
    #[must_use]
    pub async fn new(
        server_stream: DuplexStream,
        grpc_router: Router,
        grpc_event_rx: UnboundedReceiver<GrpcEvent>,
        wireguard_tx: Sender<GatewayEvent>,
        client_state: Arc<Mutex<ClientMap>>,
        client_channel: Channel,
        peer_stats_rx: UnboundedReceiver<PeerStatsUpdate>,
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
            client_state,
            client_channel,
            peer_stats_rx,
        }
    }

    pub fn get_client_map(&self) -> std::sync::MutexGuard<'_, ClientMap> {
        self.client_state
            .lock()
            .expect("failed to acquire lock on client state")
    }

    pub fn send_wireguard_event(&self, event: GatewayEvent) {
        self.wireguard_tx
            .send(event)
            .expect("failed to send gateway event");
    }
}

impl Drop for TestGrpcServer {
    fn drop(&mut self) {
        // explicitly stop spawned gRPC server task
        self.grpc_server_task_handle.abort();
    }
}

pub(crate) async fn create_client_channel(client_stream: DuplexStream) -> Channel {
    // Move client to an option so we can _move_ the inner value
    // on the first attempt to connect. All other attempts will fail.
    // reference: https://github.com/hyperium/tonic/blob/master/examples/src/mock/mock.rs#L31
    let mut client = Some(client_stream);
    Endpoint::try_from("http://[::]:50051")
        .expect("Failed to create channel")
        .connect_with_connector(service_fn(move |_: Uri| {
            let client = client.take();

            async move {
                if let Some(client) = client {
                    Ok(TokioIo::new(client))
                } else {
                    Err(std::io::Error::other("Client already taken"))
                }
            }
        }))
        .await
        .expect("Failed to create client channel")
}

pub(crate) async fn make_grpc_test_server(pool: &PgPool) -> TestGrpcServer {
    // create communication channel for clients
    let (client_stream, server_stream) = tokio::io::duplex(1024);
    let client_channel = create_client_channel(client_stream).await;

    // setup helper structs
    let (grpc_event_tx, grpc_event_rx) = unbounded_channel::<GrpcEvent>();
    let (app_event_tx, _app_event_rx) = unbounded_channel::<AppEvent>();
    let worker_state = Arc::new(Mutex::new(WorkerState::new(app_event_tx.clone())));
    let (wg_tx, _wg_rx) = broadcast::channel::<GatewayEvent>(16);
    let (peer_stats_tx, peer_stats_rx) = unbounded_channel::<PeerStatsUpdate>();
    let gateway_state = Arc::new(Mutex::new(GatewayMap::new()));
    let client_state = Arc::new(Mutex::new(ClientMap::new()));

    let failed_logins = FailedLoginMap::new();
    let failed_logins = Arc::new(Mutex::new(failed_logins));

    let config = init_config(None, pool).await;
    initialize_users(pool, &config).await;
    initialize_current_settings(pool)
        .await
        .expect("Could not initialize settings");

    let license = License::new(
        "test_customer".to_string(),
        false,
        // Permanent license
        None,
        None,
        None,
        LicenseTier::Business,
    );

    set_cached_license(Some(license));
    let server = Server::builder();

    let grpc_router = build_grpc_service_router(server, pool.clone(), worker_state, failed_logins)
        .await
        .unwrap();

    TestGrpcServer::new(
        server_stream,
        grpc_router,
        grpc_event_rx,
        wg_tx,
        client_state,
        client_channel,
        peer_stats_rx,
    )
    .await
}
