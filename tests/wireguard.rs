use chrono::{Datelike, Duration, NaiveDate, SubsecRound, Timelike, Utc};
use defguard::{
    build_webapp,
    config::DefGuardConfig,
    db::{
        models::wireguard::{
            WireguardDeviceTransferRow, WireguardNetworkStats, WireguardUserStatsRow,
        },
        AppEvent, DbPool, Device, GatewayEvent, User, WireguardNetwork, WireguardPeerStats,
    },
    grpc::{GatewayState, WorkerState},
    handlers::{
        wireguard::{ImportedNetworkData, UserDevices, WireguardNetworkData},
        Auth,
    },
};
use matches::assert_matches;
use rocket::{
    http::Status,
    local::asynchronous::Client,
    serde::json::{serde_json::json, Value},
};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::{error::TryRecvError, unbounded_channel};

mod common;
use common::init_test_db;

async fn make_client(pool: DbPool, config: DefGuardConfig) -> (Client, Arc<Mutex<GatewayState>>) {
    let (tx, rx) = unbounded_channel::<AppEvent>();
    let worker_state = Arc::new(Mutex::new(WorkerState::new(tx.clone())));
    let (wg_tx, wg_rx) = unbounded_channel::<GatewayEvent>();
    let gateway_state = Arc::new(Mutex::new(GatewayState::new(wg_rx)));

    User::init_admin_user(&pool, &config.default_admin_password)
        .await
        .unwrap();

    let mut user = User::new(
        "hpotter".into(),
        "pass123",
        "Potter".into(),
        "Harry".into(),
        "h.potter@hogwart.edu.uk".into(),
        None,
    );
    user.save(&pool).await.unwrap();

    let webapp = build_webapp(
        config,
        tx,
        rx,
        wg_tx,
        worker_state,
        Arc::clone(&gateway_state),
        pool,
    )
    .await;
    (Client::tracked(webapp).await.unwrap(), gateway_state)
}

fn make_network() -> Value {
    json!({
        "name": "network",
        "address": "10.1.1.1/24",
        "port": 55555,
        "endpoint": "192.168.4.14",
        "allowed_ips": "10.1.1.0/24",
        "dns": "1.1.1.1",
    })
}

#[rocket::async_test]
async fn test_network() {
    let (pool, config) = init_test_db().await;
    let (client, gateway_state) = make_client(pool, config).await;
    let wg_rx = Arc::clone(&gateway_state.lock().unwrap().wireguard_rx);

    let auth = Auth::new("admin".into(), "pass123".into());
    let response = &client.post("/api/v1/auth").json(&auth).dispatch().await;
    assert_eq!(response.status(), Status::Ok);

    // create network
    let response = client
        .post("/api/v1/network")
        .json(&make_network())
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Created);
    let network: WireguardNetwork = response.into_json().await.unwrap();
    assert_eq!(network.name, "network");
    let event = wg_rx.lock().await.try_recv().unwrap();
    assert_matches!(event, GatewayEvent::NetworkCreated(_));

    // modify network
    let network_data = WireguardNetworkData {
        name: "my network".into(),
        address: "10.1.1.0/24".parse().unwrap(),
        endpoint: "10.1.1.1".parse().unwrap(),
        port: 55555,
        allowed_ips: Some("10.1.1.0/24".into()),
        dns: None,
    };
    let response = client
        .put(format!("/api/v1/network/{}", network.id.unwrap()))
        .json(&network_data)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);
    let event = wg_rx.lock().await.try_recv().unwrap();
    assert_matches!(event, GatewayEvent::NetworkModified(_));

    // list networks
    let response = client.get("/api/v1/network").dispatch().await;
    assert_eq!(response.status(), Status::Ok);
    let networks: Vec<WireguardNetwork> = response.into_json().await.unwrap();
    assert_eq!(networks.len(), 1);

    // network details
    let network_from_list = networks[0].clone();
    assert_eq!(network_from_list.name, "my network");
    let response = client
        .get(format!("/api/v1/network/{}", network_from_list.id.unwrap()))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);
    let network_from_details: WireguardNetwork = response.into_json().await.unwrap();
    assert_eq!(network_from_details, network_from_list);

    // delete network
    let response = client
        .delete(format!("/api/v1/network/{}", network.id.unwrap()))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);
    let event = wg_rx.lock().await.try_recv().unwrap();
    assert_matches!(event, GatewayEvent::NetworkDeleted(_));
}

