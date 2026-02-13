use std::net::IpAddr;

use defguard_common::db::models::{
    Device, DeviceType, WireguardNetwork,
    device::UserDevice,
    wireguard::{
        DEFAULT_DISCONNECT_THRESHOLD, DEFAULT_KEEPALIVE_INTERVAL, DEFAULT_WIREGUARD_MTU,
        LocationMfaMode, ServiceLocationMode,
    },
};
use defguard_core::{
    grpc::GatewayEvent,
    handlers::{Auth, wireguard::ImportedNetworkData},
};
use matches::assert_matches;
use reqwest::StatusCode;
use serde_json::json;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use tokio::sync::broadcast::error::TryRecvError;

use super::common::{fetch_user_details, make_test_client, setup_pool};

#[sqlx::test]
async fn test_config_import(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

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

        [Peer]
        PublicKey = l07+qPWs4jzW3Gp1DKbHgBMRRm4Jg3q2BJxw0ZYl6c4=
        AllowedIPs = 10.0.0.12/24
        PersistentKeepalive = 300
    ";
    let (client, client_state) = make_test_client(pool).await;
    let pool = client_state.pool;

    // setup initial network
    let initial_network = WireguardNetwork::new(
        "initial".into(),
        vec!["10.1.9.0/24".parse().unwrap()],
        51515,
        String::new(),
        None,
        DEFAULT_WIREGUARD_MTU,
        0,
        Vec::new(),
        DEFAULT_KEEPALIVE_INTERVAL,
        DEFAULT_DISCONNECT_THRESHOLD,
        false,
        false,
        LocationMfaMode::Disabled,
        ServiceLocationMode::Disabled,
    );
    initial_network.save(&pool).await.unwrap();

    // add existing devices
    let mut transaction = pool.begin().await.unwrap();

    let device_1 = Device::new(
        "test device".into(),
        "l07+qPWs4jzW3Gp1DKbHgBMRRm4Jg3q2BJxw0ZYl6c4=".into(),
        1,
        DeviceType::User,
        None,
        true,
    )
    .save(&mut *transaction)
    .await
    .unwrap();
    device_1
        .add_to_all_networks(&mut transaction)
        .await
        .unwrap();

    let device_2 = Device::new(
        "another test device".into(),
        "v2U14sjNN4tOYD3P15z0WkjriKY9Hl85I3vIEPomrYs=".into(),
        1,
        DeviceType::User,
        None,
        true,
    )
    .save(&mut *transaction)
    .await
    .unwrap();
    device_2
        .add_to_all_networks(&mut transaction)
        .await
        .unwrap();

    transaction.commit().await.unwrap();

    let mut wg_rx = client_state.wireguard_rx;

    let auth = Auth::new("admin", "pass123");
    let response = &client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // import network
    let response = client
        .post("/api/v1/network/import")
        .json(&json!({"name": "network", "endpoint": "192.168.1.1", "config": wg_config, "allowed_groups": []}))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let response: ImportedNetworkData = response.json().await;

    // network assertions
    let network = response.network;
    assert_eq!(network.id, 2);
    assert_eq!(network.name, "network");
    assert_eq!(network.address, vec!["10.0.0.1/24".parse().unwrap()]);
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
    let event = wg_rx.try_recv().unwrap();
    assert_matches!(event, GatewayEvent::NetworkCreated(..));

    // existing devices assertion
    // imported config for an existing device
    assert_matches!(wg_rx.try_recv().unwrap(), GatewayEvent::DeviceModified(..));
    let user_device_1 = UserDevice::from_device(&pool, device_1)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(user_device_1.networks.len(), 2);
    assert_eq!(
        user_device_1.networks[1].device_wireguard_ips,
        vec!["10.0.0.12"]
    );
    // generated IP for other existing device
    assert_matches!(wg_rx.try_recv().unwrap(), GatewayEvent::DeviceCreated(..));
    let user_device_2 = UserDevice::from_device(&pool, device_2)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(user_device_2.networks.len(), 2);

    // device assertions
    let devices = response.devices;
    assert_eq!(devices.len(), 2);

    let mut device1 = devices[0].clone();
    assert_eq!(
        device1.wireguard_ips,
        ["10.0.0.10".parse::<IpAddr>().unwrap()]
    );
    assert_eq!(
        device1.wireguard_pubkey,
        "2LYRr2HgSSpGCdXKDDAlcFe0Uuc6RR8TFgSquNc9VAE="
    );
    assert_eq!(device1.name, "2LYRr2HgSSpGCdXKDDAlcFe0Uuc6RR8TFgSquNc9VAE=");
    assert_eq!(device1.user_id, None);

    let mut device2 = devices[1].clone();
    assert_eq!(
        device2.wireguard_ips,
        ["10.0.0.11".parse::<IpAddr>().unwrap()]
    );
    assert_eq!(
        device2.wireguard_pubkey,
        "OLQNaEH3FxW0hiodaChEHoETzd+7UzcqIbsLs+X8rD0="
    );
    assert_eq!(device2.name, "OLQNaEH3FxW0hiodaChEHoETzd+7UzcqIbsLs+X8rD0=");
    assert_eq!(device2.user_id, None);

    // modify devices
    device1.user_id = Some(1);
    device1.name = "device_1".into();
    device2.user_id = Some(1);
    device2.name = "device_2".into();

    // post modified devices
    let response = client
        .post(format!("/api/v1/network/{}/devices", network.id))
        .json(&json!({"devices": [device1, device2]}))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // assert events
    let event = wg_rx.try_recv().unwrap();
    match event {
        GatewayEvent::DeviceCreated(device_info) => {
            assert_eq!(device_info.device.name, "device_1");
        }
        _ => unreachable!("Invalid event type received"),
    }

    let event = wg_rx.try_recv().unwrap();
    match event {
        GatewayEvent::DeviceCreated(device_info) => {
            assert_eq!(device_info.device.name, "device_2");
        }
        _ => unreachable!("Invalid event type received"),
    }

    let event = wg_rx.try_recv();
    assert_matches!(event, Err(TryRecvError::Empty));

    // assert user devices
    let user_info = fetch_user_details(&client, "admin").await;
    assert_eq!(user_info.devices.len(), 4);
    assert_eq!(user_info.devices[0].device.name, "test device");
    assert_eq!(
        user_info.devices[0].networks[1].device_wireguard_ips,
        vec!["10.0.0.12"]
    );
    assert_eq!(user_info.devices[1].device.name, "another test device");
    assert_eq!(
        user_info.devices[1].networks[1].device_wireguard_ips,
        vec!["10.0.0.2"]
    );
    assert_eq!(user_info.devices[2].device.name, "device_1");
    assert_eq!(
        user_info.devices[2].networks[1].device_wireguard_ips,
        vec!["10.0.0.10"]
    );
    assert_eq!(user_info.devices[3].device.name, "device_2");
    assert_eq!(
        user_info.devices[3].networks[1].device_wireguard_ips,
        vec!["10.0.0.11"]
    );
}

