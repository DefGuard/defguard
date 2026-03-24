use std::sync::{Arc, Mutex};

use defguard_common::{
    auth::claims::{Claims, ClaimsType},
    db::{
        models::{
            Settings,
            settings::{initialize_current_settings, update_current_settings},
        },
        setup_pool,
    },
};
use defguard_core::{
    db::AppEvent,
    grpc::{AUTHORIZATION_HEADER, WorkerState, build_grpc_service_router},
};
use hyper_util::rt::TokioIo;
use sqlx::{
    PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
};
use tokio::{
    io::DuplexStream,
    sync::mpsc::{UnboundedReceiver, unbounded_channel},
    task::JoinHandle,
};
use tonic::{
    Request,
    transport::{Channel, Endpoint, Server, Uri, server::Router},
};
use tower::service_fn;

use crate::common::initialize_users;

pub const TEST_SECRET_KEY: &str =
    "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

pub struct TestGrpcServer {
    grpc_server_task_handle: JoinHandle<()>,
    pub worker_state: Arc<Mutex<WorkerState>>,
    pub client_channel: Channel,
    pub app_event_rx: UnboundedReceiver<AppEvent>,
}

impl TestGrpcServer {
    #[must_use]
    pub async fn new(
        server_stream: DuplexStream,
        grpc_router: Router,
        worker_state: Arc<Mutex<WorkerState>>,
        client_channel: Channel,
        app_event_rx: UnboundedReceiver<AppEvent>,
    ) -> Self {
        let grpc_server_task_handle = tokio::spawn(async move {
            grpc_router
                .serve_with_incoming(tokio_stream::once(Ok::<_, std::io::Error>(server_stream)))
                .await
                .map_err(|err| eprintln!("Unexpected test gRPC server error: {err}"))
                .unwrap()
        });

        Self {
            grpc_server_task_handle,
            worker_state,
            client_channel,
            app_event_rx,
        }
    }
}

impl Drop for TestGrpcServer {
    fn drop(&mut self) {
        self.grpc_server_task_handle.abort();
    }
}

pub(crate) async fn setup_grpc_pool(_: PgPoolOptions, options: PgConnectOptions) -> PgPool {
    setup_pool(options).await
}

pub(crate) async fn create_client_channel(client_stream: DuplexStream) -> Channel {
    let mut client = Some(client_stream);
    let connector = service_fn(move |_: Uri| {
        let client = client.take();

        async move {
            if let Some(client) = client {
                Ok::<_, std::io::Error>(TokioIo::new(client))
            } else {
                Err(std::io::Error::other("Client already taken"))
            }
        }
    });

    Endpoint::try_from("http://[::]:50051")
        .expect("Failed to create channel")
        .connect_with_connector(connector)
        .await
        .expect("Failed to create client channel")
}

pub(crate) async fn make_grpc_test_server(pool: &PgPool) -> TestGrpcServer {
    initialize_users(pool).await;
    initialize_current_settings(pool)
        .await
        .expect("failed to initialize current settings for gRPC tests");

    // set test secret for generating JWT tokens
    let mut settings = Settings::get_current_settings();
    settings.secret_key = Some(TEST_SECRET_KEY.to_string());
    update_current_settings(pool, settings)
        .await
        .expect("Failed to update settings");

    let (client_stream, server_stream) = tokio::io::duplex(1024);
    let client_channel = create_client_channel(client_stream).await;

    let (app_event_tx, app_event_rx) = unbounded_channel::<AppEvent>();
    let worker_state = Arc::new(Mutex::new(WorkerState::new(app_event_tx)));
    let grpc_router =
        build_grpc_service_router(Server::builder(), pool.clone(), worker_state.clone())
            .await
            .expect("failed to build gRPC router");

    TestGrpcServer::new(
        server_stream,
        grpc_router,
        worker_state,
        client_channel,
        app_event_rx,
    )
    .await
}

pub(crate) fn create_yubibridge_jwt(username: &str) -> String {
    Claims::new(
        ClaimsType::YubiBridge,
        username.to_string(),
        String::new(),
        u32::MAX.into(),
    )
    .to_jwt()
    .expect("failed to generate YubiBridge token")
}

pub(crate) fn create_gateway_jwt(username: &str, client_id: &str) -> String {
    Claims::new(
        ClaimsType::Gateway,
        username.to_string(),
        client_id.to_string(),
        u32::MAX.into(),
    )
    .to_jwt()
    .expect("failed to generate gateway token")
}

pub(crate) fn add_authorization_metadata<T>(request: &mut Request<T>, token: &str) {
    request.metadata_mut().insert(
        AUTHORIZATION_HEADER,
        token.parse().expect("failed to encode authorization token"),
    );
}

pub(crate) fn add_worker_auth_metadata<T>(request: &mut Request<T>, username: &str) {
    add_authorization_metadata(request, &create_yubibridge_jwt(username));
}

pub(crate) fn worker_request<T>(message: T, username: &str) -> Request<T> {
    let mut request = Request::new(message);
    add_worker_auth_metadata(&mut request, username);
    request
}
