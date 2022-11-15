use defguard::{
    build_webapp,
    db::{AppEvent, GatewayEvent, User},
    handlers::Auth,
};
use rocket::{
    http::{ContentType, Header, Status},
    local::asynchronous::Client,
};
use serde::Serialize;
use tokio::sync::mpsc::unbounded_channel;

mod common;
use common::{init_test_db, LICENSE_ENTERPRISE};

async fn make_client() -> Client {
    let (pool, mut config) = init_test_db().await;
    config.license = LICENSE_ENTERPRISE.into();

    User::new(
        "hpotter".into(),
        "pass123",
        "Potter".into(),
        "Harry".into(),
        "h.potter@hogwart.edu.uk".into(),
        None,
    )
    .save(&pool)
    .await
    .unwrap();

    let (tx, rx) = unbounded_channel::<AppEvent>();
    let (wg_tx, _) = unbounded_channel::<GatewayEvent>();

    let webapp = build_webapp(config, tx, rx, wg_tx, pool).await;
    Client::tracked(webapp).await.unwrap()
}

#[derive(Serialize)]
pub struct OAuth2Client {
    client_id: String,
    client_secret: String,
    redirect_uri: String,
    scope: String,
}

#[rocket::async_test]
async fn test_authorize() {
    let client = make_client().await;

    let auth = Auth::new("hpotter".into(), "pass123".into());
    let response = client.post("/api/v1/auth").json(&auth).dispatch().await;
    assert_eq!(response.status(), Status::Ok);

    let oc = OAuth2Client {
        client_id: "LocalClient".into(),
        client_secret: "secret".into(),
        redirect_uri: "http://localhost:3000/".into(),
        scope: "default-scope".into(),
    };
    let response = client
        .post("/api/v1/user/hpotter/oauth2client")
        .json(&oc)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);

    let response = client
        .get(
            "/api/oauth/authorize?\
            response_type=code&\
            client_id=LocalClient&\
            redirect_uri=http%3A%2F%2Flocalhost%3A3000%2F&\
            scope=default-scope&\
            state=ABCDEF",
        )
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Found);
}

#[rocket::async_test]
async fn test_authorize_consent() {
    let client = make_client().await;

    let auth = Auth::new("hpotter".into(), "pass123".into());
    let response = client.post("/api/v1/auth").json(&auth).dispatch().await;
    assert_eq!(response.status(), Status::Ok);

    let oc = OAuth2Client {
        client_id: "LocalClient".into(),
        client_secret: "secret".into(),
        redirect_uri: "http://localhost:3000/".into(),
        scope: "default-scope".into(),
    };
    let response = client
        .post("/api/v1/user/hpotter/oauth2client")
        .json(&oc)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);

    let response = client
        .post(
            "/api/oauth/authorize?\
            allow=true&\
            response_type=code&\
            client_id=LocalClient&\
            redirect_uri=http%3A%2F%2Flocalhost%3A3000%2F&\
            scope=default-scope&\
            state=ABCDEF",
        )
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Found);

    let localtion = response.headers().get_one("Location").unwrap();
    assert!(localtion.starts_with("http://localhost:3000/?code="));

    // extract code
    let index = localtion.find("&state").unwrap();
    let code = localtion.get(28..index).unwrap();

    let response = client
        .post("/api/oauth/token")
        .header(ContentType::Form)
        .header(Header::new(
            "Authorization",
            // echo -n 'LocalClient:secret' | base64
            "Basic TG9jYWxDbGllbnQ6c2VjcmV0",
        ))
        .body(format!(
            "grant_type=authorization_code&\
            code={}&\
            redirect_uri=http%3A%2F%2Flocalhost%3A3000%2F",
            code
        ))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);
}

#[rocket::async_test]
async fn test_authorize_consent_wrong_client() {
    let client = make_client().await;

    let response = client
        .post(
            "/api/oauth/authorize?\
            allow=true&\
            response_type=code&\
            client_id=NonExistentClient&\
            redirect_uri=http%3A%2F%2Flocalhost%3A3000%2F&\
            scope=default-scope&\
            state=ABCDEF",
        )
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::BadRequest);
}
