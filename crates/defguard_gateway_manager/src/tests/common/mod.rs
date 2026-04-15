use std::{
    collections::HashMap,
    env::temp_dir,
    io,
    path::PathBuf,
    process,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU16, AtomicU64, Ordering},
    },
    time::Duration,
};

use defguard_common::{
    db::{
        Id, NoId,
        models::{
            gateway::Gateway, settings::initialize_current_settings, wireguard::WireguardNetwork,
        },
        setup_pool,
    },
    messages::peer_stats_update::PeerStatsUpdate,
};
use defguard_core::grpc::GatewayEvent;
use defguard_proto::gateway::{CoreRequest, CoreResponse, PeerStats, core_request, gateway_server};
use prost_types::Timestamp;
use sqlx::{PgPool, postgres::PgConnectOptions};
use tokio::{
    net::UnixListener,
    sync::{
        Notify, broadcast,
        mpsc::{self, UnboundedReceiver, UnboundedSender},
        oneshot, watch,
    },
    task::JoinHandle,
    time::{sleep, timeout},
};
use tokio_stream::{once, wrappers::UnboundedReceiverStream};
use tonic::{Request, Response, Status, Streaming, transport::Server};

use crate::{GatewayManager, GatewayManagerTestSupport, GatewayTxSet, handler::GatewayHandler};

const TEST_TIMEOUT: Duration = Duration::from_secs(2);

macro_rules! assert_some {
    ($expr:expr, $message:literal) => {
        match $expr {
            Some(value) => value,
            None => panic!($message),
        }
    };
}

static TEST_ID: AtomicU16 = AtomicU16::new(0);

fn next_test_id() -> u16 {
    TEST_ID.fetch_add(1, Ordering::Relaxed)
}

fn unique_name(prefix: &str) -> String {
    format!("{prefix}-{}", next_test_id())
}

fn unique_socket_path() -> PathBuf {
    PathBuf::from(format!(
        "{}/defguard-gateway-manager-{}-{}.sock",
        temp_dir().display(),
        process::id(),
        next_test_id()
    ))
}

pub(crate) fn unique_mock_gateway_socket_path() -> PathBuf {
    unique_socket_path()
}

#[derive(Clone)]
struct MockGatewayService {
    state: Arc<MockGatewayState>,
}

struct MockGatewayState {
    outbound_tx: UnboundedSender<CoreResponse>,
    inbound_rx: Mutex<Option<UnboundedReceiver<Result<CoreRequest, Status>>>>,
    connected_tx: Mutex<Option<oneshot::Sender<()>>>,
    connection_count: AtomicU16,
    connection_notify: Notify,
    purge_count: AtomicU16,
    purge_notify: Notify,
}

impl MockGatewayState {
    fn notify_connected(&self) {
        self.connection_count.fetch_add(1, Ordering::Relaxed);
        self.connection_notify.notify_waiters();

        if let Some(tx) = self
            .connected_tx
            .lock()
            .expect("failed to lock connected notifier")
            .take()
        {
            let _ = tx.send(());
        }
    }

    fn take_inbound_rx(&self) -> Result<UnboundedReceiver<Result<CoreRequest, Status>>, Status> {
        self.inbound_rx
            .lock()
            .expect("failed to lock inbound receiver")
            .take()
            .ok_or_else(|| Status::failed_precondition("mock gateway already connected"))
    }

    fn note_purge(&self) {
        self.purge_count.fetch_add(1, Ordering::Relaxed);
        self.purge_notify.notify_waiters();
    }
}

#[tonic::async_trait]
impl gateway_server::Gateway for MockGatewayService {
    type BidiStream = UnboundedReceiverStream<Result<CoreRequest, Status>>;

    async fn bidi(
        &self,
        request: Request<Streaming<CoreResponse>>,
    ) -> Result<Response<Self::BidiStream>, Status> {
        let inbound_rx = self.state.take_inbound_rx()?;
        self.state.notify_connected();

        let mut outbound_stream = request.into_inner();
        let outbound_tx = self.state.outbound_tx.clone();
        tokio::spawn(async move {
            while let Ok(Some(response)) = outbound_stream.message().await {
                if outbound_tx.send(response).is_err() {
                    break;
                }
            }
        });

        Ok(Response::new(UnboundedReceiverStream::new(inbound_rx)))
    }

    async fn purge(&self, _request: Request<()>) -> Result<Response<()>, Status> {
        self.state.note_purge();
        Ok(Response::new(()))
    }
}

