mod common;

use chrono::{Datelike, Duration, NaiveDate, SubsecRound, Timelike, Utc};
use defguard::{
    db::{
        models::wireguard::{
            WireguardDeviceStatsRow, WireguardDeviceTransferRow, WireguardNetworkStats,
            WireguardUserStatsRow,
        },
        Device, Id, NoId, WireguardPeerStats,
    },
    handlers::Auth,
};
use reqwest::StatusCode;
use serde::Deserialize;
use serde_json::{json, Value};

use self::common::make_test_client;

static DATE_FORMAT: &str = "%Y-%m-%dT%H:%M:00Z";

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
        "peer_disconnect_threshold": 180
    })
}

#[tokio::test]
async fn test_stats() {
    let (client, client_state) = make_test_client().await;
    let pool = client_state.pool;

    let auth = Auth::new("admin", "pass123");
    let response = &client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // create network
    let response = client
        .post("/api/v1/network")
        .json(&make_network())
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // create devices
    let device = json!({
        "name": "device-1",
        "wireguard_pubkey": "LQKsT6/3HWKuJmMulH63R8iK+5sI8FyYEL6WDIi6lQU=",
    });
    let response = client
        .post("/api/v1/device/admin")
        .json(&device)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    let device = json!({
        "name": "device-2",
        "wireguard_pubkey": "sIhx53MsX+iLk83sssybHrD7M+5m+CmpLzWL/zo8C38=",
    });
    let response = client
        .post("/api/v1/device/admin")
        .json(&device)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // get devices
    let mut devices = Vec::<Device<Id>>::new();
    let response = client.get("/api/v1/device/1").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    devices.push(response.json().await);

    let response = client.get("/api/v1/device/2").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    devices.push(response.json().await);

    // empty stats
    let now = Utc::now().naive_utc();
    let hour_ago = now - Duration::hours(1);
    let response = client
        .get(format!(
            "/api/v1/network/1/stats/users?from={}",
            hour_ago.format(DATE_FORMAT),
        ))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    #[derive(Deserialize)]
    struct StatsResponse {
        user_devices: Vec<WireguardUserStatsRow>,
        _network_devices: Vec<WireguardDeviceStatsRow>,
    }
    let stats = response.json::<StatsResponse>().await;
    let stats = stats.user_devices;
    assert!(stats.is_empty());

    // insert stats
    let samples = 60 * 11; // 11 hours of samples
    for i in 0..samples {
        for (d, device) in devices.iter().enumerate().take(2) {
            WireguardPeerStats {
                id: NoId,
                device_id: device.id,
                collected_at: now - Duration::minutes(i),
                network: 1,
                endpoint: Some("11.22.33.44".into()),
                upload: (samples - i) * 10 * (d as i64 + 1),
                download: (samples - i) * 20 * (d as i64 + 1),
                latest_handshake: now - Duration::minutes(i * 10),
                allowed_ips: Some("10.1.1.0/24".into()),
            }
            .save(&pool)
            .await
            .unwrap();
        }
    }

    // minute aggregation
    let response = client
        .get(format!(
            "/api/v1/network/1/stats/users?from={}",
            hour_ago.format(DATE_FORMAT),
        ))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let stats: Vec<WireguardUserStatsRow> = response.json().await;
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
            "/api/v1/network/1/stats/users?from={}",
            ten_hours_ago.format(DATE_FORMAT),
        ))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let stats: Vec<WireguardUserStatsRow> = response.json().await;
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
            "/api/v1/network/1/stats?from={}",
            ten_hours_ago.format(DATE_FORMAT),
        ))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let stats: WireguardNetworkStats = response.json().await;
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
