use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use defguard_common::db::{
    Id,
    models::{
        Device, User, WireguardNetwork,
        device::WireguardNetworkDevice,
        group::Group,
        wireguard::{
            DEFAULT_DISCONNECT_THRESHOLD, DEFAULT_KEEPALIVE_INTERVAL, DEFAULT_WIREGUARD_MTU,
            ServiceLocationMode,
        },
    },
};
use defguard_core::{
    enterprise::{
        db::models::acl::{AclRule, AclRuleNetwork, AclRuleUser, RuleState},
        license::{License, LicenseTier, SupportType, get_cached_license, set_cached_license},
        limits::update_counts,
    },
    events::ApiEventType,
    grpc::{GatewayCommand, proto::enterprise::license::LicenseLimits},
    handlers::{Auth, GroupInfo, wireguard::WireguardNetworkData},
};
use ipnetwork::IpNetwork;
use matches::assert_matches;
use reqwest::StatusCode;
use serde_json::{Value, json};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

use super::common::{
    authenticate_admin,
    client::{TestClient, TestResponse},
    fetch_user_details, make_network, make_test_client, setup_pool, update_location_mfa_flows,
    update_location_posture_checks,
};

const INVALID_MFA_PEER_DISCONNECT_THRESHOLD: i32 = 119;
const MINIMUM_MFA_PEER_DISCONNECT_THRESHOLD: i32 = 120;

async fn create_mfa_flow(client: &TestClient, title: &str, steps: Value) -> Id {
    let response = client
        .post("/api/v1/mfa-flow")
        .json(&json!({"title": title, "steps": steps}))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    response.json::<Value>().await["id"].as_i64().unwrap()
}

async fn create_network_with_mfa_flows(
    client: &TestClient,
    name: &str,
    address: &str,
    mfa_flows: Value,
) -> TestResponse {
    client
        .post("/api/v1/network")
        .json(&json!({
            "name": name,
            "address": address,
            "port": 55555,
            "endpoint": "192.168.4.14",
            "allowed_ips": address,
            "dns": "1.1.1.1",
            "mtu": 1420,
            "fwmark": 0,
            "allowed_groups": ["admin"],
            "allow_all_groups": false,
            "keepalive_interval": 25,
            "peer_disconnect_threshold": 300,
            "acl_enabled": false,
            "acl_default_allow": false,
            "allowed_ips_from_acl": false,
            "mfa_enabled": false,
            "service_location_mode": "disabled",
            "posture_checks": [],
            "mfa_flows": mfa_flows,
        }))
        .send()
        .await
}

async fn assert_assignment_license_error(response: TestResponse, code: &str) {
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let body: Value = response.json().await;
    assert_eq!(body["error"], "license_required");
    assert_eq!(body["fields"][0]["field"], "mfa_flows");
    assert_eq!(body["fields"][0]["code"], code);
}

#[sqlx::test]
async fn test_network(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (client, client_state) = make_test_client(pool).await;

    let mut gateway_rx = client_state.gateway_rx;

    let auth = Auth::new("admin", "pass123");
    let response = &client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // create network
    let response = make_network(&client, "network").await;
    let network: WireguardNetwork<Id> = response.json().await;
    assert_eq!(network.name, "network");
    let event = gateway_rx.try_recv().unwrap();
    assert_matches!(event, GatewayCommand::NetworkCreated(..));

    // check vpn locations for `admin` group
    let admin_id = Group::find_by_name(&client_state.pool, "admin")
        .await
        .unwrap()
        .unwrap()
        .id;
    let response = client.get(format!("/api/v1/group/{admin_id}")).send().await;
    let group_info: GroupInfo = response.json().await;
    assert_eq!(group_info.vpn_locations, vec!["network"]);

    // modify network
    let network_data = WireguardNetworkData {
        name: "my network".into(),
        address: "10.1.1.1/24".into(),
        endpoint: "10.1.1.1".parse().unwrap(),
        port: 55555,
        allowed_ips: Some("10.1.1.0/24, 10.2.0.1/16, 10.10.10.54/32".into()),
        dns: None,
        mtu: DEFAULT_WIREGUARD_MTU,
        fwmark: 0,
        allow_all_groups: false,
        allowed_groups: vec!["admin".into()],
        keepalive_interval: DEFAULT_KEEPALIVE_INTERVAL,
        peer_disconnect_threshold: DEFAULT_DISCONNECT_THRESHOLD,
        acl_enabled: false,
        acl_default_allow: false,
        allowed_ips_from_acl: false,
        mfa_enabled: false,
        service_location_mode: ServiceLocationMode::Disabled,
        posture_checks: Vec::new(),
        mfa_flows: Vec::new(),
    };
    let response = client
        .put(format!("/api/v1/network/{}", network.id))
        .json(&network_data)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let network: WireguardNetwork<Id> = response.json().await;
    assert_eq!(
        network.allowed_ips,
        vec![
            IpNetwork::V4("10.1.1.0/24".parse().unwrap()),
            IpNetwork::V4("10.2.0.0/16".parse().unwrap()),
            IpNetwork::V4("10.10.10.54/32".parse().unwrap())
        ]
    );

    let event = gateway_rx.try_recv().unwrap();
    assert_matches!(event, GatewayCommand::NetworkModified(..));

    // check vpn locations for `admin` group
    let response = client.get(format!("/api/v1/group/{admin_id}")).send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let group_info: GroupInfo = response.json().await;
    assert_eq!(group_info.vpn_locations, vec!["my network"]);

    // list networks
    let response = client.get("/api/v1/network").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let networks: Vec<WireguardNetwork<Id>> = response.json().await;
    assert_eq!(networks.len(), 1);

    // network details
    let network_from_list = networks[0].clone();
    assert_eq!(network_from_list.name, "my network");
    let response = client
        .get(format!("/api/v1/network/{}", network_from_list.id))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let network_from_details: WireguardNetwork<Id> = response.json().await;
    assert_eq!(network_from_details, network_from_list);

    // delete network
    let response = client
        .delete(format!("/api/v1/network/{}", network.id))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let event = gateway_rx.try_recv().unwrap();
    assert_matches!(event, GatewayCommand::NetworkDeleted(..));
}