#[rocket::async_test]
async fn test_device() {
    let (pool, config) = init_test_db().await;
    let (client, gateway_state) = make_client(pool, config).await;
    let wg_rx = Arc::clone(&gateway_state.lock().unwrap().wireguard_rx);

    let auth = Auth::new("admin".into(), "pass123".into());
    let response = &client.post("/api/v1/auth").json(&auth).dispatch().await;
    assert_eq!(response.status(), Status::Ok);

    // create network
    let response = client
        .post("/api/v1/network")
        .json(&make_network())
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Created);
    let event = wg_rx.lock().await.try_recv().unwrap();
    assert_matches!(event, GatewayEvent::NetworkCreated(_));

    // network details
    let response = client.get("/api/v1/network/1").dispatch().await;
    assert_eq!(response.status(), Status::Ok);
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
    assert_eq!(response.status(), Status::Created);
    let event = wg_rx.lock().await.try_recv().unwrap();
    assert_matches!(event, GatewayEvent::DeviceCreated(_));

    // list devices
    let response = client.get("/api/v1/device").json(&device).dispatch().await;
    assert_eq!(response.status(), Status::Ok);
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
    assert_eq!(response.status(), Status::Ok);
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
    assert_eq!(response.status(), Status::Ok);
    let event = wg_rx.lock().await.try_recv().unwrap();
    assert_matches!(event, GatewayEvent::DeviceModified(_));

    // device details
    let response = client
        .get(format!("/api/v1/device/{}", device.id.unwrap()))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);
    let device_from_details: Device = response.into_json().await.unwrap();
    assert_eq!(device_from_details.name, modified_name);
    assert_eq!(device_from_details.wireguard_pubkey, modified_key);

    // device config
    let response = client
        .get(format!("/api/v1/device/{}/config", device.id.unwrap()))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);
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

    // FIXME: try to delete network, which should fail because there is a device
    let response = client
        .delete(format!(
            "/api/v1/network/{}",
            network_from_details.id.unwrap()
        ))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);
    let event = wg_rx.lock().await.try_recv().unwrap();
    assert_matches!(event, GatewayEvent::NetworkDeleted(_));

    // delete device
    let response = client
        .delete(format!("/api/v1/device/{}", device.id.unwrap()))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);
    let event = wg_rx.lock().await.try_recv().unwrap();
    assert_matches!(event, GatewayEvent::DeviceDeleted(_));

    let response = client.get("/api/v1/device").json(&device).dispatch().await;
    assert_eq!(response.status(), Status::Ok);
    let devices: Vec<Device> = response.into_json().await.unwrap();
    assert!(devices.is_empty());
}

