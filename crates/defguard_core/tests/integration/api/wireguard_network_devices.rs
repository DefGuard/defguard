use std::{net::IpAddr, str::FromStr};

use defguard_common::db::{
    Id,
    models::{Device, WireguardNetwork},
};
use defguard_core::{
    grpc::GatewayEvent,
    handlers::{Auth, network_devices::AddNetworkDevice},
};
use ipnetwork::IpNetwork;
use matches::assert_matches;
use reqwest::StatusCode;
use serde::Deserialize;
use serde_json::{Value, json};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

use super::common::{
    client::{TestClient, TestResponse},
    make_test_client, setup_pool,
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
            "allowed_groups": [],
            "keepalive_interval": 25,
            "peer_disconnect_threshold": 300,
            "acl_enabled": false,
            "acl_default_allow": false,
            "location_mfa_mode": "disabled",
            "service_location_mode": "disabled"
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
            "allowed_groups": [],
            "keepalive_interval": 25,
            "peer_disconnect_threshold": 300,
            "acl_enabled": false,
            "acl_default_allow": false,
            "location_mfa_mode": "disabled",
            "service_location_mode": "disabled"
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

#[sqlx::test]
async fn test_network_devices(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (client, client_state) = make_test_client(pool).await;

    let mut wg_rx = client_state.wireguard_rx;

    let auth = Auth::new("admin", "pass123");
    let response = &client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // create networks
    let response = make_first_network(&client).await;
    let network_1: WireguardNetwork<Id> = response.json().await;
    assert_eq!(network_1.name, "network");
    let event = wg_rx.try_recv().unwrap();
    assert_matches!(event, GatewayEvent::NetworkCreated(..));
    let response = make_second_network(&client).await;
    let network_2: WireguardNetwork<Id> = response.json().await;
    assert_eq!(network_2.name, "network-2");
    let event = wg_rx.try_recv().unwrap();
    assert_matches!(event, GatewayEvent::NetworkCreated(..));

    // ip suggestions
    let response = client.get("/api/v1/device/network/ip/1").send().await;
    assert_eq!(response.status(), StatusCode::OK);

    let ips: Vec<SplitIp> = response.json().await;
    assert_eq!(ips.len(), 1);
    let network_range = IpNetwork::from_str("10.1.1.1/24").unwrap();
    assert!(network_range.contains(ips[0].ip));

    // checking whether ip is valid/available
    let ip_check = json!({
        "ips": ["10.1.1.2".to_string()],
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
        "ips": ["10.1.1.0".to_string()],
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
        "ips": ["10.1.1.1".to_string()],
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
        "ips": ["10.1.1.abc".to_string()],
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
    let event = wg_rx.try_recv().unwrap();
    assert_matches!(event, GatewayEvent::DeviceCreated(..));

    // download WG config
    let response = client.get("/api/v1/device/network/1/config").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let response_config = response.text().await;
    assert_eq!(response_config, config_text);

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
    assert_eq!(device.description, Some("new description".to_string()));
    let event = wg_rx.try_recv().unwrap();
    assert_matches!(event, GatewayEvent::DeviceModified(..));

    // Make sure the device is only in the selected network
    let device_networks = device
        .find_network_device_networks(&client_state.pool)
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
    let device_network = device
        .find_network_device_networks(&client_state.pool)
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
        "allowed_groups": [],
        "keepalive_interval": 25,
        "peer_disconnect_threshold": 300,
        "acl_enabled": false,
        "acl_default_allow": false,
        "location_mfa_mode": "disabled",
        "service_location_mode": "disabled"
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
        "ips": ["10.1.1.2".to_string(), "10.2.2.2".to_string(), "10.3.3.2".to_string()],
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
        "ips": ["10.11.1.2".to_string(), "10.2.2.2".to_string(), "10.3.3.1".to_string()],
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
