mod common;

use common::exceed_enterprise_limits;
use defguard::{
    enterprise::{
        db::models::enterprise_settings::EnterpriseSettings,
        license::{get_cached_license, set_cached_license},
    },
    handlers::Auth,
};
use reqwest::StatusCode;
use serde_json::json;

use self::common::{make_network, make_test_client};

#[tokio::test]
async fn test_only_enterprise_can_modify() {
    // admin login
    let (client, _client_state) = make_test_client().await;
    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    exceed_enterprise_limits(&client).await;

    // unset the license
    let license = get_cached_license().clone();
    set_cached_license(None);

    // try to patch enterprise settings
    let settings = EnterpriseSettings {
        admin_device_management: true,
        disable_all_traffic: false,
        only_client_activation: false,
    };

    let response = client
        .patch("/api/v1/settings_enterprise")
        .json(&settings)
        .send()
        .await;

    // server should say nono
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

#[tokio::test]
async fn test_admin_devices_management_is_enforced() {
    // admin login
    let (client, _) = make_test_client().await;
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
        disable_all_traffic: false,
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

#[tokio::test]
async fn test_regular_user_device_management() {
    // admin login
    let (client, _) = make_test_client().await;
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
        disable_all_traffic: false,
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
}