#[rocket::async_test]
async fn test_device_permissions() {
    let (pool, config) = init_test_db().await;
    let (client, _gateway_state) = make_client(pool, config).await;

    let auth = Auth::new("admin".into(), "pass123".into());
    let response = &client.post("/api/v1/auth").json(&auth).dispatch().await;
    assert_eq!(response.status(), Status::Ok);

    // create network
    let response = client
        .post("/api/v1/network")
        .json(&make_network())
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Created);

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
    assert_eq!(response.status(), Status::Created);
    let device = json!({"devices": [{
        "name": "device_2",
        "wireguard_ip": "10.0.0.3",
        "wireguard_pubkey": "LQKsT6/3HWKuJmMulH63R8iK+5sI8FyYEL6WDIi6lQU=",
        "user_id": 1,
        "created": "2023-05-05T23:56:04"
    }]});
    let response = client
        .post("/api/v1/network/devices")
        .json(&device)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Created);

    let device = json!({
        "name": "device_3",
        "wireguard_pubkey": "LQKsT6/3HWKuJmMulH63R8iK+5sI8FyYEL6WDIi6lQU=",
    });
    let response = client
        .post("/api/v1/device/hpotter")
        .json(&device)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Created);
    let device = json!({"devices": [{
        "name": "device_4",
        "wireguard_ip": "10.0.0.5",
        "wireguard_pubkey": "LQKsT6/3HWKuJmMulH63R8iK+5sI8FyYEL6WDIi6lQU=",
        "user_id": 2,
        "created": "2023-05-05T23:56:04"
    }]});
    let response = client
        .post("/api/v1/network/devices")
        .json(&device)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Created);

    // normal user cannot add devices for other users or import multiple devices
    let auth = Auth::new("hpotter".into(), "pass123".into());
    let response = &client.post("/api/v1/auth").json(&auth).dispatch().await;
    assert_eq!(response.status(), Status::Ok);

    let device = json!({
        "name": "device_5",
        "wireguard_pubkey": "LQKsT6/3HWKuJmMulH63R8iK+5sI8FyYEL6WDIi6lQU=",
    });
    let response = client
        .post("/api/v1/device/hpotter")
        .json(&device)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Created);
    let device = json!({"devices": [{
        "name": "device_6",
        "wireguard_ip": "10.0.0.7",
        "wireguard_pubkey": "LQKsT6/3HWKuJmMulH63R8iK+5sI8FyYEL6WDIi6lQU=",
        "user_id": 2,
        "created": "2023-05-05T23:56:04"
    }]});
    let response = client
        .post("/api/v1/network/devices")
        .json(&device)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Forbidden);

    let device = json!({
        "name": "device_7",
        "wireguard_pubkey": "LQKsT6/3HWKuJmMulH63R8iK+5sI8FyYEL6WDIi6lQU=",
    });
    let response = client
        .post("/api/v1/device/admin")
        .json(&device)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Forbidden);
    let device = json!({"devices": [{
        "name": "device_8",
        "wireguard_ip": "10.0.0.9",
        "wireguard_pubkey": "LQKsT6/3HWKuJmMulH63R8iK+5sI8FyYEL6WDIi6lQU=",
        "user_id": 1,
        "created": "2023-05-05T23:56:04"
    }]});
    let response = client
        .post("/api/v1/network/devices")
        .json(&device)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Forbidden);

    // normal user cannot list devices of other users
    let response = client.get("/api/v1/device/user/admin").dispatch().await;
    assert_eq!(response.status(), Status::Forbidden);

    let response = client.get("/api/v1/device/user/hpotter").dispatch().await;
    assert_eq!(response.status(), Status::Ok);
    let user_devices: Vec<Device> = response.into_json().await.unwrap();
    assert_eq!(user_devices.len(), 3);

    // admin can list devices of other users
    let auth = Auth::new("admin".into(), "pass123".into());
    let response = &client.post("/api/v1/auth").json(&auth).dispatch().await;
    assert_eq!(response.status(), Status::Ok);

    let response = client.get("/api/v1/device/user/admin").dispatch().await;
    assert_eq!(response.status(), Status::Ok);
    let user_devices: Vec<Device> = response.into_json().await.unwrap();
    assert_eq!(user_devices.len(), 2);

    let response = client.get("/api/v1/device/user/hpotter").dispatch().await;
    assert_eq!(response.status(), Status::Ok);
    let user_devices: Vec<Device> = response.into_json().await.unwrap();
    assert_eq!(user_devices.len(), 3);
}

