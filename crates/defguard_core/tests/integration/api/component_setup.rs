use std::{
    collections::VecDeque,
    net::SocketAddr,
    sync::{Arc, Mutex, Once},
    time::Duration,
};

use defguard_certs::{Csr, DnType, generate_key_pair};
use defguard_core::setup_logs::{CoreSetupLogLayer, MAX_CORE_LOG_LINES, scope_setup_logs};
use defguard_proto::{
    common::{CertBundle, CertificateInfo, DerPayload, LogEntry},
    gateway::gateway_setup_server::{GatewaySetup, GatewaySetupServer},
};
use reqwest::StatusCode;
use serde_json::Value;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use tokio::{net::TcpListener, sync::Notify, task::JoinHandle, time::timeout};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tonic::{Request, Response, Status, transport::Server};
use tracing::{debug, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use super::common::{make_network, make_test_client, set_enterprise_license, setup_ca, setup_pool};

fn init_tracing_once() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        tracing_subscriber::registry()
            .with(CoreSetupLogLayer)
            .try_init()
            .ok();
    });
}

fn parse_sse_data_events(body: &str) -> Vec<Value> {
    body.lines()
        .filter_map(|line| line.strip_prefix("data: "))
        .map(|line| {
            serde_json::from_str::<Value>(line)
                .unwrap_or_else(|e| panic!("failed to parse SSE data line {line:?}: {e}"))
        })
        .collect()
}

fn read_logs(buffer: &Arc<Mutex<VecDeque<String>>>) -> Vec<String> {
    buffer
        .lock()
        .expect("test log buffer mutex poisoned")
        .iter()
        .cloned()
        .collect()
}

async fn log_from_nested_function() {
    info!("nested awaited log");
}

#[sqlx::test]
async fn test_proxy_setup_error_includes_core_logs(_: PgPoolOptions, options: PgConnectOptions) {
    init_tracing_once();

    let pool = setup_pool(options).await;

    let (mut client, _) = make_test_client(pool).await;
    client.login_user("admin", "pass123").await;

    let response = client
        .get("/api/v1/proxy/setup/stream?ip_or_domain=bad%20host&grpc_port=50051&common_name=edge")
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.text().await;
    let events = parse_sse_data_events(&body);

    let error_event = events
        .iter()
        .find(|event| event.get("error") == Some(&Value::Bool(true)))
        .unwrap();

    let logs = error_event
        .get("logs")
        .and_then(Value::as_array)
        .expect("expected `logs` array in proxy setup error event");
    assert!(
        !logs.is_empty(),
        "expected Core logs to be present in proxy setup error event"
    );

    let has_core_error = logs.iter().filter_map(Value::as_str).any(|line| {
        line.contains("ERROR") && line.contains("defguard_core::handlers::component_setup")
    });
    assert!(
        has_core_error,
        "expected at least one captured Core tracing line in error logs"
    );
}

#[sqlx::test]
async fn test_gateway_setup_error_includes_core_logs(_: PgPoolOptions, options: PgConnectOptions) {
    init_tracing_once();

    let pool = setup_pool(options).await;

    let (mut client, _) = make_test_client(pool).await;
    client.login_user("admin", "pass123").await;

    let response = client
        .get("/api/v1/network/1/gateways/setup?ip_or_domain=bad%20host&grpc_port=50051&common_name=gateway")
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.text().await;
    let events = parse_sse_data_events(&body);

    let error_event = events
        .iter()
        .find(|event| event.get("error") == Some(&Value::Bool(true)))
        .unwrap();

    let logs = error_event
        .get("logs")
        .and_then(Value::as_array)
        .expect("expected `logs` array in gateway setup error event");
    assert!(
        !logs.is_empty(),
        "expected Core logs to be present in gateway setup error event"
    );

    let has_core_error = logs.iter().filter_map(Value::as_str).any(|line| {
        line.contains("ERROR") && line.contains("defguard_core::handlers::component_setup")
    });
    assert!(
        has_core_error,
        "expected at least one captured Core tracing line in error logs"
    );
}

