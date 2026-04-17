use std::{
    collections::HashMap,
    env::temp_dir,
    io,
    path::PathBuf,
    process,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU16, Ordering},
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use axum::{
    extract::{Form, State},
    response::Json,
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use defguard_common::db::{
    Id, NoId,
    models::{
        proxy::Proxy,
        settings::{Settings, initialize_current_settings},
    },
    setup_pool,
};
use defguard_core::{events::BidiStreamEvent, grpc::GatewayEvent};
use defguard_proto::proxy::{
    AcmeChallenge, AcmeIssueEvent, CoreRequest, CoreResponse, InitialInfo, core_response,
    proxy_server,
};
use defguard_version::server::DefguardVersionLayer;
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use rsa::{RsaPrivateKey, pkcs8::EncodePrivateKey, traits::PublicKeyParts};
use sqlx::{PgPool, postgres::PgConnectOptions};
use tokio::{
    net::{TcpListener, UnixListener},
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

use crate::{ProxyManager, ProxyManagerTestSupport, ProxyTxSet, handler::ProxyHandler};

pub(crate) const TEST_TIMEOUT: Duration = Duration::from_secs(10);

pub(crate) const CORE_RESPONSE_TIMEOUT: Duration = Duration::from_millis(200);

pub(crate) const PROXY_CONNECT_DELAY: Duration = Duration::from_millis(20);

pub(crate) const RECEIVE_TIMEOUT: Duration = Duration::from_secs(5);

/// Minimum proxy version that passes `is_proxy_version_supported()`.
const MOCK_PROXY_VERSION: defguard_version::Version = defguard_version::Version::new(2, 0, 0);

macro_rules! assert_some {
    ($expr:expr, $message:literal) => {
        match $expr {
            Some(value) => value,
            None => panic!($message),
        }
    };
}

/// Returns a per-process unique Unix socket path for a mock proxy.
/// Returns a unique socket path for a mock proxy harness instance.
///
/// With `cargo nextest` each test runs in its own process, so combining the
/// PID with a per-process counter gives a unique path for every harness
/// created within the same test.
pub(crate) fn mock_proxy_socket_path() -> PathBuf {
    static SOCK_CTR: AtomicU16 = AtomicU16::new(0);
    let socket_number = SOCK_CTR.fetch_add(1, Ordering::Relaxed);
    PathBuf::from(format!(
        "{}/defguard-proxy-manager-{}-{}.sock",
        temp_dir().display(),
        process::id(),
        socket_number,
    ))
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
    connection_count: AtomicU16,
    connection_notify: Notify,
    purge_count: AtomicU16,
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
    type TriggerAcmeStream =
        std::pin::Pin<Box<dyn tokio_stream::Stream<Item = Result<AcmeIssueEvent, Status>> + Send>>;

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

    async fn trigger_acme(
        &self,
        _request: Request<AcmeChallenge>,
    ) -> Result<Response<Self::TriggerAcmeStream>, Status> {
        Err(Status::unimplemented(
            "trigger_acme not implemented in mock",
        ))
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
}

impl MockProxyHarness {
    pub(crate) async fn start() -> Self {
        Self::start_at(mock_proxy_socket_path()).await
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
            connection_count: AtomicU16::new(0),
            connection_notify: Notify::new(),
            purge_count: AtomicU16::new(0),
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

    pub(crate) fn connection_count(&self) -> u16 {
        self.state.connection_count.load(Ordering::Relaxed)
    }

    pub(crate) fn purge_count(&self) -> u16 {
        self.state.purge_count.load(Ordering::Relaxed)
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
        if let Ok(Some(_message)) = timeout(CORE_RESPONSE_TIMEOUT, self.outbound_rx.recv()).await {
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
    /// Keep-alive handle: holds the sender so the handler's shutdown receiver
    /// does not see a premature cancellation.
    _shutdown_tx: Option<oneshot::Sender<bool>>,
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
            _shutdown_tx: Some(shutdown_tx),
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

        // No PgListener in proxy manager - just yield to let the manager start.
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
            if proxy.is_connected() == expected_connected {
                return proxy;
            }
            sleep(PROXY_CONNECT_DELAY).await;
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
    static PORT_CTR: AtomicU16 = AtomicU16::new(0);
    let port_number = PORT_CTR.fetch_add(1, Ordering::Relaxed);
    let port = 50051 + i32::from(port_number);
    let mut proxy = Proxy::new(
        format!("proxy-{port_number}"),
        "127.0.0.1".to_string(),
        port,
        "test-admin".to_string(),
    );
    proxy.enabled = enabled;
    proxy
}

// ---------------------------------------------------------------------------
// MockOidcProvider - a minimal OIDC identity provider for tests
// ---------------------------------------------------------------------------

/// Shared state injected into axum route handlers.
#[derive(Clone)]
struct OidcProviderState {
    /// PEM-encoded RSA-2048 private key used to sign ID tokens.
    encoding_key: Arc<jsonwebtoken::EncodingKey>,
    /// Base URL of this mock server, e.g. `http://127.0.0.1:PORT`.
    base_url: String,
    /// `client_id` the server expects in the `aud` claim.
    client_id: String,
    /// Base64Url(n) and Base64Url(e) of the RSA public key for the JWKS endpoint.
    jwks_n: String,
    jwks_e: String,
}

/// A mock OpenID Connect provider that handles the three endpoints that
/// `user_from_claims` / `make_oidc_client` call:
///
/// * `GET  /.well-known/openid-configuration`  – provider discovery
/// * `GET  /keys`                              – JWKS (RSA public key)
/// * `POST /token`                             – exchange authorization code for ID token
///
/// ### Code format
/// The authorization code must be `"{sub}:{email}:{nonce}"`.  The `/token`
/// handler parses those three components and embeds them in the signed JWT.
pub(crate) struct MockOidcProvider {
    /// HTTP base URL of the mock server, e.g. `http://127.0.0.1:45321`.
    pub(crate) base_url: String,
    /// OAuth2 / OIDC `client_id`.
    pub(crate) client_id: String,
    /// OAuth2 / OIDC `client_secret`.
    pub(crate) client_secret: String,
    server_task: JoinHandle<()>,
}

impl MockOidcProvider {
    /// Spawn a new mock OIDC provider on a random loopback port.
    pub(crate) async fn start() -> Self {
        // generate RSA-2048 key pair
        let mut rng = rand::thread_rng();
        let private_key = RsaPrivateKey::new(&mut rng, 2048).expect("failed to generate RSA key");

        // Export as PKCS#8 PEM for jsonwebtoken
        let pem = private_key
            .to_pkcs8_pem(rsa::pkcs8::LineEnding::LF)
            .expect("failed to encode private key as PKCS#8 PEM");
        let encoding_key = EncodingKey::from_rsa_pem(pem.as_bytes())
            .expect("failed to build EncodingKey from PEM");

        // build JWKS n / e
        let pub_key = private_key.to_public_key();
        let n_bytes = pub_key.n().to_bytes_be();
        let e_bytes = pub_key.e().to_bytes_be();
        let jwks_n = URL_SAFE_NO_PAD.encode(&n_bytes);
        let jwks_e = URL_SAFE_NO_PAD.encode(&e_bytes);

        // bind to random port
        let tcp_listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("failed to bind mock OIDC server");
        let addr = tcp_listener.local_addr().expect("no local addr");
        let base_url = format!("http://{addr}");
        let client_id = "test-client".to_string();
        let client_secret = "test-secret".to_string();

        let state = OidcProviderState {
            encoding_key: Arc::new(encoding_key),
            base_url: base_url.clone(),
            client_id: client_id.clone(),
            jwks_n,
            jwks_e,
        };

        // build axum router
        use axum::{
            Router,
            routing::{get, post},
        };
        let app = Router::new()
            .route("/.well-known/openid-configuration", get(oidc_discovery))
            .route("/keys", get(oidc_jwks))
            .route("/token", post(oidc_token))
            .with_state(state);

        let server_task = tokio::spawn(async move {
            axum::serve(tcp_listener, app)
                .await
                .expect("mock OIDC server error");
        });

        Self {
            base_url,
            client_id,
            client_secret,
            server_task,
        }
    }
}

impl Drop for MockOidcProvider {
    fn drop(&mut self) {
        self.server_task.abort();
    }
}

// axum handlers

async fn oidc_discovery(State(state): State<OidcProviderState>) -> Json<serde_json::Value> {
    let base = &state.base_url;
    Json(serde_json::json!({
        "issuer": base,
        "authorization_endpoint": format!("{base}/authorize"),
        "token_endpoint": format!("{base}/token"),
        "jwks_uri": format!("{base}/keys"),
        "response_types_supported": ["code"],
        "subject_types_supported": ["public"],
        "id_token_signing_alg_values_supported": ["RS256"]
    }))
}

async fn oidc_jwks(State(state): State<OidcProviderState>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "keys": [{
            "kty": "RSA",
            "alg": "RS256",
            "use": "sig",
            "n": state.jwks_n,
            "e": state.jwks_e
        }]
    }))
}

/// Parses the authorization code as `"{sub}:{email}:{nonce}"` and returns a
/// signed RS256 ID token JWT.
async fn oidc_token(
    State(state): State<OidcProviderState>,
    Form(params): Form<HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let code = params.get("code").cloned().unwrap_or_default();
    // code format: "{sub}:{email}:{nonce}"
    let mut parts = code.splitn(3, ':');
    let sub = parts.next().unwrap_or("unknown-sub").to_string();
    let email = parts.next().unwrap_or("unknown@example.com").to_string();
    let nonce = parts.next().unwrap_or("").to_string();

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let claims = serde_json::json!({
        "iss": state.base_url,
        "sub": sub,
        "aud": state.client_id,
        "exp": now + 3600,
        "iat": now,
        "email": email,
        "nonce": nonce,
        "given_name": "Test",
        "family_name": "OidcUser",
        "name": "Test OidcUser",
    });

    let mut header = Header::new(Algorithm::RS256);
    header.kid = None;

    let id_token = encode(&header, &claims, &state.encoding_key).expect("failed to sign ID token");

    Json(serde_json::json!({
        "access_token": "dummy-access-token",
        "token_type": "Bearer",
        "id_token": id_token,
        "expires_in": 3600
    }))
}
