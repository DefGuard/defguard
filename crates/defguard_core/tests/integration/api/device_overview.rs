use chrono::{NaiveDate, Utc};
use defguard_common::db::{
    Id,
    models::{
        Device, DeviceState, DeviceType, DeviceVerification, User, WireguardNetwork,
        device_client_info::{DeviceClientInfo, OsType},
        device_identity::DeviceAttestationStatus,
        gateway::Gateway,
        vpn_client_session::{VpnClientSession, VpnClientSessionState},
        vpn_session_stats::VpnSessionStats,
    },
};
use defguard_core::{events::ApiEventType, handlers::Auth};
use reqwest::StatusCode;
use serde_json::{Value, json};
use sqlx::{
    PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
};

use super::common::{client::TestClient, make_network, make_test_client, setup_pool};

#[sqlx::test]
async fn test_devices_overview(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (mut client, client_state) = make_test_client(pool).await;

    let user_device = Device::new(
        "work-laptop".to_owned(),
        "user-device-pubkey".to_owned(),
        client_state.test_user.id,
        DeviceType::User,
        None,
        true,
    )
    .save(&client_state.pool)
    .await
    .unwrap();
    let mut network_device = Device::new(
        "office-router".to_owned(),
        "network-device-pubkey".to_owned(),
        client_state.test_user.id,
        DeviceType::Network,
        None,
        true,
    )
    .save(&client_state.pool)
    .await
    .unwrap();
    network_device.state = DeviceState::Blocked;
    network_device.save(&client_state.pool).await.unwrap();
    DeviceClientInfo {
        device_id: user_device.id,
        client_version: Some("2.3.0".to_owned()),
        os_type: OsType::Ios,
        os_version: "27.1".to_owned(),
        os_edition: None,
        updated_at: Utc::now().naive_utc(),
    }
    .upsert(&client_state.pool)
    .await
    .unwrap();

    let response = client.get("/api/v1/device/overview").send().await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    client.login_user("hpotter", "pass123").await;
    let response = client.get("/api/v1/device/overview").send().await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    let network: WireguardNetwork<Id> = make_network(&client, "network").await.json().await;
    let earlier_connected_at = NaiveDate::from_ymd_opt(2026, 1, 2)
        .unwrap()
        .and_hms_opt(3, 4, 5)
        .unwrap();
    let latest_connected_at = NaiveDate::from_ymd_opt(2026, 1, 3)
        .unwrap()
        .and_hms_opt(4, 5, 6)
        .unwrap();
    let mut disconnected_session = VpnClientSession::new(
        network.id,
        client_state.test_user.id,
        user_device.id,
        Some(earlier_connected_at),
        false,
    );
    disconnected_session.state = VpnClientSessionState::Disconnected;
    disconnected_session.disconnected_at = Some(latest_connected_at);
    disconnected_session.save(&client_state.pool).await.unwrap();
    let latest_session = VpnClientSession::new(
        network.id,
        client_state.test_user.id,
        user_device.id,
        Some(latest_connected_at),
        false,
    )
    .save(&client_state.pool)
    .await
    .unwrap();
    let gateway = Gateway::new(network.id, "gateway", "198.51.100.1", 51820, "tester")
        .save(&client_state.pool)
        .await
        .unwrap();
    for (collected_at, endpoint) in [
        (earlier_connected_at, "198.51.100.1:51820"),
        (latest_connected_at, "[2001:db8::1]:51820"),
    ] {
        VpnSessionStats::new(
            latest_session.id,
            gateway.id,
            collected_at,
            collected_at,
            endpoint.to_owned(),
            0,
            0,
            0,
            0,
        )
        .save(&client_state.pool)
        .await
        .unwrap();
    }

    let response = client.get("/api/v1/device/overview").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = response.json().await;
    assert_eq!(body["pagination"]["total_items"], 2);
    let data = body["data"].as_array().unwrap();
    assert_eq!(data[0]["id"], network_device.id);
    assert_eq!(data[0]["name"], "office-router");
    assert_eq!(data[0]["state"], "blocked");
    assert!(data[0]["user_id"].is_null());
    assert!(data[0]["os_type"].is_null());
    assert!(data[0]["os_version"].is_null());
    assert!(data[0]["client_version"].is_null());
    assert_eq!(data[0]["connected"], false);
    assert!(data[0]["last_connected_at"].is_null());
    assert!(data[0]["last_connected_ip"].is_null());
    assert!(data[0]["last_connected_location"].is_null());
    assert_eq!(data[1]["id"], user_device.id);
    assert_eq!(data[1]["state"], "enabled");
    assert_eq!(data[1]["verification"], "unverified");
    assert_eq!(data[1]["attestation_status"], "not_attested");
    assert_eq!(data[1]["user_id"], client_state.test_user.id);
    assert_eq!(data[1]["os_type"], "ios");
    assert_eq!(data[1]["os_version"], "27.1");
    assert_eq!(data[1]["client_version"], "2.3.0");
    assert_eq!(data[1]["connected"], true);
    assert_eq!(data[1]["last_connected_at"], "2026-01-03T04:05:06");
    assert_eq!(data[1]["last_connected_ip"], "2001:db8::1");
    assert_eq!(data[1]["last_connected_location"], "network");

    let response = client
        .get("/api/v1/device/overview?search=LAPTOP&page=1&per_page=1")
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = response.json().await;
    assert_eq!(body["pagination"]["total_items"], 1);
    assert_eq!(body["data"][0]["id"], user_device.id);
}

