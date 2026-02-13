use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use defguard_common::db::{
    Id,
    models::{
        Device, WireguardNetwork,
        device::WireguardNetworkDevice,
        settings::OpenIdUsernameHandling,
        wireguard::{
            DEFAULT_DISCONNECT_THRESHOLD, DEFAULT_KEEPALIVE_INTERVAL, DEFAULT_WIREGUARD_MTU,
            LocationMfaMode, ServiceLocationMode,
        },
    },
};
use defguard_core::{
    enterprise::{
        db::models::openid_provider::{
            DirectorySyncTarget, DirectorySyncUserBehavior, OpenIdProviderKind,
        },
        handlers::openid_providers::AddProviderData,
        license::{get_cached_license, set_cached_license},
    },
    grpc::GatewayEvent,
    handlers::{Auth, GroupInfo, wireguard::WireguardNetworkData},
};
use ipnetwork::IpNetwork;
use matches::assert_matches;
use reqwest::StatusCode;
use serde_json::json;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

use super::common::{
    authenticate_admin, exceed_enterprise_limits, make_network, make_test_client, setup_pool,
};

#[sqlx::test]
async fn test_network(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (client, client_state) = make_test_client(pool).await;

    let mut wg_rx = client_state.wireguard_rx;

    let auth = Auth::new("admin", "pass123");
    let response = &client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // create network
    let response = make_network(&client, "network").await;
    let network: WireguardNetwork<Id> = response.json().await;
    assert_eq!(network.name, "network");
    let event = wg_rx.try_recv().unwrap();
    assert_matches!(event, GatewayEvent::NetworkCreated(..));

    // check vpn locations for `admin` group
    let response = client.get("/api/v1/group/admin").send().await;
    let group_info: GroupInfo = response.json().await;
    assert!(group_info.vpn_locations.is_empty());

    // modify network
    let network_data = WireguardNetworkData {
        name: "my network".into(),
        address: "10.1.1.0/24".into(),
        endpoint: "10.1.1.1".parse().unwrap(),
        port: 55555,
        allowed_ips: Some("10.1.1.0/24, 10.2.0.1/16, 10.10.10.54/32".into()),
        dns: None,
        mtu: DEFAULT_WIREGUARD_MTU,
        fwmark: 0,
        allowed_groups: vec!["admin".into()],
        keepalive_interval: DEFAULT_KEEPALIVE_INTERVAL,
        peer_disconnect_threshold: DEFAULT_DISCONNECT_THRESHOLD,
        acl_enabled: false,
        acl_default_allow: false,
        location_mfa_mode: LocationMfaMode::Disabled,
        service_location_mode: ServiceLocationMode::Disabled,
    };
    let response = client
        .put(format!("/api/v1/network/{}", network.id))
        .json(&network_data)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let network: WireguardNetwork<Id> = response.json().await;
    assert_eq!(
        network.allowed_ips,
        vec![
            IpNetwork::V4("10.1.1.0/24".parse().unwrap()),
            IpNetwork::V4("10.2.0.0/16".parse().unwrap()),
            IpNetwork::V4("10.10.10.54/32".parse().unwrap())
        ]
    );

    let event = wg_rx.try_recv().unwrap();
    assert_matches!(event, GatewayEvent::NetworkModified(..));

    // check vpn locations for `admin` group
    let response = client.get("/api/v1/group/admin").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let group_info: GroupInfo = response.json().await;
    assert_eq!(group_info.vpn_locations, vec!["my network"]);

    // list networks
    let response = client.get("/api/v1/network").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let networks: Vec<WireguardNetwork<Id>> = response.json().await;
    assert_eq!(networks.len(), 1);

    // network details
    let network_from_list = networks[0].clone();
    assert_eq!(network_from_list.name, "my network");
    let response = client
        .get(format!("/api/v1/network/{}", network_from_list.id))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let network_from_details: WireguardNetwork<Id> = response.json().await;
    assert_eq!(network_from_details, network_from_list);

    // delete network
    let response = client
        .delete(format!("/api/v1/network/{}", network.id))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let event = wg_rx.try_recv().unwrap();
    assert_matches!(event, GatewayEvent::NetworkDeleted(..));
}