#[rocket::async_test]
async fn test_device_pubkey() {
    let (pool, config) = init_test_db().await;
    let (client, gateway_state) = make_client(pool, config).await;
    let wg_rx = Arc::clone(&gateway_state.lock().unwrap().wireguard_rx);

    let auth = Auth::new("admin".into(), "pass123".into());
    let response = &client.post("/api/v1/auth").json(&auth).dispatch().await;
    assert_eq!(response.status(), Status::Ok);

    // create network
    let response = client
        .post("/api/v1/network")
        .json(&make_network())
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Created);
    let event = wg_rx.lock().await.try_recv().unwrap();
    assert_matches!(event, GatewayEvent::NetworkCreated(_));

    // network details
    let response = client.get("/api/v1/network/1").dispatch().await;
    assert_eq!(response.status(), Status::Ok);
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
    assert_eq!(response.status(), Status::BadRequest);

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
    assert_eq!(response.status(), Status::BadRequest);

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
    assert_eq!(response.status(), Status::Created);

    // list devices
    let response = client.get("/api/v1/device").json(&device).dispatch().await;
    assert_eq!(response.status(), Status::Ok);
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
    assert_eq!(response.status(), Status::BadRequest);
}

#[rocket::async_test]
async fn test_stats() {
    let (pool, config) = init_test_db().await;
    let (client, _) = make_client(pool.clone(), config).await;

    let auth = Auth::new("admin".into(), "pass123".into());
    let response = &client.post("/api/v1/auth").json(&auth).dispatch().await;
    assert_eq!(response.status(), Status::Ok);

    // create network
    let response = client
        .post("/api/v1/network")
        .json(&make_network())
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Created);

    // create devices
    let device = json!({
        "name": "device-1",
        "wireguard_pubkey": "LQKsT6/3HWKuJmMulH63R8iK+5sI8FyYEL6WDIi6lQU=",
    });
    let response = client
        .post("/api/v1/device/admin")
        .json(&device)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Created);

    let device = json!({
        "name": "device-2",
        "wireguard_pubkey": "sIhx53MsX+iLk83sssybHrD7M+5m+CmpLzWL/zo8C38=",
    });
    let response = client
        .post("/api/v1/device/admin")
        .json(&device)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Created);

    // get devices
    let mut devices = Vec::<Device>::new();
    let response = client.get("/api/v1/device/1").dispatch().await;
    assert_eq!(response.status(), Status::Ok);
    devices.push(response.into_json().await.unwrap());

    let response = client.get("/api/v1/device/2").dispatch().await;
    assert_eq!(response.status(), Status::Ok);
    devices.push(response.into_json().await.unwrap());

    // empty stats
    let now = Utc::now().naive_utc();
    let hour_ago = now - Duration::hours(1);
    let response = client
        .get(format!(
            "/api/v1/network/stats/users?from={}",
            hour_ago.format("%Y-%m-%dT%H:%M:00Z"),
        ))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);
    let stats: Vec<WireguardUserStatsRow> = response.into_json().await.unwrap();
    assert!(stats.is_empty());

    // insert stats
    let samples = 60 * 11; // 11 hours of samples
    for i in 0..samples {
        for (d, device) in devices.iter().enumerate().take(2) {
            let mut wps = WireguardPeerStats {
                id: None,
                device_id: device.id.unwrap(),
                collected_at: now - Duration::minutes(i),
                network: 1,
                endpoint: Some("11.22.33.44".into()),
                upload: (samples - i) * 10 * (d as i64 + 1),
                download: (samples - i) * 20 * (d as i64 + 1),
                latest_handshake: now - Duration::minutes(i * 10),
                allowed_ips: Some("10.1.1.0/24".into()),
            };
            wps.save(&pool).await.unwrap();
        }
    }

    // minute aggregation
    let response = client
        .get(format!(
            "/api/v1/network/stats/users?from={}",
            hour_ago.format("%Y-%m-%dT%H:%M:00Z"),
        ))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);
    let stats: Vec<WireguardUserStatsRow> = response.into_json().await.unwrap();
    assert_eq!(stats.len(), 1);
    assert_eq!(stats[0].devices.len(), 2);
    assert_eq!(
        stats[0].devices[0].connected_at.unwrap(),
        now.trunc_subsecs(6)
    );
    assert_eq!(
        stats[0].devices[1].connected_at.unwrap(),
        now.trunc_subsecs(6)
    );
    assert_eq!(stats[0].devices[0].stats.len(), 61);
    assert_eq!(stats[0].devices[1].stats.len(), 61);
    let now_trunc = NaiveDate::from_ymd_opt(now.year(), now.month(), now.day())
        .unwrap_or_default()
        .and_hms_opt(now.hour(), now.minute(), 0)
        .unwrap_or_default();
    assert_eq!(
        stats[0].devices[0].stats.last().unwrap().clone(),
        WireguardDeviceTransferRow {
            device_id: 1,
            collected_at: Some(now_trunc),
            upload: 10,
            download: 20,
        }
    );
    assert_eq!(
        stats[0].devices[1].stats.last().unwrap().clone(),
        WireguardDeviceTransferRow {
            device_id: 2,
            collected_at: Some(now_trunc),
            upload: 10 * 2,
            download: 20 * 2,
        }
    );
    assert_eq!(
        stats[0].devices[0]
            .stats
            .iter()
            .map(|s| s.upload)
            .sum::<i64>(),
        10 * 61
    );
    assert_eq!(
        stats[0].devices[0]
            .stats
            .iter()
            .map(|s| s.download)
            .sum::<i64>(),
        20 * 61
    );
    assert_eq!(
        stats[0].devices[1]
            .stats
            .iter()
            .map(|s| s.upload)
            .sum::<i64>(),
        10 * 2 * 61
    );
    assert_eq!(
        stats[0].devices[1]
            .stats
            .iter()
            .map(|s| s.download)
            .sum::<i64>(),
        20 * 2 * 61
    );

    assert!(stats[0].devices[0].stats[0].upload > 0);
    assert!(stats[0].devices[1].stats[0].upload > 0);
    assert!(stats[0].devices[0].stats[0].download > 0);
    assert!(stats[0].devices[1].stats[0].download > 0);
    assert_eq!(stats[0].devices[0].stats.last().unwrap().upload, 10);
    assert_eq!(stats[0].devices[1].stats.last().unwrap().upload, 20);
    assert_eq!(stats[0].devices[0].stats.last().unwrap().download, 20);
    assert_eq!(stats[0].devices[1].stats.last().unwrap().download, 40);
    assert_eq!(
        stats[0].devices[0]
            .stats
            .iter()
            .filter(|s| s.upload != 10 || s.download != 20)
            .count(),
        0
    );
    assert_eq!(
        stats[0].devices[1]
            .stats
            .iter()
            .filter(|s| s.upload != 20 || s.download != 40)
            .count(),
        0
    );

    // hourly aggregation
    let ten_hours_ago = now - Duration::hours(10);
    let ten_hours_samples = 10 * 60 + 1;
    let response = client
        .get(format!(
            "/api/v1/network/stats/users?from={}",
            ten_hours_ago.format("%Y-%m-%dT%H:%M:00Z"),
        ))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);
    let stats: Vec<WireguardUserStatsRow> = response.into_json().await.unwrap();
    assert_eq!(stats.len(), 1);
    assert_eq!(stats[0].devices.len(), 2);
    assert_eq!(
        stats[0].devices[0].connected_at.unwrap(),
        now.trunc_subsecs(6)
    );
    assert_eq!(
        stats[0].devices[1].connected_at.unwrap(),
        now.trunc_subsecs(6)
    );
    assert_eq!(stats[0].devices[0].stats.len(), 11);
    assert_eq!(stats[0].devices[1].stats.len(), 11);
    assert!(stats[0].devices[0].stats[0].upload > 0);
    assert!(stats[0].devices[1].stats[0].upload > 0);
    assert!(stats[0].devices[0].stats[0].download > 0);
    assert!(stats[0].devices[1].stats[0].download > 0);
    assert_eq!(stats[0].devices[0].stats[5].upload, 10 * 60);
    assert_eq!(stats[0].devices[1].stats[5].upload, 20 * 60);
    assert_eq!(stats[0].devices[0].stats[5].download, 20 * 60);
    assert_eq!(stats[0].devices[1].stats[5].download, 40 * 60);

    // network stats
    let response = client
        .get(format!(
            "/api/v1/network/stats?from={}",
            ten_hours_ago.format("%Y-%m-%dT%H:%M:00Z"),
        ))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);
    let stats: WireguardNetworkStats = response.into_json().await.unwrap();
    assert_eq!(stats.active_users, 1);
    assert_eq!(stats.active_devices, 2);
    assert_eq!(stats.upload, ten_hours_samples * (10 + 20));
    assert_eq!(stats.download, ten_hours_samples * (20 + 40));
    assert_eq!(stats.transfer_series.len(), 11);
    assert!(stats.transfer_series[0].download.is_some());
    assert!(stats.transfer_series[0].upload.is_some());
    assert_eq!(stats.transfer_series[5].upload, Some((10 + 20) * 60));

    assert_eq!(stats.transfer_series[5].download, Some((20 + 40) * 60));
    assert_eq!(
        stats.upload,
        stats
            .transfer_series
            .iter()
            .map(|v| v.upload.unwrap())
            .sum::<i64>()
    );
    assert_eq!(
        stats.download,
        stats
            .transfer_series
            .iter()
            .map(|v| v.download.unwrap())
            .sum::<i64>()
    );
}