async fn overview_ids(client: &TestClient, query: &str) -> Vec<Id> {
    let response = client
        .get(format!("/api/v1/device/overview?{query}"))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK, "query: {query}");
    let body: Value = response.json().await;
    let ids: Vec<Id> = body["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|device| device["id"].as_i64().unwrap())
        .collect();
    assert_eq!(
        body["pagination"]["total_items"],
        ids.len(),
        "query: {query}"
    );
    ids
}

#[sqlx::test]
async fn test_devices_overview_filters_and_sorting(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (mut client, client_state) = make_test_client(pool).await;
    let pool = &client_state.pool;
    client.login_user("admin", "pass123").await;

    let harry = client_state.test_user.id;
    let albus = User::new(
        "adumbledore",
        Some("pass123"),
        "Dumbledore",
        "Albus",
        "a.dumbledore@hogwart.edu.uk",
        None,
    )
    .save(pool)
    .await
    .unwrap()
    .id;

    let mut phone = make_device(pool, "a-phone", harry).await;
    phone.verification = DeviceVerification::Verified;
    phone.save(pool).await.unwrap();
    let mut laptop = make_device(pool, "b-laptop", albus).await;
    laptop.attestation_status = DeviceAttestationStatus::Attested;
    laptop.save(pool).await.unwrap();
    let mut router = Device::new(
        "c-router".to_owned(),
        "c-router-pubkey".to_owned(),
        harry,
        DeviceType::Network,
        None,
        true,
    )
    .save(pool)
    .await
    .unwrap();
    router.state = DeviceState::Blocked;
    router.save(pool).await.unwrap();
    let desktop = make_device(pool, "d-desktop", albus).await;

    for (device_id, os_type) in [(phone.id, OsType::Android), (laptop.id, OsType::Windows)] {
        DeviceClientInfo {
            device_id,
            client_version: None,
            os_type,
            os_version: "1".to_owned(),
            os_edition: None,
            updated_at: Utc::now().naive_utc(),
        }
        .upsert(pool)
        .await
        .unwrap();
    }
    let network: WireguardNetwork<Id> = make_network(&client, "network").await.json().await;
    VpnClientSession::new(
        network.id,
        harry,
        phone.id,
        Some(Utc::now().naive_utc()),
        false,
    )
    .save(pool)
    .await
    .unwrap();

    let (a, b, c, d) = (phone.id, laptop.id, router.id, desktop.id);
    let cases: &[(&str, Vec<Id>)] = &[
        ("", vec![a, b, c, d]),
        ("sort_by=name&sort_order=desc", vec![d, c, b, a]),
        ("sort_by=user&sort_order=asc", vec![b, d, a, c]),
        ("sort_by=user&sort_order=desc", vec![a, d, b, c]),
        ("state=blocked", vec![c]),
        ("state=enabled&state=blocked", vec![a, b, c, d]),
        ("verification=verified", vec![a]),
        ("attestation_status=attested", vec![b]),
        ("platform=mobile", vec![a]),
        ("platform=desktop", vec![b]),
        ("platform=unknown", vec![c, d]),
        ("platform=mobile&platform=unknown", vec![a, c, d]),
        ("system=windows", vec![b]),
        ("system=unknown", vec![c, d]),
        ("system=android&system=windows", vec![a, b]),
        ("connected=true", vec![a]),
        ("connected=false", vec![b, c, d]),
        ("platform=unknown&state=enabled", vec![d]),
        ("search=laptop", vec![b]),
        ("search=ADUMBLEDORE", vec![b, d]),
        ("search=albus", vec![b, d]),
        ("search=dumbledore&platform=unknown", vec![d]),
        ("search=hpotter", vec![a]),
        ("search=harry%20potter", vec![a]),
    ];
    for (query, expected) in cases {
        assert_eq!(
            &overview_ids(&client, query).await,
            expected,
            "query: {query}"
        );
    }

    let response = client
        .get("/api/v1/device/overview?connected=false&sort_by=user&page=1&per_page=2")
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = response.json().await;
    assert_eq!(body["pagination"]["total_items"], 3);
    assert_eq!(body["data"][0]["id"], b);
    assert_eq!(body["data"][1]["id"], d);
}