#[sqlx::test]
async fn test_location_mfa_mode_validation_create(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (mut client, _client_state) = make_test_client(pool).await;
    authenticate_admin(&mut client).await;

    exceed_enterprise_limits(&client).await;

    // unset the license
    let license = get_cached_license().clone();
    set_cached_license(None);

    let location_data = WireguardNetworkData {
        name: "test_location".into(),
        address: "10.1.1.0/24".into(),
        endpoint: "10.1.1.1".parse().unwrap(),
        port: 55555,
        allowed_ips: Some("10.1.1.0/24, 10.2.0.1/16, 10.10.10.54/32".into()),
        dns: None,
        mtu: DEFAULT_WIREGUARD_MTU,
        fwmark: 0,
        allowed_groups: vec!["admin".into()],
        keepalive_interval: DEFAULT_KEEPALIVE_INTERVAL,
        peer_disconnect_threshold: DEFAULT_DISCONNECT_THRESHOLD,
        acl_enabled: false,
        acl_default_allow: false,
        location_mfa_mode: LocationMfaMode::External,
        service_location_mode: ServiceLocationMode::Disabled,
    };

    // create network
    let response = client
        .post("/api/v1/network")
        .json(&location_data)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    // restore valid license and try again
    set_cached_license(license);
    let response = client
        .post("/api/v1/network")
        .json(&location_data)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // add external OpenID provider
    let provider_data = AddProviderData {
        name: "test".to_string(),
        base_url: "https://accounts.google.com".to_string(),
        kind: OpenIdProviderKind::Custom,
        client_id: "client_id".to_string(),
        client_secret: "client_secret".to_string(),
        display_name: Some("display_name".to_string()),
        admin_email: None,
        google_service_account_email: None,
        google_service_account_key: None,
        directory_sync_enabled: false,
        directory_sync_interval: 100,
        directory_sync_user_behavior: DirectorySyncUserBehavior::Keep.to_string(),
        directory_sync_admin_behavior: DirectorySyncUserBehavior::Keep.to_string(),
        directory_sync_target: DirectorySyncTarget::All.to_string(),
        create_account: false,
        okta_dirsync_client_id: None,
        okta_private_jwk: None,
        directory_sync_group_match: None,
        username_handling: OpenIdUsernameHandling::PruneEmailDomain,
        jumpcloud_api_key: None,
        prefetch_users: false,
    };

    let response = client
        .post("/api/v1/openid/provider")
        .json(&provider_data)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::CREATED);

    // try again
    let response = client
        .post("/api/v1/network")
        .json(&location_data)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
}

#[sqlx::test]
async fn test_location_mfa_mode_validation_modify(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (mut client, _client_state) = make_test_client(pool).await;
    authenticate_admin(&mut client).await;

    let mut location_data = WireguardNetworkData {
        name: "test_location".into(),
        address: "10.1.1.0/24".into(),
        endpoint: "10.1.1.1".parse().unwrap(),
        port: 55555,
        allowed_ips: Some("10.1.1.0/24, 10.2.0.1/16, 10.10.10.54/32".into()),
        dns: None,
        mtu: DEFAULT_WIREGUARD_MTU,
        fwmark: 0,
        allowed_groups: vec!["admin".into()],
        keepalive_interval: DEFAULT_KEEPALIVE_INTERVAL,
        peer_disconnect_threshold: DEFAULT_DISCONNECT_THRESHOLD,
        acl_enabled: false,
        acl_default_allow: false,
        location_mfa_mode: LocationMfaMode::Disabled,
        service_location_mode: ServiceLocationMode::Disabled,
    };

    // create network
    let response = client
        .post("/api/v1/network")
        .json(&location_data)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    exceed_enterprise_limits(&client).await;

    // unset the license
    let license = get_cached_license().clone();
    set_cached_license(None);

    // attempt to modify location
    location_data.location_mfa_mode = LocationMfaMode::External;
    let response = client
        .put("/api/v1/network/1")
        .json(&location_data)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    // restore valid license and try again
    set_cached_license(license);
    let response = client
        .put("/api/v1/network/1")
        .json(&location_data)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // add external OpenID provider
    let provider_data = AddProviderData {
        name: "test".to_string(),
        base_url: "https://accounts.google.com".to_string(),
        kind: OpenIdProviderKind::Google,
        client_id: "client_id".to_string(),
        client_secret: "client_secret".to_string(),
        display_name: Some("display_name".to_string()),
        admin_email: None,
        google_service_account_email: None,
        google_service_account_key: None,
        directory_sync_enabled: false,
        directory_sync_interval: 100,
        directory_sync_user_behavior: DirectorySyncUserBehavior::Keep.to_string(),
        directory_sync_admin_behavior: DirectorySyncUserBehavior::Keep.to_string(),
        directory_sync_target: DirectorySyncTarget::All.to_string(),
        create_account: false,
        okta_dirsync_client_id: None,
        okta_private_jwk: None,
        directory_sync_group_match: None,
        username_handling: OpenIdUsernameHandling::PruneEmailDomain,
        jumpcloud_api_key: None,
        prefetch_users: false,
    };

    let response = client
        .post("/api/v1/openid/provider")
        .json(&provider_data)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::CREATED);

    // try again
    let response = client
        .put("/api/v1/network/1")
        .json(&location_data)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
}

