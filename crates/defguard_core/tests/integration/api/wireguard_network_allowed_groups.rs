use std::net::IpAddr;

use claims::assert_err;
use defguard_common::{
    csv::AsCsv,
    db::{
        Id,
        models::{Device, DeviceType, User, WireguardNetwork, group::Group},
    },
};
use defguard_core::{
    grpc::GatewayEvent,
    handlers::{Auth, wireguard::ImportedNetworkData},
    location_management::allowed_peers::get_location_allowed_peers,
};
use matches::assert_matches;
use reqwest::StatusCode;
use serde_json::json;
use sqlx::{
    PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
};

use super::common::{fetch_user_details, make_test_client, setup_pool};

// setup user groups, test users and devices
async fn setup_test_users(pool: &PgPool) -> (Vec<User<Id>>, Vec<Device<Id>>) {
    let mut users = Vec::new();
    let mut devices = Vec::new();
    // create user groups
    let allowed_group = Group::new("allowed group").save(pool).await.unwrap();

    let not_allowed_group = Group::new("not allowed group").save(pool).await.unwrap();

    // admin user
    let admin_user = User::find_by_username(pool, "admin")
        .await
        .unwrap()
        .unwrap();
    let admin_device = Device::new(
        "admin device".into(),
        "nst4lmZz9kPTq6OdeQq2G2th3n+QneHKmG1wJJ3Jrq0=".into(),
        admin_user.id,
        DeviceType::User,
        None,
        true,
    )
    .save(pool)
    .await
    .unwrap();
    users.push(admin_user);
    devices.push(admin_device);

    // standard user in allowed group
    let test_user = User::find_by_username(pool, "hpotter")
        .await
        .unwrap()
        .unwrap();
    test_user.add_to_group(pool, &allowed_group).await.unwrap();
    let test_device = Device::new(
        "test device".into(),
        "wYOt6ImBaQ3BEMQ3Xf5P5fTnbqwOvjcqYkkSBt+1xOg=".into(),
        test_user.id,
        DeviceType::User,
        None,
        true,
    )
    .save(pool)
    .await
    .unwrap();
    users.push(test_user);
    devices.push(test_device);

    // standard user in other, non-allowed group
    let other_user = User::new(
        "ssnape",
        Some("pass123"),
        "Snape",
        "Severus",
        "s.snape@hogwart.edu.uk",
        None,
    )
    .save(pool)
    .await
    .unwrap();
    other_user
        .add_to_group(pool, &not_allowed_group)
        .await
        .unwrap();
    let other_device = Device::new(
        "other device".into(),
        "v2U14sjNN4tOYD3P15z0WkjriKY9Hl85I3vIEPomrYs=".into(),
        other_user.id,
        DeviceType::User,
        None,
        true,
    )
    .save(pool)
    .await
    .unwrap();
    users.push(other_user);
    devices.push(other_device);

    // standard user in no groups
    let non_group_user = User::new(
        "dobby",
        Some("pass123"),
        "Elf",
        "Dobby",
        "dobby@hogwart.edu.uk",
        None,
    )
    .save(pool)
    .await
    .unwrap();
    let non_group_device = Device::new(
        "non group device".into(),
        "6xmL/jRuxmzQ3J2/kVZnKnh+6dwODcEEczmmkIKU4sM=".into(),
        non_group_user.id,
        DeviceType::User,
        None,
        true,
    )
    .save(pool)
    .await
    .unwrap();
    users.push(non_group_user);
    devices.push(non_group_device);

    (users, devices)
}

#[sqlx::test]
async fn test_create_new_network(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (client, client_state) = make_test_client(pool).await;
    let (_users, devices) = setup_test_users(&client_state.pool).await;

    let mut wg_rx = client_state.wireguard_rx;

    let auth = Auth::new("admin", "pass123");
    let response = &client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // create network
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
            "allowed_groups": ["allowed group"],
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
    let network: WireguardNetwork<Id> = response.json().await;
    assert_eq!(network.name, "network");
    let event = wg_rx.try_recv().unwrap();
    assert_matches!(event, GatewayEvent::NetworkCreated(..));
    assert_err!(wg_rx.try_recv());

    // network configuration was created only for admin and allowed user
    let peers = get_location_allowed_peers(&network, &client_state.pool)
        .await
        .unwrap();
    assert_eq!(peers.len(), 2);
    assert_eq!(peers[0].pubkey, devices[0].wireguard_pubkey);
    assert_eq!(peers[1].pubkey, devices[1].wireguard_pubkey);
}

