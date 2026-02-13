use defguard_common::{
    db::{
        Id,
        models::{device::AddDevice, oauth2client::OAuth2Client},
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
