use std::sync::Once;

use defguard_core::setup_logs::{CoreSetupLogLayer, MAX_CORE_LOG_LINES, scope_setup_logs};
use reqwest::StatusCode;
use serde_json::Value;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use std::sync::{Arc, Mutex};
use tracing::{debug, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use super::common::{make_test_client, setup_pool};

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
        .map(|line| serde_json::from_str::<Value>(line).unwrap())
        .collect()
}

fn read_logs(buffer: &Arc<Mutex<Vec<String>>>) -> Vec<String> {
    buffer
        .lock()
        .expect("test log buffer mutex poisoned")
        .clone()
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

    let buffer = Arc::new(Mutex::new(Vec::new()));

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

    let buffer = Arc::new(Mutex::new(Vec::new()));

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

    let buffer = Arc::new(Mutex::new(Vec::new()));

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
