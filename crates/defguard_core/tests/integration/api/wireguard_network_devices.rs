use std::{assert_matches, net::IpAddr, str::FromStr};

use defguard_common::db::{
    Id,
    models::{Device, DeviceType, WireguardNetwork},
};
use defguard_core::{
    grpc::GatewayCommand,
    handlers::{Auth, network_devices::AddNetworkDevice},
};
use ipnetwork::IpNetwork;
use reqwest::StatusCode;
use serde::Deserialize;
use serde_json::{Value, json};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

use super::common::{
    client::{TestClient, TestResponse},
    make_test_client, setup_pool, update_location_mfa_flows,
};

async fn make_first_network(client: &TestClient) -> TestResponse {
    let response = client
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
        }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    response
}

async fn make_second_network(client: &TestClient) -> TestResponse {
    let response = client
        .post("/api/v1/network")
        .json(&json!({
            "name": "network-2",
            "address": "10.6.1.1/24",
            "port": 55555,
            "endpoint": "192.168.4.14",
            "allowed_ips": "10.6.1.0/24",
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
        }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    response
}

#[derive(Debug, Deserialize, PartialEq)]
struct IpCheckRes {
    available: bool,
    valid: bool,
}

#[derive(Deserialize)]
struct SplitIp {
    ip: IpAddr,
}

#[derive(Deserialize)]
struct DeviceWireGuardConfig {
    network_id: Id,
    network_name: String,
    config: String,
}

#[sqlx::test]
async fn test_network_devices(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (client, client_state) = make_test_client(pool).await;

    let mut gateway_rx = client_state.gateway_rx;

    let auth = Auth::new("admin", "pass123");
    let response = &client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // create networks
    let response = make_first_network(&client).await;
    let network_1: WireguardNetwork<Id> = response.json().await;
    assert_eq!(network_1.name, "network");
    let event = gateway_rx.try_recv().unwrap();
    assert_matches!(event, GatewayCommand::NetworkCreated(..));
    let response = make_second_network(&client).await;
    let network_2: WireguardNetwork<Id> = response.json().await;
    assert_eq!(network_2.name, "network-2");
    let event = gateway_rx.try_recv().unwrap();
    assert_matches!(event, GatewayCommand::NetworkCreated(..));

    // ip suggestions
    let response = client.get("/api/v1/device/network/ip/1").send().await;
    assert_eq!(response.status(), StatusCode::OK);

    let ips: Vec<SplitIp> = response.json().await;
    assert_eq!(ips.len(), 1);
    let network_range = IpNetwork::from_str("10.1.1.1/24").unwrap();
    assert!(network_range.contains(ips[0].ip));

    // checking whether ip is valid/available
    let ip_check = json!({
        "ips": ["10.1.1.2".to_owned()],
    });
    let response = client
        .post("/api/v1/device/network/ip/1")
        .json(&ip_check)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let res = response.json::<Vec<IpCheckRes>>().await;
    let res = res.first().unwrap();
    assert!(res.available);
    assert!(res.valid);

    let ip_check = json!({
        "ips": ["10.1.1.0".to_owned()],
    });
    let response = client
        .post("/api/v1/device/network/ip/1")
        .json(&ip_check)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let res = response.json::<Vec<IpCheckRes>>().await;
    let res = res.first().unwrap();
    assert!(!res.available);
    assert!(res.valid);

    let ip_check = json!({
        "ips": ["10.1.1.1".to_owned()],
    });
    let response = client
        .post("/api/v1/device/network/ip/1")
        .json(&ip_check)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let res = response.json::<Vec<IpCheckRes>>().await;
    let res = res.first().unwrap();
    assert!(!res.available);
    assert!(res.valid);

    let ip_check = json!({
        "ips": ["10.1.1.abc".to_owned()],
    });
    let response = client
        .post("/api/v1/device/network/ip/1")
        .json(&ip_check)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let res = response.json::<Vec<IpCheckRes>>().await;
    let res = res.first().unwrap();
    assert!(!res.available);
    assert!(!res.valid);

    // make network device (manual, WireGuard flow)
    let network_device = AddNetworkDevice {
        name: "device-1".into(),
        wireguard_pubkey: "LQKsT6/3HWKuJmMulH63R8iK+5sI8FyYEL6WDIi6lQU=".into(),
        assigned_ips: ips.iter().map(|ip| ip.ip.to_string()).collect(),
        location_id: 1,
        description: None,
        preshared_key: None,
    };
    let response = client
        .post("/api/v1/device/network")
        .json(&network_device)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let json = response.json::<Value>().await;
    let device_id = json["device"]["id"].as_i64().unwrap();
    let configured = json["device"]["configured"].as_bool().unwrap();
    let config_text = json["config"]["config"].as_str().unwrap();
    assert!(configured);
    let event = gateway_rx.try_recv().unwrap();
    assert_matches!(event, GatewayCommand::DeviceCreated(..));

    // download WG config
    let response = client.get("/api/v1/device/network/1/config").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let response_configs = response.json::<Vec<DeviceWireGuardConfig>>().await;
    assert_eq!(response_configs.len(), 1);
    assert_eq!(response_configs[0].config, config_text);
    assert_eq!(response_configs[0].network_id, network_1.id);
    assert_eq!(response_configs[0].network_name, network_1.name);

    // edit the device
    let modify_device = json!({
        "name": "device-1",
        "description": "new description",
        "assigned_ips": ["10.1.1.3"]
    });
    let response = client
        .put(format!("/api/v1/device/network/{device_id}"))
        .json(&modify_device)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let json = response.json::<Value>().await;
    let description = json["description"].as_str().unwrap();
    assert_eq!(description, "new description");
    assert_eq!(
        json["assigned_ips"],
        serde_json::from_str::<Value>("[\"10.1.1.3\"]").unwrap()
    );
    let device = Device::find_by_id(&client_state.pool, device_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(device.name, "device-1");
    assert_eq!(device.description, Some("new description".to_owned()));
    let event = gateway_rx.try_recv().unwrap();
    assert_matches!(event, GatewayCommand::DeviceModified(..));

    // Make sure the device is only in the selected network
    let device_networks =
        WireguardNetwork::find_network_device_networks(&client_state.pool, device_id)
            .await
            .unwrap();
    assert_eq!(device_networks.len(), 1);
    assert_eq!(network_1.id, device_networks[0].id);

    // Try making cli "enrollment" token for that device
    let response = client
        .post("/api/v1/device/network/start_cli/1")
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let json = response.json::<Value>().await;
    let token = json["enrollment_token"].as_str().unwrap();
    assert_eq!(token.len(), 32);
    let enrollment_url = json["enrollment_url"].as_str().unwrap();
    assert_eq!(enrollment_url, "http://localhost:8080/");

    // Enrollment flow for 2nd device
    let setup_start = json!(
        {
            "name": "device-2",
            "description": "new description",
            "assigned_ips": ["10.1.1.10"],
            "location_id": 1,
        }
    );
    let response = client
        .post("/api/v1/device/network/start_cli")
        .json(&setup_start)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let json = response.json::<Value>().await;
    let token = json["enrollment_token"].as_str().unwrap();
    assert_eq!(token.len(), 32);
    let enrollment_url = json["enrollment_url"].as_str().unwrap();
    assert_eq!(enrollment_url, "http://localhost:8080/");
    let device = Device::find_by_id(&client_state.pool, 2)
        .await
        .unwrap()
        .unwrap();
    assert!(!device.configured);
    assert_eq!(device.name, "device-2");
    let device_network =
        WireguardNetwork::find_network_device_networks(&client_state.pool, device_id)
            .await
            .unwrap();
    assert_eq!(device_network.len(), 1);
    assert_eq!(device_network[0].id, network_1.id);

    // Deleting the device
    let response = client
        .delete(format!("/api/v1/device/network/{device_id}"))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let device = Device::find_by_id(&client_state.pool, device_id)
        .await
        .unwrap();
    assert!(device.is_none());
}

#[sqlx::test]
async fn test_device_ip_validation(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (client, _client_state) = make_test_client(pool).await;

    let auth = Auth::new("admin", "pass123");
    let response = &client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // create location
    let location = json!({
        "name": "test location",
        "address": "10.1.1.1/24, 10.2.2.1/24, 10.3.3.1/24",
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
    let response = client.post("/api/v1/network").json(&location).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let location: WireguardNetwork<Id> = response.json().await;
    let location_id = location.id;

    // IP suggestions
    let response = client
        .get(format!("/api/v1/device/network/ip/{location_id}"))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let ips: Vec<SplitIp> = response.json().await;
    assert_eq!(ips.len(), 3);
    let subnet_1 = IpNetwork::from_str("10.1.1.1/24").unwrap();
    assert!(subnet_1.contains(ips[0].ip));
    let subnet_2 = IpNetwork::from_str("10.2.2.1/24").unwrap();
    assert!(subnet_2.contains(ips[1].ip));
    let subnet_3 = IpNetwork::from_str("10.3.3.1/24").unwrap();
    assert!(subnet_3.contains(ips[2].ip));

    // IP availability validation
    let ip_check = json!({
        "ips": ["10.1.1.2".to_owned(), "10.2.2.2".to_owned(), "10.3.3.2".to_owned()],
    });
    let response = client
        .post(format!("/api/v1/device/network/ip/{location_id}"))
        .json(&ip_check)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let res = response.json::<Vec<IpCheckRes>>().await;
    assert_eq!(res.len(), 3);
    assert_eq!(
        res,
        [
            IpCheckRes {
                available: true,
                valid: true
            },
            IpCheckRes {
                available: true,
                valid: true
            },
            IpCheckRes {
                available: true,
                valid: true
            }
        ]
    );

    let ip_check = json!({
        "ips": ["10.11.1.2".to_owned(), "10.2.2.2".to_owned(), "10.3.3.1".to_owned()],
    });
    let response = client
        .post(format!("/api/v1/device/network/ip/{location_id}"))
        .json(&ip_check)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let res = response.json::<Vec<IpCheckRes>>().await;
    assert_eq!(res.len(), 3);
    assert_eq!(
        res,
        [
            IpCheckRes {
                available: false,
                valid: false
            },
            IpCheckRes {
                available: true,
                valid: true
            },
            IpCheckRes {
                available: false,
                valid: true,
            }
        ]
    );
}

/// The REST network-device config view skips MFA-enabled locations: a network device cannot
/// perform MFA, so its config would be unusable (the gRPC poll rejects with `failed_precondition`).
#[sqlx::test]
async fn test_network_device_config_skips_mfa_location(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let (client, _) = make_test_client(pool).await;

    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // A two-step flow derives no legacy mode.
    let response = client
        .post("/api/v1/mfa-flow")
        .json(&json!({
            "title": "multi-step",
            "steps": [{ "methods": ["totp"] }, { "methods": ["biometric"] }]
        }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let flow_id = response.json::<Value>().await["id"].as_i64().unwrap();

    // Create the location, assign the flow as default, then enable MFA.
    let response = client
        .post("/api/v1/network")
        .json(&json!({
            "name": "multi-step-location",
            "address": "10.7.0.1/24",
            "port": 55555,
            "endpoint": "192.168.4.14",
            "allowed_ips": "10.7.0.0/24",
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
        }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let location: WireguardNetwork<Id> = response.json().await;

    let response = update_location_mfa_flows(
        &client,
        location.id,
        json!([{ "flow_id": flow_id, "is_default": true, "group_ids": [] }]),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);

    let response = client
        .put(format!("/api/v1/network/{}", location.id))
        .json(&json!({
            "name": "multi-step-location",
            "address": "10.7.0.1/24",
            "port": 55555,
            "endpoint": "192.168.4.14",
            "allowed_ips": "10.7.0.0/24",
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
            "mfa_enabled": true,
            "service_location_mode": "disabled",
            "posture_checks": [],
            "mfa_flows": [{"flow_id": flow_id, "is_default": true, "group_ids": []}]
        }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    // Create a network device on the multi-step location.
    let network_device = AddNetworkDevice {
        name: "multi-step-device".into(),
        wireguard_pubkey: "LQKsT6/3HWKuJmMulH63R8iK+5sI8FyYEL6WDIi6lQU=".into(),
        assigned_ips: vec!["10.7.0.2".to_owned()],
        location_id: location.id,
        description: None,
        preshared_key: None,
    };
    let response = client
        .post("/api/v1/device/network")
        .json(&network_device)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let json = response.json::<Value>().await;
    let device_id = json["device"]["id"].as_i64().unwrap();

    // The admin view skips the MFA-enabled location, returning an empty config list.
    let response = client
        .get(format!("/api/v1/device/network/{device_id}/config"))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let configs = response.json::<Vec<DeviceWireGuardConfig>>().await;
    assert!(
        configs.is_empty(),
        "MFA-enabled location config must be skipped"
    );
}

#[sqlx::test]
async fn test_network_device_preshared_key(_: PgPoolOptions, options: PgConnectOptions) {
    const PRESHARED_KEY: &str = "oZFW7IISUedr8RLjI1On2ZoEg4RkCppwGu4BILPwj9Q=";
    const NEW_PRESHARED_KEY: &str = "nEYgUj37nR2RFhX7i29cCrMQWeSwiHriFKKsjtjjWgY=";
    const INVALID_PRESHARED_KEY: &str = "not-a-valid-preshared-key";

    let pool = setup_pool(options).await;
    let (client, client_state) = make_test_client(pool).await;
    let mut gateway_rx = client_state.gateway_rx;

    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    let response = make_first_network(&client).await;
    let network: WireguardNetwork<Id> = response.json().await;
    assert_matches!(
        gateway_rx.try_recv().unwrap(),
        GatewayCommand::NetworkCreated(..)
    );

    let response = client
        .post("/api/v1/device/network")
        .json(&json!({
            "name": "psk-device",
            "description": null,
            "location_id": network.id,
            "assigned_ips": ["10.1.1.2"],
            "wireguard_pubkey": "LQKsT6/3HWKuJmMulH63R8iK+5sI8FyYEL6WDIi6lQU=",
            "preshared_key": INVALID_PRESHARED_KEY,
        }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert!(!response.text().await.contains(INVALID_PRESHARED_KEY));
    assert!(gateway_rx.try_recv().is_err());

    let response = client
        .post("/api/v1/device/network")
        .json(&json!({
            "name": "psk-device",
            "description": null,
            "location_id": network.id,
            "assigned_ips": ["10.1.1.2"],
            "wireguard_pubkey": "LQKsT6/3HWKuJmMulH63R8iK+5sI8FyYEL6WDIi6lQU=",
            "preshared_key": PRESHARED_KEY,
        }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let body = response.text().await;
    assert!(!body.contains(PRESHARED_KEY));
    let json: Value = serde_json::from_str(&body).unwrap();
    let device_id = json["device"]["id"].as_i64().unwrap();
    assert_eq!(json["device"]["preshared_key_set"], true);
    assert!(
        json["config"]["config"]
            .as_str()
            .unwrap()
            .contains("PresharedKey = YOUR_PRESHARED_KEY\n")
    );
    let GatewayCommand::DeviceCreated(device_info) = gateway_rx.try_recv().unwrap() else {
        panic!("expected DeviceCreated");
    };
    assert_eq!(
        device_info.network_info[0]
            .preshared_key
            .as_ref()
            .map(|key| key.expose_secret()),
        Some(PRESHARED_KEY)
    );

    let response = client
        .get(format!("/api/v1/device/network/{device_id}"))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.text().await;
    assert!(!body.contains(PRESHARED_KEY));
    let json: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json["preshared_key_set"], true);

    let response = client
        .get(format!("/api/v1/device/network/{device_id}/config"))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let configs = response.json::<Vec<DeviceWireGuardConfig>>().await;
    assert!(
        configs[0]
            .config
            .contains("PresharedKey = YOUR_PRESHARED_KEY\n")
    );

    // a missing preshared_key keeps the stored key and needs no gateway update
    let response = client
        .put(format!("/api/v1/device/network/{device_id}"))
        .json(&json!({
            "name": "psk-device-renamed",
            "description": null,
            "assigned_ips": ["10.1.1.2"],
        }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.json::<Value>().await["preshared_key_set"], true);
    assert!(gateway_rx.try_recv().is_err());
    assert_eq!(
        Device::find_preshared_key(&client_state.pool, device_id)
            .await
            .unwrap()
            .as_ref()
            .map(|key| key.expose_secret()),
        Some(PRESHARED_KEY)
    );

    let response = client
        .put(format!("/api/v1/device/network/{device_id}"))
        .json(&json!({
            "name": "psk-device-renamed",
            "description": null,
            "assigned_ips": ["10.1.1.2"],
            "preshared_key": INVALID_PRESHARED_KEY,
        }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert!(gateway_rx.try_recv().is_err());

    let response = client
        .put(format!("/api/v1/device/network/{device_id}"))
        .json(&json!({
            "name": "psk-device-renamed",
            "description": null,
            "assigned_ips": ["10.1.1.2"],
            "preshared_key": NEW_PRESHARED_KEY,
        }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let GatewayCommand::DeviceModified(device_info) = gateway_rx.try_recv().unwrap() else {
        panic!("expected DeviceModified");
    };
    assert_eq!(
        device_info.network_info[0]
            .preshared_key
            .as_ref()
            .map(|key| key.expose_secret()),
        Some(NEW_PRESHARED_KEY)
    );

    let response = client
        .put(format!("/api/v1/device/network/{device_id}"))
        .json(&json!({
            "name": "psk-device-renamed",
            "description": null,
            "assigned_ips": ["10.1.1.2"],
            "preshared_key": null,
        }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.json::<Value>().await["preshared_key_set"], false);
    // gateways keep a peer's key on modify, so removal re-creates the peer
    assert_matches!(
        gateway_rx.try_recv().unwrap(),
        GatewayCommand::DeviceDeleted(..)
    );
    let GatewayCommand::DeviceCreated(device_info) = gateway_rx.try_recv().unwrap() else {
        panic!("expected DeviceCreated");
    };
    assert!(device_info.network_info[0].preshared_key.is_none());

    let response = client
        .get(format!("/api/v1/device/network/{device_id}/config"))
        .send()
        .await;
    let configs = response.json::<Vec<DeviceWireGuardConfig>>().await;
    assert!(!configs[0].config.contains("PresharedKey"));

    let response = client
        .post("/api/v1/device/network/start_cli")
        .json(&json!({
            "name": "psk-cli-device",
            "description": null,
            "location_id": network.id,
            "assigned_ips": ["10.1.1.3"],
            "preshared_key": INVALID_PRESHARED_KEY,
        }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert!(!response.text().await.contains(INVALID_PRESHARED_KEY));

    let response = client
        .post("/api/v1/device/network/start_cli")
        .json(&json!({
            "name": "psk-cli-device",
            "description": null,
            "location_id": network.id,
            "assigned_ips": ["10.1.1.3"],
            "preshared_key": PRESHARED_KEY,
        }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    assert!(!response.text().await.contains(PRESHARED_KEY));
    let cli_device = Device::find_by_type(&client_state.pool, DeviceType::Network)
        .await
        .unwrap()
        .into_iter()
        .find(|device| device.name == "psk-cli-device")
        .unwrap();
    assert_eq!(
        Device::find_preshared_key(&client_state.pool, cli_device.id)
            .await
            .unwrap()
            .as_ref()
            .map(|key| key.expose_secret()),
        Some(PRESHARED_KEY)
    );
}

#[sqlx::test]
async fn test_network_device_preshared_key_withheld_in_posture_location(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    const PRESHARED_KEY: &str = "oZFW7IISUedr8RLjI1On2ZoEg4RkCppwGu4BILPwj9Q=";

    let pool = setup_pool(options).await;
    let (client, client_state) = make_test_client(pool).await;
    let mut gateway_rx = client_state.gateway_rx;

    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    let response = make_first_network(&client).await;
    let network: WireguardNetwork<Id> = response.json().await;
    assert_matches!(
        gateway_rx.try_recv().unwrap(),
        GatewayCommand::NetworkCreated(..)
    );

    let posture_id: Id =
        sqlx::query_scalar("INSERT INTO device_posture (name) VALUES ('posture') RETURNING id")
            .fetch_one(&client_state.pool)
            .await
            .unwrap();
    sqlx::query("INSERT INTO device_posture_location (posture_id, location_id) VALUES ($1, $2)")
        .bind(posture_id)
        .bind(network.id)
        .execute(&client_state.pool)
        .await
        .unwrap();

    let response = client
        .post("/api/v1/device/network")
        .json(&json!({
            "name": "psk-device",
            "description": null,
            "location_id": network.id,
            "assigned_ips": ["10.1.1.2"],
            "wireguard_pubkey": "LQKsT6/3HWKuJmMulH63R8iK+5sI8FyYEL6WDIi6lQU=",
            "preshared_key": PRESHARED_KEY,
        }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let json: Value = response.json().await;
    let device_id = json["device"]["id"].as_i64().unwrap();
    assert_eq!(json["device"]["preshared_key_set"], true);
    assert!(
        !json["config"]["config"]
            .as_str()
            .unwrap()
            .contains("PresharedKey")
    );
    let GatewayCommand::DeviceCreated(device_info) = gateway_rx.try_recv().unwrap() else {
        panic!("expected DeviceCreated");
    };
    assert!(device_info.network_info[0].preshared_key.is_none());

    let response = client
        .put(format!("/api/v1/device/network/{device_id}"))
        .json(&json!({
            "name": "psk-device",
            "description": null,
            "assigned_ips": ["10.1.1.2"],
            "preshared_key": "nEYgUj37nR2RFhX7i29cCrMQWeSwiHriFKKsjtjjWgY=",
        }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let GatewayCommand::DeviceModified(device_info) = gateway_rx.try_recv().unwrap() else {
        panic!("expected DeviceModified");
    };
    assert!(device_info.network_info[0].preshared_key.is_none());
}