#[rocket::async_test]
async fn test_config_import() {
    let wg_config = "
        [Interface]
        PrivateKey = GAA2X3DW0WakGVx+DsGjhDpTgg50s1MlmrLf24Psrlg=
        Address = 10.0.0.1/24
        ListenPort = 55055
        DNS = 10.0.0.2

        [Peer]
        PublicKey = 2LYRr2HgSSpGCdXKDDAlcFe0Uuc6RR8TFgSquNc9VAE=
        AllowedIPs = 10.0.0.10/24
        PersistentKeepalive = 300

        [Peer]
        PublicKey = OLQNaEH3FxW0hiodaChEHoETzd+7UzcqIbsLs+X8rD0=
        AllowedIPs = 10.0.0.11/24
        PersistentKeepalive = 300
    ";
    let (pool, config) = init_test_db().await;
    let (client, gateway_state) = make_client(pool, config).await;
    let wg_rx = Arc::clone(&gateway_state.lock().unwrap().wireguard_rx);

    let auth = Auth::new("admin".into(), "pass123".into());
    let response = &client.post("/api/v1/auth").json(&auth).dispatch().await;
    assert_eq!(response.status(), Status::Ok);

    // import network
    let response = client
        .post("/api/v1/network/import")
        .json(&json!({"name": "network", "endpoint": "192.168.1.1", "config": wg_config}))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Created);
    let response: ImportedNetworkData = response.into_json().await.unwrap();

    // network assertions
    let network = response.network;
    assert_eq!(network.id, Some(1));
    assert_eq!(network.name, "network");
    assert_eq!(network.address, "10.0.0.1/24".parse().unwrap());
    assert_eq!(network.port, 55055);
    assert_eq!(
        network.pubkey,
        "Y5ewP5RXstQd71gkmS/M0xL8wi0yVbbVY/ocLM4cQ1Y="
    );
    assert_eq!(network.prvkey, "");
    assert_eq!(network.endpoint, "192.168.1.1");
    assert_eq!(network.dns, Some("10.0.0.2".to_string()));
    assert_eq!(network.allowed_ips, vec!["10.0.0.0/24".parse().unwrap()]);
    assert_eq!(network.connected_at, None);
    let event = wg_rx.lock().await.try_recv().unwrap();
    assert_matches!(event, GatewayEvent::NetworkCreated(_));

    // device assertions
    let devices = response.devices;
    assert_eq!(devices.len(), 2);

    let mut device1 = devices[0].clone();
    assert_eq!(device1.id, None);
    assert_eq!(device1.name, "2LYRr2HgSSpGCdXKDDAlcFe0Uuc6RR8TFgSquNc9VAE=");
    assert_eq!(device1.wireguard_ip, "10.0.0.10");
    assert_eq!(
        device1.wireguard_pubkey,
        "2LYRr2HgSSpGCdXKDDAlcFe0Uuc6RR8TFgSquNc9VAE="
    );
    // TODO: do something about user_id
    assert_eq!(device1.user_id, -1);

    let mut device2 = devices[1].clone();
    assert_eq!(device2.id, None);
    assert_eq!(device2.name, "OLQNaEH3FxW0hiodaChEHoETzd+7UzcqIbsLs+X8rD0=");
    assert_eq!(device2.wireguard_ip, "10.0.0.11");
    assert_eq!(
        device2.wireguard_pubkey,
        "OLQNaEH3FxW0hiodaChEHoETzd+7UzcqIbsLs+X8rD0="
    );
    // TODO: do something about user_id
    assert_eq!(device2.user_id, -1);

    // modify devices
    device1.name = "device1".to_string();
    device1.user_id = 1;
    device2.name = "device2".to_string();
    device2.user_id = 1;

    // post modified devices
    let response = client
        .post("/api/v1/network/devices")
        .json(&json!({"devices": [device1, device2]}))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Created);

    // assert modified devices
    let response: UserDevices = response.into_json().await.unwrap();
    let device1 = response.devices[0].clone();
    assert_eq!(device1.name, "device1");
    assert_eq!(device1.user_id, 1);
    let device2 = response.devices[1].clone();
    assert_eq!(device2.name, "device2");
    assert_eq!(device2.user_id, 1);

    // assert events
    let event = wg_rx.lock().await.try_recv().unwrap();
    assert_matches!(event, GatewayEvent::DeviceCreated(_));

    let event = wg_rx.lock().await.try_recv().unwrap();
    assert_matches!(event, GatewayEvent::DeviceCreated(_));

    let event = wg_rx.lock().await.try_recv();
    assert_matches!(event, Err(TryRecvError::Empty));
}