pub(crate) struct MockGatewayHarness {
    state: Arc<MockGatewayState>,
    socket_path: PathBuf,
    inbound_tx: Option<UnboundedSender<Result<CoreRequest, Status>>>,
    outbound_rx: UnboundedReceiver<CoreResponse>,
    connected_rx: oneshot::Receiver<()>,
    server_task: Option<JoinHandle<Result<(), io::Error>>>,
    next_message_id: AtomicU64,
}

impl MockGatewayHarness {
    pub(crate) async fn start() -> Self {
        Self::start_at(unique_socket_path()).await
    }

    pub(crate) async fn start_at(socket_path: PathBuf) -> Self {
        let _ = std::fs::remove_file(&socket_path);

        let listener =
            UnixListener::bind(&socket_path).expect("failed to bind mock gateway unix socket");
        let (outbound_tx, outbound_rx) = mpsc::unbounded_channel();
        let (inbound_tx, inbound_rx) = mpsc::unbounded_channel();
        let (connected_tx, connected_rx) = oneshot::channel();
        let state = Arc::new(MockGatewayState {
            outbound_tx,
            inbound_rx: Mutex::new(Some(inbound_rx)),
            connected_tx: Mutex::new(Some(connected_tx)),
            connection_count: AtomicU16::new(0),
            connection_notify: Notify::new(),
            purge_count: AtomicU16::new(0),
            purge_notify: Notify::new(),
        });
        let service = MockGatewayService {
            state: Arc::clone(&state),
        };

        let server_task = tokio::spawn(async move {
            let (stream, _) = listener.accept().await?;
            Server::builder()
                .add_service(gateway_server::GatewayServer::new(service))
                .serve_with_incoming(once(Ok::<_, io::Error>(stream)))
                .await
                .map_err(io::Error::other)
        });

        Self {
            state,
            socket_path,
            inbound_tx: Some(inbound_tx),
            outbound_rx,
            connected_rx,
            server_task: Some(server_task),
            next_message_id: AtomicU64::new(1),
        }
    }

    pub(crate) fn socket_path(&self) -> PathBuf {
        self.socket_path.clone()
    }

    pub(crate) async fn wait_connected(&mut self) {
        timeout(TEST_TIMEOUT, &mut self.connected_rx)
            .await
            .expect("timed out waiting for mock gateway connection")
            .expect("mock gateway connection notifier dropped");
    }

    pub(crate) fn connection_count(&self) -> u16 {
        self.state.connection_count.load(Ordering::Relaxed)
    }

    pub(crate) async fn wait_for_connection_count(&self, expected_count: u16) {
        timeout(TEST_TIMEOUT, async {
            loop {
                if self.connection_count() >= expected_count {
                    return;
                }

                let notified = self.state.connection_notify.notified();
                if self.connection_count() >= expected_count {
                    return;
                }

                notified.await;
            }
        })
        .await
        .expect("timed out waiting for mock gateway connection count");
    }

    pub(crate) async fn wait_purged(&self) {
        timeout(TEST_TIMEOUT, async {
            loop {
                if self.state.purge_count.load(Ordering::Relaxed) > 0 {
                    return;
                }

                let notified = self.state.purge_notify.notified();
                if self.state.purge_count.load(Ordering::Relaxed) > 0 {
                    return;
                }

                notified.await;
            }
        })
        .await
        .expect("timed out waiting for purge request");
    }

    pub(crate) fn send_config_request(&self) {
        self.send_request(CoreRequest {
            id: self.next_message_id.fetch_add(1, Ordering::Relaxed),
            payload: Some(core_request::Payload::ConfigRequest(())),
        });
    }

    pub(crate) fn send_peer_stats(&self, peer_stats: PeerStats) {
        self.send_request(CoreRequest {
            id: self.next_message_id.fetch_add(1, Ordering::Relaxed),
            payload: Some(core_request::Payload::PeerStats(peer_stats)),
        });
    }

    pub(crate) fn send_stream_error(&self, status: Status) {
        self.inbound_tx
            .as_ref()
            .expect("mock gateway inbound channel already closed")
            .send(Err(status))
            .expect("failed to inject inbound stream error");
    }

    fn send_request(&self, request: CoreRequest) {
        self.inbound_tx
            .as_ref()
            .expect("mock gateway inbound channel already closed")
            .send(Ok(request))
            .expect("failed to inject mock gateway request");
    }

    pub(crate) fn close_stream(&mut self) {
        self.inbound_tx.take();
    }

