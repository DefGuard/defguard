use std::time::Duration;

use defguard_common::{db::models::group::Group, types::proxy::ProxyControlMessage};
use defguard_core::{
    enterprise::{
        db::models::enterprise_settings::{
            ClientTrafficPolicy, EnterpriseSettings, EnterpriseSettingsInfo,
        },
        license::{get_cached_license, set_cached_license},
    },
    events::ApiEventType,
    handlers::Auth,
};
use reqwest::StatusCode;
use serde_json::json;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use tokio::time::sleep;

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
        display_download_step: true,
        display_password_reset: true,
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
            "allowed_ips_from_acl": false,
            "location_mfa_mode": "disabled",
            "service_location_mode": "disabled"
        }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // setup admin devices management
    let settings = EnterpriseSettings {
        admin_device_management: true,
        client_traffic_policy: ClientTrafficPolicy::None,
        only_client_activation: false,
        display_download_step: true,
        display_password_reset: true,
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
            "allowed_ips_from_acl": false,
            "location_mfa_mode": "disabled",
            "service_location_mode": "disabled"
        }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // setup admin devices management
    let settings = EnterpriseSettings {
        admin_device_management: false,
        client_traffic_policy: ClientTrafficPolicy::None,
        only_client_activation: false,
        display_download_step: true,
        display_password_reset: true,
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
            "allowed_ips_from_acl": false,
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
        display_download_step: true,
        display_password_reset: true,
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
    let created_device: serde_json::Value = response.json().await;
    let device_id = created_device["device"]["id"].as_i64().unwrap();
    let device_pubkey = created_device["device"]["wireguard_pubkey"]
        .as_str()
        .unwrap();

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

    // modify: renaming an existing device is still allowed
    let device = json!({
        "name": "modifieddevice",
        "wireguard_pubkey": device_pubkey,
    });
    let response = client
        .put(format!("/api/v1/device/{device_id}"))
        .json(&device)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    // modify: changing the pubkey is not allowed
    let device = json!({
        "name": "modifieddevice",
        "wireguard_pubkey": "AJwxGkzvVVn5Q1xjpCDFo5RJSU9KOPHeoEixYaj+20M=",
    });
    let response = client
        .put(format!("/api/v1/device/{device_id}"))
        .json(&device)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // modify: changing the description is not allowed
    let device = json!({
        "name": "modifieddevice",
        "wireguard_pubkey": device_pubkey,
        "description": "new description",
    });
    let response = client
        .put(format!("/api/v1/device/{device_id}"))
        .json(&device)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
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
            "allowed_ips_from_acl": false,
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
        display_download_step: true,
        display_password_reset: true,
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
async fn test_display_flags_round_trip(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    // admin login
    let (client, _) = make_test_client(pool).await;
    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    exceed_enterprise_limits(&client).await;

    // Set both display flags to false
    let settings = EnterpriseSettings {
        admin_device_management: false,
        client_traffic_policy: ClientTrafficPolicy::None,
        only_client_activation: false,
        display_download_step: false,
        display_password_reset: false,
    };
    let response = client
        .patch("/api/v1/settings_enterprise")
        .json(&settings)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    // Read back and verify the values persisted
    let response = client.get("/api/v1/settings_enterprise").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let body: EnterpriseSettingsInfo = response.json().await;
    assert!(
        !body.settings.display_download_step,
        "display_download_step should be false"
    );
    assert!(
        !body.settings.display_password_reset,
        "display_password_reset should be false"
    );

    // Set both back to true
    let settings = EnterpriseSettings {
        admin_device_management: false,
        client_traffic_policy: ClientTrafficPolicy::None,
        only_client_activation: false,
        display_download_step: true,
        display_password_reset: true,
    };
    let response = client
        .patch("/api/v1/settings_enterprise")
        .json(&settings)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    // Read back and verify
    let response = client.get("/api/v1/settings_enterprise").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let body: EnterpriseSettingsInfo = response.json().await;
    assert!(
        body.settings.display_download_step,
        "display_download_step should be true"
    );
    assert!(
        body.settings.display_password_reset,
        "display_password_reset should be true"
    );
}

#[sqlx::test]
async fn test_display_flags_default_to_true_without_license(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;

    // Unset the license
    let license = get_cached_license().clone();
    set_cached_license(None);

    // EnterpriseSettings::get() should return defaults when no license
    let settings = EnterpriseSettings::get(&pool).await.unwrap();
    assert!(
        settings.display_download_step,
        "display_download_step should default to true"
    );
    assert!(
        settings.display_password_reset,
        "display_password_reset should default to true"
    );

    // Restore license
    set_cached_license(license);
}

/// When a license was previously active and set flags to false in the DB,
/// removing the license must return defaults (both `true`) via `get()`,
/// ignoring the DB values.  Re-adding the license must restore the real DB values.
#[sqlx::test]
async fn test_display_flags_return_defaults_when_license_removed(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;

    // admin login and set Business license
    let (client, _) = make_test_client(pool.clone()).await;
    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    exceed_enterprise_limits(&client).await;

    // With a Business license active, save false values to the DB.
    let settings = json!({
        "display_download_step": false,
        "display_password_reset": false,
    });
    let response = client
        .patch("/api/v1/settings_enterprise")
        .json(&settings)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    // Remove the license - get() should return defaults, ignoring DB.
    let license = get_cached_license().clone();
    set_cached_license(None);

    let settings = EnterpriseSettings::get(&pool).await.unwrap();
    assert!(
        settings.display_download_step,
        "without license, display_download_step should default to true regardless of DB"
    );
    assert!(
        settings.display_password_reset,
        "without license, display_password_reset should default to true regardless of DB"
    );

    // Restore the license - get() should now return the real DB values.
    set_cached_license(license);

    let settings = EnterpriseSettings::get(&pool).await.unwrap();
    assert!(
        !settings.display_download_step,
        "with license restored, display_download_step should reflect DB (false)"
    );
    assert!(
        !settings.display_password_reset,
        "with license restored, display_password_reset should reflect DB (false)"
    );
}

/// When enterprise settings are patched with changed display flags, a
/// `BroadcastPublicSettings` control message must be sent via the proxy
/// control channel.
#[sqlx::test]
async fn test_public_settings_broadcast_on_save(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (mut client, client_state) = make_test_client(pool.clone()).await;
    let mut proxy_control_rx = client_state.proxy_control_rx;

    exceed_enterprise_limits(&client).await;
    // Clear events generated during setup (login, network creation).
    client.drain_all_events();

    // Patch enterprise settings with changed display flags.
    let settings = json!({
        "display_download_step": false,
        "display_password_reset": false,
    });
    let response = client
        .patch("/api/v1/settings_enterprise")
        .json(&settings)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    // Verify the audit event was emitted.
    let events = client.drain_all_events();
    assert!(
        events
            .iter()
            .any(|(event, _, _)| matches!(event, ApiEventType::EnterpriseSettingsUpdated { .. })),
        "EnterpriseSettingsUpdated event should be emitted on patch"
    );

    // The handler should have sent a BroadcastPublicSettings message.
    sleep(Duration::from_millis(100)).await;
    let mut found = false;
    loop {
        match proxy_control_rx.try_recv() {
            Ok(ProxyControlMessage::BroadcastPublicSettings {
                display_password_reset,
                display_download_step,
            }) => {
                assert!(
                    !display_password_reset,
                    "expected display_password_reset=false"
                );
                assert!(
                    !display_download_step,
                    "expected display_download_step=false"
                );
                found = true;
            }
            Ok(_) => {} // ignore other control messages
            Err(_) => break,
        }
    }
    assert!(found, "BroadcastPublicSettings was not sent");

    // Patch again with no change - should NOT broadcast.
    let settings = json!({
        "display_download_step": false,
        "display_password_reset": false,
    });
    let response = client
        .patch("/api/v1/settings_enterprise")
        .json(&settings)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    // Audit event should still be emitted even when flags didn't change.
    let events = client.drain_all_events();
    assert!(
        events
            .iter()
            .any(|(event, _, _)| matches!(event, ApiEventType::EnterpriseSettingsUpdated { .. })),
        "EnterpriseSettingsUpdated event should be emitted on every patch"
    );

    // No BroadcastPublicSettings should appear.
    sleep(Duration::from_millis(100)).await;
    while let Ok(msg) = proxy_control_rx.try_recv() {
        if matches!(msg, ProxyControlMessage::BroadcastPublicSettings { .. }) {
            panic!("BroadcastPublicSettings should not be sent when flags didn't change");
        }
    }
}

#[sqlx::test]
async fn test_group_client_traffic_policies_are_saved_and_validated(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let (client, _) = make_test_client(pool.clone()).await;
    let auth = Auth::new("admin", "pass123");
    assert_eq!(
        client
            .post("/api/v1/auth")
            .json(&auth)
            .send()
            .await
            .status(),
        StatusCode::OK
    );
    exceed_enterprise_limits(&client).await;

    let allow_choice = Group::new("allow-choice").save(&pool).await.unwrap();
    let disable = Group::new("disable").save(&pool).await.unwrap();

    let response = client
        .patch("/api/v1/settings_enterprise")
        .json(&json!({
            "client_traffic_policy": "force_all_traffic",
            "group_client_traffic_policies": {
                "none": [allow_choice.id],
                "disable_all_traffic": [disable.id],
                "force_all_traffic": []
            }
        }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    let response = client.get("/api/v1/settings_enterprise").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let settings: EnterpriseSettingsInfo = response.json().await;
    assert_eq!(
        settings.group_client_traffic_policies.none,
        vec![allow_choice.id]
    );
    assert_eq!(
        settings.group_client_traffic_policies.disable_all_traffic,
        vec![disable.id]
    );
    assert!(
        settings
            .group_client_traffic_policies
            .force_all_traffic
            .is_empty()
    );

    let license = get_cached_license().clone();
    set_cached_license(None);
    let response = client.get("/api/v1/settings_enterprise").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let settings: EnterpriseSettingsInfo = response.json().await;
    assert_eq!(
        settings.group_client_traffic_policies.none,
        vec![allow_choice.id]
    );
    assert_eq!(
        settings.group_client_traffic_policies.disable_all_traffic,
        vec![disable.id]
    );
    set_cached_license(license);

    let response = client
        .patch("/api/v1/settings_enterprise")
        .json(&json!({"display_download_step": false}))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    let response = client.get("/api/v1/settings_enterprise").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let settings: EnterpriseSettingsInfo = response.json().await;
    assert_eq!(
        settings.group_client_traffic_policies.none,
        vec![allow_choice.id]
    );
    assert_eq!(
        settings.group_client_traffic_policies.disable_all_traffic,
        vec![disable.id]
    );

    let response = client
        .patch("/api/v1/settings_enterprise")
        .json(&json!({
            "group_client_traffic_policies": {
                "none": [allow_choice.id],
                "disable_all_traffic": [allow_choice.id],
                "force_all_traffic": []
            }
        }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let response = client.get("/api/v1/settings_enterprise").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let settings: EnterpriseSettingsInfo = response.json().await;
    assert_eq!(
        settings.group_client_traffic_policies.none,
        vec![allow_choice.id]
    );
    assert_eq!(
        settings.group_client_traffic_policies.disable_all_traffic,
        vec![disable.id]
    );

    let response = client
        .patch("/api/v1/settings_enterprise")
        .json(&json!({
            "group_client_traffic_policies": {
                "none": [999999],
                "disable_all_traffic": [],
                "force_all_traffic": []
            }
        }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let response = client.get("/api/v1/settings_enterprise").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let settings: EnterpriseSettingsInfo = response.json().await;
    assert_eq!(
        settings.group_client_traffic_policies.none,
        vec![allow_choice.id]
    );
    assert_eq!(
        settings.group_client_traffic_policies.disable_all_traffic,
        vec![disable.id]
    );
}
