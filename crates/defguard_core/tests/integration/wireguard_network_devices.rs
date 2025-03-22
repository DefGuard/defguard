use std::{net::IpAddr, str::FromStr};

use defguard_core::{
    db::{Device, GatewayEvent, Id, WireguardNetwork},
    handlers::{network_devices::AddNetworkDevice, Auth},
};
use ipnetwork::IpNetwork;
use matches::assert_matches;
use reqwest::StatusCode;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::common::make_test_client;

fn make_network() -> Value {
    json!({
        "name": "network",
        "address": "10.1.1.1/24",
        "port": 55555,
        "endpoint": "192.168.4.14",
        "allowed_ips": "10.1.1.0/24",
        "dns": "1.1.1.1",
        "allowed_groups": [],
        "mfa_enabled": false,
        "keepalive_interval": 25,
        "peer_disconnect_threshold": 180,
        "acl_enabled": false,
        "acl_default_allow": false
    })
}

fn make_second_network() -> Value {
    json!({
        "name": "network-2",
        "address": "10.6.1.1/24",
        "port": 55555,
        "endpoint": "192.168.4.14",
        "allowed_ips": "10.6.1.0/24",
        "dns": "1.1.1.1",
        "allowed_groups": [],
        "mfa_enabled": false,
        "keepalive_interval": 25,
        "peer_disconnect_threshold": 180,
        "acl_enabled": false,
        "acl_default_allow": false
    })
}

#[derive(Deserialize)]
struct IpCheckRes {
    available: bool,
    valid: bool,
}

#[tokio::test]
async fn test_network_devices() {
    let (client, client_state) = make_test_client().await;

    let mut wg_rx = client_state.wireguard_rx;

    let auth = Auth::new("admin", "pass123");
    let response = &client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // create networks
    let response = client
        .post("/api/v1/network")
        .json(&make_network())
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let network_1: WireguardNetwork<Id> = response.json().await;
    assert_eq!(network_1.name, "network");
    let event = wg_rx.try_recv().unwrap();
    assert_matches!(event, GatewayEvent::NetworkCreated(..));
    let response = client
        .post("/api/v1/network")
        .json(&make_second_network())
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let network_2: WireguardNetwork<Id> = response.json().await;
    assert_eq!(network_2.name, "network-2");
    let event = wg_rx.try_recv().unwrap();
    assert_matches!(event, GatewayEvent::NetworkCreated(..));

    // ip suggestions
    let response = client.get("/api/v1/device/network/ip/1").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let res = response.json::<Value>().await;
    let ip = res["ip"].as_str().unwrap();
    let ip = ip.parse::<IpAddr>().unwrap();
    let net_ip = IpAddr::from_str("10.1.1.1").unwrap();
    let network_range = IpNetwork::new(net_ip, 24).unwrap();
    assert!(network_range.contains(ip));

    // checking whether ip is valid/available
    let ip_check = json!(
        {
            "ip": "10.1.1.2".to_string(),
        }
    );
    let response = client
        .post("/api/v1/device/network/ip/1")
        .json(&ip_check)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let res = response.json::<IpCheckRes>().await;
    assert!(res.available);
    assert!(res.valid);

    let ip_check = json!(
        {
            "ip": "10.1.1.0".to_string(),
        }
    );
    let response = client
        .post("/api/v1/device/network/ip/1")
        .json(&ip_check)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let res = response.json::<IpCheckRes>().await;
    assert!(!res.available);
    assert!(res.valid);

    let ip_check = json!(
        {
            "ip": "10.1.1.1".to_string(),
        }
    );
    let response = client
        .post("/api/v1/device/network/ip/1")
        .json(&ip_check)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let res = response.json::<IpCheckRes>().await;
    assert!(!res.available);
    assert!(res.valid);

    let ip_check = json!(
        {
            "ip": "10.1.1.abc".to_string(),
        }
    );
    let response = client
        .post("/api/v1/device/network/ip/1")
        .json(&ip_check)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let res = response.json::<IpCheckRes>().await;
    assert!(!res.available);
    assert!(!res.valid);

    // make network device (manual, WireGuard flow)
    let network_device = AddNetworkDevice {
        name: "device-1".into(),
        wireguard_pubkey: "LQKsT6/3HWKuJmMulH63R8iK+5sI8FyYEL6WDIi6lQU=".into(),
        assigned_ip: ip.to_string(),
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
        "assigned_ip": "10.1.1.3"
    });
    let response = client
        .put(format!("/api/v1/device/network/{device_id}"))
        .json(&modify_device)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let json = response.json::<Value>().await;
    let description = json["description"].as_str().unwrap();
    let assigned_ip = json["assigned_ip"].as_str().unwrap();
    assert_eq!(description, "new description");
    assert_eq!(
        assigned_ip,
        IpAddr::from_str("10.1.1.3").unwrap().to_string()
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
            "assigned_ip": "10.1.1.10",
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
