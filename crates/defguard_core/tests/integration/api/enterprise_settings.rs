use defguard_core::{
    enterprise::{
        db::models::enterprise_settings::{ClientTrafficPolicy, EnterpriseSettings},
        license::{get_cached_license, set_cached_license},
    },
    handlers::Auth,
};
use reqwest::StatusCode;
use serde_json::{Value, json};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

use super::common::{exceed_enterprise_limits, make_test_client, setup_pool};

#[sqlx::test]
async fn test_only_enterprise_can_modify_enterpise_settings(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;

    // admin login
    let (client, _client_state) = make_test_client(pool).await;
    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    exceed_enterprise_limits(&client).await;

    // unset the license
    let license = get_cached_license().clone();
    set_cached_license(None);

    // try to patch enterprise settings
    let settings = EnterpriseSettings {
        admin_device_management: false,
        client_traffic_policy: ClientTrafficPolicy::None,
        only_client_activation: false,
    };

    let response = client
        .patch("/api/v1/settings_enterprise")
        .json(&settings)
        .send()
        .await;

    // server should say no
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    // restore valid license and try again
    set_cached_license(license);
    let response = client
        .patch("/api/v1/settings_enterprise")
        .json(&settings)
        .send()
        .await;

    // server should say ok
    assert_eq!(response.status(), StatusCode::OK);
}

