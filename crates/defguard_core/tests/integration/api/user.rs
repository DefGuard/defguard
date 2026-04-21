use std::net::{IpAddr, Ipv4Addr};

use chrono::NaiveDate;
use defguard_common::{
    db::{
        Id,
        models::{
            Device, DeviceType, MFAMethod, User, WebAuthn, WireguardNetwork,
            device::{AddDevice, WireguardNetworkDevice},
            gateway::Gateway,
            oauth2client::OAuth2Client,
            vpn_client_session::{VpnClientSession, VpnClientSessionState},
            vpn_session_stats::VpnSessionStats,
        },
    },
    types::user_info::UserInfo,
};
use defguard_core::{
    events::ApiEventType,
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
    let recovery_codes = vec!["recovery-code-1".to_string(), "recovery-code-2".to_string()];

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

    // first email received is regarding admin login
    // assert_eq!(mail.to(), "admin@defguard");
    // assert_eq!(
    //     mail.subject(),
    //     "Defguard: new device logged in to your account"
    // );

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

    // send email regarding new device being added
    // it does not contain session info
    // assert_eq!(mail.to(), "h.potter@hogwart.edu.uk");
    // assert_eq!(mail.subject(), "Defguard: new device added to your account");
    // assert!(!mail.content().contains("IP Address:</span>"));
    // assert!(!mail.content().contains("Device type:</span>"));

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

    // send email regarding new device being added
    // it should contain session info
    // assert_eq!(mail.to(), "admin@defguard");
    // assert_eq!(mail.subject(), "Defguard: new device added to your account");
    // assert!(mail.content().contains("IP Address:</span> 127.0.0.1"));
    // assert!(
    //     mail.content()
    //         .contains("Device type:</span> iPhone, OS: iOS 17.1, Mobile Safari")
    // );

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

    // send email regarding user login
    // assert_eq!(mail.to(), "h.potter@hogwart.edu.uk");
    // assert_eq!(
    //     mail.subject(),
    //     "Defguard: new device logged in to your account"
    // );
    // assert!(mail.content().contains("IP Address:</span> 127.0.0.1"));
    // assert!(
    //     mail.content()
    //         .contains("Device type:</span> iPhone, OS: iOS 17.1, Mobile Safari")
    // );

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

    // send email regarding new device being added
    // assert_eq!(mail.to(), "h.potter@hogwart.edu.uk");
    // assert_eq!(mail.subject(), "Defguard: new device added to your account");
    // assert!(mail.content().contains("IP Address:</span> 127.0.0.1"));
    // assert!(
    //     mail.content()
    //         .contains("Device type:</span> iPhone, OS: iOS 17.1, Mobile Safari")
    // );

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
