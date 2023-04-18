use defguard::{
    build_webapp,
    db::{models::NewOpenIDClient, AppEvent, GatewayEvent},
    grpc::{GatewayState, WorkerState},
    handlers::Auth,
};
use rocket::serde::json::json;
use rocket::{
    http::{ContentType, Status},
    local::asynchronous::Client,
};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::unbounded_channel;

mod common;
use common::{init_test_db, LICENSE_ENTERPRISE};
use defguard::db::models::oauth2client::OAuth2Client;
use defguard::db::User;

async fn make_client() -> Client {
    let (pool, mut config) = init_test_db().await;
    config.license = LICENSE_ENTERPRISE.into();

    User::init_admin_user(&pool, &config.default_admin_password)
        .await
        .unwrap();

    let mut user = User::new(
        "hpotter".into(),
        "pass123",
        "Potter".into(),
        "Harry".into(),
        "h.potter@hogwart.edu.uk".into(),
        None,
    );
    user.save(&pool).await.unwrap();

    let (tx, rx) = unbounded_channel::<AppEvent>();
    let worker_state = Arc::new(Mutex::new(WorkerState::new(tx.clone())));
    let (wg_tx, wg_rx) = unbounded_channel::<GatewayEvent>();
    let gateway_state = Arc::new(Mutex::new(GatewayState::new(wg_rx)));

    let webapp = build_webapp(config, tx, rx, wg_tx, worker_state, gateway_state, pool).await;
    Client::tracked(webapp).await.unwrap()
}

#[rocket::async_test]
async fn test_authorize() {
    let client = make_client().await;

    let auth = Auth::new("admin".into(), "pass123".into());
    let response = client.post("/api/v1/auth").json(&auth).dispatch().await;
    assert_eq!(response.status(), Status::Ok);

    // create OAuth2 client
    let oauth2client = NewOpenIDClient {
        name: "My test client".into(),
        redirect_uri: vec!["http://test.server.tnt:12345/".into()],
        scope: vec!["openid".into()],
        enabled: true,
    };
    let response = client
        .post("/api/v1/oauth")
        .json(&oauth2client)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Created);

    // wrong response type
    let response = client
        .get(
            "/api/v1/oauth/authorize?\
            response_type=wrong&\
            client_id=MyClient&\
            redirect_uri=http%3A%2F%2Flocalhost%3A3000%2F&\
            scope=default-scope&\
            state=ABCDEF",
        )
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::NotFound);

    let response = client
        .get(
            "/api/v1/oauth/authorize?\
            response_type=code&\
            client_id=MyClient&\
            redirect_uri=http%3A%2F%2Flocalhost%3A3000%2F&\
            scope=default-scope&\
            state=ABCDEF",
        )
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Found);
}

#[rocket::async_test]
async fn test_openid_app_management_access() {
    let client = make_client().await;

    // login as admin
    let auth = Auth::new("admin".into(), "pass123".into());
    let response = client.post("/api/v1/auth").json(&auth).dispatch().await;
    assert_eq!(response.status(), Status::Ok);

    // add app
    let oauth2client = NewOpenIDClient {
        name: "My test client".into(),
        redirect_uri: vec!["http://test.server.tnt:12345/".into()],
        scope: vec!["openid".into()],
        enabled: true,
    };
    let response = client
        .post("/api/v1/oauth")
        .json(&oauth2client)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Created);

    // list apps
    let response = client.get("/api/v1/oauth").dispatch().await;
    assert_eq!(response.status(), Status::Ok);
    let apps: Vec<OAuth2Client> = response.into_json().await.unwrap();
    assert_eq!(apps.len(), 1);
    let test_app = &apps[0];
    assert_eq!(test_app.name, oauth2client.name);

    // fetch app details
    let response = client
        .get(format!("/api/v1/oauth/{}", test_app.client_id))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);
    let app: OAuth2Client = response.into_json().await.unwrap();
    assert_eq!(app.name, oauth2client.name);

    // edit app
    let oauth2client = NewOpenIDClient {
        name: "Changed test client".into(),
        redirect_uri: vec!["http://test.server.tnt:12345/".into()],
        scope: vec!["openid email".into()],
        enabled: true,
    };
    let response = client
        .put(format!("/api/v1/oauth/{}", test_app.client_id))
        .json(&oauth2client)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);

    // change app state
    let data = json!(
        {"enabled": false}
    );
    let response = client
        .post(format!("/api/v1/oauth/{}", test_app.client_id))
        .json(&data)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);

    // fetch changed app details
    let response = client
        .get(format!("/api/v1/oauth/{}", test_app.client_id))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);
    let app: OAuth2Client = response.into_json().await.unwrap();
    assert_eq!(app.name, oauth2client.name);
    assert_eq!(app.enabled, false);

    // delete app
    let response = client
        .delete(format!("/api/v1/oauth/{}", test_app.client_id))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);

    // list apps
    let response = client.get("/api/v1/oauth").dispatch().await;
    assert_eq!(response.status(), Status::Ok);
    let apps: Vec<OAuth2Client> = response.into_json().await.unwrap();
    assert_eq!(apps.len(), 0);

    // add another app for further testing
    let oauth2client = NewOpenIDClient {
        name: "New test client".into(),
        redirect_uri: vec!["http://test.server.tnt:12345/".into()],
        scope: vec!["openid phone".into()],
        enabled: true,
    };
    let response = client
        .post("/api/v1/oauth")
        .json(&oauth2client)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Created);
    let response = client.get("/api/v1/oauth").dispatch().await;
    assert_eq!(response.status(), Status::Ok);
    let apps: Vec<OAuth2Client> = response.into_json().await.unwrap();
    let test_app = &apps[0];

    // // login as standard user
    let auth = Auth::new("hpotter".into(), "pass123".into());
    let response = client.post("/api/v1/auth").json(&auth).dispatch().await;
    assert_eq!(response.status(), Status::Ok);

    // standard user cannot list apps
    let response = client.get("/api/v1/oauth").dispatch().await;
    assert_eq!(response.status(), Status::Forbidden);

    // standard user cannot get app details
    let response = client
        .get(format!("/api/v1/oauth/{}", test_app.client_id))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Forbidden);

    // standard user cannot add apps
    let oauth2client = NewOpenIDClient {
        name: "Another test client".into(),
        redirect_uri: vec!["http://test.com/redirect".into()],
        scope: vec!["openid profile".into()],
        enabled: true,
    };
    let response = client
        .post("/api/v1/oauth")
        .json(&oauth2client)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Forbidden);

    // standard user cannot edit apps
    let response = client
        .put(format!("/api/v1/oauth/{}", test_app.client_id))
        .json(&oauth2client)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Forbidden);

    // standard user cannot change app status
    let data = json!(
        {"enabled": false}
    );
    let response = client
        .post(format!("/api/v1/oauth/{}", test_app.client_id))
        .json(&data)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Forbidden);

    // standard user cannot delete apps
    let response = client
        .delete(format!("/api/v1/oauth/{}", test_app.client_id))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Forbidden);
}