#[rocket::async_test]
async fn test_config_import_missing_interface() {
    let wg_config = "
        PrivateKey = GAA2X3DW0WakGVx+DsGjhDpTgg50s1MlmrLf24Psrlg=
        Address = 10.0.0.1/24
        ListenPort = 55055
        DNS = 10.0.0.2

        [Peer]
        PublicKey = 2LYRr2HgSSpGCdXKDDAlcFe0Uuc6RR8TFgSquNc9VAE=
        AllowedIPs = 10.0.0.10/24
        PersistentKeepalive = 300

        [Peer]
        PublicKey = OLQNaEH3FxW0hiodaChEHoETzd+7UzcqIbsLs+X8rD0=
        AllowedIPs = 10.0.0.11/24
        PersistentKeepalive = 300
    ";
    let (pool, config) = init_test_db().await;
    let (client, _) = make_client(pool, config).await;

    let auth = Auth::new("admin".into(), "pass123".into());
    let response = &client.post("/api/v1/auth").json(&auth).dispatch().await;
    assert_eq!(response.status(), Status::Ok);

    // import network
    let response = client
        .post("/api/v1/network/import")
        .json(&json!({"name": "network", "endpoint": "192.168.1.1", "config": wg_config}))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::UnprocessableEntity);
}

#[rocket::async_test]
async fn test_config_import_invalid_key() {
    let wg_config = "
        [Interface]
        PrivateKey = DEFINITELY_NOT_A_VALID_WG_KEY
        Address = 10.0.0.1/24
        ListenPort = 55055
        DNS = 10.0.0.2

        [Peer]
        PublicKey = 2LYRr2HgSSpGCdXKDDAlcFe0Uuc6RR8TFgSquNc9VAE=
        AllowedIPs = 10.0.0.10/24
        PersistentKeepalive = 300

        [Peer]
        PublicKey = OLQNaEH3FxW0hiodaChEHoETzd+7UzcqIbsLs+X8rD0=
        AllowedIPs = 10.0.0.11/24
        PersistentKeepalive = 300
    ";
    let (pool, config) = init_test_db().await;
    let (client, _) = make_client(pool, config).await;

    let auth = Auth::new("admin".into(), "pass123".into());
    let response = &client.post("/api/v1/auth").json(&auth).dispatch().await;
    assert_eq!(response.status(), Status::Ok);

    // import network
    let response = client
        .post("/api/v1/network/import")
        .json(&json!({"name": "network", "endpoint": "192.168.1.1", "config": wg_config}))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::UnprocessableEntity);
}

