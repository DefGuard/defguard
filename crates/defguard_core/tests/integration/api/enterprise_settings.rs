use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use defguard_core::{
    enterprise::{
        db::models::enterprise_settings::{ClientTrafficPolicy, EnterpriseSettings},
        license::{get_cached_license, set_cached_license},
    },
    handlers::{Auth, wireguard::AddDeviceResult},
};
use ipnetwork::IpNetwork;
use reqwest::StatusCode;
use serde_json::json;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

use super::common::{exceed_enterprise_limits, make_network, make_test_client, setup_pool};

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

    // create network
    let response = client
        .post("/api/v1/network")
        .json(&make_network())
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);

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

    // create network
    let response = client
        .post("/api/v1/network")
        .json(&make_network())
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);

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

    // create network
    let response = client
        .post("/api/v1/network")
        .json(&make_network())
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

    // create network
    let response = client
        .post("/api/v1/network")
        .json(&make_network())
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

#[sqlx::test]
async fn test_override_allowed_ips(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    // admin login
    let (client, _) = make_test_client(pool).await;
    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // create network
    let response = client
        .post("/api/v1/network")
        .json(&make_network())
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // force all traffic setting for clients
    let settings = EnterpriseSettings {
        admin_device_management: false,
        client_traffic_policy: ClientTrafficPolicy::ForceAllTraffic,
        only_client_activation: false,
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

    let response: AddDeviceResult = response.json().await;

    for config in response.configs {
        assert_eq!(
            config.allowed_ips,
            vec![
                IpNetwork::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0)
                    .expect("Failed to parse UNSPECIFIED IPv4 constant"),
                IpNetwork::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), 0)
                    .expect("Failed to parse UNSPECIFIED IPv6 constant"),
            ]
        )
    }
}
