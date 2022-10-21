use defguard::{
    enterprise::handlers::oauth::{authorize, authorize_consent, refresh, token},
    enterprise::oauth_state::OAuthState,
};
use rocket::{
    http::{ContentType, Header, Status},
    local::asynchronous::Client,
    routes,
};

mod common;
use common::init_test_db;

async fn make_client() -> Client {
    let (pool, _config) = init_test_db().await;
    let webapp = rocket::build().manage(OAuthState::new(pool).await).mount(
        "/api/oauth",
        routes![authorize, authorize_consent, token, refresh],
    );
    Client::tracked(webapp).await.unwrap()
}

#[rocket::async_test]
async fn test_authorize() {
    let client = make_client().await;

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