#[sqlx::test]
async fn test_admin_devices_management_is_enforced(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    // admin login
    let (client, _) = make_test_client(pool).await;
    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    exceed_enterprise_limits(&client).await;

    // create network with access for all groups so the user device gets assigned config
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
            "allow_all_groups": true,
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
    let network: Value = response.json().await;
    let _network_id = network["id"].as_i64().unwrap();

    // setup admin devices management
    let settings = EnterpriseSettings {
        admin_device_management: true,
        client_traffic_policy: ClientTrafficPolicy::None,
        only_client_activation: false,
    };
    let response = client
        .patch("/api/v1/settings_enterprise")
        .json(&settings)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    // make sure admin can still manage devices
    let device = json!({
        "name": "device",
        "wireguard_pubkey": "LQKsT6/3HWKuJmMulH63R8iK+5sI8FyYEL6WDIi6lQU=",
    });
    let response = client
        .post("/api/v1/device/hpotter")
        .json(&device)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    let response = client
        .post("/api/v1/user/hpotter/start_desktop")
        .json(&json!({}))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // ensure normal users can't manage devices
    let auth = Auth::new("hpotter", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // add
    let device = json!({
        "name": "userdevice",
        "wireguard_pubkey": "AJwxGkzvVVn5Q1xjpCDFo5RJSU9KOPHeoEixYaj+20M=",
    });
    let response = client
        .post("/api/v1/device/hpotter")
        .json(&device)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    // modify
    let device = json!({
        "name": "modifieddevice",
        "wireguard_pubkey": "AJwxGkzvVVn5Q1xjpCDFo5RJSU9KOPHeoEixYaj+20M=",
    });
    let response = client.put("/api/v1/device/2").json(&device).send().await;

    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    // delete
    let device = json!({
        "name": "modifieddevice",
        "wireguard_pubkey": "AJwxGkzvVVn5Q1xjpCDFo5RJSU9KOPHeoEixYaj+20M=",
    });
    let response = client.put("/api/v1/device/2").json(&device).send().await;

    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    // start desktop enrollment
    let response = client
        .post("/api/v1/user/hpotter/start_desktop")
        .json(&json!({}))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[sqlx::test]
async fn test_regular_user_device_management(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    // admin login
    let (client, _) = make_test_client(pool).await;
    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    exceed_enterprise_limits(&client).await;

    // create network with access for all groups so the user device gets assigned config
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
            "allow_all_groups": true,
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
    let network: Value = response.json().await;
    let _network_id = network["id"].as_i64().unwrap();

    // setup admin devices management
    let settings = EnterpriseSettings {
        admin_device_management: false,
        client_traffic_policy: ClientTrafficPolicy::None,
        only_client_activation: false,
    };
    let response = client
        .patch("/api/v1/settings_enterprise")
        .json(&settings)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    // make sure admin can manage devices
    let device = json!({
        "name": "device",
        "wireguard_pubkey": "LQKsT6/3HWKuJmMulH63R8iK+5sI8FyYEL6WDIi6lQU=",
    });
    let response = client
        .post("/api/v1/device/hpotter")
        .json(&device)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // ensure normal users can manage devices
    let auth = Auth::new("hpotter", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // add
    let device = json!({
        "name": "userdevice",
        "wireguard_pubkey": "AJwxGkzvVVn5Q1xjpCDFo5RJSU9KOPHeoEixYaj+20M=",
    });
    let response = client
        .post("/api/v1/device/hpotter")
        .json(&device)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // modify
    let device = json!({
        "name": "modifieddevice",
        "wireguard_pubkey": "AJwxGkzvVVn5Q1xjpCDFo5RJSU9KOPHeoEixYaj+20M=",
    });
    let response = client.put("/api/v1/device/2").json(&device).send().await;

    assert_eq!(response.status(), StatusCode::OK);

    // delete
    let device = json!({
        "name": "modifieddevice",
        "wireguard_pubkey": "AJwxGkzvVVn5Q1xjpCDFo5RJSU9KOPHeoEixYaj+20M=",
    });
    let response = client.put("/api/v1/device/2").json(&device).send().await;

    assert_eq!(response.status(), StatusCode::OK);

    let response = client
        .post("/api/v1/user/hpotter/start_desktop")
        .json(&json!({}))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
}

#[sqlx::test]
async fn dg25_12_test_enforce_client_activation_only(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    // admin login
    let (client, _) = make_test_client(pool).await;
    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    exceed_enterprise_limits(&client).await;

    // create network with access for all groups so the user device gets assigned config
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
            "allow_all_groups": true,
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

    // disable manual device management
    let settings = EnterpriseSettings {
        admin_device_management: false,
        client_traffic_policy: ClientTrafficPolicy::None,
        only_client_activation: true,
    };
    let response = client
        .patch("/api/v1/settings_enterprise")
        .json(&settings)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    // make sure admin can manage devices
    let device = json!({
        "name": "device",
        "wireguard_pubkey": "LQKsT6/3HWKuJmMulH63R8iK+5sI8FyYEL6WDIi6lQU=",
    });
    let response = client
        .post("/api/v1/device/hpotter")
        .json(&device)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // ensure normal users can't manage devices
    let auth = Auth::new("hpotter", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // add
    let device = json!({
        "name": "userdevice",
        "wireguard_pubkey": "AJwxGkzvVVn5Q1xjpCDFo5RJSU9KOPHeoEixYaj+20M=",
    });
    let response = client
        .post("/api/v1/device/hpotter")
        .json(&device)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    // modify
    let device = json!({
        "name": "modifieddevice",
        "wireguard_pubkey": "AJwxGkzvVVn5Q1xjpCDFo5RJSU9KOPHeoEixYaj+20M=",
    });
    let response = client.put("/api/v1/device/2").json(&device).send().await;

    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    // delete
    let device = json!({
        "name": "modifieddevice",
        "wireguard_pubkey": "AJwxGkzvVVn5Q1xjpCDFo5RJSU9KOPHeoEixYaj+20M=",
    });
    let response = client.put("/api/v1/device/2").json(&device).send().await;

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[sqlx::test]
async fn dg25_13_test_disable_device_config(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    // admin login
    let (client, _) = make_test_client(pool).await;
    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    exceed_enterprise_limits(&client).await;

    // Allow all groups for network 1.
    // Payload based on make_network().
    let response = client
        .put("/api/v1/network/1")
        .json(&json!({
            "name": "network1",
            "address": "10.1.1.1/24",
            "port": 55555,
            "endpoint": "192.168.4.14",
            "allowed_ips": "10.1.1.0/24",
            "dns": "1.1.1.1",
            "mtu": 1420,
            "fwmark": 0,
            "allowed_groups": ["admin"],
            "allow_all_groups": true,
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

    // disable manual device management
    let settings = EnterpriseSettings {
        admin_device_management: false,
        client_traffic_policy: ClientTrafficPolicy::None,
        only_client_activation: true,
    };
    let response = client
        .patch("/api/v1/settings_enterprise")
        .json(&settings)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    // add device for normal user
    let device = json!({
        "name": "device",
        "wireguard_pubkey": "LQKsT6/3HWKuJmMulH63R8iK+5sI8FyYEL6WDIi6lQU=",
    });
    let response = client
        .post("/api/v1/device/hpotter")
        .json(&device)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // admin can view device config
    let response = client.get("/api/v1/network/1/device/1/config").send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // ensure normal users can't access device config
    let auth = Auth::new("hpotter", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    let response = client.get("/api/v1/network/1/device/1/config").send().await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}