async fn make_device(pool: &PgPool, name: &str, user_id: Id) -> Device<Id> {
    Device::new(
        name.to_owned(),
        format!("{name}-pubkey"),
        user_id,
        DeviceType::User,
        None,
        true,
    )
    .save(pool)
    .await
    .unwrap()
}

#[sqlx::test]
async fn test_devices_bulk_actions(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (mut client, client_state) = make_test_client(pool).await;
    let pool = &client_state.pool;
    let user_id = client_state.test_user.id;

    let first = make_device(pool, "first", user_id).await;
    let second = make_device(pool, "second", user_id).await;
    let mut verified = make_device(pool, "verified", user_id).await;
    verified.verification = DeviceVerification::Verified;
    verified.save(pool).await.unwrap();
    let ids = json!({"devices": [first.id, second.id, verified.id]});

    client.login_user("hpotter", "pass123").await;
    for action in ["verify", "block", "delete"] {
        let response = client
            .post(format!("/api/v1/device/bulk/{action}"))
            .json(&ids)
            .send()
            .await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    client.login_user("admin", "pass123").await;

    let response = client
        .post("/api/v1/device/bulk/verify")
        .json(&ids)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let first_after = Device::find_by_id(pool, first.id).await.unwrap().unwrap();
    assert_eq!(first_after.verification, DeviceVerification::Verified);
    assert_eq!(
        first_after.attestation_status,
        DeviceAttestationStatus::NotAttested
    );
    assert!(first_after.attested_at.is_none());
    let events = client.drain_all_events();
    assert_eq!(events.len(), 2);
    assert!(
        events
            .iter()
            .all(|(event, ..)| matches!(event, ApiEventType::UserDeviceModified { .. }))
    );

    let response = client
        .post("/api/v1/device/bulk/block")
        .json(&json!({"devices": [first.id, second.id]}))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    for id in [first.id, second.id] {
        let device = Device::find_by_id(pool, id).await.unwrap().unwrap();
        assert_eq!(device.state, DeviceState::Blocked);
    }
    let events = client.drain_all_events();
    assert_eq!(events.len(), 2);
    assert!(
        events
            .iter()
            .all(|(event, ..)| matches!(event, ApiEventType::UserDeviceModified { .. }))
    );
    let verified_after = Device::find_by_id(pool, verified.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(verified_after.state, DeviceState::Enabled);

    let response = client
        .post("/api/v1/device/bulk/delete")
        .json(&json!({"devices": [first.id, Id::MAX]}))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert!(Device::find_by_id(pool, first.id).await.unwrap().is_some());

    let response = client
        .post("/api/v1/device/bulk/delete")
        .json(&json!({"devices": [first.id, second.id]}))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert!(Device::find_by_id(pool, first.id).await.unwrap().is_none());
    assert!(Device::find_by_id(pool, second.id).await.unwrap().is_none());
    assert!(
        Device::find_by_id(pool, verified.id)
            .await
            .unwrap()
            .is_some()
    );
    let events = client.drain_all_events();
    assert_eq!(events.len(), 2);
    assert!(
        events
            .iter()
            .all(|(event, ..)| matches!(event, ApiEventType::UserDeviceRemoved { .. }))
    );
}

#[sqlx::test]
async fn test_users_display(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (mut client, client_state) = make_test_client(pool).await;

    client.login_user("hpotter", "pass123").await;
    let response = client.get("/api/v1/user-display").send().await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    client.login_user("admin", "pass123").await;
    let response = client.get("/api/v1/user-display").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = response.json().await;
    let users = body.as_array().unwrap();
    assert!(users.contains(&json!({
        "id": client_state.test_user.id,
        "display": "Harry Potter",
    })));
}