#[sqlx::test]
async fn test_modify_network(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (client, client_state) = make_test_client(pool).await;
    let (_users, devices) = setup_test_users(&client_state.pool).await;

    let mut wg_rx = client_state.wireguard_rx;

    let auth = Auth::new("admin", "pass123");
    let response = &client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // create network without allowed groups
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
    let network: WireguardNetwork<Id> = response.json().await;
    assert_eq!(network.name, "network");
    let event = wg_rx.try_recv().unwrap();
    assert_matches!(event, GatewayEvent::NetworkCreated(..));

    // network configuration was created for all devices
    let peers = get_location_allowed_peers(&network, &client_state.pool)
        .await
        .unwrap();
    assert_eq!(peers.len(), 4);
    assert_eq!(peers[0].pubkey, devices[0].wireguard_pubkey);
    assert_eq!(peers[1].pubkey, devices[1].wireguard_pubkey);
    assert_eq!(peers[2].pubkey, devices[2].wireguard_pubkey);
    assert_eq!(peers[3].pubkey, devices[3].wireguard_pubkey);

    // add an allowed group
    let response = client
        .put("/api/v1/network/1")
        .json(&json!({
            "name": "network",
            "address": "10.1.1.1/24",
            "port": 55555,
            "endpoint": "192.168.4.14",
            "allowed_ips": "10.1.1.0/24",
            "dns": "1.1.1.1",
            "mtu": 1420,
            "fwmark": 0,
            "allowed_groups": ["allowed group"],
            "keepalive_interval": 25,
            "peer_disconnect_threshold": 300,
            "acl_enabled": false,
            "acl_default_allow": false,
            "location_mfa_mode": "disabled",
            "service_location_mode": "disabled"
        }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_matches!(wg_rx.try_recv().unwrap(), GatewayEvent::NetworkModified(..));

    let new_peers = get_location_allowed_peers(&network, &client_state.pool)
        .await
        .unwrap();
    assert_eq!(new_peers.len(), 2);
    assert_eq!(new_peers[0].pubkey, devices[0].wireguard_pubkey);
    assert_eq!(new_peers[1].pubkey, devices[1].wireguard_pubkey);

    // add another allowed group
    let response = client
        .put("/api/v1/network/1")
        .json(&json!({
            "name": "network",
            "address": "10.1.1.1/24",
            "port": 55555,
            "endpoint": "192.168.4.14",
            "allowed_ips": "10.1.1.0/24",
            "dns": "1.1.1.1",
            "mtu": 1420,
            "fwmark": 0,
            "allowed_groups": ["allowed group", "not allowed group"],
            "keepalive_interval": 25,
            "peer_disconnect_threshold": 300,
            "acl_enabled": false,
            "acl_default_allow": false,
            "location_mfa_mode": "disabled",
            "service_location_mode": "disabled"
        }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_matches!(wg_rx.try_recv().unwrap(), GatewayEvent::NetworkModified(..));

    let new_peers = get_location_allowed_peers(&network, &client_state.pool)
        .await
        .unwrap();
    assert_eq!(new_peers.len(), 3);
    assert_eq!(new_peers[0].pubkey, devices[0].wireguard_pubkey);
    assert_eq!(new_peers[1].pubkey, devices[1].wireguard_pubkey);
    assert_eq!(new_peers[2].pubkey, devices[2].wireguard_pubkey);

    // remove allowed group
    let response = client
        .put("/api/v1/network/1")
        .json(&json!({
            "name": "network",
            "address": "10.1.1.1/24",
            "port": 55555,
            "endpoint": "192.168.4.14",
            "allowed_ips": "10.1.1.0/24",
            "dns": "1.1.1.1",
            "mtu": 1420,
            "fwmark": 0,
            "allowed_groups": ["not allowed group"],
            "keepalive_interval": 25,
            "peer_disconnect_threshold": 300,
            "acl_enabled": false,
            "acl_default_allow": false,
            "location_mfa_mode": "disabled",
            "service_location_mode": "disabled"
        }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_matches!(wg_rx.try_recv().unwrap(), GatewayEvent::NetworkModified(..));

    let new_peers = get_location_allowed_peers(&network, &client_state.pool)
        .await
        .unwrap();
    assert_eq!(new_peers.len(), 2);
    assert_eq!(new_peers[0].pubkey, devices[0].wireguard_pubkey);
    assert_eq!(new_peers[1].pubkey, devices[2].wireguard_pubkey);

    // remove all allowed groups
    let response = client
        .put("/api/v1/network/1")
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
    assert_eq!(response.status(), StatusCode::OK);
    assert_matches!(wg_rx.try_recv().unwrap(), GatewayEvent::NetworkModified(..));

    let new_peers = get_location_allowed_peers(&network, &client_state.pool)
        .await
        .unwrap();
    assert_eq!(new_peers.len(), 4);
    assert_eq!(new_peers[0].pubkey, devices[0].wireguard_pubkey);
    assert_eq!(new_peers[1].pubkey, devices[1].wireguard_pubkey);
    assert_eq!(new_peers[2].pubkey, devices[2].wireguard_pubkey);
    assert_eq!(new_peers[3].pubkey, devices[3].wireguard_pubkey);

    assert_err!(wg_rx.try_recv());
}

/// Test that devices that already exist are handled correctly during config import
#[sqlx::test]
async fn test_import_network_existing_devices(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (client, client_state) = make_test_client(pool).await;
    let (_users, devices) = setup_test_users(&client_state.pool).await;

    let mut wg_rx = client_state.wireguard_rx;

    let auth = Auth::new("admin", "pass123");
    let response = &client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // config file includes some existing devices
    let wg_config = format!(
        "
        [Interface]
        PrivateKey = GAA2X3DW0WakGVx+DsGjhDpTgg50s1MlmrLf24Psrlg=
        Address = 10.0.0.1/24
        ListenPort = 55055
        DNS = 10.0.0.2

        [Peer]
        PublicKey = {}
        AllowedIPs = 10.0.0.10/24
        PersistentKeepalive = 300

        [Peer]
        PublicKey = {}
        AllowedIPs = 10.0.0.11/24
        PersistentKeepalive = 300

        [Peer]
        PublicKey = l07+qPWs4jzW3Gp1DKbHgBMRRm4Jg3q2BJxw0ZYl6c4=
        AllowedIPs = 10.0.0.12/24
        PersistentKeepalive = 300
    ",
        devices[1].wireguard_pubkey, devices[2].wireguard_pubkey
    );

    // import network
    let response = client
        .post("/api/v1/network/import")
        .json(&json!({"name": "network", "endpoint": "192.168.1.1", "config": wg_config, "allowed_groups": ["allowed group"]}))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let response: ImportedNetworkData = response.json().await;
    assert_eq!(response.devices.len(), 1);
    assert_eq!(
        response.devices[0].wireguard_pubkey,
        "l07+qPWs4jzW3Gp1DKbHgBMRRm4Jg3q2BJxw0ZYl6c4="
    );
    assert_eq!(
        response.devices[0].wireguard_ips,
        ["10.0.0.12".parse::<IpAddr>().unwrap()]
    );
    let network = response.network;

    let peers = get_location_allowed_peers(&network, &client_state.pool)
        .await
        .unwrap();
    assert_eq!(peers.len(), 2);
    assert_eq!(peers[0].pubkey, devices[0].wireguard_pubkey);
    assert_eq!(peers[1].pubkey, devices[1].wireguard_pubkey);

    let event = wg_rx.try_recv().unwrap();
    assert_matches!(event, GatewayEvent::NetworkCreated(..));

    // network config was only created for one of the existing devices and the admin device
    let GatewayEvent::DeviceModified(device_info) = wg_rx.try_recv().unwrap() else {
        panic!()
    };
    assert_eq!(device_info.device.id, devices[1].id);
    assert_eq!(device_info.network_info.len(), 1);
    assert_eq!(device_info.network_info[0].network_id, 1);
    assert_eq!(
        device_info.network_info[0].device_wireguard_ips.as_csv(),
        peers[1].allowed_ips[0]
    );

    let GatewayEvent::DeviceCreated(device_info) = wg_rx.try_recv().unwrap() else {
        panic!()
    };
    assert_eq!(device_info.device.id, devices[0].id);
    assert_eq!(device_info.network_info.len(), 1);
    assert_eq!(device_info.network_info[0].network_id, 1);
    assert_eq!(
        device_info.network_info[0].device_wireguard_ips.as_csv(),
        peers[0].allowed_ips[0]
    );

    assert_err!(wg_rx.try_recv());
}

#[sqlx::test]
async fn test_import_mapping_devices(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (client, client_state) = make_test_client(pool).await;
    let (users, devices) = setup_test_users(&client_state.pool).await;

    let mut wg_rx = client_state.wireguard_rx;

    let auth = Auth::new("admin", "pass123");
    let response = &client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

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

# device name
[Peer]
PublicKey = 8SHdUZJYfm8uKzKZXT0S8QJQGDPq+6asPUDl0ZtX8Zg=
AllowedIPs = 10.0.0.13/24
PersistentKeepalive = 300
    ";

    // import network
    let response = client
        .post("/api/v1/network/import")
        .json(&json!({"name": "network", "endpoint": "192.168.1.1", "config": wg_config, "allowed_groups": ["allowed group"]}))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let response: ImportedNetworkData = response.json().await;
    let network = response.network;
    let mut mapped_devices = response.devices;
    assert_eq!(mapped_devices.len(), 4);
    for _ in 0..3 {
        wg_rx.try_recv().unwrap();
    }

    // assign devices to users
    mapped_devices[0].user_id = Some(users[0].id);
    mapped_devices[1].user_id = Some(users[1].id);
    mapped_devices[2].user_id = Some(users[2].id);
    mapped_devices[3].user_id = Some(users[3].id);

    let response = client
        .post(format!("/api/v1/network/{}/devices", network.id))
        .json(&json!({"devices": mapped_devices.clone()}))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    let peers = get_location_allowed_peers(&network, &client_state.pool)
        .await
        .unwrap();
    assert_eq!(peers.len(), 4);
    assert_eq!(peers[0].pubkey, devices[0].wireguard_pubkey);
    assert_eq!(peers[1].pubkey, devices[1].wireguard_pubkey);
    assert_eq!(peers[2].pubkey, mapped_devices[0].wireguard_pubkey);
    assert_eq!(peers[3].pubkey, mapped_devices[1].wireguard_pubkey);

    // assert events
    let GatewayEvent::DeviceCreated(device_info) = wg_rx.try_recv().unwrap() else {
        panic!()
    };
    assert_eq!(
        device_info.device.wireguard_pubkey,
        mapped_devices[0].wireguard_pubkey
    );
    assert_eq!(device_info.network_info.len(), 1);
    assert_eq!(device_info.network_info[0].network_id, 1);
    assert_eq!(
        device_info.network_info[0].device_wireguard_ips,
        mapped_devices[0].wireguard_ips,
    );

    let GatewayEvent::DeviceCreated(device_info) = wg_rx.try_recv().unwrap() else {
        panic!()
    };
    assert_eq!(
        device_info.device.wireguard_pubkey,
        mapped_devices[1].wireguard_pubkey
    );
    assert_eq!(device_info.network_info.len(), 1);
    assert_eq!(device_info.network_info[0].network_id, 1);
    assert_eq!(
        device_info.network_info[0].device_wireguard_ips,
        mapped_devices[1].wireguard_ips,
    );

    assert_err!(wg_rx.try_recv());
}

/// Test that changing groups for a particular user generates correct update events
#[sqlx::test]
async fn test_modify_user(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (client, client_state) = make_test_client(pool).await;
    let (_users, devices) = setup_test_users(&client_state.pool).await;

    let mut wg_rx = client_state.wireguard_rx;

    let auth = Auth::new("admin", "pass123");
    let response = &client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // create network
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
            "allowed_groups": ["allowed group"],
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
    let network: WireguardNetwork<Id> = response.json().await;
    assert_eq!(network.name, "network");
    let event = wg_rx.try_recv().unwrap();
    assert_matches!(event, GatewayEvent::NetworkCreated(..));
    assert_err!(wg_rx.try_recv());

    // network configuration was created only for admin and allowed user
    let peers = get_location_allowed_peers(&network, &client_state.pool)
        .await
        .unwrap();
    assert_eq!(peers.len(), 2);
    assert_eq!(peers[0].pubkey, devices[0].wireguard_pubkey);
    assert_eq!(peers[1].pubkey, devices[1].wireguard_pubkey);

    // remove user from allowed group
    let mut user_details = fetch_user_details(&client, "hpotter").await;
    user_details.user.groups = vec![];
    let response = client
        .put("/api/v1/user/hpotter")
        .json(&user_details.user)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    let event = wg_rx.try_recv().unwrap();
    assert_matches!(event, GatewayEvent::DeviceDeleted(..));
    assert_err!(wg_rx.try_recv());

    let peers = get_location_allowed_peers(&network, &client_state.pool)
        .await
        .unwrap();
    assert_eq!(peers.len(), 1);
    assert_eq!(peers[0].pubkey, devices[0].wireguard_pubkey);

    // remove user from unrelated group
    let mut user_details = fetch_user_details(&client, "ssnape").await;
    user_details.user.groups = vec![];
    let response = client
        .put("/api/v1/user/ssnape")
        .json(&user_details.user)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    assert_err!(wg_rx.try_recv());

    let peers = get_location_allowed_peers(&network, &client_state.pool)
        .await
        .unwrap();
    assert_eq!(peers.len(), 1);
    assert_eq!(peers[0].pubkey, devices[0].wireguard_pubkey);

    // add user to allowed group
    let mut user_details = fetch_user_details(&client, "dobby").await;
    user_details.user.groups = vec!["allowed group".into()];
    let response = client
        .put("/api/v1/user/dobby")
        .json(&user_details.user)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    let event = wg_rx.try_recv().unwrap();
    assert_matches!(event, GatewayEvent::DeviceCreated(..));
    assert_err!(wg_rx.try_recv());

    let peers = get_location_allowed_peers(&network, &client_state.pool)
        .await
        .unwrap();
    assert_eq!(peers.len(), 2);
    assert_eq!(peers[0].pubkey, devices[0].wireguard_pubkey);
    assert_eq!(peers[1].pubkey, devices[3].wireguard_pubkey);
}

#[sqlx::test]
async fn test_delete_only_allowed_group(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (client, client_state) = make_test_client(pool).await;
    let (_users, devices) = setup_test_users(&client_state.pool).await;

    let mut wg_rx = client_state.wireguard_rx;

    let auth = Auth::new("admin", "pass123");
    let response = &client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // create network with an allowed group
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
            "allowed_groups": ["allowed group"],
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
    let network: WireguardNetwork<Id> = response.json().await;
    assert_eq!(network.name, "network");
    let event = wg_rx.try_recv().unwrap();
    assert_matches!(event, GatewayEvent::NetworkCreated(..));

    let peers = get_location_allowed_peers(&network, &client_state.pool)
        .await
        .unwrap();
    assert_eq!(peers.len(), 2);
    assert_eq!(peers[0].pubkey, devices[0].wireguard_pubkey);
    assert_eq!(peers[1].pubkey, devices[1].wireguard_pubkey);

    // remove an allowed group
    let response = client.delete("/api/v1/group/allowed%20group").send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // network configuration was created for all devices
    let peers = get_location_allowed_peers(&network, &client_state.pool)
        .await
        .unwrap();
    assert_eq!(peers.len(), 4);
    assert_eq!(peers[0].pubkey, devices[0].wireguard_pubkey);
    assert_eq!(peers[1].pubkey, devices[1].wireguard_pubkey);
    assert_eq!(peers[2].pubkey, devices[2].wireguard_pubkey);
    assert_eq!(peers[3].pubkey, devices[3].wireguard_pubkey);
}
