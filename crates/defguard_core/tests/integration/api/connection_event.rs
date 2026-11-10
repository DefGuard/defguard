use std::net::{IpAddr, Ipv4Addr};

use chrono::{NaiveDateTime, SecondsFormat, TimeDelta, Utc};
use defguard_common::db::{
    Id, NoId,
    models::{
        Device, DeviceType, WireguardNetwork,
        connection_event::{ConnectionDirection, ConnectionEvent, ConnectionStatus},
        device::WireguardNetworkDevice,
        gateway::Gateway,
    },
};
use defguard_core::handlers::Auth;
use reqwest::StatusCode;
use serde::Deserialize;
use sqlx::{
    PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
};

use super::common::{
    client::TestClient, make_network, make_test_client, set_enterprise_license, setup_pool,
};

#[derive(Deserialize)]
struct PaginatedResponse<T> {
    data: Vec<T>,
    pagination: PaginationMeta,
}

#[derive(Deserialize)]
struct PaginationMeta {
    total_items: u32,
}

#[derive(Debug, Deserialize)]
struct ApiConnectionEvent {
    id: Id,
    direction: ConnectionDirection,
    status: ConnectionStatus,
    location_name: String,
    gateway_name: Option<String>,
    rule_id: Option<Id>,
    rule_name: Option<String>,
    device_id: Option<Id>,
    device_name: Option<String>,
    download_bytes: i64,
    count: i64,
}

#[derive(Deserialize)]
struct FilterOption {
    id: Id,
    name: String,
    context: Option<String>,
}

#[derive(Deserialize)]
struct FilterOptions {
    locations: Vec<FilterOption>,
    gateways: Vec<FilterOption>,
    devices: Vec<FilterOption>,
}

const DEVICE_IP: IpAddr = IpAddr::V4(Ipv4Addr::new(10, 1, 1, 2));
const REMOTE_IP: IpAddr = IpAddr::V4(Ipv4Addr::new(192, 168, 5, 5));

struct Fixture {
    network: WireguardNetwork<Id>,
    gateway: Gateway<Id>,
    device: Device<Id>,
    base_time: NaiveDateTime,
}

async fn setup(client: &TestClient, pool: &PgPool, user_id: Id) -> Fixture {
    set_enterprise_license();
    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    let network: WireguardNetwork<Id> = make_network(client, "network").await.json().await;
    let gateway = Gateway::new(network.id, "gateway", "localhost", 50051, "admin")
        .save(pool)
        .await
        .unwrap();
    let device = Device::new(
        "laptop".to_owned(),
        "laptop-pubkey".to_owned(),
        user_id,
        DeviceType::User,
        None,
        true,
    )
    .save(pool)
    .await
    .unwrap();
    WireguardNetworkDevice::new(network.id, device.id, vec![DEVICE_IP])
        .insert(pool)
        .await
        .unwrap();

    Fixture {
        network,
        gateway,
        device,
        base_time: Utc::now().naive_utc(),
    }
}

/// Allowed event of the fixture's device, without a matching ACL rule.
fn event(
    fixture: &Fixture,
    minutes_ago: i64,
    direction: ConnectionDirection,
) -> ConnectionEvent<NoId> {
    // The VPN device is the source of outbound and the destination of inbound traffic.
    let (source_ip, destination_ip) = match direction {
        ConnectionDirection::Outbound => (DEVICE_IP, REMOTE_IP),
        ConnectionDirection::Inbound => (REMOTE_IP, DEVICE_IP),
    };
    ConnectionEvent {
        id: NoId,
        location_id: fixture.network.id,
        gateway_id: fixture.gateway.id,
        timestamp: fixture.base_time - TimeDelta::minutes(minutes_ago),
        direction,
        status: ConnectionStatus::Allowed,
        protocol: 6,
        source_ip,
        source_port: Some(40000),
        destination_ip,
        destination_port: Some(443),
        rule_id: None,
        upload_bytes: 10,
        download_bytes: 100,
        count: 3,
    }
}

async fn save(pool: &PgPool, event: ConnectionEvent<NoId>) -> ConnectionEvent<Id> {
    event.save(pool).await.unwrap()
}

/// Asserts that both connection event endpoints reject the client.
async fn assert_forbidden(client: &TestClient) {
    for path in [
        "/api/v1/connection_event",
        "/api/v1/connection_event/filter_options",
    ] {
        let response = client.get(path).send().await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN, "{path}");
    }
}

async fn fetch(client: &TestClient, query: &str) -> PaginatedResponse<ApiConnectionEvent> {
    let response = client
        .get(format!("/api/v1/connection_event?{query}"))
        .send()
        .await;
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "{}",
        response.text().await
    );
    response.json().await
}

fn ids(response: &PaginatedResponse<ApiConnectionEvent>) -> Vec<Id> {
    response.data.iter().map(|event| event.id).collect()
}

