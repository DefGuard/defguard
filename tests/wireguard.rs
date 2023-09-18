mod common;

use defguard::{
    db::{models::device::WireguardNetworkDevice, Device, GatewayEvent, WireguardNetwork},
    handlers::{wireguard::WireguardNetworkData, Auth},
};
use matches::assert_matches;

use self::common::make_test_client;

fn make_network() -> Value {
    json!({
        "name": "network",
        "address": "10.1.1.1/24",
        "port": 55555,
        "endpoint": "192.168.4.14",
        "allowed_ips": "10.1.1.0/24",
        "dns": "1.1.1.1",
        "allowed_groups": [],
    })
}

#[tokio::test]
async fn test_network() {
    let (client, client_state) = make_test_client().await;

    let mut wg_rx = client_state.wireguard_rx;

    let auth = Auth::new("admin".into(), "pass123".into());
    let response = &client.post("/api/v1/auth").json(&auth).dispatch().await;
    assert_eq!(response.status(), StatusCode::OK);

    // create network
    let response = client
        .post("/api/v1/network")
        .json(&make_network())
        .dispatch()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let network: WireguardNetwork = response.into_json().await.unwrap();
    assert_eq!(network.name, "network");
    let event = wg_rx.try_recv().unwrap();
    assert_matches!(event, GatewayEvent::NetworkCreated(..));

    // modify network
    let network_data = WireguardNetworkData {
        name: "my network".into(),
        address: "10.1.1.0/24".parse().unwrap(),
        endpoint: "10.1.1.1".parse().unwrap(),
        port: 55555,
        allowed_ips: Some("10.1.1.0/24".into()),
        dns: None,
        allowed_groups: vec![],
    };
    let response = client
        .put(format!("/api/v1/network/{}", network.id.unwrap()))
        .json(&network_data)
        .dispatch()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let event = wg_rx.try_recv().unwrap();
    assert_matches!(event, GatewayEvent::NetworkModified(..));

    // list networks
    let response = client.get("/api/v1/network").dispatch().await;
    assert_eq!(response.status(), StatusCode::OK);
    let networks: Vec<WireguardNetwork> = response.into_json().await.unwrap();
    assert_eq!(networks.len(), 1);

    // network details
    let network_from_list = networks[0].clone();
    assert_eq!(network_from_list.name, "my network");
    let response = client
        .get(format!("/api/v1/network/{}", network_from_list.id.unwrap()))
        .dispatch()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let network_from_details: WireguardNetwork = response.into_json().await.unwrap();
    assert_eq!(network_from_details, network_from_list);

    // delete network
    let response = client
        .delete(format!("/api/v1/network/{}", network.id.unwrap()))
        .dispatch()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let event = wg_rx.try_recv().unwrap();
    assert_matches!(event, GatewayEvent::NetworkDeleted(..));
}