#[sqlx::test]
async fn test_create_network_blocked_when_location_count_exceeds_license_limit(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;

    let (mut client, client_state) = make_test_client(pool).await;
    authenticate_admin(&mut client).await;

    make_network(&client, "network1").await;
    make_network(&client, "network2").await;
    update_counts(&client_state.pool).await.unwrap();

    let license = get_cached_license().clone();
    set_cached_license(Some(License::new(
        "test_customer".to_owned(),
        false,
        None,
        Some(LicenseLimits {
            users: 100,
            devices: 100,
            locations: 1,
            network_devices: Some(100),
        }),
        None,
        LicenseTier::Business,
        SupportType::Basic,
        vec![],
    )));

    let response = client
        .post("/api/v1/network")
        .json(&json!({
            "name": "network3",
            "address": "10.1.1.1/24",
            "port": 55555,
            "endpoint": "192.168.4.14",
            "allowed_ips": "10.1.1.0/24",
            "dns": "1.1.1.1",
            "mtu": 1420,
            "fwmark": 0,
            "allowed_groups": ["admin"],
            "allow_all_groups": false,
            "keepalive_interval": 25,
            "peer_disconnect_threshold": 300,
            "acl_enabled": false,
            "acl_default_allow": false,
            "allowed_ips_from_acl": false,
            "mfa_enabled": false,
            "service_location_mode": "disabled",
            "posture_checks": [],
            "mfa_flows": []
        }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    set_cached_license(license);
}

#[sqlx::test]
async fn test_create_network_mfa_assignment_license_gates(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let (mut client, _) = make_test_client(pool.clone()).await;
    authenticate_admin(&mut client).await;
    let business_license = get_cached_license().clone();

    let single_step_flow =
        create_mfa_flow(&client, "Single-step flow", json!([{"methods": ["totp"]}])).await;
    let second_single_step_flow = create_mfa_flow(
        &client,
        "Second single-step flow",
        json!([{"methods": ["biometric"]}]),
    )
    .await;
    let multi_step_flow = create_mfa_flow(
        &client,
        "Multi-step flow",
        json!([
            {"methods": ["totp"]},
            {"methods": ["biometric"]}
        ]),
    )
    .await;
    let admin_group_id = Group::find_by_name(&pool, "admin")
        .await
        .unwrap()
        .unwrap()
        .id;

    set_cached_license(None);

    let response = create_network_with_mfa_flows(
        &client,
        "free-single-step",
        "10.10.1.1/24",
        json!([{
            "flow_id": single_step_flow,
            "is_default": true,
            "group_ids": []
        }]),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    let response = create_network_with_mfa_flows(
        &client,
        "free-multi-step",
        "10.10.2.1/24",
        json!([{
            "flow_id": multi_step_flow,
            "is_default": true,
            "group_ids": []
        }]),
    )
    .await;
    assert_assignment_license_error(response, "business_license_required").await;

    let response = create_network_with_mfa_flows(
        &client,
        "free-multiple-flows",
        "10.10.3.1/24",
        json!([
            {
                "flow_id": single_step_flow,
                "is_default": true,
                "group_ids": []
            },
            {
                "flow_id": second_single_step_flow,
                "is_default": false,
                "group_ids": []
            }
        ]),
    )
    .await;
    assert_assignment_license_error(response, "business_license_required").await;

    set_cached_license(business_license.clone());

    let response = create_network_with_mfa_flows(
        &client,
        "business-multi-step",
        "10.10.4.1/24",
        json!([{
            "flow_id": multi_step_flow,
            "is_default": true,
            "group_ids": []
        }]),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    let scoped_assignments = json!([
        {
            "flow_id": single_step_flow,
            "is_default": true,
            "group_ids": []
        },
        {
            "flow_id": second_single_step_flow,
            "is_default": false,
            "group_ids": [admin_group_id]
        }
    ]);
    let response = create_network_with_mfa_flows(
        &client,
        "business-group-scoping",
        "10.10.5.1/24",
        scoped_assignments.clone(),
    )
    .await;
    assert_assignment_license_error(response, "enterprise_license_required").await;

    set_enterprise_license();

    let response = create_network_with_mfa_flows(
        &client,
        "enterprise-group-scoping",
        "10.10.6.1/24",
        scoped_assignments,
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    set_cached_license(business_license);
}

#[sqlx::test]
async fn test_create_network_with_posture_checks_assigns_postures(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let (mut client, _client_state) = make_test_client(pool).await;
    authenticate_admin(&mut client).await;
    set_enterprise_license();

    let response = client
        .post("/api/v1/device-posture")
        .json(&json!({
            "name": "Posture 1",
            "description": null,
            "min_desktop_client_version": null,
            "min_mobile_client_version": null,
            "allow_prerelease_client": false,
            "os_rules": []
        }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let posture_1: serde_json::Value = response.json().await;

    let response = client
        .post("/api/v1/device-posture")
        .json(&json!({
            "name": "Posture 2",
            "description": null,
            "min_desktop_client_version": null,
            "min_mobile_client_version": null,
            "allow_prerelease_client": false,
            "os_rules": []
        }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let posture_2: serde_json::Value = response.json().await;
    let posture_ids = vec![
        posture_1["id"].as_i64().unwrap(),
        posture_2["id"].as_i64().unwrap(),
    ];
    client.drain_all_events();

    let response = client
        .post("/api/v1/network")
        .json(&json!({
            "name": "network-with-postures",
            "address": "10.1.1.1/24",
            "port": 55555,
            "endpoint": "192.168.4.14",
            "allowed_ips": "10.1.1.0/24",
            "dns": "1.1.1.1",
            "mtu": 1420,
            "fwmark": 0,
            "allowed_groups": ["admin"],
            "allow_all_groups": false,
            "keepalive_interval": 25,
            "peer_disconnect_threshold": 300,
            "acl_enabled": false,
            "acl_default_allow": false,
            "allowed_ips_from_acl": false,
            "mfa_enabled": false,
            "service_location_mode": "disabled",
            "posture_checks": posture_ids,
            "mfa_flows": []
        }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let location: serde_json::Value = response.json().await;
    let location_id = location["id"].as_i64().unwrap();

    let response = client
        .get(format!("/api/v1/network/{location_id}"))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let network: serde_json::Value = response.json().await;
    let assigned_postures: Vec<i64> =
        serde_json::from_value(network["posture_checks"].clone()).unwrap();
    assert_eq!(assigned_postures.len(), 2);
    assert!(assigned_postures.contains(&posture_ids[0]));
    assert!(assigned_postures.contains(&posture_ids[1]));

    for posture_id in posture_ids {
        let response = client
            .get(format!("/api/v1/device-posture/{posture_id}"))
            .send()
            .await;
        assert_eq!(response.status(), StatusCode::OK);
        let posture: serde_json::Value = response.json().await;
        let locations: Vec<i64> = serde_json::from_value(posture["locations"].clone()).unwrap();
        assert_eq!(locations, vec![location_id]);
    }
}

#[sqlx::test]
async fn test_create_network_with_posture_checks_requires_enterprise_license(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let (mut client, mut client_state) = make_test_client(pool).await;
    authenticate_admin(&mut client).await;
    client.drain_all_events();

    let response = client
        .post("/api/v1/network")
        .json(&json!({
            "name": "network-without-enterprise-postures",
            "address": "10.1.1.1/24",
            "port": 55555,
            "endpoint": "192.168.4.14",
            "allowed_ips": "10.1.1.0/24",
            "dns": "1.1.1.1",
            "mtu": 1420,
            "fwmark": 0,
            "allowed_groups": ["admin"],
            "allow_all_groups": false,
            "keepalive_interval": 25,
            "peer_disconnect_threshold": 300,
            "acl_enabled": false,
            "acl_default_allow": false,
            "allowed_ips_from_acl": false,
            "mfa_enabled": false,
            "service_location_mode": "disabled",
            "posture_checks": [1],
            "mfa_flows": []
        }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    client.assert_event_queue_is_empty();
    assert_matches!(
        client_state.gateway_rx.try_recv(),
        Err(tokio::sync::broadcast::error::TryRecvError::Empty)
    );

    let response = client.get("/api/v1/network").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let networks: Vec<serde_json::Value> = response.json().await;
    assert!(networks.iter().all(|network| {
        network["name"].as_str() != Some("network-without-enterprise-postures")
    }));
}

/// Build a location payload with overridable name, address and mode fields.
fn location_payload(
    name: &str,
    address: &str,
    mfa_enabled: bool,
    service_location_mode: &str,
) -> serde_json::Value {
    json!({
        "name": name,
        "address": address,
        "port": 55555,
        "endpoint": "192.168.4.14",
        "allowed_ips": "10.1.1.0/24",
        "dns": "1.1.1.1",
        "mtu": 1420,
        "fwmark": 0,
        "allowed_groups": ["admin"],
        "allow_all_groups": false,
        "keepalive_interval": 25,
        "peer_disconnect_threshold": 300,
        "acl_enabled": false,
        "acl_default_allow": false,
        "allowed_ips_from_acl": false,
        "mfa_enabled": mfa_enabled,
        "service_location_mode": service_location_mode,
        "posture_checks": [],
        "mfa_flows": []
    })
}

/// Create a minimal single-step MFA flow so that locations are allowed to enable MFA.
///
/// `PUT`/`POST /api/v1/network` refuses `mfa_enabled: true` with `no_flows_exist` while no flow
/// exists globally, so any test that enables MFA on a location has to create one first.
///
/// Returns the created flow's id so callers can assign it to a location.
async fn make_mfa_flow(client: &TestClient) -> i64 {
    let response = client
        .post("/api/v1/mfa-flow")
        .json(&json!({
            "title": "Test MFA Flow",
            "steps": [{ "methods": ["totp"] }]
        }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    response.json::<serde_json::Value>().await["id"]
        .as_i64()
        .unwrap()
}

/// Assign a flow as a location's default so the location can be MFA-enabled.
async fn assign_default_mfa_flow(client: &TestClient, location_id: i64, flow_id: i64) {
    let response = update_location_mfa_flows(
        &client,
        location_id,
        json!({ "assignments": [{ "flow_id": flow_id, "is_default": true, "group_ids": [] }] }),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
}

/// Enabling MFA while no flow exists returns a structured `validation_failed` body that parses as
/// JSON in one step, not a JSON string wrapped inside `msg`.
#[sqlx::test]
async fn test_mfa_enabled_no_flows_structured_body(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (mut client, _) = make_test_client(pool).await;
    authenticate_admin(&mut client).await;

    let response = client
        .post("/api/v1/network")
        .json(&json!({
            "name": "no-flows",
            "address": "10.9.9.1/24",
            "port": 55555,
            "endpoint": "192.168.4.14",
            "allowed_ips": "10.9.9.0/24",
            "dns": "1.1.1.1",
            "mtu": 1420,
            "fwmark": 0,
            "allowed_groups": ["admin"],
            "allow_all_groups": false,
            "keepalive_interval": 25,
            "peer_disconnect_threshold": 300,
            "acl_enabled": false,
            "acl_default_allow": false,
            "allowed_ips_from_acl": false,
            "mfa_enabled": true,
            "service_location_mode": "disabled",
            "posture_checks": [],
            "mfa_flows": []
        }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body: serde_json::Value = response.json().await;
    assert_eq!(body["error"], "validation_failed");
    assert_eq!(body["fields"][0]["field"], "mfa_flows");
    assert_eq!(body["fields"][0]["code"], "no_default_designated");
    assert!(
        body.get("msg").is_none(),
        "the refusal body must not be double-encoded via msg"
    );
}

/// Clearing assignments on an MFA-disabled location is allowed, and the location still cannot be
/// MFA-enabled afterward while no flows exist: the `no_flows_exist` precondition still guards the
/// toggle.
#[sqlx::test]
async fn test_enable_mfa_after_clear_refused_without_flows(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let (mut client, _) = make_test_client(pool).await;
    authenticate_admin(&mut client).await;

    // Create a flow and a location, assign, then clear.
    let flow_id = {
        let resp = client
            .post("/api/v1/mfa-flow")
            .json(&json!({"title": "Flow", "steps": [{"methods": ["totp"]}]}))
            .send()
            .await;
        assert_eq!(resp.status(), StatusCode::CREATED);
        resp.json::<serde_json::Value>().await["id"]
            .as_i64()
            .unwrap()
    };

    let network_resp = make_network(&client, "clear-then-enable").await;
    let location_id = network_resp.json::<serde_json::Value>().await["id"]
        .as_i64()
        .unwrap();

    let response = update_location_mfa_flows(
        &client,
        location_id,
        json!({"assignments": [
            {"flow_id": flow_id, "is_default": true, "group_ids": []},
        ]}),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);

    // Clearing is allowed on the MFA-disabled location.
    let response =
        update_location_mfa_flows(&client, location_id, json!({"assignments": []})).await;
    assert_eq!(response.status(), StatusCode::OK);

    // Delete the now-unassigned flow so no flows exist globally.
    let response = client
        .delete(format!("/api/v1/mfa-flow/{flow_id}"))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    // Re-enabling MFA is still refused: no flows exist to assign.
    let response = client
        .put(format!("/api/v1/network/{location_id}"))
        .json(&json!({
            "name": "clear-then-enable",
            "address": "10.1.1.1/24",
            "port": 55555,
            "endpoint": "192.168.4.14",
            "allowed_ips": "10.1.1.0/24",
            "dns": "1.1.1.1",
            "mtu": 1420,
            "fwmark": 0,
            "allowed_groups": ["admin"],
            "allow_all_groups": false,
            "keepalive_interval": 25,
            "peer_disconnect_threshold": 300,
            "acl_enabled": false,
            "acl_default_allow": false,
            "allowed_ips_from_acl": false,
            "mfa_enabled": true,
            "service_location_mode": "disabled",
            "posture_checks": [],
            "mfa_flows": []
        }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body: serde_json::Value = response.json().await;
    assert_eq!(body["fields"][0]["code"], "no_default_designated");
}

/// Enabling MFA on a location that has no default flow assigned is refused with
/// `no_flows_assigned`, even when a flow exists globally.
#[sqlx::test]
async fn test_enable_mfa_without_assignment_refused(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (mut client, _) = make_test_client(pool).await;
    authenticate_admin(&mut client).await;

    make_mfa_flow(&client).await;
    let network_resp = make_network(&client, "no-assignment").await;
    let location_id = network_resp.json::<serde_json::Value>().await["id"]
        .as_i64()
        .unwrap();

    let response = client
        .put(format!("/api/v1/network/{location_id}"))
        .json(&json!({
            "name": "no-assignment",
            "address": "10.1.1.1/24",
            "port": 55555,
            "endpoint": "192.168.4.14",
            "allowed_ips": "10.1.1.0/24",
            "dns": "1.1.1.1",
            "mtu": 1420,
            "fwmark": 0,
            "allowed_groups": ["admin"],
            "allow_all_groups": false,
            "keepalive_interval": 25,
            "peer_disconnect_threshold": 300,
            "acl_enabled": false,
            "acl_default_allow": false,
            "allowed_ips_from_acl": false,
            "mfa_enabled": true,
            "service_location_mode": "disabled",
            "posture_checks": [],
            "mfa_flows": []
        }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body: serde_json::Value = response.json().await;
    assert_eq!(body["error"], "validation_failed");
    assert_eq!(body["fields"][0]["field"], "mfa_flows");
    assert_eq!(body["fields"][0]["code"], "no_default_designated");
}

/// Create a posture check and return its ID.
async fn make_posture_check(client: &TestClient, name: &str) -> i64 {
    let response = client
        .post("/api/v1/device-posture")
        .json(&json!({
            "name": name,
            "description": null,
            "min_desktop_client_version": null,
            "min_mobile_client_version": null,
            "allow_prerelease_client": false,
            "os_rules": []
        }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let posture: serde_json::Value = response.json().await;
    posture["id"].as_i64().unwrap()
}

/// Fetch the posture checks assigned to a location.
async fn fetch_location_postures(client: &TestClient, location_id: i64) -> Vec<i64> {
    let response = client
        .get(format!("/api/v1/network/{location_id}"))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let network: serde_json::Value = response.json().await;
    serde_json::from_value(network["posture_checks"].clone()).unwrap()
}

#[sqlx::test]
async fn test_modify_network_does_not_notify_gateway_when_commit_fails(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let (mut client, client_state) = make_test_client(pool).await;
    authenticate_admin(&mut client).await;

    let response = client
        .post("/api/v1/network")
        .json(&location_payload(
            "location",
            "10.1.1.1/24",
            false,
            "disabled",
        ))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let location: WireguardNetwork<Id> = response.json().await;

    let pool = client_state.pool.clone();
    let mut gateway_rx = client_state.gateway_rx;
    assert_matches!(
        gateway_rx.try_recv().unwrap(),
        GatewayCommand::NetworkCreated(..)
    );

    sqlx::query(
        "CREATE FUNCTION fail_network_update_commit() RETURNS trigger AS $$
         BEGIN
             RAISE EXCEPTION 'forced commit failure';
         END;
         $$ LANGUAGE plpgsql",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "CREATE CONSTRAINT TRIGGER fail_network_update_commit
         AFTER UPDATE ON wireguard_network
         DEFERRABLE INITIALLY DEFERRED
         FOR EACH ROW EXECUTE FUNCTION fail_network_update_commit()",
    )
    .execute(&pool)
    .await
    .unwrap();

    let response = client
        .put(format!("/api/v1/network/{}", location.id))
        .json(&location_payload(
            "renamed-location",
            "10.1.1.1/24",
            false,
            "disabled",
        ))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert_matches!(
        gateway_rx.try_recv(),
        Err(tokio::sync::broadcast::error::TryRecvError::Empty)
    );

    let response = client
        .get(format!("/api/v1/network/{}", location.id))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let persisted: WireguardNetwork<Id> = response.json().await;
    assert_eq!(persisted.name, "location");
}

#[sqlx::test]
async fn test_create_network_rejects_service_location_with_mfa(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let (mut client, _client_state) = make_test_client(pool).await;
    authenticate_admin(&mut client).await;
    set_enterprise_license();

    for service_location_mode in ["prelogon", "alwayson"] {
        let response = client
            .post("/api/v1/network")
            .json(&location_payload(
                "mfa-service-location",
                "10.1.1.1/24",
                true,
                service_location_mode,
            ))
            .send()
            .await;
        assert_eq!(
            response.status(),
            StatusCode::BAD_REQUEST,
            "MFA + service location mode {service_location_mode} must be rejected"
        );
    }

    // A plain location (no service location mode, no MFA) is fine
    let response = client
        .post("/api/v1/network")
        .json(&location_payload("plain", "10.1.1.1/24", false, "disabled"))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // service location mode without MFA is fine
    let response = client
        .post("/api/v1/network")
        .json(&location_payload(
            "service-location-only",
            "10.2.2.1/24",
            false,
            "prelogon",
        ))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
}

/// A zero keepalive stops `last_handshake` from ever advancing on an idle tunnel, which would make
/// the posture health check re-authorize forever (D6/R7). The web forms block it, but an API caller
/// bypasses them entirely, so core has to reject it too.
#[sqlx::test]
async fn test_network_rejects_zero_keepalive_interval(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (mut client, _client_state) = make_test_client(pool).await;
    authenticate_admin(&mut client).await;
    set_enterprise_license();

    let mut payload = location_payload("zero-keepalive", "10.1.1.1/24", false, "disabled");
    payload["keepalive_interval"] = json!(0);
    let response = client.post("/api/v1/network").json(&payload).send().await;
    assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
        "keepalive_interval 0 must be rejected on create"
    );

    // A valid location, so the same rule can be checked on the modify path.
    let response = client
        .post("/api/v1/network")
        .json(&location_payload(
            "good-keepalive",
            "10.2.2.1/24",
            false,
            "disabled",
        ))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let created: serde_json::Value = response.json().await;
    let location_id = created["id"].as_i64().unwrap();

    let mut payload = location_payload("good-keepalive", "10.2.2.1/24", false, "disabled");
    payload["keepalive_interval"] = json!(0);
    let response = client
        .put(format!("/api/v1/network/{location_id}"))
        .json(&payload)
        .send()
        .await;
    assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
        "keepalive_interval 0 must be rejected on modify"
    );

    // 1 is the floor, not a rejected edge.
    payload["keepalive_interval"] = json!(1);
    let response = client
        .put(format!("/api/v1/network/{location_id}"))
        .json(&payload)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
}

#[sqlx::test]
async fn test_modify_network_rejects_service_location_with_mfa(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let (mut client, _client_state) = make_test_client(pool).await;
    authenticate_admin(&mut client).await;
    set_enterprise_license();

    let response = client
        .post("/api/v1/network")
        .json(&location_payload(
            "location",
            "10.1.1.1/24",
            false,
            "disabled",
        ))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let location: WireguardNetwork<Id> = response.json().await;

    for service_location_mode in ["prelogon", "alwayson"] {
        let response = client
            .put(format!("/api/v1/network/{}", location.id))
            .json(&location_payload(
                "location",
                "10.1.1.1/24",
                true,
                service_location_mode,
            ))
            .send()
            .await;
        assert_eq!(
            response.status(),
            StatusCode::BAD_REQUEST,
            "MFA + service location mode {service_location_mode} must be rejected"
        );
    }

    // the rejected combination was not persisted in any form
    let response = client
        .get(format!("/api/v1/network/{}", location.id))
        .send()
        .await;
    let fetched: WireguardNetwork<Id> = response.json().await;
    assert!(!fetched.mfa_enabled);
    assert_eq!(fetched.service_location_mode, ServiceLocationMode::Disabled);

    // enabling service location mode alone is accepted and persisted
    let response = client
        .put(format!("/api/v1/network/{}", location.id))
        .json(&location_payload(
            "location",
            "10.1.1.1/24",
            false,
            "prelogon",
        ))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let modified: WireguardNetwork<Id> = response.json().await;
    assert_eq!(
        modified.service_location_mode,
        ServiceLocationMode::PreLogon
    );
}

#[sqlx::test]
async fn test_modify_network_replaces_posture_checks(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (mut client, _client_state) = make_test_client(pool).await;
    authenticate_admin(&mut client).await;
    set_enterprise_license();

    let posture = make_posture_check(&client, "Posture").await;
    let replacement_posture = make_posture_check(&client, "Replacement posture").await;

    let mut payload = location_payload("location", "10.1.1.1/24", false, "disabled");
    payload["posture_checks"] = json!([posture]);
    let response = client.post("/api/v1/network").json(&payload).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let location: WireguardNetwork<Id> = response.json().await;
    assert_eq!(
        fetch_location_postures(&client, location.id).await,
        vec![posture]
    );

    // an explicit list replaces the current assignments with the location save
    let mut payload = location_payload("renamed-location", "10.1.1.1/24", false, "disabled");
    payload["posture_checks"] = json!([replacement_posture]);
    let response = client
        .put(format!("/api/v1/network/{}", location.id))
        .json(&payload)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let modified: WireguardNetwork<Id> = response.json().await;
    assert_eq!(modified.name, "renamed-location");
    assert_eq!(
        fetch_location_postures(&client, location.id).await,
        vec![replacement_posture]
    );

    // an empty list clears assignments
    let response = client
        .put(format!("/api/v1/network/{}", location.id))
        .json(&location_payload(
            "location",
            "10.1.1.1/24",
            false,
            "disabled",
        ))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert!(
        fetch_location_postures(&client, location.id)
            .await
            .is_empty()
    );
}

#[sqlx::test]
async fn test_modify_network_preserves_posture_checks_without_enterprise_license(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let (mut client, _client_state) = make_test_client(pool).await;
    authenticate_admin(&mut client).await;
    set_enterprise_license();

    let posture = make_posture_check(&client, "Posture").await;
    let mut payload = location_payload("location", "10.1.1.1/24", false, "disabled");
    payload["posture_checks"] = json!([posture]);
    let response = client.post("/api/v1/network").json(&payload).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let location: WireguardNetwork<Id> = response.json().await;

    let license = get_cached_license().clone();
    set_cached_license(None);
    let response = client
        .put(format!("/api/v1/network/{}", location.id))
        .json(&location_payload(
            "renamed-location",
            "10.1.1.1/24",
            false,
            "disabled",
        ))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let modified: WireguardNetwork<Id> = response.json().await;
    assert_eq!(modified.name, "renamed-location");
    assert_eq!(
        fetch_location_postures(&client, location.id).await,
        vec![posture]
    );
    set_cached_license(license);
}

#[sqlx::test]
async fn test_posture_checks_allowed_on_service_locations(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let (mut client, _client_state) = make_test_client(pool).await;
    authenticate_admin(&mut client).await;
    set_enterprise_license();

    let posture = make_posture_check(&client, "Posture").await;

    // create path: a service location may carry posture checks
    let mut payload = location_payload("service-location", "10.1.1.1/24", false, "prelogon");
    payload["posture_checks"] = json!([posture]);
    let response = client.post("/api/v1/network").json(&payload).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let service_location: WireguardNetwork<Id> = response.json().await;
    assert_eq!(
        service_location.service_location_mode,
        ServiceLocationMode::PreLogon
    );
    assert_eq!(
        fetch_location_postures(&client, service_location.id).await,
        vec![posture]
    );

    // modify path: turning a posture-carrying regular location into a service
    // location keeps its posture checks
    let mut payload = location_payload("regular-location", "10.2.2.1/24", false, "disabled");
    payload["posture_checks"] = json!([posture]);
    let response = client.post("/api/v1/network").json(&payload).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let location: WireguardNetwork<Id> = response.json().await;

    let mut payload = location_payload("regular-location", "10.2.2.1/24", false, "alwayson");
    payload["posture_checks"] = json!([posture]);
    let response = client
        .put(format!("/api/v1/network/{}", location.id))
        .json(&payload)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let modified: WireguardNetwork<Id> = response.json().await;
    assert_eq!(
        modified.service_location_mode,
        ServiceLocationMode::AlwaysOn
    );
    assert_eq!(
        fetch_location_postures(&client, location.id).await,
        vec![posture]
    );

    // posture checks can be assigned to an existing service location through the location save
    let response = client
        .post("/api/v1/network")
        .json(&location_payload(
            "service-location-without-postures",
            "10.3.3.1/24",
            false,
            "alwayson",
        ))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let service_location_without_postures: WireguardNetwork<Id> = response.json().await;
    assert!(
        fetch_location_postures(&client, service_location_without_postures.id)
            .await
            .is_empty()
    );

    let response = update_location_posture_checks(
        &client,
        service_location_without_postures.id,
        json!([posture]),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        fetch_location_postures(&client, service_location_without_postures.id).await,
        vec![posture]
    );
}

#[sqlx::test]
async fn test_peer_disconnect_threshold_validation_create(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;

    let (mut client, _client_state) = make_test_client(pool).await;
    authenticate_admin(&mut client).await;
    make_mfa_flow(&client).await;

    let mut location_data = WireguardNetworkData {
        name: "test_location_disabled".into(),
        address: "10.1.1.1/24".into(),
        endpoint: "10.1.1.1".parse().unwrap(),
        port: 55555,
        allowed_ips: Some("10.1.1.0/24, 10.2.0.1/16, 10.10.10.54/32".into()),
        dns: None,
        mtu: DEFAULT_WIREGUARD_MTU,
        fwmark: 0,
        allow_all_groups: false,
        allowed_groups: vec!["admin".into()],
        keepalive_interval: DEFAULT_KEEPALIVE_INTERVAL,
        peer_disconnect_threshold: INVALID_MFA_PEER_DISCONNECT_THRESHOLD,
        acl_enabled: false,
        acl_default_allow: false,
        allowed_ips_from_acl: false,
        mfa_enabled: false,
        service_location_mode: ServiceLocationMode::Disabled,
        posture_checks: Vec::new(),
        mfa_flows: Vec::new(),
    };

    let response = client
        .post("/api/v1/network")
        .json(&location_data)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    location_data.name = "test_location_internal".into();
    location_data.mfa_enabled = true;
    let response = client
        .post("/api/v1/network")
        .json(&location_data)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // Even with a valid threshold, creating a location already MFA-enabled is refused: a new
    // location has no default flow assigned yet, so enabling at create is always rejected.
    location_data.name = "test_location_internal_boundary".into();
    location_data.peer_disconnect_threshold = MINIMUM_MFA_PEER_DISCONNECT_THRESHOLD;
    let response = client
        .post("/api/v1/network")
        .json(&location_data)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body: serde_json::Value = response.json().await;
    assert_eq!(body["fields"][0]["code"], "no_default_designated");
}

#[sqlx::test]
async fn test_peer_disconnect_threshold_validation_modify(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;

    let (mut client, _client_state) = make_test_client(pool).await;
    authenticate_admin(&mut client).await;
    let flow_id = make_mfa_flow(&client).await;

    let mut location_data = WireguardNetworkData {
        name: "test_location".into(),
        address: "10.1.1.1/24".into(),
        endpoint: "10.1.1.1".parse().unwrap(),
        port: 55555,
        allowed_ips: Some("10.1.1.0/24, 10.2.0.1/16, 10.10.10.54/32".into()),
        dns: None,
        mtu: DEFAULT_WIREGUARD_MTU,
        fwmark: 0,
        allow_all_groups: false,
        allowed_groups: vec!["admin".into()],
        keepalive_interval: DEFAULT_KEEPALIVE_INTERVAL,
        peer_disconnect_threshold: INVALID_MFA_PEER_DISCONNECT_THRESHOLD,
        acl_enabled: false,
        acl_default_allow: false,
        allowed_ips_from_acl: false,
        mfa_enabled: false,
        service_location_mode: ServiceLocationMode::Disabled,
        posture_checks: Vec::new(),
        mfa_flows: Vec::new(),
    };

    let response = client
        .post("/api/v1/network")
        .json(&location_data)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // Give the location a default flow so the threshold checks below operate on a
    // MFA-enableable location.
    assign_default_mfa_flow(&client, 1, flow_id).await;
    location_data.mfa_flows = vec![
        defguard_common::db::models::mfa_flow::LocationMfaFlowAssignment {
            flow_id,
            is_default: true,
            group_ids: Vec::new(),
        },
    ];

    let response = client
        .put("/api/v1/network/1")
        .json(&location_data)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    location_data.mfa_enabled = true;
    let response = client
        .put("/api/v1/network/1")
        .json(&location_data)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    location_data.peer_disconnect_threshold = MINIMUM_MFA_PEER_DISCONNECT_THRESHOLD;
    let response = client
        .put("/api/v1/network/1")
        .json(&location_data)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
}

#[sqlx::test]
async fn test_device(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (client, client_state) = make_test_client(pool).await;

    let mut gateway_rx = client_state.gateway_rx;

    let auth = Auth::new("admin", "pass123");
    let response = &client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // create network
    make_network(&client, "network").await;
    let event = gateway_rx.try_recv().unwrap();
    assert_matches!(event, GatewayCommand::NetworkCreated(..));

    // network details
    let response = client.get("/api/v1/network/1").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let network_from_details: WireguardNetwork<Id> = response.json().await;

    // create device
    let device = json!({
        "name": "device",
        "wireguard_pubkey": "LQKsT6/3HWKuJmMulH63R8iK+5sI8FyYEL6WDIi6lQU=",
    });
    let response = client
        .post("/api/v1/device/admin")
        .json(&device)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let event = gateway_rx.try_recv().unwrap();
    assert_matches!(event, GatewayCommand::DeviceCreated(..));

    // an IP was assigned for new device
    let network_devices = WireguardNetworkDevice::find_by_device(&client_state.pool, 1)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        network_devices[0].wireguard_network_id,
        network_from_details.id
    );

    // add another network
    make_network(&client, "network").await;
    assert_matches!(
        gateway_rx.try_recv().unwrap(),
        GatewayCommand::NetworkCreated(..)
    );

    // an IP was assigned for an existing device
    let network_devices = WireguardNetworkDevice::find_by_device(&client_state.pool, 1)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(network_devices.len(), 2);

    // list devices
    let response = client.get("/api/v1/device").json(&device).send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let devices: Vec<Device<Id>> = response.json().await;
    assert_eq!(devices.len(), 1);
    let device = devices[0].clone();
    assert_eq!(device.name, "device");
    assert_eq!(
        device.wireguard_pubkey,
        "LQKsT6/3HWKuJmMulH63R8iK+5sI8FyYEL6WDIi6lQU="
    );

    // list user devices
    let response = client
        .get("/api/v1/device/user/admin")
        .json(&device)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let user_devices: Vec<Device<Id>> = response.json().await;
    assert_eq!(user_devices.len(), 1);
    assert_eq!(devices.len(), 1);
    assert_eq!(device.id, user_devices[0].id);

    // modify device
    let modified_name = "modified-device";
    let modified_key = "sIhx53MsX+iLk83sssybHrD7M+5m+CmpLzWL/zo8C38=";
    let mut modified_device = device.clone();
    modified_device.name = modified_name.into();
    modified_device.wireguard_pubkey = modified_key.into();
    let response = client
        .put(format!("/api/v1/device/{}", device.id))
        .json(&modified_device)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let event = gateway_rx.try_recv().unwrap();
    assert_matches!(event, GatewayCommand::DeviceModified(..));

    // device details
    let response = client
        .get(format!("/api/v1/device/{}", device.id))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let device_from_details: Device<Id> = response.json().await;
    assert_eq!(device_from_details.name, modified_name);
    assert_eq!(device_from_details.wireguard_pubkey, modified_key);

    // device config
    let response = client
        .get(format!("/api/v1/network/1/device/{}/config", device.id))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let config = response.text().await;
    assert_eq!(
        config,
        format!(
            "[Interface]\n\
            PrivateKey = YOUR_PRIVATE_KEY\n\
            Address = 10.1.1.2\n\
            DNS = 1.1.1.1\n\
            \n\
            [Peer]\n\
            PublicKey = {}\n\
            AllowedIPs = 10.1.1.0/24\n\
            Endpoint = 192.168.4.14:55555\n\
            PersistentKeepalive = 25",
            network_from_details.pubkey
        )
    );

    let response = client
        .delete(format!("/api/v1/network/{}", network_from_details.id))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let event = gateway_rx.try_recv().unwrap();
    assert_matches!(event, GatewayCommand::NetworkDeleted(..));

    // delete device
    let response = client
        .delete(format!("/api/v1/device/{}", device.id))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let event = gateway_rx.try_recv().unwrap();
    assert_matches!(event, GatewayCommand::DeviceDeleted(..));

    let response = client.get("/api/v1/device").json(&device).send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let devices: Vec<Device<Id>> = response.json().await;
    assert!(devices.is_empty());
}

#[sqlx::test]
async fn test_network_address_reassignment(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (client, client_state) = make_test_client(pool).await;

    let auth = Auth::new("admin", "pass123");
    let response = &client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // create network
    let response = make_network(&client, "network").await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // network details
    let response = client.get("/api/v1/network/1").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let network_details: serde_json::Value = response.json().await;
    let network_id = network_details["id"].as_i64().unwrap();

    // create devices
    let device = json!({
        "name": "device1",
        "wireguard_pubkey": "LQKsT6/3HWKuJmMulH63R8iK+5sI8FyYEL6WDIi6lQU=",
    });
    let response = client
        .post("/api/v1/device/admin")
        .json(&device)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let device1: serde_json::Value = response.json().await;
    let device1_id = device1["device"]["id"].as_i64().unwrap();
    let device = json!({
        "name": "device2",
        "wireguard_pubkey": "ZqDlG4LQZRO9v57Sd27AHdtTLxegbMp5oVThjYrg21I=",
    });
    let response = client
        .post("/api/v1/device/admin")
        .json(&device)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let device2: serde_json::Value = response.json().await;
    let device2_id = device2["device"]["id"].as_i64().unwrap();

    // ensure IPs were assigned for new devices
    let network_devices = WireguardNetworkDevice::find_by_device(&client_state.pool, device1_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        network_devices[0].wireguard_ips,
        vec![IpAddr::V4(Ipv4Addr::new(10, 1, 1, 2))],
    );
    let network_devices = WireguardNetworkDevice::find_by_device(&client_state.pool, device2_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        network_devices[0].wireguard_ips,
        vec![IpAddr::V4(Ipv4Addr::new(10, 1, 1, 3))],
    );

    // trying to modify network addresses while devices exist shouldn't fail
    let network = json!({
        "id": network_id,
        "name": "network",
        "address": "10.1.1.1/24,fc00::1/112",
        "port": 55555,
        "endpoint": "192.168.4.14",
        "allowed_ips": "10.1.1.0/24",
        "dns": "1.1.1.1",
        "mtu": 1420,
        "fwmark": 0,
        "allow_all_groups": false,
        "allowed_groups": ["admin"],
        "keepalive_interval": 25,
        "peer_disconnect_threshold": 300,
        "acl_enabled": false,
        "acl_default_allow": false,
            "allowed_ips_from_acl": false,
        "mfa_enabled": false,
        "service_location_mode": "disabled",
        "posture_checks": [],
        "mfa_flows": []
    });
    let response = client
        .put(format!("/api/v1/network/{network_id}"))
        .json(&network)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    // delete both devices
    let response = client
        .delete(format!("/api/v1/device/{device1_id}"))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let response = client
        .delete(format!("/api/v1/device/{device2_id}"))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    // re-create a device and verify it gets IPs in both subnets
    let device = json!({
        "name": "device3",
        "wireguard_pubkey": "o/8q3kmv5nnbrcb/7aceQWGE44a0yI707wObXRyyWGU=",
    });
    let response = client
        .post("/api/v1/device/admin")
        .json(&device)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let device3: serde_json::Value = response.json().await;
    let device3_id = device3["device"]["id"].as_i64().unwrap();

    let network_devices = WireguardNetworkDevice::find_by_device(&client_state.pool, device3_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        network_devices[0].wireguard_ips,
        vec![
            IpAddr::V4(Ipv4Addr::new(10, 1, 1, 2)),
            IpAddr::V6(Ipv6Addr::new(0xfc00, 0, 0, 0, 0, 0, 0, 2)),
        ],
    );
}

#[sqlx::test]
async fn test_device_permissions(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (client, _) = make_test_client(pool).await;

    let auth = Auth::new("admin", "pass123");
    let response = &client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // create network
    make_network(&client, "network").await;

    // admin can add devices for other users
    let device = json!({
        "name": "device_1",
        "wireguard_pubkey": "LQKsT6/3HWKuJmMulH63R8iK+5sI8FyYEL6WDIi6lQU=",
    });
    let response = client
        .post("/api/v1/device/admin")
        .json(&device)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let device = json!({"devices": [{
        "name": "device_2",
        "wireguard_ips": ["10.0.0.3"],
        "wireguard_pubkey": "TJgN9JzUF5zdZAPYD96G/Wys2M3TvaT5TIrErUl20nI=",
        "user_id": 1,
        "created": "2023-05-05T23:56:04"
    }]});
    let response = client
        .post("/api/v1/network/1/devices")
        .json(&device)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    let device = json!({
        "name": "device_3",
        "wireguard_pubkey": "PKY3zg5/ecNyMjqLi6yJ3jwb4PvC/SGzjhJ3jrn2vVQ=",
    });
    let response = client
        .post("/api/v1/device/hpotter")
        .json(&device)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let device = json!({"devices": [{
        "name": "device_4",
        "wireguard_ips": ["10.0.0.5"],
        "wireguard_pubkey": "gTMFF29nNLkJR1UhoiO3ZJLF60h2hW+WxmIu5DGJ0B4=",
        "user_id": 2,
        "created": "2023-05-05T23:56:04"
    }]});
    let response = client
        .post("/api/v1/network/1/devices")
        .json(&device)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // normal user cannot add devices for other users or import multiple devices
    let auth = Auth::new("hpotter", "pass123");
    let response = &client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    let device = json!({
        "name": "device_5",
        "wireguard_pubkey": "qhLnyggsD1nVOcLdTk0q43kOZHHknPQgftBY+ZLy40Q=",
    });
    let response = client
        .post("/api/v1/device/hpotter")
        .json(&device)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let device = json!({"devices": [{
        "name": "device_6",
        "wireguard_ips": ["10.0.0.7"],
        "wireguard_pubkey": "xGLqgxVAnmk9+tsj5X/wzwouwx3bF1b3W+VWAb4NLjM=",
        "user_id": 2,
        "created": "2023-05-05T23:56:04"
    }]});
    let response = client
        .post("/api/v1/network/1/devices")
        .json(&device)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let device = json!({
        "name": "device_7",
        "wireguard_pubkey": "J4p/w6R0xt4c2dIBDJ73BmzGJeF0QLW/iihPnISJMkg=",
    });
    let response = client
        .post("/api/v1/device/admin")
        .json(&device)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let device = json!({"devices": [{
        "name": "device_8",
        "wireguard_ips": ["10.0.0.9"],
        "wireguard_pubkey": "A2cg4qMe+s0MSFlV6xyhz7XY6PrET6mli9GVSUshXAk=",
        "user_id": 1,
        "created": "2023-05-05T23:56:04"
    }]});
    let response = client
        .post("/api/v1/network/1/devices")
        .json(&device)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    // normal user cannot list devices of other users
    let response = client.get("/api/v1/device/user/admin").send().await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let response = client.get("/api/v1/device/user/hpotter").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let user_devices: Vec<Device<Id>> = response.json().await;
    assert_eq!(user_devices.len(), 3);

    // admin can list devices of other users
    let auth = Auth::new("admin", "pass123");
    let response = &client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    let response = client.get("/api/v1/device/user/admin").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let user_devices: Vec<Device<Id>> = response.json().await;
    assert_eq!(user_devices.len(), 2);

    let response = client.get("/api/v1/device/user/hpotter").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let user_devices: Vec<Device<Id>> = response.json().await;
    assert_eq!(user_devices.len(), 3);
}

#[sqlx::test]
async fn test_device_pubkey(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (client, client_state) = make_test_client(pool).await;

    let mut gateway_rx = client_state.gateway_rx;

    let auth = Auth::new("admin", "pass123");
    let response = &client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // create network
    make_network(&client, "network").await;
    let event = gateway_rx.try_recv().unwrap();
    assert_matches!(event, GatewayCommand::NetworkCreated(..));

    // network details
    let response = client.get("/api/v1/network/1").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let network_from_details: WireguardNetwork<Id> = response.json().await;

    // create bad device
    let device = json!({
        "name": "device",
        "wireguard_pubkey": network_from_details.pubkey.clone(),
    });
    let response = client
        .post("/api/v1/device/admin")
        .json(&device)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // create another bad device
    let device = json!({
        "name": "device",
        "wireguard_pubkey": "invalid_key",
    });
    let response = client
        .post("/api/v1/device/admin")
        .json(&device)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // create good device
    let device = json!({
        "name": "device",
        "wireguard_pubkey": "LQKsT6/3HWKuJmMulH63R8iK+5sI8FyYEL6WDIi6lQU=",
    });
    let response = client
        .post("/api/v1/device/admin")
        .json(&device)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // list devices
    let response = client.get("/api/v1/device").json(&device).send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let devices: Vec<Device<Id>> = response.json().await;
    assert_eq!(devices.len(), 1);

    // modify device
    let mut device = devices[0].clone();
    device.wireguard_pubkey = network_from_details.pubkey;
    let response = client
        .put(format!("/api/v1/device/{}", device.id))
        .json(&device)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // try to create multiple devices
    let devices = json!({"devices": [{
        "name": "device_2",
        "wireguard_ips": ["10.0.0.9"],
        "wireguard_pubkey": "o/8q3kmv5nnbrcb/7aceQWGE44a0yI707wObXRyyWGU=",
        "user_id": 1,
        "created": "2023-05-05T23:56:04"
    },
    {
        "name": "device_3",
        "wireguard_ips": ["10.0.0.10"],
        "wireguard_pubkey": "invalid_key",
        "user_id": 1,
        "created": "2023-05-05T23:56:04"
    }]});
    let response = client
        .post("/api/v1/network/1/devices")
        .json(&devices)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // make sure no device was created
    let response = client.get("/api/v1/device").json(&device).send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let devices: Vec<Device<Id>> = response.json().await;
    assert_eq!(devices.len(), 1);
}

#[sqlx::test]
async fn test_network_size_validation(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (client, _client_state) = make_test_client(pool).await;

    let auth = Auth::new("admin", "pass123");
    let response = &client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // create network
    let response = make_network(&client, "network").await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // network details
    let response = client.get("/api/v1/network/1").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let network_from_details: WireguardNetwork<Id> = response.json().await;

    // try to add subnet with invalid mask (/0)
    let network = json!({
        "id": network_from_details.id,
        "name": "network",
        "address": "10.2.0.1/24,10.1.1.1/0",
        "port": 55555,
        "endpoint": "192.168.4.14",
        "allowed_ips": "10.1.1.0/24",
        "dns": "1.1.1.1",
        "mtu": 1420,
        "fwmark": 0,
        "allow_all_groups": false,
        "allowed_groups": ["admin"],
        "keepalive_interval": 25,
        "peer_disconnect_threshold": 300,
        "acl_enabled": false,
        "acl_default_allow": false,
            "allowed_ips_from_acl": false,
        "mfa_enabled": false,
        "service_location_mode": "disabled",
        "posture_checks": [],
        "mfa_flows": []
    });
    let response = client
        .put(format!("/api/v1/network/{}", network_from_details.id))
        .json(&network)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // try to add no network (empty address)
    let network = json!({
        "id": network_from_details.id,
        "name": "network",
        "address": "",
        "port": 55555,
        "endpoint": "192.168.4.14",
        "allowed_ips": "10.1.1.0/24",
        "dns": "1.1.1.1",
        "mtu": 1420,
        "fwmark": 0,
        "allow_all_groups": false,
        "allowed_groups": ["admin"],
        "keepalive_interval": 25,
        "peer_disconnect_threshold": 300,
        "acl_enabled": false,
        "acl_default_allow": false,
            "allowed_ips_from_acl": false,
        "mfa_enabled": false,
        "service_location_mode": "disabled",
        "posture_checks": [],
        "mfa_flows": []
    });
    let response = client
        .put(format!("/api/v1/network/{}", network_from_details.id))
        .json(&network)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[derive(serde::Deserialize)]
struct DeviceWireGuardConfig {
    network_id: Id,
    network_name: String,
    config: String,
}

/// A user allowed in a single location returns exactly one device config.
#[sqlx::test]
async fn test_user_device_configs_single_network(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (client, _) = make_test_client(pool).await;

    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    let network: WireguardNetwork<Id> = make_network(&client, "network").await.json().await;

    let device_payload = json!({
        "name": "device",
        "wireguard_pubkey": "LQKsT6/3HWKuJmMulH63R8iK+5sI8FyYEL6WDIi6lQU=",
    });
    let response = client
        .post("/api/v1/device/admin")
        .json(&device_payload)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let device: serde_json::Value = response.json().await;
    let device_id = device["device"]["id"].as_i64().unwrap();

    let response = client
        .get(format!("/api/v1/device/{device_id}/config"))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let configs: Vec<DeviceWireGuardConfig> = response.json().await;

    assert_eq!(configs.len(), 1);
    assert_eq!(configs[0].network_id, network.id);
    assert_eq!(configs[0].network_name, network.name);
    assert!(configs[0].config.contains("[Interface]"));
    assert!(configs[0].config.contains("[Peer]"));
}

/// A user allowed in multiple networks returns a device config entry for each location.
#[sqlx::test]
async fn test_user_device_configs_multiple_networks(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (client, _) = make_test_client(pool).await;

    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    let location1: WireguardNetwork<Id> = make_network(&client, "location-1").await.json().await;
    let location2: WireguardNetwork<Id> = make_network(&client, "location-2").await.json().await;

    // Both locations use allow_all_groups=true (make_network default), so the device
    // will be allowed in both when created.
    let device_payload = json!({
        "name": "device",
        "wireguard_pubkey": "LQKsT6/3HWKuJmMulH63R8iK+5sI8FyYEL6WDIi6lQU=",
    });
    let response = client
        .post("/api/v1/device/admin")
        .json(&device_payload)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let device: serde_json::Value = response.json().await;
    let device_id = device["device"]["id"].as_i64().unwrap();

    let response = client
        .get(format!("/api/v1/device/{device_id}/config"))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let configs: Vec<DeviceWireGuardConfig> = response.json().await;

    assert_eq!(configs.len(), 2, "expected configs for both locations");
    let ids: Vec<Id> = configs.iter().map(|c| c.network_id).collect();
    assert!(ids.contains(&location1.id), "config for location-1 missing");
    assert!(ids.contains(&location2.id), "config for location-2 missing");
    for cfg in &configs {
        assert!(cfg.config.contains("[Interface]"));
        assert!(cfg.config.contains("[Peer]"));
    }
}

/// A non-admin user can fetch configs for their own device but not for another user's device.
#[sqlx::test]
async fn test_user_device_configs_auth(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (client, _) = make_test_client(pool).await;

    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // Create a location that allows all users (not just admin group)
    client
        .post("/api/v1/network")
        .json(&json!({
            "name": "network",
            "address": "10.1.1.1/24",
            "port": 55555,
            "endpoint": "192.168.4.14",
            "allowed_ips": "10.1.1.0/24",
            "dns": "1.1.1.1",
            "mtu": 1420,
            "fwmark": 0,
            "allowed_groups": [],
            "allow_all_groups": true,
            "keepalive_interval": 25,
            "peer_disconnect_threshold": 300,
            "acl_enabled": false,
            "acl_default_allow": false,
            "allowed_ips_from_acl": false,
            "mfa_enabled": false,
            "service_location_mode": "disabled",
            "posture_checks": [],
            "mfa_flows": []
        }))
        .send()
        .await;

    // Create a device for hpotter (non-admin user)
    let device_payload = json!({
        "name": "hpotter-device",
        "wireguard_pubkey": "LQKsT6/3HWKuJmMulH63R8iK+5sI8FyYEL6WDIi6lQU=",
    });
    let response = client
        .post("/api/v1/device/hpotter")
        .json(&device_payload)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let hpotter_device: serde_json::Value = response.json().await;
    let hpotter_device_id = hpotter_device["device"]["id"].as_i64().unwrap();

    // Create a device for admin
    let device_payload = json!({
        "name": "admin-device",
        "wireguard_pubkey": "sIhx53MsX+iLk83sssybHrD7M+5m+CmpLzWL/zo8C38=",
    });
    let response = client
        .post("/api/v1/device/admin")
        .json(&device_payload)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let admin_device: serde_json::Value = response.json().await;
    let admin_device_id = admin_device["device"]["id"].as_i64().unwrap();

    // Switch to hpotter
    let auth = Auth::new("hpotter", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // hpotter can fetch their own device config
    let response = client
        .get(format!("/api/v1/device/{hpotter_device_id}/config"))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let configs: Vec<DeviceWireGuardConfig> = response.json().await;
    assert_eq!(configs.len(), 1);

    // hpotter cannot fetch admin's device config
    let response = client
        .get(format!("/api/v1/device/{admin_device_id}/config"))
        .send()
        .await;
    assert_eq!(
        response.status(),
        StatusCode::NOT_FOUND,
        "non-admin user should not access another user's device config"
    );
}

/// Regression test: an admin manually adding a device for a disabled user used to
/// silently succeed with an empty config list, since the device was never actually
/// allowed to join any network. Disabled users' devices are also stripped from every
/// network on the next sync anyway (see `process_device_access_changes`), so letting
/// an admin add one would only work until that sync runs.
#[sqlx::test]
async fn test_add_device_for_disabled_user(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (mut client, _) = make_test_client(pool).await;
    authenticate_admin(&mut client).await;

    // Network open to all users, so the only thing blocking the device is `is_active`.
    client
        .post("/api/v1/network")
        .json(&json!({
            "name": "network",
            "address": "10.1.1.1/24",
            "port": 55555,
            "endpoint": "192.168.4.14",
            "allowed_ips": "10.1.1.0/24",
            "dns": "1.1.1.1",
            "mtu": 1420,
            "fwmark": 0,
            "allowed_groups": [],
            "allow_all_groups": true,
            "keepalive_interval": 25,
            "peer_disconnect_threshold": 300,
            "acl_enabled": false,
            "acl_default_allow": false,
            "allowed_ips_from_acl": false,
            "mfa_enabled": false,
            "service_location_mode": "disabled",
            "posture_checks": [],
            "mfa_flows": []
        }))
        .send()
        .await;

    // Disable hpotter.
    let mut user_details = fetch_user_details(&client, "hpotter").await;
    user_details.user.is_active = false;
    let response = client
        .put("/api/v1/user/hpotter")
        .json(&user_details.user)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    // Admin tries to add a device for the disabled user manually. Should be rejected
    // outright, not silently created with no usable config.
    let device_payload = json!({
        "name": "disabled-user-device",
        "wireguard_pubkey": "LQKsT6/3HWKuJmMulH63R8iK+5sI8FyYEL6WDIi6lQU=",
    });
    let response = client
        .post("/api/v1/device/hpotter")
        .json(&device_payload)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

/// MFA locations (internal/external) must be excluded from the user device config endpoint.
/// A user should only receive configs for regular (non-MFA) locations since MFA location
/// connections are possible only with the Defguard client apps, not standard WireGuard clients.
#[sqlx::test]
async fn test_user_device_configs_excludes_mfa_locations(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let (client, _) = make_test_client(pool).await;

    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    let flow_id = make_mfa_flow(&client).await;

    // Create a normal location (allow_all_groups so the device is allowed)
    let normal_location: WireguardNetwork<Id> = client
        .post("/api/v1/network")
        .json(&json!({
            "name": "normal-location",
            "address": "10.1.1.1/24",
            "port": 55555,
            "endpoint": "192.168.4.14",
            "allowed_ips": "10.1.1.0/24",
            "dns": "1.1.1.1",
            "mtu": 1420,
            "fwmark": 0,
            "allowed_groups": [],
            "allow_all_groups": true,
            "keepalive_interval": 25,
            "peer_disconnect_threshold": 300,
            "acl_enabled": false,
            "acl_default_allow": false,
            "allowed_ips_from_acl": false,
            "mfa_enabled": false,
            "service_location_mode": "disabled",
            "posture_checks": [],
            "mfa_flows": []
        }))
        .send()
        .await
        .json()
        .await;

    // Create an MFA location (internal mode, no enterprise license required). It starts with MFA
    // off: a new location cannot be created already enabled, so we assign a flow and enable it.
    let mfa_location: WireguardNetwork<Id> = client
        .post("/api/v1/network")
        .json(&json!({
            "name": "mfa-location",
            "address": "10.1.2.1/24",
            "port": 55556,
            "endpoint": "192.168.4.15",
            "allowed_ips": "10.1.2.0/24",
            "dns": "1.1.1.1",
            "mtu": 1420,
            "fwmark": 0,
            "allowed_groups": [],
            "allow_all_groups": true,
            "keepalive_interval": 25,
            "peer_disconnect_threshold": 300,
            "acl_enabled": false,
            "acl_default_allow": false,
            "allowed_ips_from_acl": false,
            "mfa_enabled": false,
            "service_location_mode": "disabled",
            "posture_checks": [],
            "mfa_flows": []
        }))
        .send()
        .await
        .json()
        .await;

    assign_default_mfa_flow(&client, mfa_location.id, flow_id).await;

    let response = client
        .put(format!("/api/v1/network/{}", mfa_location.id))
        .json(&json!({
            "name": "mfa-location",
            "address": "10.1.2.1/24",
            "port": 55556,
            "endpoint": "192.168.4.15",
            "allowed_ips": "10.1.2.0/24",
            "dns": "1.1.1.1",
            "mtu": 1420,
            "fwmark": 0,
            "allowed_groups": [],
            "allow_all_groups": true,
            "keepalive_interval": 25,
            "peer_disconnect_threshold": 300,
            "acl_enabled": false,
            "acl_default_allow": false,
            "allowed_ips_from_acl": false,
            "mfa_enabled": true,
            "service_location_mode": "disabled",
            "posture_checks": [],
            "mfa_flows": [{"flow_id": flow_id, "is_default": true, "group_ids": []}]
        }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    // Create a user device
    let device_payload = json!({
        "name": "device",
        "wireguard_pubkey": "LQKsT6/3HWKuJmMulH63R8iK+5sI8FyYEL6WDIi6lQU=",
    });
    let response = client
        .post("/api/v1/device/admin")
        .json(&device_payload)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let device: serde_json::Value = response.json().await;
    let device_id = device["device"]["id"].as_i64().unwrap();

    let response = client
        .get(format!("/api/v1/device/{device_id}/config"))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let configs: Vec<DeviceWireGuardConfig> = response.json().await;

    // Only the normal location config should be returned
    assert_eq!(configs.len(), 1, "MFA location should be excluded");
    assert_eq!(
        configs[0].network_id, normal_location.id,
        "config should belong to the normal location"
    );
    assert_ne!(
        configs[0].network_id, mfa_location.id,
        "MFA location config must not be returned"
    );
}

#[sqlx::test]
async fn test_location_allowed_ips_from_acl_flag(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (mut client, _client_state) = make_test_client(pool).await;
    authenticate_admin(&mut client).await;

    // Create location with flag enabled
    let create_response = client
        .post("/api/v1/network")
        .json(&json!({
            "name": "acl-ips-location",
            "address": "10.20.1.1/24",
            "port": 55555,
            "endpoint": "192.168.20.1",
            "allowed_ips": "",
            "dns": "",
            "mtu": 1420,
            "fwmark": 0,
            "allowed_groups": ["admin"],
            "allow_all_groups": false,
            "keepalive_interval": 25,
            "peer_disconnect_threshold": 300,
            "acl_enabled": false,
            "acl_default_allow": false,
            "allowed_ips_from_acl": true,
            "mfa_enabled": false,
            "service_location_mode": "disabled",
            "posture_checks": [],
            "mfa_flows": []
        }))
        .send()
        .await;
    assert_eq!(create_response.status(), StatusCode::CREATED);
    let location: WireguardNetwork<Id> = create_response.json().await;
    assert!(
        location.allowed_ips_from_acl,
        "flag should be true after create"
    );

    // Verify API event was emitted for location creation
    let events = client.drain_all_events();
    assert_eq!(
        events.len(),
        1,
        "location save must emit only a location event"
    );
    let (event_type, _user_id, _username) = events
        .iter()
        .find(|event| matches!(event.0, ApiEventType::VpnLocationAdded { .. }))
        .expect("missing location event");
    assert_matches!(
        event_type,
        ApiEventType::VpnLocationAdded { location: event_location }
            if event_location.id == location.id && event_location.allowed_ips_from_acl
    );

    // Edit: toggle flag to false
    let edit_response_off = client
        .put(format!("/api/v1/network/{}", location.id))
        .json(&json!({
            "name": "acl-ips-location",
            "address": "10.20.1.1/24",
            "port": 55555,
            "endpoint": "192.168.20.1",
            "allowed_ips": "",
            "dns": "",
            "mtu": 1420,
            "fwmark": 0,
            "allowed_groups": ["admin"],
            "allow_all_groups": false,
            "keepalive_interval": 25,
            "peer_disconnect_threshold": 300,
            "acl_enabled": false,
            "acl_default_allow": false,
            "allowed_ips_from_acl": false,
            "mfa_enabled": false,
            "service_location_mode": "disabled",
            "posture_checks": [],
            "mfa_flows": []
        }))
        .send()
        .await;
    assert_eq!(edit_response_off.status(), StatusCode::OK);
    let location_off: WireguardNetwork<Id> = edit_response_off.json().await;
    assert!(
        !location_off.allowed_ips_from_acl,
        "flag should be false after toggle off"
    );

    let events = client.drain_all_events();
    assert_eq!(
        events.len(),
        2,
        "location save must emit assignment and location events"
    );
    let (event_type, _user_id, _username) = events
        .iter()
        .find(|event| matches!(event.0, ApiEventType::VpnLocationModified { .. }))
        .expect("missing location event");
    assert_matches!(
        event_type,
        ApiEventType::VpnLocationModified { before: before_loc, after: after_loc }
            if before_loc.id == location.id
                && before_loc.allowed_ips_from_acl
                && after_loc.id == location_off.id
                && !after_loc.allowed_ips_from_acl
    );

    // Edit: toggle flag back to true
    let edit_response_on = client
        .put(format!("/api/v1/network/{}", location_off.id))
        .json(&json!({
            "name": "acl-ips-location",
            "address": "10.20.1.1/24",
            "port": 55555,
            "endpoint": "192.168.20.1",
            "allowed_ips": "",
            "dns": "",
            "mtu": 1420,
            "fwmark": 0,
            "allowed_groups": ["admin"],
            "allow_all_groups": false,
            "keepalive_interval": 25,
            "peer_disconnect_threshold": 300,
            "acl_enabled": false,
            "acl_default_allow": false,
            "allowed_ips_from_acl": true,
            "mfa_enabled": false,
            "service_location_mode": "disabled",
            "posture_checks": [],
            "mfa_flows": []
        }))
        .send()
        .await;
    assert_eq!(edit_response_on.status(), StatusCode::OK);
    let location_on: WireguardNetwork<Id> = edit_response_on.json().await;
    assert!(
        location_on.allowed_ips_from_acl,
        "flag should be true after toggle back on"
    );

    let events = client.drain_all_events();
    assert_eq!(
        events.len(),
        2,
        "location save must emit assignment and location events"
    );
    let (event_type, _user_id, _username) = events
        .iter()
        .find(|event| matches!(event.0, ApiEventType::VpnLocationModified { .. }))
        .expect("missing location event");
    assert_matches!(
        event_type,
        ApiEventType::VpnLocationModified { before: before_loc, after: after_loc }
            if before_loc.id == location_off.id
                && !before_loc.allowed_ips_from_acl
                && after_loc.id == location_on.id
                && after_loc.allowed_ips_from_acl
    );

    // Fetch location and verify flag persisted
    let get_response = client
        .get(format!("/api/v1/network/{}", location_on.id))
        .send()
        .await;
    assert_eq!(get_response.status(), StatusCode::OK);
    let fetched: WireguardNetwork<Id> = get_response.json().await;
    assert!(
        fetched.allowed_ips_from_acl,
        "flag should persist across GET fetch"
    );
}

/// Set a cached enterprise-tier license for tests that need ACL AllowedIPs.
fn set_enterprise_license() {
    set_cached_license(Some(License::new(
        "test_customer".to_owned(),
        false,
        None,
        None,
        None,
        LicenseTier::Enterprise,
        SupportType::Basic,
        vec![],
    )));
}

/// Parse the AllowedIPs line from a WireGuard config string.
/// Returns the comma-separated value, e.g. "10.0.0.0/24, 192.168.1.0/24".
fn parse_allowed_ips_from_config(config: &str) -> String {
    for line in config.lines() {
        let trimmed = line.trim();
        if let Some(ips) = trimmed.strip_prefix("AllowedIPs = ") {
            return ips.to_owned();
        }
    }
    String::new()
}

/// Insert an applied ACL rule that allows all users and targets a
/// specific location with the given destination addresses.
async fn insert_acl_rule_for_location(
    pool: &sqlx::PgPool,
    location_id: Id,
    destination: IpNetwork,
) {
    let mut conn = pool.acquire().await.unwrap();
    let rule = AclRule {
        name: "test-acl-rule".into(),
        state: RuleState::Applied,
        enabled: true,
        allow_all_users: true,
        addresses: vec![destination],
        any_address: false,
        any_port: true,
        any_protocol: true,
        use_manual_destination_settings: true,
        ..Default::default()
    }
    .save(&mut *conn)
    .await
    .unwrap();
    AclRuleNetwork::new(rule.id, location_id)
        .save(&mut *conn)
        .await
        .unwrap();
}

/// Insert an applied ACL rule with `any_address: true` that allows all
/// users and targets a specific location.
async fn insert_any_address_rule_for_location(pool: &sqlx::PgPool, location_id: Id) {
    let mut conn = pool.acquire().await.unwrap();
    let rule = AclRule {
        name: "test-any-address-rule".into(),
        state: RuleState::Applied,
        enabled: true,
        allow_all_users: true,
        any_address: true,
        any_port: true,
        any_protocol: true,
        use_manual_destination_settings: true,
        ..Default::default()
    }
    .save(&mut *conn)
    .await
    .unwrap();
    AclRuleNetwork::new(rule.id, location_id)
        .save(&mut *conn)
        .await
        .unwrap();
}

#[sqlx::test]
async fn test_config_allowed_ips_from_acl_merged(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (mut client, _client_state) = make_test_client(pool.clone()).await;
    set_enterprise_license();
    authenticate_admin(&mut client).await;

    let create_response = client
        .post("/api/v1/network")
        .json(&json!({
            "name": "acl-merged",
            "address": "10.30.1.1/24",
            "port": 55555,
            "endpoint": "192.168.30.1",
            "allowed_ips": "10.100.0.0/16",
            "dns": "",
            "mtu": 1420,
            "fwmark": 0,
            "allowed_groups": ["admin"],
            "allow_all_groups": false,
            "keepalive_interval": 25,
            "peer_disconnect_threshold": 300,
            "acl_enabled": true,
            "acl_default_allow": false,
            "allowed_ips_from_acl": true,
            "mfa_enabled": false,
            "service_location_mode": "disabled",
            "posture_checks": [],
            "mfa_flows": []
        }))
        .send()
        .await;
    assert_eq!(create_response.status(), StatusCode::CREATED);
    let location: WireguardNetwork<Id> = create_response.json().await;

    let acl_destination: IpNetwork = "192.168.1.0/24".parse().unwrap();
    insert_acl_rule_for_location(&pool, location.id, acl_destination).await;

    let device_payload = json!({
        "name": "acl-device",
        "wireguard_pubkey": "LQKsT6/3HWKuJmMulH63R8iK+5sI8FyYEL6WDIi6lQU=",
    });
    let device_response = client
        .post("/api/v1/device/admin")
        .json(&device_payload)
        .send()
        .await;
    assert_eq!(device_response.status(), StatusCode::CREATED);
    let device_json: serde_json::Value = device_response.json().await;
    let device_id = device_json["device"]["id"].as_i64().unwrap();

    let config_response = client
        .get(format!("/api/v1/device/{device_id}/config"))
        .send()
        .await;
    assert_eq!(config_response.status(), StatusCode::OK);
    let configs: Vec<serde_json::Value> = config_response.json().await;
    assert_eq!(configs.len(), 1);
    let config_text = configs[0]["config"].as_str().unwrap();

    let allowed_ips = parse_allowed_ips_from_config(config_text);
    assert!(
        allowed_ips.contains("10.100.0.0/16"),
        "config should contain manual IP 10.100.0.0/16, got: {allowed_ips}"
    );
    assert!(
        allowed_ips.contains("192.168.1.0/24"),
        "config should contain ACL-derived IP 192.168.1.0/24, got: {allowed_ips}"
    );
}

#[sqlx::test]
async fn test_config_allowed_ips_from_acl_no_match(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (mut client, _client_state) = make_test_client(pool.clone()).await;
    set_enterprise_license();
    authenticate_admin(&mut client).await;

    let create_response = client
        .post("/api/v1/network")
        .json(&json!({
            "name": "acl-no-match",
            "address": "10.40.1.1/24",
            "port": 55555,
            "endpoint": "192.168.40.1",
            "allowed_ips": "10.100.0.0/16",
            "dns": "",
            "mtu": 1420,
            "fwmark": 0,
            "allowed_groups": ["admin"],
            "allow_all_groups": false,
            "keepalive_interval": 25,
            "peer_disconnect_threshold": 300,
            "acl_enabled": true,
            "acl_default_allow": false,
            "allowed_ips_from_acl": true,
            "mfa_enabled": false,
            "service_location_mode": "disabled",
            "posture_checks": [],
            "mfa_flows": []
        }))
        .send()
        .await;
    assert_eq!(create_response.status(), StatusCode::CREATED);
    let location: WireguardNetwork<Id> = create_response.json().await;

    // Create ACL rule that only allows a specific user (not admin).
    // Create a dummy user for this purpose.
    let other_user = User::new(
        "other-user",
        Some("password"),
        "Other",
        "User",
        "other@example.com",
        None,
    )
    .save(&pool)
    .await
    .unwrap();

    let destination: IpNetwork = "192.168.1.0/24".parse().unwrap();
    let mut conn = pool.acquire().await.unwrap();
    let rule = AclRule {
        name: "other-user-only".into(),
        state: RuleState::Applied,
        enabled: true,
        allow_all_users: false,
        addresses: vec![destination],
        any_address: false,
        any_port: true,
        any_protocol: true,
        use_manual_destination_settings: true,
        ..Default::default()
    }
    .save(&mut *conn)
    .await
    .unwrap();
    AclRuleNetwork::new(rule.id, location.id)
        .save(&mut *conn)
        .await
        .unwrap();
    AclRuleUser::new(rule.id, other_user.id, true)
        .save(&mut *conn)
        .await
        .unwrap();
    drop(conn);

    let device_payload = json!({
        "name": "acl-no-match-device",
        "wireguard_pubkey": "LQKsT6/3HWKuJmMulH63R8iK+5sI8FyYEL6WDIi6lQU=",
    });
    let device_response = client
        .post("/api/v1/device/admin")
        .json(&device_payload)
        .send()
        .await;
    assert_eq!(device_response.status(), StatusCode::CREATED);
    let device_json: serde_json::Value = device_response.json().await;
    let device_id = device_json["device"]["id"].as_i64().unwrap();

    let config_response = client
        .get(format!("/api/v1/device/{device_id}/config"))
        .send()
        .await;
    assert_eq!(config_response.status(), StatusCode::OK);
    let configs: Vec<serde_json::Value> = config_response.json().await;
    assert_eq!(configs.len(), 1);
    let config_text = configs[0]["config"].as_str().unwrap();

    let allowed_ips = parse_allowed_ips_from_config(config_text);
    assert!(
        allowed_ips.contains("10.100.0.0/16"),
        "config should contain manual IP 10.100.0.0/16, got: {allowed_ips}"
    );
    assert!(
        !allowed_ips.contains("192.168.1.0/24"),
        "config should NOT contain ACL destination (user does not match), got: {allowed_ips}"
    );
}

#[sqlx::test]
async fn test_config_allowed_ips_from_acl_toggle_off(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (mut client, _client_state) = make_test_client(pool.clone()).await;
    set_enterprise_license();
    authenticate_admin(&mut client).await;

    let create_response = client
        .post("/api/v1/network")
        .json(&json!({
            "name": "acl-toggle-off",
            "address": "10.50.1.1/24",
            "port": 55555,
            "endpoint": "192.168.50.1",
            "allowed_ips": "10.100.0.0/16",
            "dns": "",
            "mtu": 1420,
            "fwmark": 0,
            "allowed_groups": ["admin"],
            "allow_all_groups": false,
            "keepalive_interval": 25,
            "peer_disconnect_threshold": 300,
            "acl_enabled": true,
            "acl_default_allow": false,
            "allowed_ips_from_acl": false,
            "mfa_enabled": false,
            "service_location_mode": "disabled",
            "posture_checks": [],
            "mfa_flows": []
        }))
        .send()
        .await;
    assert_eq!(create_response.status(), StatusCode::CREATED);
    let location: WireguardNetwork<Id> = create_response.json().await;

    let destination: IpNetwork = "192.168.1.0/24".parse().unwrap();
    insert_acl_rule_for_location(&pool, location.id, destination).await;

    let device_payload = json!({
        "name": "acl-off-device",
        "wireguard_pubkey": "LQKsT6/3HWKuJmMulH63R8iK+5sI8FyYEL6WDIi6lQU=",
    });
    let device_response = client
        .post("/api/v1/device/admin")
        .json(&device_payload)
        .send()
        .await;
    assert_eq!(device_response.status(), StatusCode::CREATED);
    let device_json: serde_json::Value = device_response.json().await;
    let device_id = device_json["device"]["id"].as_i64().unwrap();

    let config_response = client
        .get(format!("/api/v1/device/{device_id}/config"))
        .send()
        .await;
    assert_eq!(config_response.status(), StatusCode::OK);
    let configs: Vec<serde_json::Value> = config_response.json().await;
    assert_eq!(configs.len(), 1);
    let config_text = configs[0]["config"].as_str().unwrap();

    let allowed_ips = parse_allowed_ips_from_config(config_text);
    assert!(
        allowed_ips.contains("10.100.0.0/16"),
        "config should contain manual IP 10.100.0.0/16, got: {allowed_ips}"
    );
    assert!(
        !allowed_ips.contains("192.168.1.0/24"),
        "config should NOT contain ACL IP when toggle is off, got: {allowed_ips}"
    );
}

#[sqlx::test]
async fn test_config_allowed_ips_from_acl_any_address_skipped(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let (mut client, _client_state) = make_test_client(pool.clone()).await;
    set_enterprise_license();
    authenticate_admin(&mut client).await;

    let create_response = client
        .post("/api/v1/network")
        .json(&json!({
            "name": "acl-any-skipped",
            "address": "10.60.1.1/24",
            "port": 55555,
            "endpoint": "192.168.60.1",
            "allowed_ips": "10.100.0.0/16",
            "dns": "",
            "mtu": 1420,
            "fwmark": 0,
            "allowed_groups": ["admin"],
            "allow_all_groups": false,
            "keepalive_interval": 25,
            "peer_disconnect_threshold": 300,
            "acl_enabled": true,
            "acl_default_allow": false,
            "allowed_ips_from_acl": true,
            "mfa_enabled": false,
            "service_location_mode": "disabled",
            "posture_checks": [],
            "mfa_flows": []
        }))
        .send()
        .await;
    assert_eq!(create_response.status(), StatusCode::CREATED);
    let location: WireguardNetwork<Id> = create_response.json().await;

    insert_any_address_rule_for_location(&pool, location.id).await;
    let concrete_dest: IpNetwork = "192.168.99.0/24".parse().unwrap();
    insert_acl_rule_for_location(&pool, location.id, concrete_dest).await;

    let device_payload = json!({
        "name": "acl-any-device",
        "wireguard_pubkey": "LQKsT6/3HWKuJmMulH63R8iK+5sI8FyYEL6WDIi6lQU=",
    });
    let device_response = client
        .post("/api/v1/device/admin")
        .json(&device_payload)
        .send()
        .await;
    assert_eq!(device_response.status(), StatusCode::CREATED);
    let device_json: serde_json::Value = device_response.json().await;
    let device_id = device_json["device"]["id"].as_i64().unwrap();

    let config_response = client
        .get(format!("/api/v1/device/{device_id}/config"))
        .send()
        .await;
    assert_eq!(config_response.status(), StatusCode::OK);
    let configs: Vec<serde_json::Value> = config_response.json().await;
    assert_eq!(configs.len(), 1);
    let config_text = configs[0]["config"].as_str().unwrap();

    let allowed_ips = parse_allowed_ips_from_config(config_text);
    assert!(
        allowed_ips.contains("10.100.0.0/16"),
        "config should contain manual IP 10.100.0.0/16, got: {allowed_ips}"
    );
    assert!(
        allowed_ips.contains("192.168.99.0/24"),
        "config should contain concrete ACL destination 192.168.99.0/24, got: {allowed_ips}"
    );
    assert!(
        !allowed_ips.contains("0.0.0.0/0"),
        "config should NOT contain 0.0.0.0/0 from any_address rule, got: {allowed_ips}"
    );
    assert!(
        !allowed_ips.contains("::/0"),
        "config should NOT contain ::/0 from any_address rule, got: {allowed_ips}"
    );
}

/// When the enterprise license is not active, the config should only contain
/// manual AllowedIPs even when the toggle is on.
#[sqlx::test]
async fn test_config_allowed_ips_from_acl_no_license(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    // Business license is set by make_test_client; no set_enterprise_license().
    let (mut client, _client_state) = make_test_client(pool.clone()).await;
    authenticate_admin(&mut client).await;

    let create_response = client
        .post("/api/v1/network")
        .json(&json!({
            "name": "acl-no-license",
            "address": "10.70.1.1/24",
            "port": 55555,
            "endpoint": "192.168.70.1",
            "allowed_ips": "10.100.0.0/16",
            "dns": "",
            "mtu": 1420,
            "fwmark": 0,
            "allowed_groups": ["admin"],
            "allow_all_groups": false,
            "keepalive_interval": 25,
            "peer_disconnect_threshold": 300,
            "acl_enabled": true,
            "acl_default_allow": false,
            "allowed_ips_from_acl": true,
            "mfa_enabled": false,
            "service_location_mode": "disabled",
            "posture_checks": [],
            "mfa_flows": []
        }))
        .send()
        .await;
    assert_eq!(create_response.status(), StatusCode::CREATED);
    let location: WireguardNetwork<Id> = create_response.json().await;

    let destination: IpNetwork = "192.168.1.0/24".parse().unwrap();
    insert_acl_rule_for_location(&pool, location.id, destination).await;

    let device_payload = json!({
        "name": "acl-no-license-device",
        "wireguard_pubkey": "LQKsT6/3HWKuJmMulH63R8iK+5sI8FyYEL6WDIi6lQU=",
    });
    let device_response = client
        .post("/api/v1/device/admin")
        .json(&device_payload)
        .send()
        .await;
    assert_eq!(device_response.status(), StatusCode::CREATED);
    let device_json: serde_json::Value = device_response.json().await;
    let device_id = device_json["device"]["id"].as_i64().unwrap();

    let config_response = client
        .get(format!("/api/v1/device/{device_id}/config"))
        .send()
        .await;
    assert_eq!(config_response.status(), StatusCode::OK);
    let configs: Vec<serde_json::Value> = config_response.json().await;
    assert_eq!(configs.len(), 1);
    let config_text = configs[0]["config"].as_str().unwrap();

    let allowed_ips = parse_allowed_ips_from_config(config_text);
    assert!(
        allowed_ips.contains("10.100.0.0/16"),
        "config should contain manual IP 10.100.0.0/16, got: {allowed_ips}"
    );
    assert!(
        !allowed_ips.contains("192.168.1.0/24"),
        "config should NOT contain ACL IP without enterprise license, got: {allowed_ips}"
    );
}

/// When ACL is not enabled on the location but the toggle is on,
/// the config should still only contain manual AllowedIPs.
#[sqlx::test]
async fn test_config_allowed_ips_from_acl_disabled(_: PgPoolOptions, options: PgConnectOptions) {
    set_enterprise_license();
    let pool = setup_pool(options).await;
    let (mut client, _client_state) = make_test_client(pool.clone()).await;
    authenticate_admin(&mut client).await;

    let create_response = client
        .post("/api/v1/network")
        .json(&json!({
            "name": "acl-disabled",
            "address": "10.80.1.1/24",
            "port": 55555,
            "endpoint": "192.168.80.1",
            "allowed_ips": "10.100.0.0/16",
            "dns": "",
            "mtu": 1420,
            "fwmark": 0,
            "allowed_groups": ["admin"],
            "allow_all_groups": false,
            "keepalive_interval": 25,
            "peer_disconnect_threshold": 300,
            "acl_enabled": false,
            "acl_default_allow": false,
            "allowed_ips_from_acl": true,
            "mfa_enabled": false,
            "service_location_mode": "disabled",
            "posture_checks": [],
            "mfa_flows": []
        }))
        .send()
        .await;
    assert_eq!(create_response.status(), StatusCode::CREATED);
    let location: WireguardNetwork<Id> = create_response.json().await;

    let destination: IpNetwork = "192.168.1.0/24".parse().unwrap();
    insert_acl_rule_for_location(&pool, location.id, destination).await;

    let device_payload = json!({
        "name": "acl-disabled-device",
        "wireguard_pubkey": "LQKsT6/3HWKuJmMulH63R8iK+5sI8FyYEL6WDIi6lQU=",
    });
    let device_response = client
        .post("/api/v1/device/admin")
        .json(&device_payload)
        .send()
        .await;
    assert_eq!(device_response.status(), StatusCode::CREATED);
    let device_json: serde_json::Value = device_response.json().await;
    let device_id = device_json["device"]["id"].as_i64().unwrap();

    let config_response = client
        .get(format!("/api/v1/device/{device_id}/config"))
        .send()
        .await;
    assert_eq!(config_response.status(), StatusCode::OK);
    let configs: Vec<serde_json::Value> = config_response.json().await;
    assert_eq!(configs.len(), 1);
    let config_text = configs[0]["config"].as_str().unwrap();

    let allowed_ips = parse_allowed_ips_from_config(config_text);
    assert!(
        allowed_ips.contains("10.100.0.0/16"),
        "config should contain manual IP 10.100.0.0/16, got: {allowed_ips}"
    );
    assert!(
        !allowed_ips.contains("192.168.1.0/24"),
        "config should NOT contain ACL IP when ACL disabled, got: {allowed_ips}"
    );
}