    pub(crate) async fn recv_outbound(&mut self) -> CoreResponse {
        timeout(TEST_TIMEOUT, self.outbound_rx.recv())
            .await
            .expect("timed out waiting for outbound response")
            .expect("mock gateway outbound response channel closed unexpectedly")
    }

    pub(crate) async fn expect_no_outbound(&mut self) {
        if let Ok(Some(_message)) =
            timeout(Duration::from_millis(200), self.outbound_rx.recv()).await
        {
            panic!("unexpected outbound response");
        }
    }

    pub(crate) async fn expect_server_finished(mut self) {
        let server_task = assert_some!(
            self.server_task.take(),
            "mock gateway server task already taken"
        );
        let server_result = timeout(TEST_TIMEOUT, server_task)
            .await
            .expect("timed out waiting for mock gateway server to finish")
            .expect("mock gateway server task panicked");
        server_result.expect("mock gateway server exited with error");
    }
}

impl Drop for MockGatewayHarness {
    fn drop(&mut self) {
        if let Some(server_task) = self.server_task.take() {
            server_task.abort();
        }
        let _ = std::fs::remove_file(&self.socket_path);
    }
}

pub(crate) struct ManagerTestContext {
    pub(crate) pool: PgPool,
    control: GatewayManagerTestSupport,
    manager_task: Option<JoinHandle<Result<(), anyhow::Error>>>,
}

impl ManagerTestContext {
    pub(crate) async fn new(options: PgConnectOptions) -> Self {
        let pool = setup_pool(options).await;
        initialize_current_settings(&pool)
            .await
            .expect("failed to initialize global settings for gateway manager tests");

        Self {
            pool,
            control: GatewayManagerTestSupport::default(),
            manager_task: None,
        }
    }

    pub(crate) fn register_gateway_mock(
        &self,
        gateway: &Gateway<Id>,
        mock_gateway: &MockGatewayHarness,
    ) {
        self.register_gateway_url(gateway.url(), mock_gateway);
    }

    pub(crate) fn register_gateway_url(
        &self,
        gateway_url: String,
        mock_gateway: &MockGatewayHarness,
    ) {
        self.register_gateway_socket_path(gateway_url, mock_gateway.socket_path());
    }

    pub(crate) fn register_gateway_socket_path(&self, gateway_url: String, socket_path: PathBuf) {
        self.control.register_gateway_url(gateway_url, socket_path);
    }

    pub(crate) fn handler_spawn_attempt_count(&self, gateway_id: Id) -> u64 {
        self.control.handler_spawn_attempt_count(gateway_id)
    }

    pub(crate) fn handler_connection_attempt_count(&self, gateway_id: Id) -> u64 {
        self.control.handler_connection_attempt_count(gateway_id)
    }

    pub(crate) fn gateway_notification_count(&self, gateway_id: Id) -> u64 {
        self.control.gateway_notification_count(gateway_id)
    }

    pub(crate) async fn wait_for_handler_spawn_attempt_count(
        &self,
        gateway_id: Id,
        expected_count: u64,
    ) {
        timeout(
            TEST_TIMEOUT,
            self.control
                .wait_for_handler_spawn_attempt_count(gateway_id, expected_count),
        )
        .await
        .expect("timed out waiting for gateway manager handler spawn attempt");
    }

    pub(crate) async fn wait_for_handler_connection_attempt_count(
        &self,
        gateway_id: Id,
        expected_count: u64,
    ) {
        timeout(
            TEST_TIMEOUT,
            self.control
                .wait_for_handler_connection_attempt_count(gateway_id, expected_count),
        )
        .await
        .expect("timed out waiting for gateway manager handler connection attempt");
    }

    pub(crate) async fn wait_for_gateway_notification_count(
        &self,
        gateway_id: Id,
        expected_count: u64,
    ) {
        timeout(
            TEST_TIMEOUT,
            self.control
                .wait_for_gateway_notification_count(gateway_id, expected_count),
        )
        .await
        .expect("timed out waiting for gateway manager database notification");
    }

    pub(crate) async fn start(&mut self) {
        assert!(
            self.manager_task.is_none(),
            "gateway manager already started"
        );

        let (events_tx, _) = broadcast::channel(16);
        let (peer_stats_tx, _peer_stats_rx) = mpsc::unbounded_channel();
        let tx = GatewayTxSet::new(events_tx, peer_stats_tx);
        let mut manager = GatewayManager::new(self.pool.clone(), tx, self.control.clone());
        let manager_task = tokio::spawn(async move { manager.run().await });

        timeout(TEST_TIMEOUT, self.control.wait_until_listener_ready())
            .await
            .expect("timed out waiting for gateway manager listener to become ready");
        self.manager_task = Some(manager_task);
    }