#[rocket::async_test]
async fn test_config_import_invalid_ip() {
    let wg_config = "
        [Interface]
        PrivateKey = 2LYRr2HgSSpGCdXKDDAlcFe0Uuc6RR8TFgSquNc9VAE=
        Address = 10.0.0.256/24
        ListenPort = 55055
        DNS = 10.0.0.2

        [Peer]
        PublicKey = 2LYRr2HgSSpGCdXKDDAlcFe0Uuc6RR8TFgSquNc9VAE=
        AllowedIPs = 10.0.0.10/24
        PersistentKeepalive = 300

        [Peer]
        PublicKey = OLQNaEH3FxW0hiodaChEHoETzd+7UzcqIbsLs+X8rD0=
        AllowedIPs = 10.0.0.11/24
        PersistentKeepalive = 300
    ";
    let (pool, config) = init_test_db().await;
    let (client, _) = make_client(pool, config).await;

    let auth = Auth::new("admin".into(), "pass123".into());
    let response = &client.post("/api/v1/auth").json(&auth).dispatch().await;
    assert_eq!(response.status(), Status::Ok);

    // import network
    let response = client
        .post("/api/v1/network/import")
        .json(&json!({"name": "network", "endpoint": "192.168.1.1", "config": wg_config}))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::UnprocessableEntity);
}