#[sqlx::test]
async fn test_list_connection_events(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (client, state) = make_test_client(pool.clone()).await;
    let fixture = setup(&client, &pool, state.test_user.id).await;

    let outbound = save(&pool, event(&fixture, 2, ConnectionDirection::Outbound)).await;
    let inbound = ConnectionEvent {
        status: ConnectionStatus::Blocked,
        // Rule ID without a matching rule, e.g. a deleted one.
        rule_id: Some(12345),
        ..event(&fixture, 1, ConnectionDirection::Inbound)
    };
    let inbound = save(&pool, inbound).await;

    let response = fetch(&client, "").await;
    assert_eq!(response.pagination.total_items, 2);
    // Newest first by default.
    assert_eq!(ids(&response), vec![inbound.id, outbound.id]);

    for event in &response.data {
        assert_eq!(event.location_name, "network");
        assert_eq!(event.gateway_name.as_deref(), Some("gateway"));
        assert_eq!(event.device_id, Some(fixture.device.id));
        assert_eq!(event.device_name.as_deref(), Some("laptop"));
        assert_eq!(event.count, 3);
    }
    let first = &response.data[0];
    assert_eq!(first.direction, ConnectionDirection::Inbound);
    assert_eq!(first.status, ConnectionStatus::Blocked);
    assert_eq!(first.rule_id, Some(12345));
    assert_eq!(first.rule_name, None);
}

#[sqlx::test]
async fn test_filter_and_sort_connection_events(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (client, state) = make_test_client(pool.clone()).await;
    let fixture = setup(&client, &pool, state.test_user.id).await;

    let old = ConnectionEvent {
        download_bytes: 300,
        ..event(&fixture, 120, ConnectionDirection::Outbound)
    };
    let old = save(&pool, old).await;
    let blocked = ConnectionEvent {
        status: ConnectionStatus::Blocked,
        ..event(&fixture, 10, ConnectionDirection::Inbound)
    };
    let blocked = save(&pool, blocked).await;
    let recent = ConnectionEvent {
        download_bytes: 200,
        ..event(&fixture, 5, ConnectionDirection::Outbound)
    };
    let recent = save(&pool, recent).await;

    let response = fetch(&client, "status=blocked").await;
    assert_eq!(ids(&response), vec![blocked.id]);
    assert_eq!(response.pagination.total_items, 1);

    let response = fetch(&client, "direction=outbound&direction=inbound").await;
    assert_eq!(response.pagination.total_items, 3);

    let response = fetch(&client, "direction=outbound").await;
    assert_eq!(ids(&response), vec![recent.id, old.id]);

    // `Z` suffix rather than `+00:00`, which would need escaping in a query string.
    let from = (fixture.base_time - TimeDelta::minutes(60))
        .and_utc()
        .to_rfc3339_opts(SecondsFormat::Secs, true);
    let response = fetch(&client, &format!("from={from}")).await;
    assert_eq!(ids(&response), vec![recent.id, blocked.id]);

    let response = fetch(&client, &format!("device={}", fixture.device.id)).await;
    assert_eq!(response.pagination.total_items, 3);
    let response = fetch(&client, &format!("device={}", fixture.device.id + 1)).await;
    assert_eq!(response.pagination.total_items, 0);

    let response = fetch(&client, &format!("gateway={}", fixture.gateway.id)).await;
    assert_eq!(response.pagination.total_items, 3);
    let response = fetch(&client, &format!("location={}", fixture.network.id + 1)).await;
    assert_eq!(response.pagination.total_items, 0);

    let response = fetch(&client, "protocol=17").await;
    assert_eq!(response.pagination.total_items, 0);

    // Search matches IP addresses and names.
    let response = fetch(&client, "search=192.168.5").await;
    assert_eq!(response.pagination.total_items, 3);
    let response = fetch(&client, "search=LAPTOP").await;
    assert_eq!(response.pagination.total_items, 3);
    let response = fetch(&client, "search=no-such-thing").await;
    assert_eq!(response.pagination.total_items, 0);

    let response = fetch(&client, "sort_by=download_bytes&sort_order=asc").await;
    assert_eq!(ids(&response), vec![blocked.id, recent.id, old.id]);
    assert_eq!(
        response
            .data
            .iter()
            .map(|event| event.download_bytes)
            .collect::<Vec<_>>(),
        vec![100, 200, 300]
    );

    // Pagination keeps the requested order and reports the total.
    let response = fetch(&client, "per_page=1&page=2").await;
    assert_eq!(ids(&response), vec![blocked.id]);
    assert_eq!(response.pagination.total_items, 3);

    let response = client
        .get("/api/v1/connection_event?sort_by=nonsense")
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[sqlx::test]
async fn test_connection_event_filter_options(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (client, state) = make_test_client(pool.clone()).await;
    let fixture = setup(&client, &pool, state.test_user.id).await;

    let response = client
        .get("/api/v1/connection_event/filter_options")
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let options: FilterOptions = response.json().await;

    assert!(
        options
            .locations
            .iter()
            .any(|option| option.id == fixture.network.id && option.name == "network")
    );
    let gateway = options
        .gateways
        .iter()
        .find(|option| option.id == fixture.gateway.id)
        .unwrap();
    assert_eq!(gateway.context.as_deref(), Some("network"));
    let device = options
        .devices
        .iter()
        .find(|option| option.id == fixture.device.id)
        .unwrap();
    assert_eq!(device.name, "laptop");
    assert_eq!(
        device.context.as_deref(),
        Some(state.test_user.username.as_str())
    );
}

#[sqlx::test]
async fn test_connection_events_require_admin(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (client, _) = make_test_client(pool).await;
    // Enterprise license, so that only the missing admin role can cause rejection.
    set_enterprise_license();

    let auth = Auth::new("hpotter", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    assert_forbidden(&client).await;
}

#[sqlx::test]
async fn test_connection_events_require_enterprise_license(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    // Business-tier license is the default for test clients.
    let (client, _) = make_test_client(pool).await;

    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    assert_forbidden(&client).await;

    set_enterprise_license();
    let response = client.get("/api/v1/connection_event").send().await;
    assert_eq!(response.status(), StatusCode::OK);
}
