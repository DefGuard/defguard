use std::{
    collections::HashMap,
    io,
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

use defguard_common::db::{
    Id, NoId,
    models::{proxy::Proxy, settings::{Settings, initialize_current_settings}},
    setup_pool,
};
use defguard_core::{events::BidiStreamEvent, grpc::GatewayEvent};
use defguard_proto::proxy::{CoreRequest, CoreResponse, InitialInfo, core_response, proxy_server};
use defguard_version::server::DefguardVersionLayer;
use sqlx::{PgPool, postgres::PgConnectOptions};
use tokio::{
    net::UnixListener,
    sync::{
        Notify, broadcast,
        mpsc::{self, UnboundedReceiver, UnboundedSender},
        oneshot, watch,
    },
    task::JoinHandle,
    time::timeout,
};
use tokio_stream::{once, wrappers::UnboundedReceiverStream};
use tonic::{Request, Response, Status, Streaming, transport::Server};

use crate::{ProxyManager, ProxyManagerTestSupport, ProxyTxSet, handler::ProxyHandler};

pub(crate) const TEST_TIMEOUT: Duration = Duration::from_secs(5);

/// Minimum proxy version that passes `is_proxy_version_supported()`.
const MOCK_PROXY_VERSION: defguard_version::Version = defguard_version::Version::new(1, 6, 0);

macro_rules! assert_some {
    ($expr:expr, $message:literal) => {
        match $expr {
            Some(value) => value,
            None => panic!($message),
        }
    };
}

static TEST_ID: AtomicU64 = AtomicU64::new(0);

fn next_test_id() -> u64 {
    TEST_ID.fetch_add(1, Ordering::Relaxed)
}

fn unique_name(prefix: &str) -> String {
    format!("{prefix}-{}", next_test_id())
}

fn unique_socket_path() -> PathBuf {
    PathBuf::from(format!(
        "/tmp/defguard-proxy-manager-{}-{}.sock",
        std::process::id(),
        next_test_id()
    ))
}

pub(crate) fn unique_mock_proxy_socket_path() -> PathBuf {
    unique_socket_path()
}

// ---------------------------------------------------------------------------
// MockProxyService & state
// ---------------------------------------------------------------------------

/// Shared mutable state for `MockProxyService`.
///
/// Protocol note: the proxy bidi stream is the **reverse** of the gateway.
/// `ProxyClient::bidi` sends a `stream CoreResponse` from Core to Proxy and
/// receives a `stream CoreRequest` back. Therefore the mock server:
///   - receives `CoreResponse` messages from the handler (stored in `outbound_rx`)
///   - sends `CoreRequest` messages back (injected via `inbound_tx`)
struct MockProxyState {
    /// Messages sent by the handler (CoreResponse) forwarded here for assertions.
    outbound_tx: UnboundedSender<CoreResponse>,
    /// Receiver side for requests we want to inject into the handler (CoreRequest).
    inbound_rx: Mutex<Option<UnboundedReceiver<Result<CoreRequest, Status>>>>,
    /// One-shot notifier that fires once on the first connection.
    connected_tx: Mutex<Option<oneshot::Sender<()>>>,
    connection_count: AtomicU64,
    connection_notify: Notify,
    purge_count: AtomicU64,
    purge_notify: Notify,
}

impl MockProxyState {
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
            .ok_or_else(|| Status::failed_precondition("mock proxy already connected"))
    }

    fn note_purge(&self) {
        self.purge_count.fetch_add(1, Ordering::Relaxed);
        self.purge_notify.notify_waiters();
    }
}

#[derive(Clone)]
struct MockProxyService {
    state: Arc<MockProxyState>,
}

#[tonic::async_trait]
impl proxy_server::Proxy for MockProxyService {
    type BidiStream = UnboundedReceiverStream<Result<CoreRequest, Status>>;