#[sqlx::test]
async fn test_device(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (client, client_state) = make_test_client(pool).await;

    let mut wg_rx = client_state.wireguard_rx;

    let auth = Auth::new("admin", "pass123");
    let response = &client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // create network
    make_network(&client, "network").await;
    let event = wg_rx.try_recv().unwrap();
    assert_matches!(event, GatewayEvent::NetworkCreated(..));

    // network details
    let response = client.get("/api/v1/network/1").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let network_from_details: WireguardNetwork<Id> = response.json().await;

    // create device
    let device = json!({
        "name": "device",
        "wireguard_pubkey": "LQKsT6/3HWKuJmMulH63R8iK+5sI8FyYEL6WDIi6lQU=",
    });
    let response = client
        .post("/api/v1/device/admin")
        .json(&device)
        .send()
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
        network_from_details.id
    );

    // add another network
    make_network(&client, "network").await;
    assert_matches!(wg_rx.try_recv().unwrap(), GatewayEvent::NetworkCreated(..));

    // an IP was assigned for an existing device
    let network_devices = WireguardNetworkDevice::find_by_device(&client_state.pool, 1)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(network_devices.len(), 2);

    // list devices
    let response = client.get("/api/v1/device").json(&device).send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let devices: Vec<Device<Id>> = response.json().await;
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
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let user_devices: Vec<Device<Id>> = response.json().await;
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
        .put(format!("/api/v1/device/{}", device.id))
        .json(&modified_device)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let event = wg_rx.try_recv().unwrap();
    assert_matches!(event, GatewayEvent::DeviceModified(..));

    // device details
    let response = client
        .get(format!("/api/v1/device/{}", device.id))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let device_from_details: Device<Id> = response.json().await;
    assert_eq!(device_from_details.name, modified_name);
    assert_eq!(device_from_details.wireguard_pubkey, modified_key);

    // device config
    let response = client
        .get(format!("/api/v1/network/1/device/{}/config", device.id))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let config = response.text().await;
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
        .delete(format!("/api/v1/network/{}", network_from_details.id))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let event = wg_rx.try_recv().unwrap();
    assert_matches!(event, GatewayEvent::NetworkDeleted(..));

    // delete device
    let response = client
        .delete(format!("/api/v1/device/{}", device.id))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let event = wg_rx.try_recv().unwrap();
    assert_matches!(event, GatewayEvent::DeviceDeleted(..));

    let response = client.get("/api/v1/device").json(&device).send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let devices: Vec<Device<Id>> = response.json().await;
    assert!(devices.is_empty());
}

#[sqlx::test]
async fn test_network_address_reassignment(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (client, client_state) = make_test_client(pool).await;

    let auth = Auth::new("admin", "pass123");
    let response = &client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // create network
    let response = make_network(&client, "network").await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // network details
    let response = client.get("/api/v1/network/1").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let network_from_details: WireguardNetwork<Id> = response.json().await;

    // create devices
    let device = json!({
        "name": "device1",
        "wireguard_pubkey": "LQKsT6/3HWKuJmMulH63R8iK+5sI8FyYEL6WDIi6lQU=",
    });
    let response = client
        .post("/api/v1/device/admin")
        .json(&device)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let device = json!({
        "name": "device2",
        "wireguard_pubkey": "ZqDlG4LQZRO9v57Sd27AHdtTLxegbMp5oVThjYrg21I=",
    });
    let response = client
        .post("/api/v1/device/admin")
        .json(&device)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // ensure IPs were assigned for new devices
    let network_devices = WireguardNetworkDevice::find_by_device(&client_state.pool, 1)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        network_devices[0].wireguard_ips,
        vec![IpAddr::V4(Ipv4Addr::new(10, 1, 1, 2))],
    );
    let network_devices = WireguardNetworkDevice::find_by_device(&client_state.pool, 2)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        network_devices[0].wireguard_ips,
        vec![IpAddr::V4(Ipv4Addr::new(10, 1, 1, 3))],
    );

    // delete the first device
    let response = client.delete("/api/v1/device/1").json(&device).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // modify network addresses
    let network = json!({
        "id": network_from_details.id,
        "name": "network",
        "address": "10.1.1.1/24,fc00::1/112",
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
    let response = client
        .put(format!("/api/v1/network/{}", network_from_details.id))
        .json(&network)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    // ensure IPv4 address wasn't reassigned
    let network_devices = WireguardNetworkDevice::find_by_device(&client_state.pool, 2)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        network_devices[0].wireguard_ips,
        vec![
            IpAddr::V4(Ipv4Addr::new(10, 1, 1, 3)),
            IpAddr::V6(Ipv6Addr::new(0xfc00, 0, 0, 0, 0, 0, 0, 2)),
        ],
    );
}