// #[rocket::async_test]
// async fn test_authorize_consent() {
//     let client = make_client().await;

//     let auth = Auth::new("admin".into(), "pass123".into());
//     let response = client.post("/api/v1/auth").json(&auth).dispatch().await;
//     assert_eq!(response.status(), Status::Ok);

//     let response = client
//         .post("/api/v1/user/admin/oauth2client")
//         .json(&json!({
//             "client_id": "MyClient",
//             "client_secret": "secret",
//             "redirect_uri": "http://localhost:3000/",
//             "scope": ["default-scope"],
//             "name": "Test",
//             "enabled": true,
//         }))
//         .dispatch()
//         .await;
//     assert_eq!(response.status(), Status::Ok);

//     let response = client
//         .post(
//             "/api/v1/oauth/authorize?\
//             allow=true&\
//             response_type=code&\
//             client_id=MyClient&\
//             redirect_uri=http%3A%2F%2Flocalhost%3A3000%2F&\
//             scope=default-scope&\
//             state=ABCDEF",
//         )
//         .dispatch()
//         .await;
//     assert_eq!(response.status(), Status::Found);

//     let localtion = response.headers().get_one("Location").unwrap();
//     assert!(localtion.starts_with("http://localhost:3000/?code="));

//     // extract code
//     let index = localtion.find("&state").unwrap();
//     let code = localtion.get(28..index).unwrap();

//     let response = client
//         .post("/api/v1/oauth/token")
//         .header(ContentType::Form)
//         .header(Header::new(
//             "Authorization",
//             // echo -n 'LocalClient:secret' | base64
//             "Basic TG9jYWxDbGllbnQ6c2VjcmV0",
//         ))
//         .body(format!(
//             "grant_type=authorization_code&\
//             code={}&\
//             redirect_uri=http%3A%2F%2Flocalhost%3A3000%2F",
//             code
//         ))
//         .dispatch()
//         .await;
//     assert_eq!(response.status(), Status::Ok);
// }

// #[rocket::async_test]
// async fn test_authorize_consent_wrong_client() {
//     let client = make_client().await;

//     let response = client
//         .post(
//             "/api/v1/oauth/authorize?\
//             allow=true&\
//             response_type=code&\
//             client_id=NonExistentClient&\
//             redirect_uri=http%3A%2F%2Flocalhost%3A3000%2F&\
//             scope=default-scope&\
//             state=ABCDEF",
//         )
//         .dispatch()
//         .await;
//     assert_eq!(response.status(), Status::BadRequest);
// }

#[rocket::async_test]
async fn test_token_client_credentials() {
    let client = make_client().await;

    let response = client
        .post("/api/v1/oauth/token")
        .header(ContentType::Form)
        .body("client_id=WrongClient&client_secret=WrongSecret&grant_type=code")
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::BadRequest);
}
