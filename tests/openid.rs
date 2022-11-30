use std::sync::{Arc, Mutex};

use defguard::{
    build_webapp,
    db::{AppEvent, GatewayEvent},
    enterprise::db::openid::{AuthorizedApp, NewOpenIDClient, OpenIDClient},
    grpc::GatewayState,
    handlers::Auth,
};
use rocket::{
    http::{ContentType, Status},
    local::asynchronous::Client,
};
use tokio::sync::mpsc::unbounded_channel;

mod common;
use common::{init_test_db, LICENSE_ENTERPRISE};

async fn make_client() -> Client {
    let (pool, mut config) = init_test_db().await;
    config.license = LICENSE_ENTERPRISE.into();

    let (tx, rx) = unbounded_channel::<AppEvent>();

    let (wg_tx, wg_rx) = unbounded_channel::<GatewayEvent>();
    let gateway_state = Arc::new(Mutex::new(GatewayState::new(wg_rx)));

    let webapp = build_webapp(config, tx, rx, wg_tx, gateway_state, pool).await;
    Client::tracked(webapp).await.unwrap()
}

#[rocket::async_test]
async fn test_openid_client() {
    let client = make_client().await;

    let auth = Auth::new("admin".into(), "pass123".into());
    let response = client.post("/api/v1/auth").json(&auth).dispatch().await;
    assert_eq!(response.status(), Status::Ok);

    let mut openid_client = NewOpenIDClient {
        name: "Test".into(),
        description: "Test".into(),
        home_url: "http://localhost:3000".into(),
        redirect_uri: "http://localhost:3000/".into(),
        enabled: true,
    };

    let response = client
        .post("/api/v1/openid")
        .json(&openid_client)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Created);

    let response = client.get("/api/v1/openid").dispatch().await;
    assert_eq!(response.status(), Status::Ok);
    let openid_clients: Vec<OpenIDClient> = response.into_json().await.unwrap();
    assert_eq!(openid_clients.len(), 1);

    openid_client.description = "Changed".into();
    openid_client.name = "Test changed".into();
    let response = client
        .put(format!("/api/v1/openid/{}", openid_clients[0].id.unwrap()))
        .json(&openid_client)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);

    let response = client
        .get(format!("/api/v1/openid/{}", openid_clients[0].id.unwrap()))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);
    let fetched_client: OpenIDClient = response.into_json().await.unwrap();
    assert_eq!(fetched_client.home_url, openid_client.home_url);
    assert_eq!(fetched_client.description, openid_client.description);
    assert_eq!(fetched_client.name, openid_client.name);

    // Openid flow tests
    // test unsupported response type
    // Test client delete
    let response = client
        .delete(format!("/api/v1/openid/{}", openid_clients[0].id.unwrap()))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);

    let response = client.get("/api/v1/openid").dispatch().await;
    assert_eq!(response.status(), Status::Ok);

    let openid_clients: Vec<OpenIDClient> = response.into_json().await.unwrap();
    assert!(openid_clients.is_empty());
}