#[sqlx::test]
async fn test_device_permissions(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (client, _) = make_test_client(pool).await;

    let auth = Auth::new("admin", "pass123");
    let response = &client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // create network
    make_network(&client, "network").await;

    // admin can add devices for other users
    let device = json!({
        "name": "device_1",
        "wireguard_pubkey": "LQKsT6/3HWKuJmMulH63R8iK+5sI8FyYEL6WDIi6lQU=",
    });
    let response = client
        .post("/api/v1/device/admin")
        .json(&device)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let device = json!({"devices": [{
        "name": "device_2",
        "wireguard_ips": ["10.0.0.3"],
        "wireguard_pubkey": "TJgN9JzUF5zdZAPYD96G/Wys2M3TvaT5TIrErUl20nI=",
        "user_id": 1,
        "created": "2023-05-05T23:56:04"
    }]});
    let response = client
        .post("/api/v1/network/1/devices")
        .json(&device)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    let device = json!({
        "name": "device_3",
        "wireguard_pubkey": "PKY3zg5/ecNyMjqLi6yJ3jwb4PvC/SGzjhJ3jrn2vVQ=",
    });
    let response = client
        .post("/api/v1/device/hpotter")
        .json(&device)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let device = json!({"devices": [{
        "name": "device_4",
        "wireguard_ips": ["10.0.0.5"],
        "wireguard_pubkey": "gTMFF29nNLkJR1UhoiO3ZJLF60h2hW+WxmIu5DGJ0B4=",
        "user_id": 2,
        "created": "2023-05-05T23:56:04"
    }]});
    let response = client
        .post("/api/v1/network/1/devices")
        .json(&device)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // normal user cannot add devices for other users or import multiple devices
    let auth = Auth::new("hpotter", "pass123");
    let response = &client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    let device = json!({
        "name": "device_5",
        "wireguard_pubkey": "qhLnyggsD1nVOcLdTk0q43kOZHHknPQgftBY+ZLy40Q=",
    });
    let response = client
        .post("/api/v1/device/hpotter")
        .json(&device)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let device = json!({"devices": [{
        "name": "device_6",
        "wireguard_ips": ["10.0.0.7"],
        "wireguard_pubkey": "xGLqgxVAnmk9+tsj5X/wzwouwx3bF1b3W+VWAb4NLjM=",
        "user_id": 2,
        "created": "2023-05-05T23:56:04"
    }]});
    let response = client
        .post("/api/v1/network/1/devices")
        .json(&device)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let device = json!({
        "name": "device_7",
        "wireguard_pubkey": "J4p/w6R0xt4c2dIBDJ73BmzGJeF0QLW/iihPnISJMkg=",
    });
    let response = client
        .post("/api/v1/device/admin")
        .json(&device)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let device = json!({"devices": [{
        "name": "device_8",
        "wireguard_ips": ["10.0.0.9"],
        "wireguard_pubkey": "A2cg4qMe+s0MSFlV6xyhz7XY6PrET6mli9GVSUshXAk=",
        "user_id": 1,
        "created": "2023-05-05T23:56:04"
    }]});
    let response = client
        .post("/api/v1/network/1/devices")
        .json(&device)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    // normal user cannot list devices of other users
    let response = client.get("/api/v1/device/user/admin").send().await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let response = client.get("/api/v1/device/user/hpotter").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let user_devices: Vec<Device<Id>> = response.json().await;
    assert_eq!(user_devices.len(), 3);

    // admin can list devices of other users
    let auth = Auth::new("admin", "pass123");
    let response = &client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    let response = client.get("/api/v1/device/user/admin").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let user_devices: Vec<Device<Id>> = response.json().await;
    assert_eq!(user_devices.len(), 2);

    let response = client.get("/api/v1/device/user/hpotter").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let user_devices: Vec<Device<Id>> = response.json().await;
    assert_eq!(user_devices.len(), 3);
}

