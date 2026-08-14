use std::net::{IpAddr, Ipv4Addr};

use chrono::NaiveDate;
use defguard_common::{
    db::{
        Id,
        models::{
            Device, DeviceType, MFAMethod, User, WebAuthn, WireguardNetwork,
            device::{AddDevice, WireguardNetworkDevice},
            gateway::Gateway,
            group::Group,
            oauth2client::OAuth2Client,
            settings::{Settings, update_current_settings},
            vpn_client_session::{VpnClientSession, VpnClientSessionState},
            vpn_session_stats::VpnSessionStats,
        },
    },
    testing::smtp::MockSmtpServer,
    types::user_info::UserInfo,
};
use defguard_core::{
    enterprise::{
        db::models::openid_provider::{
            DirectorySyncTarget, DirectorySyncUserBehavior, OpenIdProvider, OpenIdProviderKind,
        },
        license::{License, LicenseTier, SupportType, get_cached_license, set_cached_license},
        limits::update_counts,
    },
    events::ApiEventType,
    grpc::proto::enterprise::license::LicenseLimits,
    handlers::{
        AddUserData, Auth, PasswordChange, PasswordChangeSelf, Username,
        openid_clients::NewOpenIDClient,
    },
};
use reqwest::{StatusCode, header::USER_AGENT};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use tokio_stream::{self as stream, StreamExt};

use super::{
    TEST_SERVER_URL,
    common::{fetch_user_details, make_client, make_network, make_test_client, setup_pool},
};
use crate::api::common::{get_db_device, get_db_location, get_db_user, make_client_with_db};

async fn seed_user_with_mfa_artifacts(pool: &sqlx::PgPool, username: &str) -> Vec<String> {
    let test_user = get_db_user(pool, username).await;
    let recovery_codes = vec!["recovery-code-1".to_owned(), "recovery-code-2".to_owned()];

    sqlx::query(
        "UPDATE \"user\" SET mfa_enabled = TRUE, totp_enabled = TRUE, email_mfa_enabled = TRUE, \
        totp_secret = $2, email_mfa_secret = $3, mfa_method = 'one_time_password', recovery_codes = $4 WHERE id = $1",
    )
    .bind(test_user.id)
    .bind(vec![1_u8, 2, 3])
    .bind(vec![4_u8, 5, 6])
    .bind(recovery_codes.clone())
    .execute(pool)
    .await
    .unwrap();

    sqlx::query("INSERT INTO webauthn (user_id, name, passkey) VALUES ($1, $2, $3)")
        .bind(test_user.id)
        .bind("Test passkey")
        .bind(vec![7_u8, 8, 9])
        .execute(pool)
        .await
        .unwrap();

    recovery_codes
}