#[tokio::test]
async fn scope_setup_logs_captures_logs_inside_scope() {
    init_tracing_once();

    let buffer = Arc::new(Mutex::new(VecDeque::new()));

    scope_setup_logs(Arc::clone(&buffer), async {
        info!("captured in setup scope");
    })
    .await;

    let logs = read_logs(&buffer);
    assert_eq!(logs.len(), 1);
    assert!(logs[0].contains("captured in setup scope"));
}

#[tokio::test]
async fn nested_awaited_calls_are_captured() {
    init_tracing_once();

    let buffer = Arc::new(Mutex::new(VecDeque::new()));

    scope_setup_logs(Arc::clone(&buffer), async {
        log_from_nested_function().await;
    })
    .await;

    let logs = read_logs(&buffer);
    assert_eq!(logs.len(), 1);
    assert!(logs[0].contains("nested awaited log"));
}

#[tokio::test]
async fn buffer_is_bounded_to_max_core_log_lines() {
    init_tracing_once();

    let buffer = Arc::new(Mutex::new(VecDeque::new()));

    scope_setup_logs(Arc::clone(&buffer), async {
        for idx in 0..(MAX_CORE_LOG_LINES + 5) {
            debug!("bounded log line {idx}");
        }
    })
    .await;

    let logs = read_logs(&buffer);
    assert_eq!(logs.len(), MAX_CORE_LOG_LINES);
    assert!(logs[0].contains("bounded log line 5"));
    assert!(
        logs[MAX_CORE_LOG_LINES - 1]
            .contains(&format!("bounded log line {}", MAX_CORE_LOG_LINES + 4))
    );
}

const MOCK_GATEWAY_VERSION: &str = "2.0.0";
const MOCK_LOG_TIMESTAMP: &str = "2026-01-01T00:00:00Z";

struct MockGatewaySetupState {
    received_bundle: Mutex<Option<CertBundle>>,
    cert_received: Notify,
}

#[derive(Clone)]
struct MockGatewaySetupService {
    state: Arc<MockGatewaySetupState>,
}

#[tonic::async_trait]
impl GatewaySetup for MockGatewaySetupService {
    type StartStream = UnboundedReceiverStream<Result<LogEntry, Status>>;

    async fn start(&self, _request: Request<()>) -> Result<Response<Self::StartStream>, Status> {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

        tokio::spawn(async move {
            for i in 0..3u32 {
                let entry = LogEntry {
                    level: "INFO".to_string(),
                    target: "mock_gateway".to_string(),
                    message: format!("setup log {i}"),
                    timestamp: MOCK_LOG_TIMESTAMP.to_string(),
                    fields: Default::default(),
                };
                if tx.send(Ok(entry)).is_err() {
                    break;
                }
            }
        });

        let mut response = Response::new(UnboundedReceiverStream::new(rx));
        response.metadata_mut().insert(
            "defguard-component-version",
            MOCK_GATEWAY_VERSION.parse().expect("valid metadata value"),
        );
        Ok(response)
    }

    async fn get_csr(
        &self,
        request: Request<CertificateInfo>,
    ) -> Result<Response<DerPayload>, Status> {
        let hostname = request.into_inner().cert_hostname;
        let key_pair = generate_key_pair().map_err(|e| Status::internal(e.to_string()))?;
        let csr = Csr::new(
            &key_pair,
            std::slice::from_ref(&hostname),
            vec![(DnType::CommonName, hostname.as_str())],
        )
        .map_err(|e| Status::internal(e.to_string()))?;
        let der = csr.to_der().to_vec();
        Ok(Response::new(DerPayload { der_data: der }))
    }

    async fn send_cert(&self, request: Request<CertBundle>) -> Result<Response<()>, Status> {
        let bundle = request.into_inner();
        *self
            .state
            .received_bundle
            .lock()
            .expect("mock state mutex poisoned") = Some(bundle);
        self.state.cert_received.notify_one();
        Ok(Response::new(()))
    }
}

struct MockGatewaySetupHarness {
    port: u16,
    state: Arc<MockGatewaySetupState>,
    _server_task: JoinHandle<()>,
}