#[rocket::async_test]
async fn test_openid_flow() {
    let client = make_client().await;
    let auth = Auth::new("admin".into(), "pass123".into());
    let response = client.post("/api/v1/auth").json(&auth).dispatch().await;
    assert_eq!(response.status(), Status::Ok);
    let openid_client = NewOpenIDClient {
        name: "Test".into(),
        description: "Test".into(),
        home_url: "http://localhost:3000".into(),
        redirect_uri: "http://localhost:3000/".into(),
        enabled: true,
    };

    let response = client
        .post("/api/v1/openid")
        .json(&openid_client)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Created);
    let openid_client: OpenIDClient = response.into_json().await.unwrap();
    assert_eq!(openid_client.name, "Test");

    // all clients
    let response = client.get("/api/v1/openid").dispatch().await;
    assert_eq!(response.status(), Status::Ok);

    let response = client
        .post(format!(
            "/api/v1/openid/authorize?\
            response_type=code%20id_token%20token&\
            client_id={}&\
            redirect_uri=http%3A%2F%2Flocalhost%3A3000%2F&\
            scope=openid&\
            state=ABCDEF&\
            allow=true&\
            nonce=blabla",
            openid_client.client_id
        ))
        .dispatch()
        .await;
    let location = response.headers().get_one("Location").unwrap();
    assert!(location.contains("error=unsupported_response_type"));

    // unsupported_response_type
    let response = client
        .post(format!(
            "/api/v1/openid/authorize?\
            response_type=code%20id_token%20token&\
            client_id={}&\
            redirect_uri=http%3A%2F%2Flocalhost%3A3000%2F&\
            scope=openid&\
            state=ABCDEF&\
            allow=true&\
            nonce=blabla",
            openid_client.client_id
        ))
        .dispatch()
        .await;
    let location = response.headers().get_one("Location").unwrap();
    assert!(location.contains("error=unsupported_response_type"));

    let response = client
        .post(format!(
            "/api/v1/openid/authorize?\
            response_type=id_token&\
            client_id={}&\
            redirect_uri=http%3A%2F%2Flocalhost%3A3000%2F&\
            scope=openid&\
            state=ABCDEF&\
            allow=true&\
            nonce=blabla",
            openid_client.client_id
        ))
        .dispatch()
        .await;
    let location = response.headers().get_one("Location").unwrap();
    assert!(location.contains("error=unsupported_response_type"));

    // Obtain code
    let response = client
        .post(format!(
            "/api/v1/openid/authorize?\
            response_type=code&\
            client_id={}&\
            redirect_uri=http%3A%2F%2Flocalhost%3A3000%2F&\
            scope=openid&\
            state=ABCDEF&\
            allow=true&\
            nonce=blabla",
            openid_client.client_id
        ))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Found);

    let location = response.headers().get_one("Location").unwrap();
    assert!(location.starts_with("http://localhost:3000/?code="));

    // check returned state
    let index = location.find("&state").unwrap();
    assert_eq!("&state=ABCDEF", location.get(index..).unwrap());
    // exchange wrong code for token should fail
    let response = client
        .post("/api/v1/openid/token")
        .header(ContentType::Form)
        .body(
            "grant_type=authorization_code&\
            code=ncuoew2323&\
            redirect_uri=http%3A%2F%2Flocalhost%3A3000%2F",
        )
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::BadRequest);

    // exchange code for token
    let code = location.get(28..index).unwrap();
    let response = client
        .post("/api/v1/openid/token")
        .header(ContentType::Form)
        .body(format!(
            "grant_type=authorization_code&\
            code={}&\
            redirect_uri=http%3A%2F%2Flocalhost%3A3000%2F",
            code
        ))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);

    // check used code
    let response = client
        .post("/api/v1/openid/token")
        .header(ContentType::Form)
        .body(format!(
            "grant_type=authorization_code&\
            code={}&\
            redirect_uri=http%3A%2F%2Flocalhost%3A3000%2F",
            code
        ))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::BadRequest);

    // test non-existing client
    let response = client
        .post(
            "/api/v1/openid/authorize?\
            response_type=code&\
            client_id=666&\
            redirect_uri=http%3A%2F%2Flocalhost%3A3000%2F&\
            scope=openid&\
            state=ABCDEF&\
            nonce=blabla",
        )
        .dispatch()
        .await;
    let location = response.headers().get_one("Location").unwrap();
    assert!(location.contains("error"));

    // test wrong redirect uri
    let response = client
        .post(
            "/api/v1/openid/authorize?\
            response_type=code&\
            client_id=1&\
            redirect_uri=http%3A%2F%example%3A3000%2F&\
            scope=openid&\
            state=ABCDEF&\
            nonce=blabla",
        )
        .dispatch()
        .await;
    let location = response.headers().get_one("Location").unwrap();
    assert!(location.contains("error"));

    // test scope doesnt contain openid
    let response = client
        .post(format!(
            "/api/v1/openid/authorize?\
            response_type=code&\
            client_id={}&\
            redirect_uri=http%3A%2F%2Flocalhost%3A3000%2F&\
            scope=blabla&\
            state=ABCDEF&\
            allow=true&\
            nonce=blabla",
            openid_client.client_id
        ))
        .dispatch()
        .await;
    let location = response.headers().get_one("Location").unwrap();
    assert!(location.contains("error=wrong_scope&error_description=scope_must_contain_openid"));

    // test allow false
    let response = client
        .post(format!(
            "/api/v1/openid/authorize?\
            response_type=code&\
            client_id={}&\
            redirect_uri=http%3A%2F%2Flocalhost%3A3000%2F&\
            scope=blabla&\
            state=ABCDEF&\
            allow=false&\
            nonce=blabla",
            openid_client.client_id
        ))
        .dispatch()
        .await;
    let location = response.headers().get_one("Location").unwrap();
    assert!(location.contains("error=user_unauthorized"));
}