#[tokio::test]
async fn test_device() {
    let (client, client_state) = make_test_client().await;

    let mut wg_rx = client_state.wireguard_rx;

    let auth = Auth::new("admin".into(), "pass123".into());
    let response = &client.post("/api/v1/auth").json(&auth).dispatch().await;
    assert_eq!(response.status(), StatusCode::OK);

    // create network
    let response = client
        .post("/api/v1/network")
        .json(&make_network())
        .dispatch()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let event = wg_rx.try_recv().unwrap();
    assert_matches!(event, GatewayEvent::NetworkCreated(..));

    // network details
    let response = client.get("/api/v1/network/1").dispatch().await;
    assert_eq!(response.status(), StatusCode::OK);
    let network_from_details: WireguardNetwork = response.into_json().await.unwrap();

    // create device
    let device = json!({
        "name": "device",
        "wireguard_pubkey": "LQKsT6/3HWKuJmMulH63R8iK+5sI8FyYEL6WDIi6lQU=",
    });
    let response = client
        .post("/api/v1/device/admin")
        .json(&device)
        .dispatch()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let event = wg_rx.try_recv().unwrap();
    assert_matches!(event, GatewayEvent::DeviceCreated(..));

    // an IP was assigned for new device
    let network_devices = WireguardNetworkDevice::find_by_device(&client_state.pool, 1)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        network_devices[0].wireguard_network_id,
        network_from_details.id.unwrap()
    );

    // add another network
    let response = client
        .post("/api/v1/network")
        .json(&make_network())
        .dispatch()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    assert_matches!(wg_rx.try_recv().unwrap(), GatewayEvent::NetworkCreated(..));

    // an IP was assigned for an existing device
    let network_devices = WireguardNetworkDevice::find_by_device(&client_state.pool, 1)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(network_devices.len(), 2);

    // list devices
    let response = client.get("/api/v1/device").json(&device).dispatch().await;
    assert_eq!(response.status(), StatusCode::OK);
    let devices: Vec<Device> = response.into_json().await.unwrap();
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
        .dispatch()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let user_devices: Vec<Device> = response.into_json().await.unwrap();
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
        .put(format!("/api/v1/device/{}", device.id.unwrap()))
        .json(&modified_device)
        .dispatch()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let event = wg_rx.try_recv().unwrap();
    assert_matches!(event, GatewayEvent::DeviceModified(..));

    // device details
    let response = client
        .get(format!("/api/v1/device/{}", device.id.unwrap()))
        .dispatch()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let device_from_details: Device = response.into_json().await.unwrap();
    assert_eq!(device_from_details.name, modified_name);
    assert_eq!(device_from_details.wireguard_pubkey, modified_key);

    // device config
    let response = client
        .get(format!(
            "/api/v1/network/1/device/{}/config",
            device.id.unwrap()
        ))
        .dispatch()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let config = response.into_string().await.unwrap();
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
            PersistentKeepalive = 300",
            network_from_details.pubkey
        )
    );

    let response = client
        .delete(format!(
            "/api/v1/network/{}",
            network_from_details.id.unwrap()
        ))
        .dispatch()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let event = wg_rx.try_recv().unwrap();
    assert_matches!(event, GatewayEvent::NetworkDeleted(..));

    // delete device
    let response = client
        .delete(format!("/api/v1/device/{}", device.id.unwrap()))
        .dispatch()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let event = wg_rx.try_recv().unwrap();
    assert_matches!(event, GatewayEvent::DeviceDeleted(..));

    let response = client.get("/api/v1/device").json(&device).dispatch().await;
    assert_eq!(response.status(), StatusCode::OK);
    let devices: Vec<Device> = response.into_json().await.unwrap();
    assert!(devices.is_empty());
}

#[tokio::test]
async fn test_device_permissions() {
    let (client, _) = make_test_client().await;

    let auth = Auth::new("admin".into(), "pass123".into());
    let response = &client.post("/api/v1/auth").json(&auth).dispatch().await;
    assert_eq!(response.status(), StatusCode::OK);

    // create network
    let response = client
        .post("/api/v1/network")
        .json(&make_network())
        .dispatch()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // admin can add devices for other users
    let device = json!({
        "name": "device_1",
        "wireguard_pubkey": "LQKsT6/3HWKuJmMulH63R8iK+5sI8FyYEL6WDIi6lQU=",
    });
    let response = client
        .post("/api/v1/device/admin")
        .json(&device)
        .dispatch()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let device = json!({"devices": [{
        "name": "device_2",
        "wireguard_ip": "10.0.0.3",
        "wireguard_pubkey": "LQKsT6/3HWKuJmMulH63R8iK+5sI8FyYEL6WDIi6lQU=",
        "user_id": 1,
        "created": "2023-05-05T23:56:04"
    }]});
    let response = client
        .post("/api/v1/network/1/devices")
        .json(&device)
        .dispatch()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    let device = json!({
        "name": "device_3",
        "wireguard_pubkey": "LQKsT6/3HWKuJmMulH63R8iK+5sI8FyYEL6WDIi6lQU=",
    });
    let response = client
        .post("/api/v1/device/hpotter")
        .json(&device)
        .dispatch()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let device = json!({"devices": [{
        "name": "device_4",
        "wireguard_ip": "10.0.0.5",
        "wireguard_pubkey": "LQKsT6/3HWKuJmMulH63R8iK+5sI8FyYEL6WDIi6lQU=",
        "user_id": 2,
        "created": "2023-05-05T23:56:04"
    }]});
    let response = client
        .post("/api/v1/network/1/devices")
        .json(&device)
        .dispatch()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // normal user cannot add devices for other users or import multiple devices
    let auth = Auth::new("hpotter".into(), "pass123".into());
    let response = &client.post("/api/v1/auth").json(&auth).dispatch().await;
    assert_eq!(response.status(), StatusCode::OK);

    let device = json!({
        "name": "device_5",
        "wireguard_pubkey": "LQKsT6/3HWKuJmMulH63R8iK+5sI8FyYEL6WDIi6lQU=",
    });
    let response = client
        .post("/api/v1/device/hpotter")
        .json(&device)
        .dispatch()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let device = json!({"devices": [{
        "name": "device_6",
        "wireguard_ip": "10.0.0.7",
        "wireguard_pubkey": "LQKsT6/3HWKuJmMulH63R8iK+5sI8FyYEL6WDIi6lQU=",
        "user_id": 2,
        "created": "2023-05-05T23:56:04"
    }]});
    let response = client
        .post("/api/v1/network/1/devices")
        .json(&device)
        .dispatch()
        .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let device = json!({
        "name": "device_7",
        "wireguard_pubkey": "LQKsT6/3HWKuJmMulH63R8iK+5sI8FyYEL6WDIi6lQU=",
    });
    let response = client
        .post("/api/v1/device/admin")
        .json(&device)
        .dispatch()
        .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let device = json!({"devices": [{
        "name": "device_8",
        "wireguard_ip": "10.0.0.9",
        "wireguard_pubkey": "LQKsT6/3HWKuJmMulH63R8iK+5sI8FyYEL6WDIi6lQU=",
        "user_id": 1,
        "created": "2023-05-05T23:56:04"
    }]});
    let response = client
        .post("/api/v1/network/1/devices")
        .json(&device)
        .dispatch()
        .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    // normal user cannot list devices of other users
    let response = client.get("/api/v1/device/user/admin").dispatch().await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let response = client.get("/api/v1/device/user/hpotter").dispatch().await;
    assert_eq!(response.status(), StatusCode::OK);
    let user_devices: Vec<Device> = response.into_json().await.unwrap();
    assert_eq!(user_devices.len(), 3);

    // admin can list devices of other users
    let auth = Auth::new("admin".into(), "pass123".into());
    let response = &client.post("/api/v1/auth").json(&auth).dispatch().await;
    assert_eq!(response.status(), StatusCode::OK);

    let response = client.get("/api/v1/device/user/admin").dispatch().await;
    assert_eq!(response.status(), StatusCode::OK);
    let user_devices: Vec<Device> = response.into_json().await.unwrap();
    assert_eq!(user_devices.len(), 2);

    let response = client.get("/api/v1/device/user/hpotter").dispatch().await;
    assert_eq!(response.status(), StatusCode::OK);
    let user_devices: Vec<Device> = response.into_json().await.unwrap();
    assert_eq!(user_devices.len(), 3);
}