    pub(crate) fn set_retry_delay(&self, retry_delay: Duration) {
        self.control.set_retry_delay(retry_delay);
    }

    pub(crate) async fn finish(mut self) {
        if let Some(manager_task) = self.manager_task.take() {
            manager_task.abort();

            match manager_task.await {
                Err(err) if err.is_cancelled() => {}
                Err(err) => panic!("gateway manager task panicked: {err}"),
                Ok(Ok(())) => {}
                Ok(Err(err)) => panic!("gateway manager exited with error: {err}"),
            }
        }

        self.pool.close().await;
    }
}

impl Drop for ManagerTestContext {
    fn drop(&mut self) {
        if let Some(manager_task) = self.manager_task.take() {
            manager_task.abort();
        }
    }
}

pub(crate) struct HandlerTestContext {
    pub(crate) pool: PgPool,
    pub(crate) network: WireguardNetwork<Id>,
    pub(crate) gateway: Gateway<Id>,
    pub(crate) peer_stats_rx: UnboundedReceiver<PeerStatsUpdate>,
    events_tx: Option<broadcast::Sender<GatewayEvent>>,
    pub(crate) mock_gateway: Option<MockGatewayHarness>,
    handler_task: Option<JoinHandle<anyhow::Result<()>>>,
}

impl HandlerTestContext {
    pub(crate) async fn new(options: PgConnectOptions) -> Self {
        let (events_tx, _) = broadcast::channel(16);
        Self::new_with_events_tx(options, events_tx).await
    }

    pub(crate) async fn new_with_events_tx(
        options: PgConnectOptions,
        events_tx: broadcast::Sender<GatewayEvent>,
    ) -> Self {
        let pool = setup_pool(options).await;
        initialize_current_settings(&pool)
            .await
            .expect("failed to initialize global settings for gateway handler tests");
        let network = create_network(&pool).await;
        let gateway = create_gateway(&pool, network.id).await;
        let (peer_stats_tx, peer_stats_rx) = mpsc::unbounded_channel();
        let (_, certs_rx) = watch::channel(Arc::new(HashMap::new()));
        let mut mock_gateway = MockGatewayHarness::start().await;
        let mut handler = GatewayHandler::new_with_test_socket(
            gateway.clone(),
            pool.clone(),
            events_tx.clone(),
            peer_stats_tx,
            certs_rx,
            mock_gateway.socket_path(),
        )
        .expect("failed to create gateway handler");
        let handler_task = tokio::spawn(async move { handler.handle_connection_once().await });

        mock_gateway.wait_connected().await;

        Self {
            pool,
            network,
            gateway,
            peer_stats_rx,
            events_tx: Some(events_tx),
            mock_gateway: Some(mock_gateway),
            handler_task: Some(handler_task),
        }
    }

    pub(crate) fn events_tx(&self) -> &broadcast::Sender<GatewayEvent> {
        self.events_tx
            .as_ref()
            .expect("events sender already taken from context")
    }

    pub(crate) fn mock_gateway(&self) -> &MockGatewayHarness {
        self.mock_gateway
            .as_ref()
            .expect("mock gateway already taken from context")
    }

    pub(crate) fn mock_gateway_mut(&mut self) -> &mut MockGatewayHarness {
        self.mock_gateway
            .as_mut()
            .expect("mock gateway already taken from context")
    }

    pub(crate) async fn reload_gateway(&self) -> Gateway<Id> {
        Gateway::find_by_id(&self.pool, self.gateway.id)
            .await
            .expect("failed to query gateway from database")
            .expect("expected gateway in database")
    }

    pub(crate) async fn create_other_network(&self) -> WireguardNetwork<Id> {
        create_network(&self.pool).await
    }

    pub(crate) async fn expect_no_peer_stats(&mut self) {
        if let Ok(Some(message)) =
            timeout(Duration::from_millis(200), self.peer_stats_rx.recv()).await
        {
            panic!("unexpected peer stats update: {message:?}");
        }
    }

    pub(crate) async fn recv_peer_stats(&mut self) -> PeerStatsUpdate {
        timeout(TEST_TIMEOUT, self.peer_stats_rx.recv())
            .await
            .expect("timed out waiting for peer stats update")
            .expect("peer stats channel unexpectedly closed")
    }

