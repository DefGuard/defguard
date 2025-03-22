use defguard_core::{handlers::Auth, SERVER_CONFIG};
use reqwest::StatusCode;

use crate::common::{client::TestClient, make_test_client, X_FORWARDED_HOST, X_FORWARDED_URI};

async fn make_client() -> TestClient {
    let (client, _client_state) = make_test_client().await;
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
    let auth = Auth::new("hpotter", "pass123");
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