#[sqlx::test]
async fn test_config_import_missing_interface(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

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
    let (client, _) = make_test_client(pool).await;

    let auth = Auth::new("admin", "pass123");
    let response = &client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // import network
    let response = client
        .post("/api/v1/network/import")
        .json(&json!({"name": "network", "endpoint": "192.168.1.1", "config": wg_config}))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[sqlx::test]
async fn test_config_import_invalid_key(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

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
    let (client, _) = make_test_client(pool).await;

    let auth = Auth::new("admin", "pass123");
    let response = &client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // import network
    let response = client
        .post("/api/v1/network/import")
        .json(&json!({"name": "network", "endpoint": "192.168.1.1", "config": wg_config}))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    // invalid device pubkey
    let wg_config = "
        [Interface]
        PrivateKey = GAA2X3DW0WakGVx+DsGjhDpTgg50s1MlmrLf24Psrlg=
        Address = 10.0.0.1/24
        ListenPort = 55055
        DNS = 10.0.0.2

        [Peer]
        PublicKey = OLQNaEH3FxW0hiodaChEHoETzd+7UzcqIbsLs+X8rD0=
        AllowedIPs = 10.0.0.10/24
        PersistentKeepalive = 300

        [Peer]
        PublicKey = invalid_key
        AllowedIPs = 10.0.0.11/24
        PersistentKeepalive = 300
    ";
    let response = client
        .post("/api/v1/network/import")
        .json(&json!({"name": "network", "endpoint": "192.168.1.1", "config": wg_config}))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[sqlx::test]
async fn test_config_import_invalid_ip(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

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
    let (client, _) = make_test_client(pool).await;

    let auth = Auth::new("admin", "pass123");
    let response = &client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // import network
    let response = client
        .post("/api/v1/network/import")
        .json(&json!({"name": "network", "endpoint": "192.168.1.1", "config": wg_config}))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[sqlx::test]
async fn test_config_import_nonadmin(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

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
    let (client, _) = make_test_client(pool).await;
    let auth = Auth::new("hpotter", "pass123");
    let response = &client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // import network
    let response = client
        .post("/api/v1/network/import")
        .json(&json!({"name": "network", "endpoint": "192.168.1.1", "config": wg_config}))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}