#[rocket::async_test]
async fn test_openid_apps() {
    let client = make_client().await;

    let auth = Auth::new("admin".into(), "pass123".into());
    let response = client.post("/api/v1/auth").json(&auth).dispatch().await;
    assert_eq!(response.status(), Status::Ok);

    let openid_client = NewOpenIDClient {
        name: "Test".into(),
        description: "Test".into(),
        home_url: "http://localhost:3000".into(),
        redirect_uri: "http://localhost:3000/".into(),
        enabled: true,
    };
    let response = client
        .post("/api/v1/openid")
        .json(&openid_client)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Created);
    let fetched_client: OpenIDClient = response.into_json().await.unwrap();
    assert_eq!(fetched_client.name, "Test");

    let response = client
        .post(format!(
            "/api/v1/openid/authorize?\
            response_type=code&\
            client_id={}&\
            redirect_uri=http%3A%2F%2Flocalhost%3A3000%2F&\
            scope=openid&\
            state=ABCDEF&\
            allow=true&\
            nonce=blabla",
            fetched_client.client_id
        ))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Found);

    let location = response.headers().get_one("Location").unwrap();
    let index = location.find("&state").unwrap();
    let code = location.get(28..index).unwrap();
    let response = client
        .post("/api/v1/openid/token")
        .header(ContentType::Form)
        .body(format!(
            "grant_type=authorization_code&\
            code={}&\
            redirect_uri=http%3A%2F%2Flocalhost%3A3000%2F",
            code
        ))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);

    // fetch applications
    let response = client.get("/api/v1/openid/apps/admin").dispatch().await;
    assert_eq!(response.status(), Status::Ok);
    let mut apps: Vec<AuthorizedApp> = response.into_json().await.unwrap();
    assert_eq!(apps.len(), 1);

    let mut app = apps.pop().unwrap();
    assert_eq!(app.name, "Test");

    // rename application
    app.name = "My app".into();
    let response = client
        .put(format!("/api/v1/openid/apps/{}", app.id.unwrap()))
        .json(&app)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);

    // fetch again to check if the name has been changed
    let response = client.get("/api/v1/openid/apps/admin").dispatch().await;
    assert_eq!(response.status(), Status::Ok);
    let apps: Vec<AuthorizedApp> = response.into_json().await.unwrap();
    assert_eq!(apps.len(), 1);
    assert_eq!(apps[0].name, "My app");

    // delete application
    let response = client
        .delete(format!("/api/v1/openid/apps/{}", app.id.unwrap()))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);

    // fetch once more to check if the application has been deleted
    let response = client.get("/api/v1/openid/apps/admin").dispatch().await;
    assert_eq!(response.status(), Status::Ok);
    let apps: Vec<AuthorizedApp> = response.into_json().await.unwrap();
    assert_eq!(apps.len(), 0);
}