    pub(crate) async fn complete_config_handshake(&mut self) -> Gateway<Id> {
        let initial_event_receivers = self.events_tx().receiver_count();
        self.mock_gateway().send_config_request();
        let _ = self.mock_gateway_mut().recv_outbound().await;
        let connected_gateway =
            wait_for_gateway_connection_state(&self.pool, self.gateway.id, true).await;
        timeout(TEST_TIMEOUT, async {
            while self.events_tx().receiver_count() <= initial_event_receivers {
                sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("timed out waiting for gateway updates handler subscription");
        tokio::task::yield_now().await;
        connected_gateway
    }

    pub(crate) async fn finish(mut self) -> MockGatewayHarness {
        let mut mock_gateway = assert_some!(
            self.mock_gateway.take(),
            "mock gateway already taken from context"
        );
        mock_gateway.close_stream();
        let handler_task = self
            .handler_task
            .as_mut()
            .expect("handler task already taken from context");
        let result = timeout(TEST_TIMEOUT, handler_task)
            .await
            .expect("timed out waiting for handler task to finish")
            .expect("gateway handler task panicked");
        result.expect("gateway handler returned an unexpected error");
        self.handler_task.take();
        self.events_tx.take();
        mock_gateway
    }

    pub(crate) async fn finish_after_error(mut self) -> MockGatewayHarness {
        let mock_gateway = assert_some!(
            self.mock_gateway.take(),
            "mock gateway already taken from context"
        );
        let handler_task = self
            .handler_task
            .as_mut()
            .expect("handler task already taken from context");
        let result = timeout(TEST_TIMEOUT, handler_task)
            .await
            .expect("timed out waiting for handler task to finish after stream error")
            .expect("gateway handler task panicked after stream error");
        result.expect("gateway handler returned an unexpected error after stream error");
        self.handler_task.take();
        self.events_tx.take();
        mock_gateway
    }
}

impl Drop for HandlerTestContext {
    fn drop(&mut self) {
        if let Some(handler_task) = self.handler_task.take() {
            handler_task.abort();
        }
    }
}

pub(crate) async fn reload_gateway(pool: &PgPool, gateway_id: Id) -> Gateway<Id> {
    Gateway::find_by_id(pool, gateway_id)
        .await
        .expect("failed to query gateway from database")
        .expect("expected gateway in database")
}

pub(crate) async fn wait_for_gateway_connection_state(
    pool: &PgPool,
    gateway_id: Id,
    expected_connected: bool,
) -> Gateway<Id> {
    timeout(TEST_TIMEOUT, async {
        loop {
            let gateway = reload_gateway(pool, gateway_id).await;
            if gateway.is_connected() == expected_connected {
                return gateway;
            }

            sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("timed out waiting for gateway connection state change")
}

pub(crate) fn build_peer_stats(endpoint: &str) -> PeerStats {
    PeerStats {
        public_key: "peer-public-key".to_string(),
        endpoint: endpoint.to_string(),
        upload: 123,
        download: 456,
        keepalive_interval: 25,
        latest_handshake: Some(Timestamp {
            seconds: 1_700_000_000,
            nanos: 0,
        }),
        allowed_ips: "10.10.0.2/32".to_string(),
    }
}

pub(crate) async fn create_network(pool: &PgPool) -> WireguardNetwork<Id> {
    let network = WireguardNetwork::new(
        unique_name("network"),
        51820,
        "198.51.100.10".to_string(),
        None,
        Vec::new(),
        false,
        false,
        false,
        Default::default(),
        Default::default(),
    )
    .try_set_address("10.10.0.1/24")
    .expect("failed to set network address");
    network
        .save(pool)
        .await
        .expect("failed to create test network")
}

pub(crate) async fn create_gateway(pool: &PgPool, location_id: Id) -> Gateway<Id> {
    create_gateway_with_enabled(pool, location_id, true).await
}

pub(crate) async fn create_gateway_with_enabled(
    pool: &PgPool,
    location_id: Id,
    enabled: bool,
) -> Gateway<Id> {
    let gateway = build_gateway_with_enabled(location_id, enabled);
    gateway
        .save(pool)
        .await
        .expect("failed to create test gateway")
}

pub(crate) fn build_gateway_with_enabled(location_id: Id, enabled: bool) -> Gateway<NoId> {
    let port = 20_000 + i32::from(next_test_id() % 40_000);
    let mut gateway = Gateway::new(
        location_id,
        unique_name("gateway"),
        "127.0.0.1".to_string(),
        port,
        "test-admin".to_string(),
    );
    gateway.enabled = enabled;
    gateway
}