    /// The bidi stream: Core sends `CoreResponse` messages; Proxy (mock) sends
    /// `CoreRequest` messages back.  We forward received `CoreResponse` items to
    /// `outbound_tx` and return an `UnboundedReceiverStream` driven by
    /// `inbound_rx` so tests can inject `CoreRequest` messages.
    async fn bidi(
        &self,
        request: Request<Streaming<CoreResponse>>,
    ) -> Result<Response<Self::BidiStream>, Status> {
        let inbound_rx = self.state.take_inbound_rx()?;
        self.state.notify_connected();

        let mut outbound_stream = request.into_inner();
        let outbound_tx = self.state.outbound_tx.clone();
        tokio::spawn(async move {
            while let Ok(Some(msg)) = outbound_stream.message().await {
                if outbound_tx.send(msg).is_err() {
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

// ---------------------------------------------------------------------------
// MockProxyHarness
// ---------------------------------------------------------------------------

pub(crate) struct MockProxyHarness {
    state: Arc<MockProxyState>,
    socket_path: PathBuf,
    /// Sender to inject `CoreRequest` messages into the live stream.
    inbound_tx: Option<UnboundedSender<Result<CoreRequest, Status>>>,
    /// Receiver of `CoreResponse` messages forwarded from the handler.
    outbound_rx: UnboundedReceiver<CoreResponse>,
    connected_rx: oneshot::Receiver<()>,
    server_task: Option<JoinHandle<Result<(), io::Error>>>,
    next_message_id: AtomicU64,
}

impl MockProxyHarness {
    pub(crate) async fn start() -> Self {
        Self::start_at(unique_socket_path()).await
    }

    pub(crate) async fn start_at(socket_path: PathBuf) -> Self {
        let _ = std::fs::remove_file(&socket_path);

        let listener =
            UnixListener::bind(&socket_path).expect("failed to bind mock proxy unix socket");
        let (outbound_tx, outbound_rx) = mpsc::unbounded_channel();
        let (inbound_tx, inbound_rx) = mpsc::unbounded_channel();
        let (connected_tx, connected_rx) = oneshot::channel();
        let state = Arc::new(MockProxyState {
            outbound_tx,
            inbound_rx: Mutex::new(Some(inbound_rx)),
            connected_tx: Mutex::new(Some(connected_tx)),
            connection_count: AtomicU64::new(0),
            connection_notify: Notify::new(),
            purge_count: AtomicU64::new(0),
            purge_notify: Notify::new(),
        });
        let service = MockProxyService {
            state: Arc::clone(&state),
        };

        let server_task = tokio::spawn(async move {
            let (stream, _) = listener.accept().await?;
            Server::builder()
                .layer(DefguardVersionLayer::new(MOCK_PROXY_VERSION))
                .add_service(proxy_server::ProxyServer::new(service))
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
            .expect("timed out waiting for mock proxy connection")
            .expect("mock proxy connection notifier dropped");
    }

    pub(crate) fn connection_count(&self) -> u64 {
        self.state.connection_count.load(Ordering::Relaxed)
    }

    pub(crate) async fn wait_for_connection_count(&self, expected_count: u64) {
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
        .expect("timed out waiting for mock proxy connection count");
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

    /// Inject a `CoreRequest` message into the running bidi stream.
    pub(crate) fn send_request(&self, request: CoreRequest) {
        self.inbound_tx
            .as_ref()
            .expect("mock proxy inbound channel already closed")
            .send(Ok(request))
            .expect("failed to inject mock proxy request");
    }

    pub(crate) fn send_stream_error(&self, status: Status) {
        self.inbound_tx
            .as_ref()
            .expect("mock proxy inbound channel already closed")
            .send(Err(status))
            .expect("failed to inject inbound stream error");
    }

    /// Close the inbound stream, causing the handler's `message_loop` to exit.
    pub(crate) fn close_stream(&mut self) {
        self.inbound_tx.take();
    }

    /// Receive the next `CoreResponse` message sent by the handler.
    pub(crate) async fn recv_outbound(&mut self) -> CoreResponse {
        timeout(TEST_TIMEOUT, self.outbound_rx.recv())
            .await
            .expect("timed out waiting for outbound response")
            .expect("mock proxy outbound response channel closed unexpectedly")
    }

    /// Assert that no outbound response arrives within a short window.
    pub(crate) async fn expect_no_outbound(&mut self) {
        if let Ok(Some(_message)) =
            timeout(Duration::from_millis(200), self.outbound_rx.recv()).await
        {
            panic!("unexpected outbound response");
        }
    }

    /// Receive the `InitialInfo` message that the handler always sends first.
    pub(crate) async fn recv_initial_info(&mut self) -> InitialInfo {
        let response = self.recv_outbound().await;
        match response.payload {
            Some(core_response::Payload::InitialInfo(info)) => info,
            _ => panic!("expected InitialInfo as first message from handler"),
        }
    }

    pub(crate) async fn expect_server_finished(mut self) {
        let server_task = assert_some!(
            self.server_task.take(),
            "mock proxy server task already taken"
        );
        let server_result = timeout(TEST_TIMEOUT, server_task)
            .await
            .expect("timed out waiting for mock proxy server to finish")
            .expect("mock proxy server task panicked");
        server_result.expect("mock proxy server exited with error");
    }
}

impl Drop for MockProxyHarness {
    fn drop(&mut self) {
        if let Some(server_task) = self.server_task.take() {
            server_task.abort();
        }
        let _ = std::fs::remove_file(&self.socket_path);
    }
}

// ---------------------------------------------------------------------------
// HandlerTestContext
// ---------------------------------------------------------------------------

pub(crate) struct HandlerTestContext {
    pub(crate) pool: PgPool,
    pub(crate) proxy: Proxy<Id>,
    pub(crate) wireguard_tx: broadcast::Sender<GatewayEvent>,
    pub(crate) bidi_events_rx: UnboundedReceiver<BidiStreamEvent>,
    pub(crate) mock_proxy: Option<MockProxyHarness>,
    handler_task: Option<JoinHandle<Result<(), crate::error::ProxyError>>>,
    shutdown_tx: Option<oneshot::Sender<bool>>,
}

impl HandlerTestContext {
    pub(crate) async fn new(options: PgConnectOptions) -> Self {
        let pool = setup_pool(options).await;
        initialize_current_settings(&pool)
            .await
            .expect("failed to initialize global settings for proxy handler tests");
        Settings::initialize_runtime_defaults(&pool)
            .await
            .expect("failed to initialize runtime default settings for proxy handler tests");
        // Reload settings after runtime defaults have been applied.
        initialize_current_settings(&pool)
            .await
            .expect("failed to reload settings after runtime defaults");

        let proxy = create_proxy(&pool).await;

        let (wireguard_tx, _) = broadcast::channel(16);
        let (bidi_events_tx, bidi_events_rx) = mpsc::unbounded_channel::<BidiStreamEvent>();
        let tx_set = ProxyTxSet::new(wireguard_tx.clone(), bidi_events_tx);

        let (_, certs_rx) = watch::channel(Arc::new(HashMap::new()));
        let incompatible_components = Arc::new(std::sync::RwLock::new(
            defguard_core::version::IncompatibleComponents::default(),
        ));

        let mut mock_proxy = MockProxyHarness::start().await;

        let url = reqwest::Url::parse(&format!("http://{}:{}", proxy.address, proxy.port))
            .expect("failed to build proxy url");

        let remote_mfa_responses = Arc::default();
        let sessions = Arc::default();
        let (shutdown_tx, shutdown_rx) = oneshot::channel::<bool>();

        let handler = ProxyHandler::new_with_test_socket(
            pool.clone(),
            url,
            &tx_set,
            remote_mfa_responses,
            sessions,
            Arc::new(tokio::sync::Mutex::new(shutdown_rx)),
            proxy.id,
            axum_extra::extract::cookie::Key::derive_from(
                b"test-secret-key-at-least-64-bytes-long-xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
            ),
            mock_proxy.socket_path(),
        );

        let incompatible_components_clone = incompatible_components.clone();
        let handler_task = tokio::spawn(async move {
            handler
                .run_once(tx_set, incompatible_components_clone, certs_rx)
                .await
        });

        mock_proxy.wait_connected().await;

        Self {
            pool,
            proxy,
            wireguard_tx,
            bidi_events_rx,
            mock_proxy: Some(mock_proxy),
            handler_task: Some(handler_task),
            shutdown_tx: Some(shutdown_tx),
        }
    }

    pub(crate) fn mock_proxy(&self) -> &MockProxyHarness {
        self.mock_proxy
            .as_ref()
            .expect("mock proxy already taken from context")
    }

    pub(crate) fn mock_proxy_mut(&mut self) -> &mut MockProxyHarness {
        self.mock_proxy
            .as_mut()
            .expect("mock proxy already taken from context")
    }

    pub(crate) async fn reload_proxy(&self) -> Proxy<Id> {
        Proxy::find_by_id(&self.pool, self.proxy.id)
            .await
            .expect("failed to query proxy from database")
            .expect("expected proxy in database")
    }

    pub(crate) async fn finish(mut self) -> MockProxyHarness {
        let mut mock_proxy = assert_some!(
            self.mock_proxy.take(),
            "mock proxy already taken from context"
        );
        mock_proxy.close_stream();
        let handler_task = self
            .handler_task
            .as_mut()
            .expect("handler task already taken from context");
        let result = timeout(TEST_TIMEOUT, handler_task)
            .await
            .expect("timed out waiting for handler task to finish")
            .expect("proxy handler task panicked");
        result.expect("proxy handler returned an unexpected error");
        self.handler_task.take();
        mock_proxy
    }

    pub(crate) async fn finish_after_error(mut self) -> MockProxyHarness {
        let mock_proxy = assert_some!(
            self.mock_proxy.take(),
            "mock proxy already taken from context"
        );
        let handler_task = self
            .handler_task
            .as_mut()
            .expect("handler task already taken from context");
        let result = timeout(TEST_TIMEOUT, handler_task)
            .await
            .expect("timed out waiting for handler task to finish after stream error")
            .expect("proxy handler task panicked after stream error");
        result.expect("proxy handler returned an unexpected error after stream error");
        self.handler_task.take();
        mock_proxy
    }
}

impl Drop for HandlerTestContext {
    fn drop(&mut self) {
        if let Some(handler_task) = self.handler_task.take() {
            handler_task.abort();
        }
    }
}

// ---------------------------------------------------------------------------
// ManagerTestContext
// ---------------------------------------------------------------------------

pub(crate) struct ManagerTestContext {
    pub(crate) pool: PgPool,
    control: ProxyManagerTestSupport,
    /// Sender for the proxy control channel used by `ProxyManager`.
    pub(crate) proxy_control_tx:
        tokio::sync::mpsc::Sender<defguard_common::types::proxy::ProxyControlMessage>,
    manager_task: Option<JoinHandle<Result<(), crate::error::ProxyError>>>,
}

impl ManagerTestContext {
    pub(crate) async fn new(options: PgConnectOptions) -> Self {
        let pool = setup_pool(options).await;
        initialize_current_settings(&pool)
            .await
            .expect("failed to initialize global settings for proxy manager tests");

        let (proxy_control_tx, _proxy_control_rx_placeholder) =
            tokio::sync::mpsc::channel::<defguard_common::types::proxy::ProxyControlMessage>(16);

        let control = ProxyManagerTestSupport::default();

        Self {
            pool,
            control,
            proxy_control_tx,
            manager_task: None,
        }
    }

    pub(crate) fn register_proxy_mock(&self, proxy: &Proxy<Id>, mock_proxy: &MockProxyHarness) {
        let url = format!("http://{}:{}/", proxy.address, proxy.port);
        self.register_proxy_socket_path(url, mock_proxy.socket_path());
    }

    pub(crate) fn register_proxy_socket_path(&self, proxy_url: String, socket_path: PathBuf) {
        self.control.register_proxy_url(proxy_url, socket_path);
    }

    pub(crate) fn handler_spawn_attempt_count(&self, proxy_id: Id) -> u64 {
        self.control.handler_spawn_attempt_count(proxy_id)
    }

    pub(crate) async fn wait_for_handler_spawn_attempt_count(
        &self,
        proxy_id: Id,
        expected_count: u64,
    ) {
        timeout(
            TEST_TIMEOUT,
            self.control
                .wait_for_handler_spawn_attempt_count(proxy_id, expected_count),
        )
        .await
        .expect("timed out waiting for proxy manager handler spawn attempt");
    }

    pub(crate) async fn start(&mut self) {
        assert!(self.manager_task.is_none(), "proxy manager already started");

        let (wireguard_tx, _) = broadcast::channel(16);
        let (bidi_events_tx, _bidi_events_rx) = mpsc::unbounded_channel::<BidiStreamEvent>();
        let tx_set = ProxyTxSet::new(wireguard_tx, bidi_events_tx);

        let incompatible_components = Arc::new(std::sync::RwLock::new(
            defguard_core::version::IncompatibleComponents::default(),
        ));

        let (proxy_control_tx, proxy_control_rx) =
            tokio::sync::mpsc::channel::<defguard_common::types::proxy::ProxyControlMessage>(16);
        self.proxy_control_tx = proxy_control_tx;

        let manager = ProxyManager::new_for_test(
            self.pool.clone(),
            tx_set,
            incompatible_components,
            proxy_control_rx,
            "test-secret-key-at-least-64-bytes-long-xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
            self.control.clone(),
        );
        let manager_task = tokio::spawn(async move { manager.run().await });

        // No PgListener in proxy manager — just yield to let the manager start.
        tokio::task::yield_now().await;

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
                Err(err) => panic!("proxy manager task panicked: {err}"),
                Ok(Ok(())) => {}
                Ok(Err(err)) => panic!("proxy manager exited with error: {err}"),
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

// ---------------------------------------------------------------------------
// Database helpers
// ---------------------------------------------------------------------------

pub(crate) async fn reload_proxy(pool: &PgPool, proxy_id: Id) -> Proxy<Id> {
    Proxy::find_by_id(pool, proxy_id)
        .await
        .expect("failed to query proxy from database")
        .expect("expected proxy in database")
}

pub(crate) async fn wait_for_proxy_connection_state(
    pool: &PgPool,
    proxy_id: Id,
    expected_connected: bool,
) -> Proxy<Id> {
    timeout(TEST_TIMEOUT, async {
        loop {
            let proxy = reload_proxy(pool, proxy_id).await;
            // Proxy is considered connected when connected_at > disconnected_at.
            let is_connected = match (proxy.connected_at, proxy.disconnected_at) {
                (Some(connected), Some(disconnected)) => connected > disconnected,
                (Some(_), None) => true,
                _ => false,
            };
            if is_connected == expected_connected {
                return proxy;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("timed out waiting for proxy connection state change")
}

pub(crate) async fn create_proxy(pool: &PgPool) -> Proxy<Id> {
    create_proxy_with_enabled(pool, true).await
}

pub(crate) async fn create_proxy_with_enabled(pool: &PgPool, enabled: bool) -> Proxy<Id> {
    build_proxy_with_enabled(enabled)
        .save(pool)
        .await
        .expect("failed to create test proxy")
}

pub(crate) fn build_proxy_with_enabled(enabled: bool) -> Proxy<NoId> {
    let port = 50_000 + i32::try_from(next_test_id() % 15_000).expect("port offset fits in i32");
    let mut proxy = Proxy::new(
        unique_name("proxy"),
        "127.0.0.1".to_string(),
        port,
        "test-admin".to_string(),
    );
    proxy.enabled = enabled;
    proxy
}