#[sqlx::test]
async fn test_device_pubkey(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (client, client_state) = make_test_client(pool).await;

    let mut wg_rx = client_state.wireguard_rx;

    let auth = Auth::new("admin", "pass123");
    let response = &client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // create network
    make_network(&client, "network").await;
    let event = wg_rx.try_recv().unwrap();
    assert_matches!(event, GatewayEvent::NetworkCreated(..));

    // network details
    let response = client.get("/api/v1/network/1").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let network_from_details: WireguardNetwork<Id> = response.json().await;

    // create bad device
    let device = json!({
        "name": "device",
        "wireguard_pubkey": network_from_details.pubkey.clone(),
    });
    let response = client
        .post("/api/v1/device/admin")
        .json(&device)
        .send()
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
        .send()
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
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // list devices
    let response = client.get("/api/v1/device").json(&device).send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let devices: Vec<Device<Id>> = response.json().await;
    assert_eq!(devices.len(), 1);

    // modify device
    let mut device = devices[0].clone();
    device.wireguard_pubkey = network_from_details.pubkey;
    let response = client
        .put(format!("/api/v1/device/{}", device.id))
        .json(&device)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // try to create multiple devices
    let devices = json!({"devices": [{
        "name": "device_2",
        "wireguard_ips": ["10.0.0.9"],
        "wireguard_pubkey": "o/8q3kmv5nnbrcb/7aceQWGE44a0yI707wObXRyyWGU=",
        "user_id": 1,
        "created": "2023-05-05T23:56:04"
    },
    {
        "name": "device_3",
        "wireguard_ips": ["10.0.0.10"],
        "wireguard_pubkey": "invalid_key",
        "user_id": 1,
        "created": "2023-05-05T23:56:04"
    }]});
    let response = client
        .post("/api/v1/network/1/devices")
        .json(&devices)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // make sure no device was created
    let response = client.get("/api/v1/device").json(&device).send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let devices: Vec<Device<Id>> = response.json().await;
    assert_eq!(devices.len(), 1);
}

#[sqlx::test]
async fn test_network_size_validation(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (client, _client_state) = make_test_client(pool).await;

    let auth = Auth::new("admin", "pass123");
    let response = &client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // create network
    let response = make_network(&client, "network").await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // network details
    let response = client.get("/api/v1/network/1").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let network_from_details: WireguardNetwork<Id> = response.json().await;

    // create devices
    let device = json!({
        "name": "device1",
        "wireguard_pubkey": "LQKsT6/3HWKuJmMulH63R8iK+5sI8FyYEL6WDIi6lQU=",
    });
    let response = client
        .post("/api/v1/device/admin")
        .json(&device)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let device = json!({
        "name": "device2",
        "wireguard_pubkey": "ZqDlG4LQZRO9v57Sd27AHdtTLxegbMp5oVThjYrg21I=",
    });
    let response = client
        .post("/api/v1/device/admin")
        .json(&device)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let device = json!({
        "name": "device3",
        "wireguard_pubkey": "o/8q3kmv5nnbrcb/7aceQWGE44a0yI707wObXRyyWGU=",
    });
    let response = client
        .post("/api/v1/device/admin")
        .json(&device)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // try to add subnet with not enough IPs
    let network = json!({
        "id": network_from_details.id,
        "name": "network",
        "address": "10.1.1.1/24,10.2.1.1/30",
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
    let response = client
        .put(format!("/api/v1/network/{}", network_from_details.id))
        .json(&network)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // try to add subnet with invalid mask
    let network = json!({
        "id": network_from_details.id,
        "name": "network",
        "address": "10.2.0.1/24,10.1.1.1/0",
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
    let response = client
        .put(format!("/api/v1/network/{}", network_from_details.id))
        .json(&network)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // try to add no network
    let network = json!({
        "id": network_from_details.id,
        "name": "network",
        "address": "",
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
    let response = client
        .put(format!("/api/v1/network/{}", network_from_details.id))
        .json(&network)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
