use chrono::{TimeDelta, Utc};
use defguard::{
    config::DefGuardConfig,
    enterprise::{
        handlers::openid_providers::AddProviderData,
        license::{set_cached_license, License},
    },
    handlers::Auth,
};
use reqwest::{StatusCode, Url};
use serde::Deserialize;
use sqlx::PgPool;

mod common;
use self::common::{client::TestClient, make_base_client, make_test_client};

async fn make_client() -> TestClient {
    let (client, _) = make_test_client().await;
    client
}

#[allow(dead_code)]
async fn make_client_v2(pool: PgPool, config: DefGuardConfig) -> TestClient {
    let (client, _) = make_base_client(pool, config).await;
    client
}

#[tokio::test]
async fn test_openid_providers() {
    let client = make_client().await;

    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    let provider_data = AddProviderData::new(
        "test",
        "https://accounts.google.com",
        "client_id",
        "client_secret",
    );

    let response = client
        .post("/api/v1/openid/provider")
        .json(&provider_data)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::CREATED);

    let response = client.get("/api/v1/openid/auth_info").send().await;

    assert_eq!(response.status(), StatusCode::OK);

    #[derive(Deserialize)]
    struct UrlResponse {
        url: String,
    }

    let provider: UrlResponse = response.json::<UrlResponse>().await;

    let url = Url::parse(&provider.url).unwrap();

    let client_id = url
        .query_pairs()
        .find(|(key, _)| key == "client_id")
        .unwrap();
    assert_eq!(client_id.1, "client_id");

    let nonce = url.query_pairs().find(|(key, _)| key == "nonce");
    assert!(nonce.is_some());
    let state = url.query_pairs().find(|(key, _)| key == "state");
    assert!(state.is_some());
    let redirect_uri = url.query_pairs().find(|(key, _)| key == "redirect_uri");
    assert!(redirect_uri.is_some());

    // Test that the endpoint is forbidden when the license is expired
    let new_license = License {
        customer_id: "test".to_string(),
        subscription: false,
        valid_until: Some(Utc::now() - TimeDelta::days(1)),
    };
    set_cached_license(Some(new_license));
    let response = client.get("/api/v1/openid/auth_info").send().await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}