#[sqlx::test]
async fn test_authenticate(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let mut client = make_client(pool).await;

    let auth = Auth::new("hpotter", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    let auth = Auth::new("hpotter", "-wrong-");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let auth = Auth::new("adumbledore", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // second user does not exist so we are unable to emit audit log event
    client.verify_api_events_with_user(&[
        (ApiEventType::UserLogin, 2, "hpotter"),
        (
            ApiEventType::UserLoginFailed {
                message: "Authentication for hpotter failed: invalid password".into(),
            },
            2,
            "hpotter",
        ),
    ]);
}

#[sqlx::test]
async fn test_me(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let mut client = make_client(pool).await;

    client.login_user("hpotter", "pass123").await;

    let response = client.get("/api/v1/me").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let user_info: UserInfo = response.json().await;
    assert_eq!(user_info.first_name, "Harry");
    assert_eq!(user_info.last_name, "Potter");

    client.assert_event_queue_is_empty();
}

#[sqlx::test]
async fn test_change_self_password(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let mut client = make_client(pool).await;

    client.login_user("hpotter", "pass123").await;

    let bad_old = "notCurrentPassword123!$";

    let new_password = "strongPassword123$!1";

    let bad_old_request = PasswordChangeSelf {
        old_password: bad_old.into(),
        new_password: new_password.into(),
    };

    let bad_new_request = PasswordChangeSelf {
        old_password: "pass123".into(),
        new_password: "badnew".into(),
    };

    let change_password = PasswordChangeSelf {
        old_password: "pass123".into(),
        new_password: new_password.into(),
    };

    let response = client
        .put("/api/v1/user/change_password")
        .json(&bad_old_request)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let response = client
        .put("/api/v1/user/change_password")
        .json(&bad_new_request)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let response = client
        .put("/api/v1/user/change_password")
        .json(&change_password)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    // old pass login
    let auth = Auth::new("hpotter", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let new_auth = Auth::new("hpotter", new_password);

    let response = client.post("/api/v1/auth").json(&new_auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    client.verify_api_events_with_user(&[
        (ApiEventType::PasswordChanged, 2, "hpotter"),
        (
            ApiEventType::UserLoginFailed {
                message: "Authentication for hpotter failed: invalid password".into(),
            },
            2,
            "hpotter",
        ),
        (ApiEventType::UserLogin, 2, "hpotter"),
    ]);
}

#[sqlx::test]
async fn test_change_password(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (mut client, pool) = make_client_with_db(pool).await;

    client.login_user("admin", "pass123").await;

    let new_password = "newPassword43$!";

    let change_others_password = PasswordChange {
        new_password: new_password.into(),
    };

    let response = client
        .put("/api/v1/user/admin/password")
        .json(&change_others_password)
        .send()
        .await;

    // can't change own password with this endpoint
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // can change others password

    let response = client
        .put("/api/v1/user/hpotter/password")
        .json(&change_others_password)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);

    let auth = Auth::new("hpotter", new_password);
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // route is only for admins
    let response = client
        .put("/api/v1/user/admin/password")
        .json(&change_others_password)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let test_user = get_db_user(&pool, "hpotter").await;

    client.verify_api_events_with_user(&[
        (
            ApiEventType::PasswordChangedByAdmin { user: test_user },
            1,
            "admin",
        ),
        (ApiEventType::UserLogin, 2, "hpotter"),
    ]);
}

#[sqlx::test]
async fn test_list_users(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let mut client = make_client(pool).await;

    let response = client.get("/api/v1/user").send().await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // normal user cannot list users
    client.login_user("hpotter", "pass123").await;

    let response = client.get("/api/v1/user").send().await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    // admin can list users
    client.login_user("admin", "pass123").await;

    let response = client.get("/api/v1/user").send().await;
    assert_eq!(response.status(), StatusCode::OK);

    client.assert_event_queue_is_empty();
}

#[sqlx::test]
async fn test_list_users_group_filter(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (mut client, pool) = make_client_with_db(pool).await;

    // Create an "engineering" group and assign hpotter to it.
    // admin is already in the "admin" group via init_admin_user.
    let engineering = Group::new("engineering").save(&pool).await.unwrap();
    let hpotter = get_db_user(&pool, "hpotter").await;
    sqlx::query("INSERT INTO group_user (group_id, user_id) VALUES ($1, $2)")
        .bind(engineering.id)
        .bind(hpotter.id)
        .execute(&pool)
        .await
        .unwrap();

    // Admin login
    client.login_user("admin", "pass123").await;

    // Filter by admin group - should only return admin
    let response = client.get("/api/v1/user?groups=admin").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await;
    let usernames: Vec<&str> = body["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|u| u["username"].as_str().unwrap())
        .collect();
    assert_eq!(usernames, vec!["admin"]);
    assert_eq!(body["pagination"]["total_items"].as_u64().unwrap(), 1);

    // Filter by engineering group - should only return hpotter
    let response = client.get("/api/v1/user?groups=engineering").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await;
    let usernames: Vec<&str> = body["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|u| u["username"].as_str().unwrap())
        .collect();
    assert_eq!(usernames, vec!["hpotter"]);
    assert_eq!(body["pagination"]["total_items"].as_u64().unwrap(), 1);

    // Multiple groups (OR semantics) - should return both users
    let response = client
        .get("/api/v1/user?groups=admin&groups=engineering")
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await;
    let mut usernames: Vec<&str> = body["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|u| u["username"].as_str().unwrap())
        .collect();
    usernames.sort_unstable();
    assert_eq!(usernames, vec!["admin", "hpotter"]);
    assert_eq!(body["pagination"]["total_items"].as_u64().unwrap(), 2);

    // Nonexistent group - should return empty results
    let response = client.get("/api/v1/user?groups=nonexistent").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await;
    assert_eq!(body["data"].as_array().unwrap().len(), 0);
    assert_eq!(body["pagination"]["total_items"].as_u64().unwrap(), 0);

    // Unauthorized access
    client.login_user("hpotter", "pass123").await;
    let response = client.get("/api/v1/user?groups=admin").send().await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    client.assert_event_queue_is_empty();
}

#[sqlx::test]
async fn test_list_users_no_group_filter(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let mut client = make_client(pool).await;

    // Admin login
    client.login_user("admin", "pass123").await;

    // no_group=true should return only hpotter (the only ungrouped user)
    let response = client.get("/api/v1/user?no_group=true").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await;
    let usernames: Vec<&str> = body["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|u| u["username"].as_str().unwrap())
        .collect();
    assert_eq!(usernames, vec!["hpotter"]);
    assert_eq!(body["pagination"]["total_items"].as_u64().unwrap(), 1);

    // no_group=false with groups filter - should work normally
    let response = client
        .get("/api/v1/user?no_group=false&groups=admin")
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await;
    let usernames: Vec<&str> = body["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|u| u["username"].as_str().unwrap())
        .collect();
    assert_eq!(usernames, vec!["admin"]);

    // Combined: no_group=true with groups=admin - returns union (ungrouped + in group)
    let response = client
        .get("/api/v1/user?no_group=true&groups=admin")
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await;
    let mut usernames: Vec<&str> = body["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|u| u["username"].as_str().unwrap())
        .collect();
    usernames.sort_unstable();
    assert_eq!(usernames, vec!["admin", "hpotter"]);
    assert_eq!(body["pagination"]["total_items"].as_u64().unwrap(), 2);

    // Unauthorized access
    client.login_user("hpotter", "pass123").await;
    let response = client.get("/api/v1/user?no_group=true").send().await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    client.assert_event_queue_is_empty();
}

#[sqlx::test]
async fn test_list_users_no_group_multi_group_filter(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (mut client, pool) = make_client_with_db(pool).await;

    // Create a "qa" group and a new user rweasley assigned to it.
    // Existing state: admin is in "admin" group, hpotter is ungrouped.
    let qa = Group::new("qa").save(&pool).await.unwrap();
    let rweasley = User::new(
        "rweasley",
        Some("pass123"),
        "Weasley",
        "Ron",
        "r.weasley@hogwart.edu.uk",
        None,
    )
    .save(&pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO group_user (group_id, user_id) VALUES ($1, $2)")
        .bind(qa.id)
        .bind(rweasley.id)
        .execute(&pool)
        .await
        .unwrap();

    // Admin login
    client.login_user("admin", "pass123").await;

    // Combined no_group + multiple groups: should return users in admin, users in qa,
    // and users with no group (hpotter).
    let response = client
        .get("/api/v1/user?no_group=true&groups=admin&groups=qa")
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await;
    let mut usernames: Vec<&str> = body["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|u| u["username"].as_str().unwrap())
        .collect();
    usernames.sort_unstable();
    assert_eq!(usernames, vec!["admin", "hpotter", "rweasley"]);
    assert_eq!(body["pagination"]["total_items"].as_u64().unwrap(), 3);

    client.assert_event_queue_is_empty();
}

#[sqlx::test]
async fn test_list_users_search(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let mut client = make_client(pool).await;
    client.login_user("admin", "pass123").await;

    // Search by username
    let response = client.get("/api/v1/user?search=admin").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await;
    let usernames: Vec<&str> = body["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|u| u["username"].as_str().unwrap())
        .collect();
    assert_eq!(usernames, vec!["admin"]);

    // Search by first name
    let response = client.get("/api/v1/user?search=Harry").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await;
    let usernames: Vec<&str> = body["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|u| u["username"].as_str().unwrap())
        .collect();
    assert_eq!(usernames, vec!["hpotter"]);

    // Search by last name
    let response = client.get("/api/v1/user?search=Potter").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await;
    let usernames: Vec<&str> = body["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|u| u["username"].as_str().unwrap())
        .collect();
    assert_eq!(usernames, vec!["hpotter"]);

    // Search by email
    let response = client.get("/api/v1/user?search=h.potter").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await;
    let usernames: Vec<&str> = body["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|u| u["username"].as_str().unwrap())
        .collect();
    assert_eq!(usernames, vec!["hpotter"]);

    // Search by non-existent term
    let response = client.get("/api/v1/user?search=nonexistent").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await;
    assert_eq!(body["data"].as_array().unwrap().len(), 0);
    assert_eq!(body["pagination"]["total_items"].as_u64().unwrap(), 0);

    client.assert_event_queue_is_empty();
}

#[sqlx::test]
async fn test_list_users_sort(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let mut client = make_client(pool).await;
    client.login_user("admin", "pass123").await;

    // Sort by username ascending
    let response = client
        .get("/api/v1/user?sort_by=username&sort_order=asc")
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await;
    let usernames: Vec<&str> = body["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|u| u["username"].as_str().unwrap())
        .collect();
    assert_eq!(usernames, vec!["admin", "hpotter"]);

    // Sort by username descending
    let response = client
        .get("/api/v1/user?sort_by=username&sort_order=desc")
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await;
    let usernames: Vec<&str> = body["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|u| u["username"].as_str().unwrap())
        .collect();
    assert_eq!(usernames, vec!["hpotter", "admin"]);

    // Sort by name ascending (first_name, last_name)
    let response = client
        .get("/api/v1/user?sort_by=name&sort_order=asc")
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await;
    let usernames: Vec<&str> = body["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|u| u["username"].as_str().unwrap())
        .collect();
    // DefGuard < Harry alphabetically
    assert_eq!(usernames, vec!["admin", "hpotter"]);

    // Sort by name descending (first_name, last_name)
    let response = client
        .get("/api/v1/user?sort_by=name&sort_order=desc")
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await;
    let usernames: Vec<&str> = body["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|u| u["username"].as_str().unwrap())
        .collect();
    // Harry > DefGuard alphabetically
    assert_eq!(usernames, vec!["hpotter", "admin"]);

    // Sort by email descending
    let response = client
        .get("/api/v1/user?sort_by=email&sort_order=desc")
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await;
    let usernames: Vec<&str> = body["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|u| u["username"].as_str().unwrap())
        .collect();
    // h.potter@hogwart.edu.uk > admin@defguard alphabetically
    assert_eq!(usernames, vec!["hpotter", "admin"]);

    // Default sort (no params) should still work
    let response = client.get("/api/v1/user").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await;
    assert_eq!(body["data"].as_array().unwrap().len(), 2);

    client.assert_event_queue_is_empty();
}

#[sqlx::test]
async fn test_list_users_search_with_group_filter(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let mut client = make_client(pool).await;
    client.login_user("admin", "pass123").await;

    // Search "admin" within admin group only - should return admin
    let response = client
        .get("/api/v1/user?search=admin&groups=admin")
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await;
    let usernames: Vec<&str> = body["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|u| u["username"].as_str().unwrap())
        .collect();
    assert_eq!(usernames, vec!["admin"]);
    assert_eq!(body["pagination"]["total_items"].as_u64().unwrap(), 1);

    // Search "Potter" within admin group only - admin is not Potter, should be empty
    let response = client
        .get("/api/v1/user?search=Potter&groups=admin")
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await;
    assert_eq!(body["data"].as_array().unwrap().len(), 0);
    assert_eq!(body["pagination"]["total_items"].as_u64().unwrap(), 0);

    client.assert_event_queue_is_empty();
}

#[sqlx::test]
async fn test_get_user(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let mut client = make_client(pool).await;

    let response = client.get("/api/v1/user/hpotter").send().await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    client.login_user("hpotter", "pass123").await;

    let user_info = fetch_user_details(&client, "hpotter").await;
    assert_eq!(user_info.user.first_name, "Harry");
    assert_eq!(user_info.user.last_name, "Potter");

    client.assert_event_queue_is_empty();
}

#[sqlx::test]
async fn test_get_user_exposes_active_network_state(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (mut client, pool) = make_client_with_db(pool).await;
    client.login_user("admin", "pass123").await;

    let username = "active-user";
    let device_name = "active-device";
    let device_wireguard_ip = IpAddr::V4(Ipv4Addr::new(10, 1, 1, 2));

    let user = User::new(
        username,
        Some("pass123"),
        "Active",
        "User",
        "active.user@example.com",
        None,
    )
    .save(&pool)
    .await
    .unwrap();

    let network_response = make_network(&client, "active-network").await;
    let network: WireguardNetwork<Id> = network_response.json().await;

    let device = Device::new(
        device_name.into(),
        "key".into(),
        user.id,
        DeviceType::User,
        None,
        true,
    )
    .save(&pool)
    .await
    .unwrap();

    WireguardNetworkDevice::new(network.id, device.id, [device_wireguard_ip])
        .insert(&pool)
        .await
        .unwrap();

    let session_connected_at = NaiveDate::from_ymd_opt(2026, 1, 2)
        .expect("expected valid connected_at date")
        .and_hms_opt(3, 4, 5)
        .expect("expected valid connected_at time");

    VpnClientSession::new(
        network.id,
        user.id,
        device.id,
        Some(session_connected_at),
        None,
    )
    .save(&pool)
    .await
    .unwrap();

    let user_details = fetch_user_details(&client, username).await;

    assert_eq!(user_details.user.username, username);
    assert_eq!(user_details.user.devices.len(), 1);

    let user_device = user_details
        .user
        .devices
        .iter()
        .find(|user_device| user_device.device.id == device.id)
        .expect("expected created device in user details response");
    assert_eq!(user_device.device.name, device_name);
    assert_eq!(user_device.networks.len(), 1);

    let network_info = user_device
        .networks
        .iter()
        .find(|network_info| network_info.network_id == network.id)
        .expect("expected created network in user details response");
    assert_eq!(network_info.network_name, "active-network");
    assert_eq!(network_info.network_gateway_ip, "192.168.4.14");
    assert_eq!(
        network_info.device_wireguard_ips,
        vec![device_wireguard_ip.to_string()]
    );
    assert!(network_info.is_active);
    assert_eq!(network_info.last_connected_at, Some(session_connected_at));
}

#[sqlx::test]
async fn test_get_user_keeps_last_successful_connection_for_newer_disconnected_session(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;

    let (mut client, pool) = make_client_with_db(pool).await;
    client.login_user("admin", "pass123").await;

    let username = "inactive-user";
    let device_name = "inactive-device";
    let device_wireguard_ip = IpAddr::V4(Ipv4Addr::new(10, 1, 1, 2));

    let user = User::new(
        username,
        Some("pass123"),
        "Inactive",
        "User",
        "inactive.user@example.com",
        None,
    )
    .save(&pool)
    .await
    .unwrap();

    let network_response = make_network(&client, "inactive-network").await;
    let network: WireguardNetwork<Id> = network_response.json().await;

    let device = Device::new(
        device_name.into(),
        "key".into(),
        user.id,
        DeviceType::User,
        None,
        true,
    )
    .save(&pool)
    .await
    .unwrap();

    WireguardNetworkDevice::new(network.id, device.id, [device_wireguard_ip])
        .insert(&pool)
        .await
        .unwrap();

    let gateway = Gateway::new(network.id, "gateway", "198.51.100.1", 51820, "tester")
        .save(&pool)
        .await
        .unwrap();

    let last_successful_connection = NaiveDate::from_ymd_opt(2026, 1, 2)
        .expect("expected valid connected_at date")
        .and_hms_opt(3, 4, 5)
        .expect("expected valid connected_at time");
    let last_successful_stats_at = NaiveDate::from_ymd_opt(2026, 1, 2)
        .expect("expected valid collected_at date")
        .and_hms_opt(3, 5, 6)
        .expect("expected valid collected_at time");
    let disconnected_at = NaiveDate::from_ymd_opt(2026, 1, 3)
        .expect("expected valid disconnected date")
        .and_hms_opt(4, 5, 6)
        .expect("expected valid disconnected time");
    let disconnected_stats_at = NaiveDate::from_ymd_opt(2026, 1, 3)
        .expect("expected valid collected_at date")
        .and_hms_opt(4, 6, 7)
        .expect("expected valid collected_at time");

    let mut connected_session = VpnClientSession::new(
        network.id,
        user.id,
        device.id,
        Some(last_successful_connection),
        None,
    );
    connected_session.created_at = last_successful_connection;
    let connected_session = connected_session.save(&pool).await.unwrap();

    VpnSessionStats::new(
        connected_session.id,
        gateway.id,
        last_successful_stats_at,
        last_successful_stats_at,
        "203.0.113.10:51820".into(),
        1,
        1,
        1,
        1,
    )
    .save(&pool)
    .await
    .unwrap();

    let mut disconnected_session =
        VpnClientSession::new(network.id, user.id, device.id, None, None);
    disconnected_session.created_at = disconnected_at;
    disconnected_session.disconnected_at = Some(disconnected_at);
    disconnected_session.state = VpnClientSessionState::Disconnected;
    let disconnected_session = disconnected_session.save(&pool).await.unwrap();

    VpnSessionStats::new(
        disconnected_session.id,
        gateway.id,
        disconnected_stats_at,
        disconnected_stats_at,
        "198.51.100.99:51820".into(),
        2,
        2,
        2,
        2,
    )
    .save(&pool)
    .await
    .unwrap();

    let user_details = fetch_user_details(&client, username).await;

    let user_device = user_details
        .user
        .devices
        .iter()
        .find(|user_device| user_device.device.id == device.id)
        .expect("expected created device in user details response");
    let network_info = user_device
        .networks
        .iter()
        .find(|network_info| network_info.network_id == network.id)
        .expect("expected created network in user details response");

    assert!(network_info.is_active);
    assert_eq!(
        network_info.last_connected_at,
        Some(last_successful_connection)
    );
    assert_eq!(network_info.last_connected_ip, Some("203.0.113.10".into()));
}

#[sqlx::test]
async fn test_username_available(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let mut client = make_client(pool).await;

    // standard user cannot check username availability
    client.login_user("hpotter", "pass123").await;

    let avail = Username {
        username: "hpotter".into(),
    };
    let response = client
        .post("/api/v1/user/available")
        .json(&avail)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    // log in as admin
    client.login_user("admin", "pass123").await;

    let avail = Username {
        username: "_CrashTestDummy".into(),
    };
    let response = client
        .post("/api/v1/user/available")
        .json(&avail)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let avail = Username {
        username: "crashtestdummy42".into(),
    };
    let response = client
        .post("/api/v1/user/available")
        .json(&avail)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    let avail = Username {
        username: "hpotter".into(),
    };
    let response = client
        .post("/api/v1/user/available")
        .json(&avail)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    client.assert_event_queue_is_empty();
}

#[sqlx::test]
async fn test_crud_user(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (mut client, pool) = make_client_with_db(pool).await;

    client.login_user("admin", "pass123").await;

    // create user
    let new_user = AddUserData {
        username: "adumbledore".into(),
        last_name: "Dumbledore".into(),
        first_name: "Albus".into(),
        email: "a.dumbledore@hogwart.edu.uk".into(),
        phone: Some("1234".into()),
        password: Some("Password1234543$!".into()),
    };
    let response = client.post("/api/v1/user").json(&new_user).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // get user
    let mut user_details = fetch_user_details(&client, "adumbledore").await;
    assert_eq!(user_details.user.first_name, "Albus");

    let old_test_user = get_db_user(&pool, "adumbledore").await;

    // edit user
    user_details.user.phone = Some("5678".into());
    let response = client
        .put("/api/v1/user/adumbledore")
        .json(&user_details.user)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    let new_test_user = get_db_user(&pool, "adumbledore").await;

    // delete user
    let response = client.delete("/api/v1/user/adumbledore").send().await;
    assert_eq!(response.status(), StatusCode::OK);

    client.verify_api_events(&[
        ApiEventType::UserAdded {
            user: old_test_user.clone(),
        },
        ApiEventType::UserModified {
            before: old_test_user,
            after: new_test_user.clone(),
        },
        ApiEventType::UserRemoved {
            user: new_test_user,
        },
    ]);
}

#[sqlx::test]
async fn test_add_user_blocked_when_user_count_exceeds_license_limit(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;

    let (mut client, pool) = make_client_with_db(pool).await;

    client.login_user("admin", "pass123").await;
    update_counts(&pool).await.unwrap();

    let license = get_cached_license().clone();
    set_cached_license(Some(License::new(
        "test_customer".to_owned(),
        false,
        None,
        Some(LicenseLimits {
            users: 1,
            devices: 100,
            locations: 100,
            network_devices: Some(100),
        }),
        None,
        LicenseTier::Business,
        SupportType::Basic,
        vec![],
    )));

    let new_user = AddUserData {
        username: "adumbledore".into(),
        last_name: "Dumbledore".into(),
        first_name: "Albus".into(),
        email: "a.dumbledore@hogwart.edu.uk".into(),
        phone: Some("1234".into()),
        password: Some("Password1234543$!".into()),
    };
    let response = client.post("/api/v1/user").json(&new_user).send().await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    set_cached_license(license);
    client.assert_event_queue_is_empty();
}

#[sqlx::test]
async fn test_disabled_users_not_counted_towards_license_limit(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;

    let (mut client, pool) = make_client_with_db(pool).await;

    client.login_user("admin", "pass123").await;

    // baseline is admin + hpotter, both active
    let hpotter = get_db_user(&pool, "hpotter").await;
    let response = client
        .post("/api/v1/user/bulk-disable")
        .json(&serde_json::json!({ "users": [hpotter.id] }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    let license = get_cached_license().clone();
    set_cached_license(Some(License::new(
        "test_customer".to_owned(),
        false,
        None,
        Some(LicenseLimits {
            users: 2,
            devices: 100,
            locations: 100,
            network_devices: Some(100),
        }),
        None,
        LicenseTier::Business,
        SupportType::Basic,
        vec![],
    )));

    // only admin is active, so there is still room under the limit of 2
    let new_user = AddUserData {
        username: "adumbledore".into(),
        last_name: "Dumbledore".into(),
        first_name: "Albus".into(),
        email: "a.dumbledore@hogwart.edu.uk".into(),
        phone: Some("1234".into()),
        password: Some("Password1234543$!".into()),
    };
    let response = client.post("/api/v1/user").json(&new_user).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);

    set_cached_license(license);
}

#[sqlx::test]
async fn test_modify_user_enable_blocked_when_it_would_exceed_license_limit(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;

    let (mut client, pool) = make_client_with_db(pool).await;

    client.login_user("admin", "pass123").await;

    let new_user = AddUserData {
        username: "adumbledore".into(),
        last_name: "Dumbledore".into(),
        first_name: "Albus".into(),
        email: "a.dumbledore@hogwart.edu.uk".into(),
        phone: Some("1234".into()),
        password: Some("Password1234543$!".into()),
    };
    let response = client.post("/api/v1/user").json(&new_user).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let added_dumbledore = get_db_user(&pool, "adumbledore").await;

    // disable the new user so there is something to re-enable; active count drops back to 2
    let response = client
        .post("/api/v1/user/bulk-disable")
        .json(&serde_json::json!({ "users": [added_dumbledore.id] }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let disabled_dumbledore = get_db_user(&pool, "adumbledore").await;
    assert!(!disabled_dumbledore.is_active);

    let license = get_cached_license().clone();
    set_cached_license(Some(License::new(
        "test_customer".to_owned(),
        false,
        None,
        Some(LicenseLimits {
            users: 2,
            devices: 100,
            locations: 100,
            network_devices: Some(100),
        }),
        None,
        LicenseTier::Business,
        SupportType::Basic,
        vec![],
    )));

    // active count is already at the limit of 2 (admin, hpotter), so re-enabling must be blocked
    let mut user_details = fetch_user_details(&client, "adumbledore").await;
    user_details.user.is_active = true;
    let response = client
        .put("/api/v1/user/adumbledore")
        .json(&user_details.user)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let still_disabled_dumbledore = get_db_user(&pool, "adumbledore").await;
    assert!(!still_disabled_dumbledore.is_active);

    set_cached_license(license);
    client.verify_api_events(&[
        ApiEventType::UserAdded {
            user: added_dumbledore.clone(),
        },
        ApiEventType::UserModified {
            before: added_dumbledore,
            after: disabled_dumbledore,
        },
    ]);
}

#[sqlx::test]
async fn test_bulk_enable_users_blocked_when_it_would_exceed_license_limit(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;

    let (mut client, pool) = make_client_with_db(pool).await;

    client.login_user("admin", "pass123").await;

    for (username, email) in [
        ("adumbledore", "a.dumbledore@hogwart.edu.uk"),
        ("mmcgonagall", "m.mcgonagall@hogwart.edu.uk"),
    ] {
        let new_user = AddUserData {
            username: username.into(),
            last_name: format!("{username}-last"),
            first_name: format!("{username}-first"),
            email: email.into(),
            phone: Some("1234".into()),
            password: Some("Password1234543$!".into()),
        };
        let response = client.post("/api/v1/user").json(&new_user).send().await;
        assert_eq!(response.status(), StatusCode::CREATED);
    }

    let added_dumbledore = get_db_user(&pool, "adumbledore").await;
    let added_mcgonagall = get_db_user(&pool, "mmcgonagall").await;

    // disable both new users; active count drops back to 2 (admin, hpotter)
    let response = client
        .post("/api/v1/user/bulk-disable")
        .json(&serde_json::json!({ "users": [added_dumbledore.id, added_mcgonagall.id] }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let disabled_dumbledore = get_db_user(&pool, "adumbledore").await;
    let disabled_mcgonagall = get_db_user(&pool, "mmcgonagall").await;

    let license = get_cached_license().clone();
    set_cached_license(Some(License::new(
        "test_customer".to_owned(),
        false,
        None,
        Some(LicenseLimits {
            users: 3,
            devices: 100,
            locations: 100,
            network_devices: Some(100),
        }),
        None,
        LicenseTier::Business,
        SupportType::Basic,
        vec![],
    )));

    // active count is 2 (admin, hpotter); re-enabling both would bring it to 4, over the limit of 3
    let response = client
        .post("/api/v1/user/bulk-enable")
        .json(&serde_json::json!({ "users": [disabled_dumbledore.id, disabled_mcgonagall.id] }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let still_disabled_dumbledore = get_db_user(&pool, "adumbledore").await;
    let still_disabled_mcgonagall = get_db_user(&pool, "mmcgonagall").await;
    assert!(!still_disabled_dumbledore.is_active);
    assert!(!still_disabled_mcgonagall.is_active);

    set_cached_license(license);
    client.verify_api_events(&[
        ApiEventType::UserAdded {
            user: added_dumbledore.clone(),
        },
        ApiEventType::UserAdded {
            user: added_mcgonagall.clone(),
        },
        ApiEventType::UserModified {
            before: added_dumbledore,
            after: disabled_dumbledore,
        },
        ApiEventType::UserModified {
            before: added_mcgonagall,
            after: disabled_mcgonagall,
        },
    ]);
}

#[sqlx::test]
async fn test_check_username(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (mut client, pool) = make_client_with_db(pool).await;

    client.login_user("admin", "pass123").await;

    let invalid_usernames = ["ADumble dore", ".1user"];
    let valid_usernames = ["user1", "use2r3", "not_wrong"];

    for (i, username) in invalid_usernames.into_iter().enumerate() {
        let new_user = AddUserData {
            username: username.into(),
            last_name: "Dumbledore".into(),
            first_name: "Albus".into(),
            email: format!("a.dumbledore{i}@hogwart.edu.uk"),
            phone: Some("1234".into()),
            password: Some("Alohomora!12".into()),
        };
        let response = client.post("/api/v1/user").json(&new_user).send().await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    let mut expected_events = Vec::new();
    for (i, username) in valid_usernames.into_iter().enumerate() {
        let new_user = AddUserData {
            username: username.into(),
            last_name: "Dumbledore".into(),
            first_name: "Albus".into(),
            email: format!("a.dumbledore{i}@hogwart.edu.uk"),
            phone: Some("1234".into()),
            password: Some("Alohomora!12".into()),
        };
        let response = client.post("/api/v1/user").json(&new_user).send().await;
        assert_eq!(response.status(), StatusCode::CREATED);

        let test_user = get_db_user(&pool, username).await;
        expected_events.push(ApiEventType::UserAdded { user: test_user });
    }

    client.verify_api_events(&expected_events);
}

#[sqlx::test]
async fn test_check_password_strength(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (mut client, pool) = make_client_with_db(pool).await;

    // auth session with admin
    client.login_user("admin", "pass123").await;

    // test
    let strong_password = "strongPass1234$!";
    let too_short = "1H$";
    let no_upper = "notsostrong1!";
    let no_numbers = "notSostrong!";
    let no_specials = "noSoStrong1234";
    let weak_passwords = [too_short, no_upper, no_specials, no_numbers];
    let mut stream = stream::iter(weak_passwords.iter().enumerate());
    while let Some((index, password)) = stream.next().await {
        let weak_password_user = AddUserData {
            username: format!("weakpass{index}"),
            first_name: "testpassfn".into(),
            last_name: "testpassln".into(),
            email: format!("testpass{index}@test.test"),
            password: Some(password.to_owned().into()),
            phone: None,
        };
        let response = client
            .post("/api/v1/user")
            .json(&weak_password_user)
            .send()
            .await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
    let strong_password_user = AddUserData {
        username: "strongpass".into(),
        first_name: "Strong".into(),
        last_name: "Pass".into(),
        email: "strongpass@test.test".into(),
        phone: None,
        password: Some(strong_password.into()),
    };
    let response = client
        .post("/api/v1/user")
        .json(&strong_password_user)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    let test_user = get_db_user(&pool, "strongpass").await;

    client.verify_api_events(&[ApiEventType::UserAdded { user: test_user }]);
}

#[sqlx::test]
async fn test_user_unregister_authorized_app(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (mut client, pool) = make_client_with_db(pool).await;
    client.login_user("admin", "pass123").await;

    // add OpenID app
    let openid_client = NewOpenIDClient {
        name: "Test".into(),
        redirect_uri: vec![TEST_SERVER_URL.into()],
        scope: vec!["openid".into()],
        enabled: true,
    };
    let response = client
        .post("/api/v1/oauth")
        .json(&openid_client)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let openid_client: OAuth2Client<Id> = response.json().await;
    assert_eq!(openid_client.name, "Test");

    // verify app is not authorized yet
    let response = client.get("/api/v1/me").send().await;
    let user_info: UserInfo = response.json().await;
    assert_eq!(user_info.authorized_apps.len(), 0);

    // authorize app
    let response = client
        .post(format!(
            "/api/v1/oauth/authorize?\
            response_type=code&\
            client_id={}&\
            redirect_uri=http%3A%2F%2Flocalhost%3A3000&\
            scope=openid&\
            state=ABCDEF&\
            allow=true&\
            nonce=blabla",
            openid_client.client_id
        ))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FOUND);
    let response = client.get("/api/v1/me").send().await;
    let mut user_info: UserInfo = response.json().await;
    assert_eq!(user_info.authorized_apps.len(), 1);

    let old_test_user = get_db_user(&pool, "admin").await;

    // unregister app
    user_info.authorized_apps = [].into();
    let response = client
        .put("/api/v1/user/admin")
        .json(&user_info)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let response = client.get("/api/v1/me").send().await;
    let user_info: UserInfo = response.json().await;
    assert_eq!(user_info.authorized_apps.len(), 0);

    let new_test_user = get_db_user(&pool, "admin").await;

    client.verify_api_events(&[
        ApiEventType::OpenIdAppAdded { app: openid_client },
        ApiEventType::UserModified {
            before: old_test_user,
            after: new_test_user.clone(),
        },
    ]);
}

#[sqlx::test]
async fn test_user_add_device(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (mut client, state) = make_test_client(pool).await;
    let user_agent_header = "Mozilla/5.0 (iPhone; CPU iPhone OS 17_1 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.1 Mobile/15E148 Safari/604.1";

    // point SMTP at a mock server so device/login notifications are delivered
    let smtp = MockSmtpServer::start().await;
    smtp.configure(&state.pool).await;

    let mut expected_events = Vec::new();

    // log in as admin
    let auth = Auth::new("admin", "pass123");
    let response = client
        .post("/api/v1/auth")
        .header(USER_AGENT, user_agent_header)
        .json(&auth)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    expected_events.push(ApiEventType::UserLogin);

    // create network
    make_network(&client, "network").await;
    expected_events.push(ApiEventType::VpnLocationAdded {
        location: get_db_location(&state.pool, 1).await,
    });

    // add device for user
    let device_data = AddDevice {
        name: "TestDevice1".into(),
        wireguard_pubkey: "mgVXE8WcfStoD8mRatHcX5aaQ0DlcpjvPXibHEOr9y8=".into(),
    };
    let response = client
        .post("/api/v1/device/hpotter")
        .header(USER_AGENT, user_agent_header)
        .json(&device_data)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    expected_events.push(ApiEventType::UserDeviceAdded {
        owner: get_db_user(&state.pool, "hpotter").await,
        device: get_db_device(&state.pool, 1).await,
    });

    // add device for themselves
    let device_data = AddDevice {
        name: "TestDevice2".into(),
        wireguard_pubkey: "hNuapt7lOxF93KUqZGUY00oKJxH8LYwwsUVB1uUa0y4=".into(),
    };
    let response = client
        .post("/api/v1/device/admin")
        .header(USER_AGENT, user_agent_header)
        .json(&device_data)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    expected_events.push(ApiEventType::UserDeviceAdded {
        owner: get_db_user(&state.pool, "admin").await,
        device: get_db_device(&state.pool, 2).await,
    });

    // log in as normal user
    let auth = Auth::new("hpotter", "pass123");
    let response = client
        .post("/api/v1/auth")
        .header(USER_AGENT, user_agent_header)
        .json(&auth)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    expected_events.push(ApiEventType::UserLogin);

    let response = client.get("/api/v1/me").send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // a device with duplicate pubkey cannot be added
    let response = client
        .post("/api/v1/device/hpotter")
        .header(USER_AGENT, user_agent_header)
        .json(&device_data)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // normal user cannot add a device for other users
    let device_data = AddDevice {
        name: "TestDevice3".into(),
        wireguard_pubkey: "fF9K0tgatZTEJRvzpNUswr0h8HqCIi+v39B45+QZZzE=".into(),
    };
    let response = client
        .post("/api/v1/device/admin")
        .header(USER_AGENT, user_agent_header)
        .json(&device_data)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    // user adds a device for themselves
    let response = client
        .post("/api/v1/device/hpotter")
        .header(USER_AGENT, user_agent_header)
        .json(&device_data)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    expected_events.push(ApiEventType::UserDeviceAdded {
        owner: get_db_user(&state.pool, "hpotter").await,
        device: get_db_device(&state.pool, 3).await,
    });

    // Verify the notifications delivered across the flow (all fire-and-forget,
    // so assert the recipient/subject multiset rather than relying on order):
    //  - admin login                 -> new-device-login  to admin
    //  - admin adds device (hpotter)  -> new-device-added  to hpotter
    //  - admin adds device (self)     -> new-device-added  to admin
    //  - hpotter login                -> new-device-login  to hpotter
    //  - hpotter adds device (self)   -> new-device-added  to hpotter
    let mails = smtp.wait_for_count(5).await;
    let login_subject = "Defguard: New device logged in to your account";
    let added_subject = "Defguard: new device added to your account";
    let count = |to: &str, subject: &str| {
        mails
            .iter()
            .filter(|m| m.sent_to(to) && m.body_contains(subject))
            .count()
    };
    assert_eq!(count("admin@defguard", login_subject), 1);
    assert_eq!(count("admin@defguard", added_subject), 1);
    assert_eq!(count("h.potter@hogwart.edu.uk", login_subject), 1);
    assert_eq!(count("h.potter@hogwart.edu.uk", added_subject), 2);
    assert_eq!(mails.len(), 5, "exactly five notifications expected");

    client.verify_api_events(&expected_events);
}

#[sqlx::test]
async fn test_disable(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (mut client, pool) = make_client_with_db(pool).await;

    client.login_user("admin", "pass123").await;

    // get yourself
    let mut user_details = fetch_user_details(&client, "admin").await;
    user_details.user.is_active = false;

    // cannot disable yourself
    let response = client
        .put("/api/v1/user/admin")
        .json(&user_details.user)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // create user
    let new_user = AddUserData {
        username: "adumbledore".into(),
        last_name: "Dumbledore".into(),
        first_name: "Albus".into(),
        email: "a.dumbledore@hogwart.edu.uk".into(),
        phone: Some("1234".into()),
        password: Some("Password1234543$!".into()),
    };
    let response = client.post("/api/v1/user").json(&new_user).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // get user
    let mut user_details = fetch_user_details(&client, "adumbledore").await;
    assert_eq!(user_details.user.first_name, "Albus");
    assert!(user_details.user.is_active);

    let old_test_user = get_db_user(&pool, "adumbledore").await;

    // disable user
    user_details.user.is_active = false;
    let response = client
        .put("/api/v1/user/adumbledore")
        .json(&user_details.user)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    let user_details = fetch_user_details(&client, "adumbledore").await;
    assert_eq!(user_details.user.first_name, "Albus");
    assert!(!user_details.user.is_active);

    let new_test_user = get_db_user(&pool, "adumbledore").await;

    client.verify_api_events(&[
        ApiEventType::UserAdded {
            user: old_test_user.clone(),
        },
        ApiEventType::UserModified {
            before: old_test_user,
            after: new_test_user.clone(),
        },
        ApiEventType::UserDisabled {
            user: new_test_user,
        },
    ]);
}

#[sqlx::test]
async fn test_admin_can_disable_another_users_mfa_emits_updated_event_and_cleans_db(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;

    let (mut client, pool) = make_client_with_db(pool).await;

    client.login_user("admin", "pass123").await;

    let admin_user = get_db_user(&pool, "admin").await;
    let recovery_codes = seed_user_with_mfa_artifacts(&pool, "hpotter").await;

    let seeded_user = get_db_user(&pool, "hpotter").await;
    assert!(seeded_user.mfa_enabled);
    assert!(seeded_user.totp_enabled);
    assert!(seeded_user.email_mfa_enabled);
    assert!(seeded_user.totp_secret.is_some());
    assert!(seeded_user.email_mfa_secret.is_some());
    assert_eq!(seeded_user.mfa_method, MFAMethod::OneTimePassword);
    assert_eq!(seeded_user.recovery_codes, recovery_codes);
    assert_eq!(
        WebAuthn::all_for_user(&pool, seeded_user.id)
            .await
            .unwrap()
            .len(),
        1
    );

    let response = client.delete("/api/v1/user/hpotter/mfa").send().await;
    assert_eq!(response.status(), StatusCode::OK);

    let updated_user = get_db_user(&pool, "hpotter").await;
    assert!(!updated_user.mfa_enabled);
    assert!(!updated_user.totp_enabled);
    assert!(!updated_user.email_mfa_enabled);
    assert!(updated_user.totp_secret.is_none());
    assert!(updated_user.email_mfa_secret.is_none());
    assert_eq!(updated_user.mfa_method, MFAMethod::None);
    assert!(updated_user.recovery_codes.is_empty());
    assert!(
        WebAuthn::all_for_user(&pool, updated_user.id)
            .await
            .unwrap()
            .is_empty()
    );

    client.verify_api_events_with_user(&[(
        ApiEventType::UserMfaDisabled {
            user: updated_user.clone(),
        },
        admin_user.id,
        "admin",
    )]);
}

#[sqlx::test]
async fn test_non_admin_cannot_disable_another_users_mfa_and_emits_no_event(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;

    let (mut client, pool) = make_client_with_db(pool).await;

    let recovery_codes = seed_user_with_mfa_artifacts(&pool, "admin").await;
    client.login_user("hpotter", "pass123").await;

    let response = client.delete("/api/v1/user/admin/mfa").send().await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let admin_user = get_db_user(&pool, "admin").await;
    assert!(admin_user.mfa_enabled);
    assert!(admin_user.totp_enabled);
    assert!(admin_user.email_mfa_enabled);
    assert!(admin_user.totp_secret.is_some());
    assert!(admin_user.email_mfa_secret.is_some());
    assert_eq!(admin_user.mfa_method, MFAMethod::OneTimePassword);
    assert_eq!(admin_user.recovery_codes, recovery_codes);
    assert_eq!(
        WebAuthn::all_for_user(&pool, admin_user.id)
            .await
            .unwrap()
            .len(),
        1
    );

    client.assert_event_queue_is_empty();
}

#[sqlx::test]
async fn test_unique_email(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (mut client, pool) = make_client_with_db(pool).await;

    client.login_user("admin", "pass123").await;

    // create user
    let new_user = AddUserData {
        username: "adumbledore".into(),
        last_name: "Dumbledore".into(),
        first_name: "Albus".into(),
        email: "a.dumbledore@hogwart.edu.uk".into(),
        phone: Some("1234".into()),
        password: Some("Password1234543$!".into()),
    };
    let response = client.post("/api/v1/user").json(&new_user).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // create user with same email
    let new_user = AddUserData {
        username: "adumbledore2".into(),
        last_name: "Dumbledore".into(),
        first_name: "Albus".into(),
        email: "a.dumbledore@hogwart.edu.uk".into(),
        phone: Some("1234".into()),
        password: Some("Password1234543$!".into()),
    };
    let response = client.post("/api/v1/user").json(&new_user).send().await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let test_user = get_db_user(&pool, "adumbledore").await;

    client.verify_api_events(&[ApiEventType::UserAdded { user: test_user }]);
}

// Admin updating another user must be able to change all profile
// fields (username, first/last name, email) and phone. The `mfa_method` must
// NOT change because the admin is not updating themselves.
#[sqlx::test]
async fn test_modify_user_admin_updates_other_user(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (mut client, pool) = make_client_with_db(pool).await;
    client.login_user("admin", "pass123").await;

    let mut user_details = fetch_user_details(&client, "hpotter").await;
    let old_user = get_db_user(&pool, "hpotter").await;

    user_details.user.first_name = "UpdatedFirst".into();
    user_details.user.last_name = "UpdatedLast".into();
    user_details.user.email = "updated@hogwart.edu.uk".into();
    user_details.user.phone = Some("+48999888777".into());
    user_details.user.mfa_method = MFAMethod::OneTimePassword;

    let response = client
        .put("/api/v1/user/hpotter")
        .json(&user_details.user)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    let updated = get_db_user(&pool, "hpotter").await;

    // Profile fields changed by admin
    assert_eq!(updated.first_name, "UpdatedFirst");
    assert_eq!(updated.last_name, "UpdatedLast");
    assert_eq!(updated.email, "updated@hogwart.edu.uk");
    assert_eq!(updated.phone, Some("+48999888777".into()));
    // mfa_method must NOT have changed - admin is not updating self
    assert_eq!(updated.mfa_method, old_user.mfa_method);

    client.verify_api_events(&[ApiEventType::UserModified {
        before: old_user,
        after: updated,
    }]);
}

// A non-admin user updating themselves may change phone and
// mfa_method, but must NOT be able to change username, name, or email.
#[sqlx::test]
async fn test_modify_user_non_admin_updates_self(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (mut client, pool) = make_client_with_db(pool).await;
    client.login_user("hpotter", "pass123").await;

    let mut user_details = fetch_user_details(&client, "hpotter").await;
    let old_user = get_db_user(&pool, "hpotter").await;

    // Non-admin tries to change protected fields
    user_details.user.username = "newusername".into();
    user_details.user.first_name = "UpdatedFirst".into();
    user_details.user.last_name = "UpdatedLast".into();
    user_details.user.email = "updated@example.com".into();
    user_details.user.phone = Some("+48111222333".into());

    let response = client
        .put("/api/v1/user/hpotter")
        .json(&user_details.user)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    let updated = get_db_user(&pool, "hpotter").await;

    // Protected fields must be unchanged
    assert_eq!(updated.username, "hpotter");
    assert_eq!(updated.first_name, "Harry");
    assert_eq!(updated.last_name, "Potter");
    assert_eq!(updated.email, "h.potter@hogwart.edu.uk");
    // Phone is allowed for self-updates
    assert_eq!(updated.phone, Some("+48111222333".into()));

    client.verify_api_events(&[ApiEventType::UserModified {
        before: old_user,
        after: updated,
    }]);
}

// A non-admin user must not be able to modify another user's fields,
// not even phone (the endpoint should return 403 via user_for_admin_or_self).
#[sqlx::test]
async fn test_modify_user_non_admin_updates_other_user(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let (mut client, _pool) = make_client_with_db(pool).await;
    client.login_user("hpotter", "pass123").await;

    // Fetch admin's profile and try to change it as hpotter
    let mut user_details = fetch_user_details(&client, "hpotter").await;
    user_details.user.phone = Some("+48000000000".into());

    let response = client
        .put("/api/v1/user/admin")
        .json(&user_details.user)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    client.assert_event_queue_is_empty();
}

// Admin updating their own account can change all fields including
// mfa_method (is_admin=true AND is_updating_self=true).
#[sqlx::test]
async fn test_modify_user_admin_updates_self(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (mut client, pool) = make_client_with_db(pool).await;
    client.login_user("admin", "pass123").await;

    let mut user_details = fetch_user_details(&client, "admin").await;
    let old_user = get_db_user(&pool, "admin").await;

    user_details.user.first_name = "NewFirst".into();
    user_details.user.last_name = "NewLast".into();
    user_details.user.phone = Some("+48777888999".into());

    let response = client
        .put("/api/v1/user/admin")
        .json(&user_details.user)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    let updated = get_db_user(&pool, "admin").await;

    assert_eq!(updated.first_name, "NewFirst");
    assert_eq!(updated.last_name, "NewLast");
    assert_eq!(updated.phone, Some("+48777888999".into()));

    client.verify_api_events(&[ApiEventType::UserModified {
        before: old_user,
        after: updated,
    }]);
}

#[sqlx::test]
async fn test_bulk_disable_users(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (mut client, pool) = make_client_with_db(pool).await;
    client.login_user("admin", "pass123").await;

    for (username, email) in [
        ("adumbledore", "a.dumbledore@hogwart.edu.uk"),
        ("mmcgonagall", "m.mcgonagall@hogwart.edu.uk"),
    ] {
        let new_user = AddUserData {
            username: username.into(),
            last_name: format!("{username}-last"),
            first_name: format!("{username}-first"),
            email: email.into(),
            phone: Some("1234".into()),
            password: Some("Password1234543$!".into()),
        };
        let response = client.post("/api/v1/user").json(&new_user).send().await;
        assert_eq!(response.status(), StatusCode::CREATED);
    }

    let added_dumbledore = get_db_user(&pool, "adumbledore").await;
    let added_mcgonagall = get_db_user(&pool, "mmcgonagall").await;
    assert!(added_dumbledore.is_active);
    assert!(added_mcgonagall.is_active);

    let response = client
        .post("/api/v1/user/bulk-disable")
        .json(&serde_json::json!({ "users": [added_dumbledore.id, added_mcgonagall.id] }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    let disabled_dumbledore = get_db_user(&pool, "adumbledore").await;
    let disabled_mcgonagall = get_db_user(&pool, "mmcgonagall").await;
    assert!(!disabled_dumbledore.is_active);
    assert!(!disabled_mcgonagall.is_active);

    client.verify_api_events(&[
        ApiEventType::UserAdded {
            user: added_dumbledore.clone(),
        },
        ApiEventType::UserAdded {
            user: added_mcgonagall.clone(),
        },
        ApiEventType::UserModified {
            before: added_dumbledore,
            after: disabled_dumbledore,
        },
        ApiEventType::UserModified {
            before: added_mcgonagall,
            after: disabled_mcgonagall,
        },
    ]);
}

#[sqlx::test]
async fn test_bulk_enable_users(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (mut client, pool) = make_client_with_db(pool).await;
    client.login_user("admin", "pass123").await;

    for (username, email) in [
        ("adumbledore", "a.dumbledore@hogwart.edu.uk"),
        ("mmcgonagall", "m.mcgonagall@hogwart.edu.uk"),
    ] {
        let new_user = AddUserData {
            username: username.into(),
            last_name: format!("{username}-last"),
            first_name: format!("{username}-first"),
            email: email.into(),
            phone: Some("1234".into()),
            password: Some("Password1234543$!".into()),
        };
        let response = client.post("/api/v1/user").json(&new_user).send().await;
        assert_eq!(response.status(), StatusCode::CREATED);
    }

    let added_dumbledore = get_db_user(&pool, "adumbledore").await;
    let added_mcgonagall = get_db_user(&pool, "mmcgonagall").await;

    // disable both users first so there is something to re-enable
    let response = client
        .post("/api/v1/user/bulk-disable")
        .json(&serde_json::json!({ "users": [added_dumbledore.id, added_mcgonagall.id] }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    let disabled_dumbledore = get_db_user(&pool, "adumbledore").await;
    let disabled_mcgonagall = get_db_user(&pool, "mmcgonagall").await;
    assert!(!disabled_dumbledore.is_active);
    assert!(!disabled_mcgonagall.is_active);

    let response = client
        .post("/api/v1/user/bulk-enable")
        .json(&serde_json::json!({ "users": [disabled_dumbledore.id, disabled_mcgonagall.id] }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    let enabled_dumbledore = get_db_user(&pool, "adumbledore").await;
    let enabled_mcgonagall = get_db_user(&pool, "mmcgonagall").await;
    assert!(enabled_dumbledore.is_active);
    assert!(enabled_mcgonagall.is_active);

    client.verify_api_events(&[
        ApiEventType::UserAdded {
            user: added_dumbledore.clone(),
        },
        ApiEventType::UserAdded {
            user: added_mcgonagall.clone(),
        },
        ApiEventType::UserModified {
            before: added_dumbledore,
            after: disabled_dumbledore.clone(),
        },
        ApiEventType::UserModified {
            before: added_mcgonagall,
            after: disabled_mcgonagall.clone(),
        },
        ApiEventType::UserModified {
            before: disabled_dumbledore,
            after: enabled_dumbledore,
        },
        ApiEventType::UserModified {
            before: disabled_mcgonagall,
            after: enabled_mcgonagall,
        },
    ]);
}

#[sqlx::test]
async fn test_bulk_enable_unknown_user(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (mut client, _pool) = make_client_with_db(pool).await;
    client.login_user("admin", "pass123").await;

    let response = client
        .post("/api/v1/user/bulk-enable")
        .json(&serde_json::json!({ "users": [9_999_999] }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    client.assert_event_queue_is_empty();
}

#[sqlx::test]
async fn test_bulk_disable_rejects_self(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (mut client, pool) = make_client_with_db(pool).await;
    client.login_user("admin", "pass123").await;

    let admin_user = get_db_user(&pool, "admin").await;
    let hpotter_user = get_db_user(&pool, "hpotter").await;

    let response = client
        .post("/api/v1/user/bulk-disable")
        .json(&serde_json::json!({ "users": [admin_user.id, hpotter_user.id] }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let hpotter_after = get_db_user(&pool, "hpotter").await;
    assert!(hpotter_after.is_active);

    client.assert_event_queue_is_empty();
}

#[sqlx::test]
async fn test_bulk_disable_unknown_user(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (mut client, _pool) = make_client_with_db(pool).await;
    client.login_user("admin", "pass123").await;

    let response = client
        .post("/api/v1/user/bulk-disable")
        .json(&serde_json::json!({ "users": [9_999_999] }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    client.assert_event_queue_is_empty();
}

#[sqlx::test]
async fn test_bulk_delete_users(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (mut client, pool) = make_client_with_db(pool).await;
    client.login_user("admin", "pass123").await;

    for (username, email) in [
        ("adumbledore", "a.dumbledore@hogwart.edu.uk"),
        ("mmcgonagall", "m.mcgonagall@hogwart.edu.uk"),
    ] {
        let new_user = AddUserData {
            username: username.into(),
            last_name: format!("{username}-last"),
            first_name: format!("{username}-first"),
            email: email.into(),
            phone: Some("1234".into()),
            password: Some("Password1234543$!".into()),
        };
        let response = client.post("/api/v1/user").json(&new_user).send().await;
        assert_eq!(response.status(), StatusCode::CREATED);
    }

    let added_dumbledore = get_db_user(&pool, "adumbledore").await;
    let added_mcgonagall = get_db_user(&pool, "mmcgonagall").await;

    let response = client
        .post("/api/v1/user/bulk-delete")
        .json(&serde_json::json!({ "users": [added_dumbledore.id, added_mcgonagall.id] }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    let response = client.get("/api/v1/user/adumbledore").send().await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let response = client.get("/api/v1/user/mmcgonagall").send().await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    client.verify_api_events(&[
        ApiEventType::UserAdded {
            user: added_dumbledore.clone(),
        },
        ApiEventType::UserAdded {
            user: added_mcgonagall.clone(),
        },
        ApiEventType::UserRemoved {
            user: added_dumbledore,
        },
        ApiEventType::UserRemoved {
            user: added_mcgonagall,
        },
    ]);
}

#[sqlx::test]
async fn test_bulk_delete_rejects_self(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (mut client, pool) = make_client_with_db(pool).await;
    client.login_user("admin", "pass123").await;

    let admin_user = get_db_user(&pool, "admin").await;
    let hpotter_user = get_db_user(&pool, "hpotter").await;

    let response = client
        .post("/api/v1/user/bulk-delete")
        .json(&serde_json::json!({ "users": [admin_user.id, hpotter_user.id] }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let response = client.get("/api/v1/user/hpotter").send().await;
    assert_eq!(response.status(), StatusCode::OK);

    client.assert_event_queue_is_empty();
}

#[sqlx::test]
async fn test_bulk_delete_unknown_user(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (mut client, _pool) = make_client_with_db(pool).await;
    client.login_user("admin", "pass123").await;

    let response = client
        .post("/api/v1/user/bulk-delete")
        .json(&serde_json::json!({ "users": [9_999_999] }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    client.assert_event_queue_is_empty();
}

#[sqlx::test]
async fn test_bulk_start_enrollment(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (mut client, pool) = make_client_with_db(pool).await;
    client.login_user("admin", "pass123").await;

    for (username, email) in [
        ("adumbledore", "a.dumbledore@hogwart.edu.uk"),
        ("mmcgonagall", "m.mcgonagall@hogwart.edu.uk"),
    ] {
        let new_user = AddUserData {
            username: username.into(),
            last_name: format!("{username}-last"),
            first_name: format!("{username}-first"),
            email: email.into(),
            phone: None,
            password: None,
        };
        let response = client.post("/api/v1/user").json(&new_user).send().await;
        assert_eq!(response.status(), StatusCode::CREATED);
    }

    let dumbledore = get_db_user(&pool, "adumbledore").await;
    let mcgonagall = get_db_user(&pool, "mmcgonagall").await;
    assert!(!dumbledore.enrollment_pending);
    assert!(!mcgonagall.enrollment_pending);

    let response = client
        .post("/api/v1/user/bulk-start-enrollment")
        .json(&serde_json::json!({
            "users": [dumbledore.id, mcgonagall.id],
            "send_enrollment_notification": false
        }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    let body: serde_json::Value = response.json().await;
    assert_eq!(body["started"], 2);
    assert_eq!(body["skipped"], 0);

    let dumbledore_after = get_db_user(&pool, "adumbledore").await;
    let mcgonagall_after = get_db_user(&pool, "mmcgonagall").await;
    assert!(dumbledore_after.enrollment_pending);
    assert!(mcgonagall_after.enrollment_pending);

    client.verify_api_events(&[
        ApiEventType::UserAdded {
            user: dumbledore.clone(),
        },
        ApiEventType::UserAdded {
            user: mcgonagall.clone(),
        },
        ApiEventType::EnrollmentTokenAdded {
            user: dumbledore_after,
        },
        ApiEventType::EnrollmentTokenAdded {
            user: mcgonagall_after,
        },
    ]);
}

#[sqlx::test]
async fn test_bulk_start_enrollment_re_enrolls_active_users(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    // Already-enrolled users (those with a password) must be re-enrolled: enrollment_pending
    // should be set to true and the response must not count them as skipped.
    let pool = setup_pool(options).await;
    let (mut client, pool) = make_client_with_db(pool).await;
    client.login_user("admin", "pass123").await;

    let new_user = AddUserData {
        username: "adumbledore".into(),
        last_name: "Dumbledore".into(),
        first_name: "Albus".into(),
        email: "a.dumbledore@hogwart.edu.uk".into(),
        phone: None,
        password: Some("Password1234543$!".into()),
    };
    let response = client.post("/api/v1/user").json(&new_user).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);

    let dumbledore = get_db_user(&pool, "adumbledore").await;
    assert!(
        dumbledore.is_enrolled(),
        "user should be enrolled after creation with password"
    );
    assert!(!dumbledore.enrollment_pending);

    let response = client
        .post("/api/v1/user/bulk-start-enrollment")
        .json(&serde_json::json!({
            "users": [dumbledore.id],
            "send_enrollment_notification": false
        }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    let body: serde_json::Value = response.json().await;
    assert_eq!(body["started"], 1);
    assert_eq!(body["skipped"], 0);

    let dumbledore_after = get_db_user(&pool, "adumbledore").await;
    assert!(
        dumbledore_after.enrollment_pending,
        "enrollment_pending should be true after re-enrollment"
    );
}

#[sqlx::test]
async fn test_bulk_start_enrollment_skips_disabled_users(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let (mut client, pool) = make_client_with_db(pool).await;
    client.login_user("admin", "pass123").await;

    for (username, email) in [
        ("adumbledore", "a.dumbledore@hogwart.edu.uk"),
        ("mmcgonagall", "m.mcgonagall@hogwart.edu.uk"),
    ] {
        let new_user = AddUserData {
            username: username.into(),
            last_name: format!("{username}-last"),
            first_name: format!("{username}-first"),
            email: email.into(),
            phone: None,
            password: None,
        };
        let response = client.post("/api/v1/user").json(&new_user).send().await;
        assert_eq!(response.status(), StatusCode::CREATED);
    }

    let dumbledore = get_db_user(&pool, "adumbledore").await;
    let mcgonagall = get_db_user(&pool, "mmcgonagall").await;

    // Disable mcgonagall via bulk-disable
    let response = client
        .post("/api/v1/user/bulk-disable")
        .json(&serde_json::json!({ "users": [mcgonagall.id] }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    let response = client
        .post("/api/v1/user/bulk-start-enrollment")
        .json(&serde_json::json!({
            "users": [dumbledore.id, mcgonagall.id],
            "send_enrollment_notification": false
        }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    let body: serde_json::Value = response.json().await;
    assert_eq!(body["started"], 1);
    assert_eq!(body["skipped"], 1);

    let dumbledore_after = get_db_user(&pool, "adumbledore").await;
    let mcgonagall_after = get_db_user(&pool, "mmcgonagall").await;
    assert!(dumbledore_after.enrollment_pending);
    assert!(
        !mcgonagall_after.enrollment_pending,
        "disabled user must not have enrollment started"
    );
}

#[sqlx::test]
async fn test_bulk_start_enrollment_rejects_self(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (mut client, pool) = make_client_with_db(pool).await;
    client.login_user("admin", "pass123").await;

    let admin = get_db_user(&pool, "admin").await;
    let hpotter = get_db_user(&pool, "hpotter").await;

    let response = client
        .post("/api/v1/user/bulk-start-enrollment")
        .json(&serde_json::json!({
            "users": [admin.id, hpotter.id],
            "send_enrollment_notification": false
        }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let hpotter_after = get_db_user(&pool, "hpotter").await;
    assert!(
        !hpotter_after.enrollment_pending,
        "no enrollment must be started when request is rejected"
    );

    client.assert_event_queue_is_empty();
}

#[sqlx::test]
async fn test_bulk_start_enrollment_unknown_user(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (mut client, _pool) = make_client_with_db(pool).await;
    client.login_user("admin", "pass123").await;

    let response = client
        .post("/api/v1/user/bulk-start-enrollment")
        .json(&serde_json::json!({
            "users": [9_999_999],
            "send_enrollment_notification": false
        }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    client.assert_event_queue_is_empty();
}

#[sqlx::test]
async fn test_bulk_disable_deduplicates_ids(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (mut client, pool) = make_client_with_db(pool).await;
    client.login_user("admin", "pass123").await;

    let new_user = AddUserData {
        username: "adumbledore".into(),
        last_name: "Dumbledore".into(),
        first_name: "Albus".into(),
        email: "a.dumbledore@hogwart.edu.uk".into(),
        phone: None,
        password: Some("Password1234543$!".into()),
    };
    let response = client.post("/api/v1/user").json(&new_user).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);

    let dumbledore = get_db_user(&pool, "adumbledore").await;

    // Send the same ID twice; must not trigger the "unknown user" 400.
    let response = client
        .post("/api/v1/user/bulk-disable")
        .json(&serde_json::json!({ "users": [dumbledore.id, dumbledore.id] }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    let dumbledore_after = get_db_user(&pool, "adumbledore").await;
    assert!(!dumbledore_after.is_active);
}

#[sqlx::test]
async fn test_bulk_enable_deduplicates_ids(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (mut client, pool) = make_client_with_db(pool).await;
    client.login_user("admin", "pass123").await;

    let new_user = AddUserData {
        username: "adumbledore".into(),
        last_name: "Dumbledore".into(),
        first_name: "Albus".into(),
        email: "a.dumbledore@hogwart.edu.uk".into(),
        phone: None,
        password: Some("Password1234543$!".into()),
    };
    let response = client.post("/api/v1/user").json(&new_user).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);

    let dumbledore = get_db_user(&pool, "adumbledore").await;

    // disable the user so re-enabling has an effect
    let response = client
        .post("/api/v1/user/bulk-disable")
        .json(&serde_json::json!({ "users": [dumbledore.id] }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    // Send the same ID twice; must not trigger the "unknown user" 400.
    let response = client
        .post("/api/v1/user/bulk-enable")
        .json(&serde_json::json!({ "users": [dumbledore.id, dumbledore.id] }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    let dumbledore_after = get_db_user(&pool, "adumbledore").await;
    assert!(dumbledore_after.is_active);
}

#[sqlx::test]
async fn test_bulk_delete_deduplicates_ids(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (mut client, pool) = make_client_with_db(pool).await;
    client.login_user("admin", "pass123").await;

    let new_user = AddUserData {
        username: "adumbledore".into(),
        last_name: "Dumbledore".into(),
        first_name: "Albus".into(),
        email: "a.dumbledore@hogwart.edu.uk".into(),
        phone: None,
        password: Some("Password1234543$!".into()),
    };
    let response = client.post("/api/v1/user").json(&new_user).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);

    let dumbledore = get_db_user(&pool, "adumbledore").await;

    // Send the same ID twice; must not trigger the "unknown user" 400.
    let response = client
        .post("/api/v1/user/bulk-delete")
        .json(&serde_json::json!({ "users": [dumbledore.id, dumbledore.id] }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    let response = client.get("/api/v1/user/adumbledore").send().await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn test_delete_user_clears_stale_default_admin_settings_cache(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let (mut client, pool) = make_client_with_db(pool).await;
    client.login_user("admin", "pass123").await;

    let new_user = AddUserData {
        username: "adumbledore".into(),
        last_name: "Dumbledore".into(),
        first_name: "Albus".into(),
        email: "a.dumbledore@hogwart.edu.uk".into(),
        phone: None,
        password: Some("Password1234543$!".into()),
    };
    let response = client.post("/api/v1/user").json(&new_user).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let dumbledore = get_db_user(&pool, "adumbledore").await;

    // simulate `dumbledore` being the default admin set up during initial setup
    let mut settings = Settings::get(&pool).await.unwrap().unwrap();
    settings.default_admin_id = Some(dumbledore.id);
    update_current_settings(&pool, settings).await.unwrap();

    let response = client.delete("/api/v1/user/adumbledore").send().await;
    assert_eq!(response.status(), StatusCode::OK);

    let from_db = Settings::get(&pool).await.unwrap().unwrap();
    assert_eq!(from_db.default_admin_id, None);

    // any settings update used to fail with a `fk_default_admin` violation here, since the
    // in-memory cache still held the now-dangling `dumbledore.id`
    let response = client
        .patch("/api/v1/settings")
        .json(&serde_json::json!({ "wireguard_enabled": false }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
}

#[sqlx::test]
async fn test_bulk_delete_users_clears_stale_default_admin_settings_cache(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let (mut client, pool) = make_client_with_db(pool).await;
    client.login_user("admin", "pass123").await;

    let new_user = AddUserData {
        username: "adumbledore".into(),
        last_name: "Dumbledore".into(),
        first_name: "Albus".into(),
        email: "a.dumbledore@hogwart.edu.uk".into(),
        phone: None,
        password: Some("Password1234543$!".into()),
    };
    let response = client.post("/api/v1/user").json(&new_user).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let dumbledore = get_db_user(&pool, "adumbledore").await;

    let mut settings = Settings::get(&pool).await.unwrap().unwrap();
    settings.default_admin_id = Some(dumbledore.id);
    update_current_settings(&pool, settings).await.unwrap();

    let response = client
        .post("/api/v1/user/bulk-delete")
        .json(&serde_json::json!({ "users": [dumbledore.id] }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    let from_db = Settings::get(&pool).await.unwrap().unwrap();
    assert_eq!(from_db.default_admin_id, None);

    let response = client
        .patch("/api/v1/settings")
        .json(&serde_json::json!({ "wireguard_enabled": false }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
}

#[sqlx::test]
async fn test_bulk_start_enrollment_deduplicates_ids(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (mut client, pool) = make_client_with_db(pool).await;
    client.login_user("admin", "pass123").await;

    let new_user = AddUserData {
        username: "adumbledore".into(),
        last_name: "Dumbledore".into(),
        first_name: "Albus".into(),
        email: "a.dumbledore@hogwart.edu.uk".into(),
        phone: None,
        password: None,
    };
    let response = client.post("/api/v1/user").json(&new_user).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);

    let dumbledore = get_db_user(&pool, "adumbledore").await;

    // Send the same ID twice; must not trigger the "unknown user" 400 and must
    // count as a single started enrollment.
    let response = client
        .post("/api/v1/user/bulk-start-enrollment")
        .json(&serde_json::json!({
            "users": [dumbledore.id, dumbledore.id],
            "send_enrollment_notification": false
        }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    let body: serde_json::Value = response.json().await;
    assert_eq!(body["started"], 1);
    assert_eq!(body["skipped"], 0);

    let dumbledore_after = get_db_user(&pool, "adumbledore").await;
    assert!(dumbledore_after.enrollment_pending);
}

/// Admin disabling a user emits both UserModified and UserDisabled events.
#[sqlx::test]
async fn test_modify_user_admin_disables_user(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (mut client, pool) = make_client_with_db(pool).await;
    client.login_user("admin", "pass123").await;

    let mut user_details = fetch_user_details(&client, "hpotter").await;
    let old_user = get_db_user(&pool, "hpotter").await;
    assert!(old_user.is_active);

    user_details.user.is_active = false;

    let response = client
        .put("/api/v1/user/hpotter")
        .json(&user_details.user)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    let updated = get_db_user(&pool, "hpotter").await;
    assert!(!updated.is_active);

    client.verify_api_events(&[
        ApiEventType::UserModified {
            before: old_user,
            after: updated.clone(),
        },
        ApiEventType::UserDisabled { user: updated },
    ]);
}

/// Admin enabling a previously disabled user emits both UserModified and UserEnabled events.
#[sqlx::test]
async fn test_modify_user_admin_enables_user(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (mut client, pool) = make_client_with_db(pool).await;
    client.login_user("admin", "pass123").await;

    // First disable the user via the API
    let mut user_details = fetch_user_details(&client, "hpotter").await;
    user_details.user.is_active = false;
    client
        .put("/api/v1/user/hpotter")
        .json(&user_details.user)
        .send()
        .await;
    client.drain_all_events();

    // Now re-enable
    user_details = fetch_user_details(&client, "hpotter").await;
    let old_user = get_db_user(&pool, "hpotter").await;
    assert!(!old_user.is_active);

    user_details.user.is_active = true;

    let response = client
        .put("/api/v1/user/hpotter")
        .json(&user_details.user)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    let updated = get_db_user(&pool, "hpotter").await;
    assert!(updated.is_active);

    client.verify_api_events(&[
        ApiEventType::UserModified {
            before: old_user,
            after: updated.clone(),
        },
        ApiEventType::UserEnabled { user: updated },
    ]);
}

/// Password management is disabled for an LDAP-sourced, non-admin user with no local password
/// when the LDAP "disable password management" flag is on.
#[sqlx::test]
async fn test_password_management_disabled_for_ldap_user(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let (mut client, pool) = make_client_with_db(pool).await;

    // Create an LDAP-sourced user with no local password hash.
    let ldap_user = User::new("ldapuser", None, "LDAP", "User", "ldap@example.com", None)
        .save(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE \"user\" SET from_ldap = true WHERE id = $1")
        .bind(ldap_user.id)
        .execute(&pool)
        .await
        .unwrap();

    // Enable the LDAP disable-password-management flag.
    let mut settings = Settings::get_current_settings();
    settings.ldap_disable_password_management = true;
    update_current_settings(&pool, settings).await.unwrap();

    // Login as admin to exercise admin-level password operations on the LDAP user.
    client.login_user("admin", "pass123").await;

    // change_password (admin → LDAP user) → 403
    let response = client
        .put("/api/v1/user/ldapuser/password")
        .json(&PasswordChange {
            new_password: "NewPass123!".into(),
        })
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    // reset_password (admin → LDAP user) → 403
    let response = client
        .post("/api/v1/user/ldapuser/reset_password")
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    // Give the LDAP user a local password and verify they are now allowed
    // (passing password_hash guard in password_management_disabled).
    client.drain_all_events();
    let mut u = get_db_user(&pool, "ldapuser").await;
    u.set_password("temppass");
    u.save(&pool).await.unwrap();

    client.login_user("ldapuser", "temppass").await;
    let response = client
        .put("/api/v1/user/change_password")
        .json(&PasswordChangeSelf {
            old_password: "temppass".into(),
            new_password: "NewPass456!".into(),
        })
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    let ldap_user = get_db_user(&pool, "ldapuser").await;
    client.verify_api_events_with_user(&[(
        ApiEventType::PasswordChanged,
        ldap_user.id,
        "ldapuser",
    )]);
}

/// An admin user is always exempt from password-management gating, even when sourced externally.
#[sqlx::test]
async fn test_password_management_disabled_admin_exempt(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let (mut client, pool) = make_client_with_db(pool).await;

    // Make the admin user appear LDAP-sourced, then enable the LDAP disable flag.
    // Admin keeps their local password so they can still authenticate;
    // the is_admin check in the helper runs before the password_hash check,
    // so having a password hash does not weaken the test.
    let admin = get_db_user(&pool, "admin").await;
    sqlx::query("UPDATE \"user\" SET from_ldap = true WHERE id = $1")
        .bind(admin.id)
        .execute(&pool)
        .await
        .unwrap();

    let mut settings = Settings::get_current_settings();
    settings.ldap_disable_password_management = true;
    update_current_settings(&pool, settings).await.unwrap();

    // Admin should still be able to change someone else's password.
    client.login_user("admin", "pass123").await;
    let response = client
        .put("/api/v1/user/hpotter/password")
        .json(&PasswordChange {
            new_password: "NewPass789!".into(),
        })
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    let hpotter = get_db_user(&pool, "hpotter").await;
    client.verify_api_events_with_user(&[(
        ApiEventType::PasswordChangedByAdmin { user: hpotter },
        1,
        "admin",
    )]);
}

/// A user with a local password hash is always allowed, even if externally sourced.
#[sqlx::test]
async fn test_password_management_disabled_allowed_with_local_password(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let (mut client, pool) = make_client_with_db(pool).await;

    // hpotter is a local user with a password => change_self_password should work.
    client.login_user("hpotter", "pass123").await;
    let response = client
        .put("/api/v1/user/change_password")
        .json(&PasswordChangeSelf {
            old_password: "pass123".into(),
            new_password: "NewPass000!".into(),
        })
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    let hpotter = get_db_user(&pool, "hpotter").await;
    client.verify_api_events_with_user(&[(ApiEventType::PasswordChanged, hpotter.id, "hpotter")]);
}

/// Password management is disabled for an OIDC-sourced non-admin user when the provider flag is on.
#[sqlx::test]
async fn test_password_management_disabled_for_oidc_user(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let (mut client, pool) = make_client_with_db(pool).await;

    // Create an OIDC provider with disable_password_management enabled.
    OpenIdProvider::new(
        "test-oidc".to_owned(),
        "https://example.com".to_owned(),
        OpenIdProviderKind::Custom,
        "client-id".to_owned(),
        "client-secret".to_owned(),
        None,
        None,
        None,
        None,
        false,
        600,
        DirectorySyncUserBehavior::Keep,
        DirectorySyncUserBehavior::Keep,
        DirectorySyncTarget::All,
        None,
        None,
        Vec::new(),
        None,
        false,
        true, // disable_password_management
        None, // directory_sync_user_groups
    )
    .save(&pool)
    .await
    .unwrap();

    // Create an OIDC-sourced user with no local password.
    let oidc_user = User::new("oidcuser", None, "OIDC", "User", "oidc@example.com", None)
        .save(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE \"user\" SET openid_sub = $1, password_hash = NULL WHERE id = $2")
        .bind("sub-123")
        .bind(oidc_user.id)
        .execute(&pool)
        .await
        .unwrap();

    // Login as admin to exercise admin-level password operations on the OIDC user.
    client.login_user("admin", "pass123").await;

    // change_password -> 403
    let response = client
        .put("/api/v1/user/oidcuser/password")
        .json(&PasswordChange {
            new_password: "NewPass123!".into(),
        })
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    // reset_password -> 403
    let response = client
        .post("/api/v1/user/oidcuser/reset_password")
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[sqlx::test]
async fn test_reset_password_sends_email(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (mut client, pool) = make_client_with_db(pool).await;

    // Configure a proxy URL (needed to build the reset link) and point SMTP at
    // an in-process mock server so the reset email is actually delivered.
    let mut settings = Settings::get_current_settings();
    settings.public_proxy_url = "https://proxy.example.com".to_string();
    update_current_settings(&pool, settings).await.unwrap();
    let smtp = MockSmtpServer::start().await;
    smtp.configure(&pool).await;

    // Admin triggers a password reset for another user.
    client.login_user("admin", "pass123").await;
    let response = client
        .post("/api/v1/user/hpotter/reset_password")
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    // The reset email (sent fire-and-forget) is delivered to the target user
    // and carries a tokenized reset link pointing at the configured proxy.
    let mail = smtp
        .wait_for(|m| m.sent_to("h.potter@hogwart.edu.uk"))
        .await;
    assert!(
        mail.body_contains("token"),
        "reset email should contain a reset token link"
    );
    assert!(
        mail.body_contains("proxy.example.com"),
        "reset link should point at the configured proxy URL"
    );
}
