mod common;

use defguard::{db::Wallet, handlers::Auth, SERVER_CONFIG};
use reqwest::StatusCode;

use self::common::{client::TestClient, make_test_client, X_FORWARDED_HOST, X_FORWARDED_URI};

async fn make_client() -> TestClient {
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

#[tokio::test]
async fn test_forward_auth() {
    let mut client = make_client().await;

    // auth request from reverse proxy
    let response = client
        .get("/api/v1/forward_auth")
        .header(X_FORWARDED_HOST, "app.example.com")
        .header(X_FORWARDED_URI, "/test")
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::TEMPORARY_REDIRECT);
    let headers = response.headers();
    assert_eq!(
        headers.get("location").unwrap().to_str().unwrap(),
        format!(
            "{}auth/login?r={}",
            SERVER_CONFIG.get().unwrap().url,
            "http://app.example.com/test"
        )
    );

    // login
    let auth = Auth::new("hpotter".into(), "pass123".into());
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // store auth cookie for later use
    let auth_cookie = response
        .cookies()
        .find(|c| c.name() == "defguard_session")
        .unwrap();

    // make another auth request after logging in
    client.set_cookie(&auth_cookie);
    let response = client
        .get("/api/v1/forward_auth")
        .header(X_FORWARDED_HOST, "app.example.com")
        .header(X_FORWARDED_URI, "/test")
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
}
