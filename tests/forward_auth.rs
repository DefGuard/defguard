use defguard::{db::Wallet, handlers::Auth, SERVER_CONFIG};
use rocket::{
    http::{Cookie, Header, Status},
    local::asynchronous::Client,
};

mod common;
use self::common::make_test_client;

async fn make_client() -> Client {
    let (client, client_state) = make_test_client().await;

    let mut wallet = Wallet::new_for_user(
        client_state.test_user.id.unwrap(),
        "0x4aF8803CBAD86BA65ED347a3fbB3fb50e96eDD3e".into(),
        "test".into(),
        5,
        String::new(),
    );
    wallet.save(&client_state.pool).await.unwrap();

    client
}

#[rocket::async_test]
async fn test_forward_auth() {
    let client = make_client().await;

    // auth request from reverse proxy
    let response = client
        .get("/api/v1/forward_auth")
        .header(Header::new("x-forwarded-host", "app.example.com"))
        .header(Header::new("x-forwarded-uri", "/test"))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::TemporaryRedirect);
    let headers = response.headers();
    assert_eq!(
        headers.get_one("location").unwrap(),
        format!(
            "{}auth/login?r={}",
            SERVER_CONFIG.get().unwrap().url,
            "http://app.example.com/test"
        )
    );

    // login
    let auth = Auth::new("hpotter".into(), "pass123".into());
    let response = client.post("/api/v1/auth").json(&auth).dispatch().await;
    assert_eq!(response.status(), Status::Ok);

    // store auth cookie for later use
    let auth_cookie = response.cookies().get("defguard_session").unwrap().value();

    // make another auth request after logging in
    let response = client
        .get("/api/v1/forward_auth")
        .cookie(Cookie::new("defguard_session", auth_cookie))
        .header(Header::new("x-forwarded-host", "app.example.com"))
        .header(Header::new("x-forwarded-uri", "/test"))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);
}
