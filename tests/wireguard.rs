use chrono::{Datelike, Duration, NaiveDate, SubsecRound, Timelike, Utc};
use defguard::{
    build_webapp,
    config::DefGuardConfig,
    db::{
        models::wireguard::{
            WireguardDeviceTransferRow, WireguardNetworkStats, WireguardUserStatsRow,
        },
        AppEvent, DbPool, Device, GatewayEvent, WireguardNetwork, WireguardPeerStats,
    },
    grpc::GatewayState,
    handlers::{wireguard::WireguardNetworkData, Auth},
};
use matches::assert_matches;
use rocket::{
    http::Status,
    local::asynchronous::Client,
    serde::json::{serde_json::json, Value},
};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::unbounded_channel;

mod common;
use common::init_test_db;

async fn make_client(pool: DbPool, config: DefGuardConfig) -> (Client, Arc<Mutex<GatewayState>>) {
    let (tx, rx) = unbounded_channel::<AppEvent>();
    let (wg_tx, wg_rx) = unbounded_channel::<GatewayEvent>();
    let gateway_state = Arc::new(Mutex::new(GatewayState::new(wg_rx)));

    let webapp = build_webapp(config, tx, rx, wg_tx, Arc::clone(&gateway_state), pool).await;
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
        "wireguard_pubkey": "sIhx53MsX+iLk83sssybHrD7M+5m+CmpLzWL/zo8C38=
    ",
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