impl MockGatewaySetupHarness {
    async fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("failed to bind mock gateway setup socket");
        let addr: SocketAddr = listener.local_addr().expect("no local addr");
        let port = addr.port();

        let state = Arc::new(MockGatewaySetupState {
            received_bundle: Mutex::new(None),
            cert_received: Notify::new(),
        });
        let service = MockGatewaySetupService {
            state: Arc::clone(&state),
        };

        let server_task = tokio::spawn(async move {
            let stream = async_stream::stream! {
                loop {
                    match listener.accept().await {
                        Ok((stream, _)) => yield Ok(stream),
                        Err(e) => {
                            yield Err(e);
                            break;
                        }
                    }
                }
            };
            Server::builder()
                .add_service(GatewaySetupServer::new(service))
                .serve_with_incoming(stream)
                .await
                .expect("mock gateway setup server error");
        });

        Self {
            port,
            state,
            _server_task: server_task,
        }
    }

    async fn wait_for_cert(&self) {
        timeout(Duration::from_secs(5), self.state.cert_received.notified())
            .await
            .expect("timed out waiting for certificate to be received by mock gateway");
    }
}

impl Drop for MockGatewaySetupHarness {
    fn drop(&mut self) {
        self._server_task.abort();
    }
}

async fn make_network_id(client: &super::common::client::TestClient, name: &str) -> i64 {
    make_network(client, name)
        .await
        .json::<Value>()
        .await
        .get("id")
        .and_then(Value::as_i64)
        .expect("network response missing id")
}

/// Set up a logged-in admin client and a test network. Does NOT configure CA certificates.
async fn setup_test_no_ca(pool: &sqlx::PgPool) -> (super::common::client::TestClient, i64) {
    let (mut client, _) = make_test_client(pool.clone()).await;
    client.login_user("admin", "pass123").await;
    let network_id = make_network_id(&client, "test-net").await;
    (client, network_id)
}

/// Set up a logged-in admin client, a test network, and a CA certificate.
async fn setup_test_with_ca(pool: &sqlx::PgPool) -> (super::common::client::TestClient, i64) {
    let (client, network_id) = setup_test_no_ca(pool).await;
    setup_ca(pool).await;
    (client, network_id)
}

#[sqlx::test]
async fn test_adopt_gateway_rest(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (client, network_id) = setup_test_with_ca(&pool).await;

    let harness = MockGatewaySetupHarness::start().await;
    let port = harness.port;

    let response = client
        .post(format!("/api/v1/network/{network_id}/gateways/adopt"))
        .json(&serde_json::json!({
            "name": "FirstGateway",
            "ip_or_domain": "127.0.0.1",
            "grpc_port": port
        }))
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::CREATED);

    let body: Value = response.json().await;
    assert!(
        body.get("certificate_serial")
            .and_then(Value::as_str)
            .is_some(),
        "expected certificate_serial in response, got: {body}"
    );

    harness.wait_for_cert().await;

    let gateways =
        defguard_common::db::models::gateway::Gateway::find_by_location_id(&pool, network_id)
            .await
            .expect("failed to query gateways");

    assert_eq!(gateways.len(), 1, "expected exactly one gateway in DB");
    assert!(
        gateways[0].certificate_serial.is_some(),
        "expected gateway in DB to have a certificate serial"
    );
}

#[sqlx::test]
async fn test_adopt_gateway_rest_missing_ca(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (client, network_id) = setup_test_no_ca(&pool).await;

    let response = client
        .post(format!("/api/v1/network/{network_id}/gateways/adopt"))
        .json(&serde_json::json!({
            "name": "FirstGateway",
            "ip_or_domain": "127.0.0.1",
            "grpc_port": 9999
        }))
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body: Value = response.json().await;
    assert!(
        body.get("msg")
            .and_then(Value::as_str)
            .is_some_and(|msg| msg.contains("CA certificate not found")),
        "expected CA certificate not found error, got: {body}"
    );
}

