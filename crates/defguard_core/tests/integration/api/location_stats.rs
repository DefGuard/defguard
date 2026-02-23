use std::net::{IpAddr, Ipv4Addr};

use chrono::{Duration, Utc};
use defguard_common::db::{
    Id,
    models::{
        Device, DeviceType, WireguardNetwork, device::WireguardNetworkDevice, gateway::Gateway,
        vpn_client_session::VpnClientSession, vpn_session_stats::VpnSessionStats,
    },
};
use defguard_core::handlers::Auth;
use reqwest::StatusCode;
use serde::Deserialize;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

use super::common::{make_network, make_test_client, setup_pool};

static DATE_FORMAT: &str = "%Y-%m-%dT%H:%M:00Z";

#[derive(Deserialize)]
struct PaginatedResponse<T> {
    data: Vec<T>,
}

#[derive(Deserialize)]
struct ConnectedUserResponse {
    user_id: i64,
    connected_devices_count: u16,
    public_ip: String,
    vpn_ips: Vec<IpAddr>,
    total_upload: i64,
    total_download: i64,
}

#[derive(Deserialize)]
struct ConnectedNetworkDeviceResponse {
    device_id: i64,
    device_name: String,
    public_ip: String,
    vpn_ips: Vec<IpAddr>,
    total_upload: i64,
    total_download: i64,
}

#[derive(Deserialize)]
struct ConnectedUserDeviceResponse {
    device_id: i64,
    device_name: String,
    public_ip: String,
    vpn_ips: Vec<IpAddr>,
    total_upload: i64,
    total_download: i64,
}

#[sqlx::test]
async fn test_location_connected_devices_stats(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (client, client_state) = make_test_client(pool).await;

    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    let response = make_network(&client, "network").await;
    let network: WireguardNetwork<Id> = response.json().await;

    let gateway = Gateway::new(network.id, "gateway", "localhost", 50051, 1)
        .save(&client_state.pool)
        .await
        .unwrap();

    let user_device = Device::new(
        "user-device".to_string(),
        "user-device-pubkey".to_string(),
        client_state.test_user.id,
        DeviceType::User,
        None,
        true,
    )
    .save(&client_state.pool)
    .await
    .unwrap();

    let network_device = Device::new(
        "network-device".to_string(),
        "network-device-pubkey".to_string(),
        client_state.test_user.id,
        DeviceType::Network,
        None,
        true,
    )
    .save(&client_state.pool)
    .await
    .unwrap();

    let user_ip = IpAddr::V4(Ipv4Addr::new(10, 1, 1, 2));
    let network_ip = IpAddr::V4(Ipv4Addr::new(10, 1, 1, 3));

    WireguardNetworkDevice::new(network.id, user_device.id, vec![user_ip])
        .insert(&client_state.pool)
        .await
        .unwrap();
    WireguardNetworkDevice::new(network.id, network_device.id, vec![network_ip])
        .insert(&client_state.pool)
        .await
        .unwrap();

    let now = Utc::now().naive_utc();
    let user_session = VpnClientSession::new(
        network.id,
        client_state.test_user.id,
        user_device.id,
        Some(now),
        None,
    )
    .save(&client_state.pool)
    .await
    .unwrap();
    let network_session = VpnClientSession::new(
        network.id,
        client_state.test_user.id,
        network_device.id,
        Some(now),
        None,
    )
    .save(&client_state.pool)
    .await
    .unwrap();

    VpnSessionStats::new(
        user_session.id,
        gateway.id,
        now,
        now,
        "1.1.1.1:51820".to_string(),
        1000,
        2000,
        1000,
        2000,
    )
    .save(&client_state.pool)
    .await
    .unwrap();

    VpnSessionStats::new(
        network_session.id,
        gateway.id,
        now,
        now,
        "2.2.2.2:51820".to_string(),
        3000,
        4000,
        3000,
        4000,
    )
    .save(&client_state.pool)
    .await
    .unwrap();

    let from = (Utc::now().naive_utc() - Duration::minutes(10)).format(DATE_FORMAT);

    let response = client
        .get(format!(
            "/api/v1/network/{}/stats/connected_users?from={}",
            network.id, from
        ))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let users = response
        .json::<PaginatedResponse<ConnectedUserResponse>>()
        .await;
    assert_eq!(users.data.len(), 1);
    let user = &users.data[0];
    assert_eq!(user.user_id, client_state.test_user.id);
    assert_eq!(user.connected_devices_count, 1);
    assert_eq!(user.public_ip, "1.1.1.1");
    assert_eq!(user.vpn_ips, vec![user_ip]);
    assert_eq!(user.total_upload, 1000);
    assert_eq!(user.total_download, 2000);

    let response = client
        .get(format!(
            "/api/v1/network/{}/stats/connected_network_devices?from={}",
            network.id, from
        ))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let devices = response
        .json::<PaginatedResponse<ConnectedNetworkDeviceResponse>>()
        .await;
    assert_eq!(devices.data.len(), 1);
    let device = &devices.data[0];
    assert_eq!(device.device_id, network_device.id);
    assert_eq!(device.device_name, "network-device");
    assert_eq!(device.public_ip, "2.2.2.2");
    assert_eq!(device.vpn_ips, vec![network_ip]);
    assert_eq!(device.total_upload, 3000);
    assert_eq!(device.total_download, 4000);

    let response = client
        .get(format!(
            "/api/v1/network/{}/stats/connected_users/{}/devices?from={}",
            network.id, client_state.test_user.id, from
        ))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let devices = response.json::<Vec<ConnectedUserDeviceResponse>>().await;
    assert_eq!(devices.len(), 1);
    let device = &devices[0];
    assert_eq!(device.device_id, user_device.id);
    assert_eq!(device.device_name, "user-device");
    assert_eq!(device.public_ip, "1.1.1.1");
    assert_eq!(device.vpn_ips, vec![user_ip]);
    assert_eq!(device.total_upload, 1000);
    assert_eq!(device.total_download, 2000);
}
