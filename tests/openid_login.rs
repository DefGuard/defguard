use std::str::FromStr;

use axum::http::header::ToStrError;
use claims::assert_err;
use defguard::{
    config::DefGuardConfig,
    db::{
        models::{oauth2client::OAuth2Client, NewOpenIDClient},
        DbPool,
    },
    enterprise::handlers::openid_providers::AddProviderData,
    handlers::Auth,
};
use openidconnect::{
    core::{
        CoreClient, CoreGenderClaim, CoreProviderMetadata, CoreResponseType, CoreTokenResponse,
    },
    http::Method,
    AuthenticationFlow, AuthorizationCode, ClientId, ClientSecret, CsrfToken,
    EmptyAdditionalClaims, HttpRequest, HttpResponse, IssuerUrl, Nonce, OAuth2TokenResponse,
    PkceCodeChallenge, RedirectUrl, Scope, UserInfoClaims,
};
use reqwest::{
    header::{HeaderName, AUTHORIZATION, CONTENT_TYPE, USER_AGENT},
    StatusCode, Url,
};
use rsa::RsaPrivateKey;
use serde::Deserialize;

mod common;
use self::common::{client::TestClient, init_test_db, make_base_client, make_test_client};

async fn make_client() -> TestClient {
    let (client, _) = make_test_client().await;
    client
}

async fn make_client_v2(pool: DbPool, config: DefGuardConfig) -> TestClient {
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
        "test".to_string(),
        "https://accounts.google.com".to_string(),
        "client_id".to_string(),
        "client_secret".to_string(),
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
}