#[sqlx::test]
async fn test_adopt_gateway_rest_bad_address(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (client, network_id) = setup_test_no_ca(&pool).await;

    let response = client
        .post(format!("/api/v1/network/{network_id}/gateways/adopt"))
        .json(&serde_json::json!({
            "name": "FirstGateway",
            "ip_or_domain": "bad host",
            "grpc_port": 9999
        }))
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body: Value = response.json().await;
    assert!(
        body.get("msg")
            .and_then(Value::as_str)
            .is_some_and(|msg| msg.contains("Invalid URL")),
        "expected invalid URL error, got: {body}"
    );
}

#[sqlx::test]
async fn test_adopt_gateway_rest_duplicate(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (client, network_id) = setup_test_with_ca(&pool).await;

    let harness = MockGatewaySetupHarness::start().await;
    let port = harness.port;

    // First adoption succeeds.
    let response = client
        .post(format!("/api/v1/network/{network_id}/gateways/adopt"))
        .json(&serde_json::json!({
            "name": "FirstGateway",
            "ip_or_domain": "127.0.0.1",
            "grpc_port": port
        }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    harness.wait_for_cert().await;

    // Upgrade to Enterprise so the license gate does not fire before the duplicate-URL check.
    set_enterprise_license();

    // Second adoption for the same address is rejected.
    let response = client
        .post(format!("/api/v1/network/{network_id}/gateways/adopt"))
        .json(&serde_json::json!({
            "name": "SecondGateway",
            "ip_or_domain": "127.0.0.1",
            "grpc_port": port
        }))
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body: Value = response.json().await;
    assert!(
        body.get("msg")
            .and_then(Value::as_str)
            .is_some_and(|msg| msg.contains("already registered")),
        "expected duplicate gateway error, got: {body}"
    );
}

#[sqlx::test]
async fn test_adopt_gateway_sse(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (client, network_id) = setup_test_with_ca(&pool).await;

    let harness = MockGatewaySetupHarness::start().await;
    let port = harness.port;

    let response = client
        .get(format!(
            "/api/v1/network/{network_id}/gateways/setup\
             ?ip_or_domain=127.0.0.1&grpc_port={port}&common_name=FirstGateway"
        ))
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.text().await;
    let events = parse_sse_data_events(&body);

    // Verify the exact sequence of steps emitted by the adoption flow.
    // CheckingVersion appears twice: once as a step-marker and once as the version-payload event.
    let steps: Vec<&str> = events
        .iter()
        .filter_map(|e| e.get("step")?.as_str())
        .collect();
    assert_eq!(
        steps,
        &[
            "CheckingConfiguration",
            "CheckingAvailability",
            "CheckingVersion", // step marker
            "CheckingVersion", // version payload event
            "ObtainingCsr",
            "SigningCertificate",
            "ConfiguringTls",
            "Done",
        ],
        "unexpected SSE step sequence: {steps:?}"
    );

    // Walk every event and assert its field-level structure.
    let mut version_payload_count = 0;
    for (i, event) in events.iter().enumerate() {
        // No event should be an error.
        assert_eq!(
            event.get("error"),
            Some(&Value::Bool(false)),
            "event {i} has unexpected error flag: {event:?}"
        );

        let step = event.get("step").and_then(Value::as_str).unwrap_or("");
        let has_version = event.get("version").and_then(Value::as_str).is_some();

        if has_version {
            // This is the version-payload event emitted after the CheckingVersion step marker.
            version_payload_count += 1;
            assert_eq!(
                step, "CheckingVersion",
                "event {i}: version field present on unexpected step {step:?}: {event:?}"
            );
            assert_eq!(
                event.get("version").and_then(Value::as_str),
                Some(MOCK_GATEWAY_VERSION),
                "event {i}: wrong version in version payload: {event:?}"
            );
        }
    }
    assert_eq!(
        version_payload_count, 1,
        "expected exactly one version-payload event in SSE stream"
    );

    harness.wait_for_cert().await;

    let gateways =
        defguard_common::db::models::gateway::Gateway::find_by_location_id(&pool, network_id)
            .await
            .expect("failed to query gateways");

    assert_eq!(gateways.len(), 1, "expected exactly one gateway in DB");
    assert!(
        gateways[0].certificate_serial.is_some(),
        "expected gateway in DB to have a certificate serial"
    );
}