#[tokio::test]
async fn test_device_pubkey() {
    let (client, client_state) = make_test_client().await;

    let mut wg_rx = client_state.wireguard_rx;

    let auth = Auth::new("admin".into(), "pass123".into());
    let response = &client.post("/api/v1/auth").json(&auth).dispatch().await;
    assert_eq!(response.status(), StatusCode::OK);

    // create network
    let response = client
        .post("/api/v1/network")
        .json(&make_network())
        .dispatch()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let event = wg_rx.try_recv().unwrap();
    assert_matches!(event, GatewayEvent::NetworkCreated(..));

    // network details
    let response = client.get("/api/v1/network/1").dispatch().await;
    assert_eq!(response.status(), StatusCode::OK);
    let network_from_details: WireguardNetwork = response.into_json().await.unwrap();

    // create bad device
    let device = json!({
        "name": "device",
        "wireguard_pubkey": network_from_details.pubkey.clone(),
    });
    let response = client
        .post("/api/v1/device/admin")
        .json(&device)
        .dispatch()
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
        .dispatch()
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
        .dispatch()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // list devices
    let response = client.get("/api/v1/device").json(&device).dispatch().await;
    assert_eq!(response.status(), StatusCode::OK);
    let devices: Vec<Device> = response.into_json().await.unwrap();
    assert_eq!(devices.len(), 1);

    // modify device
    let mut device = devices[0].clone();
    device.wireguard_pubkey = network_from_details.pubkey;
    let response = client
        .put(format!("/api/v1/device/{}", device.id.unwrap()))
        .json(&device)
        .dispatch()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // try to create multiple devices
    let devices = json!({"devices": [{
        "name": "device_2",
        "wireguard_ip": "10.0.0.9",
        "wireguard_pubkey": "LQKsT6/3HWKuJmMulH63R8iK+5sI8FyYEL6WDIi6lQU=",
        "user_id": 1,
        "created": "2023-05-05T23:56:04"
    },
    {
        "name": "device_3",
        "wireguard_ip": "10.0.0.10",
        "wireguard_pubkey": "invalid_key",
        "user_id": 1,
        "created": "2023-05-05T23:56:04"
    }]});
    let response = client
        .post("/api/v1/network/1/devices")
        .json(&devices)
        .dispatch()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // make sure no device was created
    let response = client.get("/api/v1/device").json(&device).dispatch().await;
    assert_eq!(response.status(), StatusCode::OK);
    let devices: Vec<Device> = response.into_json().await.unwrap();
    assert_eq!(devices.len(), 1);
}