#[rocket::async_test]
async fn test_config_import_nonadmin() {
    let wg_config = "
        [Interface]
        PrivateKey = GAA2X3DW0WakGVx+DsGjhDpTgg50s1MlmrLf24Psrlg=
        Address = 10.0.0.1/24
        ListenPort = 55055
        DNS = 10.0.0.2

        [Peer]
        PublicKey = 2LYRr2HgSSpGCdXKDDAlcFe0Uuc6RR8TFgSquNc9VAE=
        AllowedIPs = 10.0.0.10/24
        PersistentKeepalive = 300

        [Peer]
        PublicKey = OLQNaEH3FxW0hiodaChEHoETzd+7UzcqIbsLs+X8rD0=
        AllowedIPs = 10.0.0.11/24
        PersistentKeepalive = 300
    ";
    let (pool, config) = init_test_db().await;
    let (client, _) = make_client(pool, config).await;
    let auth = Auth::new("hpotter".into(), "pass123".into());
    let response = &client.post("/api/v1/auth").json(&auth).dispatch().await;
    assert_eq!(response.status(), Status::Ok);

    // import network
    let response = client
        .post("/api/v1/network/import")
        .json(&json!({"name": "network", "endpoint": "192.168.1.1", "config": wg_config}))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Forbidden);
}
