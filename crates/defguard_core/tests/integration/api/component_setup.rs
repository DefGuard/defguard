use std::sync::Once;

use defguard_core::setup_logs::core_setup_log_layer;
use reqwest::StatusCode;
use serde_json::Value;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use super::common::{make_test_client, setup_pool};

fn init_tracing_once() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        tracing_subscriber::registry()
            .with(core_setup_log_layer())
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
