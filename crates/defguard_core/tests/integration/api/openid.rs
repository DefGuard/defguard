use std::str::FromStr;

use axum::http::header::ToStrError;
use defguard_common::db::{
    Id,
    models::{OAuth2AuthorizedApp, Settings, User, oauth2client::OAuth2Client},
};
use defguard_core::handlers::{Auth, openid_clients::NewOpenIDClient};
use openidconnect::{
    AuthenticationFlow, AuthorizationCode, ClientId, ClientSecret, CsrfToken,
    EmptyAdditionalClaims, HttpRequest, HttpResponse, IssuerUrl, Nonce, OAuth2TokenResponse,
    PkceCodeChallenge, RedirectUrl, Scope, UserInfoClaims,
    core::{
        CoreClient, CoreGenderClaim, CoreProviderMetadata, CoreResponseType, CoreTokenResponse,
    },
    http::Method,
};
use reqwest::{
    StatusCode, Url,
    header::{AUTHORIZATION, CONTENT_TYPE, HeaderName, LOCATION, USER_AGENT},
};
use rsa::RsaPrivateKey;
use serde::Deserialize;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

use super::{
    TEST_SERVER_URL,
    common::{
        client::{TestClient, TestResponse},
        make_client, make_test_client, setup_pool,
    },
};

#[derive(Deserialize)]
pub struct AuthenticationResponse<'r> {
    pub code: &'r str,
    pub state: &'r str,
}

#[sqlx::test]
async fn test_openid_client(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let client = make_client(pool).await;

    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    let mut openid_client = NewOpenIDClient {
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

    let response = client.get("/api/v1/oauth").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let openid_clients: Vec<OAuth2Client<Id>> = response.json().await;
    assert_eq!(openid_clients.len(), 1);

    openid_client.name = "Test changed".into();
    let response = client
        .put(format!("/api/v1/oauth/{}", openid_clients[0].client_id))
        .json(&openid_client)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    let response = client
        .get(format!("/api/v1/oauth/{}", openid_clients[0].client_id))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let fetched_client: OAuth2Client<Id> = response.json().await;
    assert_eq!(fetched_client.name, openid_client.name);

    // OpenID flow tests
    // test unsupported response type
    // Test client delete
    let response = client
        .delete(format!("/api/v1/oauth/{}", openid_clients[0].client_id))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    let response = client.get("/api/v1/oauth").send().await;
    assert_eq!(response.status(), StatusCode::OK);

    let openid_clients: Vec<OAuth2Client<Id>> = response.json().await;
    assert!(openid_clients.is_empty());
}

#[sqlx::test]
async fn test_openid_flow(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (client, _) = make_test_client(pool).await;
    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let openid_client = NewOpenIDClient {
        name: "Test".into(),
        redirect_uri: vec![TEST_SERVER_URL.into(), "http://safe.net".into()],
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

    // all clients
    let response = client.get("/api/v1/oauth").send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // Try invalid request for `response_type = code id_token token`.
    let response = client
        .post(format!(
            "/api/v1/oauth/authorize?\
            response_type=code%20id_token%20token&\
            client_id={}&\
            redirect_uri=http%3A%2F%2Flocalhost%3A3000%2F&\
            scope=openid&\
            state=ABCDEF&\
            allow=true&\
            nonce=blabla",
            openid_client.client_id
        ))
        .send()
        .await;
    let location = response
        .headers()
        .get("Location")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(location.contains("error=invalid_request"));

    // Try invalid request for `response_type = id_token`.
    let response = client
        .post(format!(
            "/api/v1/oauth/authorize?\
            response_type=id_token&\
            client_id={}&\
            redirect_uri=http%3A%2F%2Flocalhost%3A3000%2F&\
            scope=openid&\
            state=ABCDEF&\
            allow=true&\
            nonce=blabla",
            openid_client.client_id
        ))
        .send()
        .await;
    let location = response
        .headers()
        .get("Location")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(location.contains("error=invalid_request"));

    // obtain authentication code
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

    let location = response
        .headers()
        .get("Location")
        .unwrap()
        .to_str()
        .unwrap();
    let (location, query) = location.split_once('?').unwrap();
    assert_eq!(location, TEST_SERVER_URL);
    let auth_response: AuthenticationResponse = serde_qs::from_str(query).unwrap();
    assert_eq!(auth_response.state, "ABCDEF");

    // Exchanging a wrong code for a token should fail.
    let response = client
        .post("/api/v1/oauth/token")
        .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(format!(
            "grant_type=authorization_code&\
            code=ncuoew2323&\
            redirect_uri=http%3A%2F%2Flocalhost%3A3000%2F&\
            client_id={}&\
            client_secret={}",
            openid_client.client_id, openid_client.client_secret
        ))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // Exchange correct code for a token.
    let token_body = format!(
        "grant_type=authorization_code&\
        code={}&\
        redirect_uri=http%3A%2F%2Flocalhost%3A3000%2F&\
        client_id={}&\
        client_secret={}",
        auth_response.code, openid_client.client_id, openid_client.client_secret
    );
    let response = client
        .post("/api/v1/oauth/token")
        .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(token_body.clone())
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    // Try to get another authentication code for the same code.
    let another_response = client
        .post("/api/v1/oauth/token")
        .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(token_body)
        .send()
        .await;
    assert_eq!(another_response.status(), StatusCode::BAD_REQUEST);

    // Make sure access token cannot be used to manage Defguard server itself.
    client.post("/api/v1/auth/logout").send().await;
    let token_response: CoreTokenResponse = response.json().await;
    let bearer = format!("Bearer {}", token_response.access_token().secret());
    let response = client
        .get("/api/v1/network")
        .header(AUTHORIZATION, &bearer)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let response = client
        .get("/api/v1/user")
        .header(AUTHORIZATION, &bearer)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // log back in
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    let fallback_url = Settings::url()
        .unwrap()
        .to_string()
        .trim_end_matches('/')
        .to_string();

    // check code cannot be reused
    let response = client
        .post("/api/v1/oauth/token")
        .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(format!(
            "grant_type=authorization_code&\
            code={}&\
            redirect_uri=http%3A%2F%2Flocalhost%3A3000%2F&\
            client_id={}&\
            client_secret={}",
            auth_response.code, openid_client.client_id, openid_client.client_secret
        ))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // test non-existing client
    let response = client
        .post(
            "/api/v1/oauth/authorize?\
            response_type=code&\
            client_id=666&\
            redirect_uri=http%3A%2F%2Flocalhost%3A3000%2F&\
            scope=openid&\
            state=ABCDEF&\
            nonce=blabla",
        )
        .send()
        .await;
    let location = response
        .headers()
        .get("Location")
        .unwrap()
        .to_str()
        .unwrap();
    assert_eq!(response.status(), StatusCode::FOUND);
    assert!(location.starts_with(&fallback_url));
    assert!(location.contains("error"));

    // test invalid redirect uri
    let response = client
        .post(format!(
            "/api/v1/oauth/authorize?\
            response_type=code&\
            client_id={}&\
            redirect_uri=http%3A%2F%example%3A3000%2F&\
            scope=openid&\
            state=ABCDEF&\
            nonce=blabla",
            openid_client.client_id
        ))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FOUND);
    assert!(location.starts_with(&fallback_url));

    // test non-whitelisted uri
    let response = client
        .post(format!(
            "/api/v1/oauth/authorize?\
            response_type=code&\
            client_id={}&\
            redirect_uri=http%3A%2F%2Fexample%3A3000%3Fvalue1=one%26value2=two&\
            scope=openid&\
            state=ABCDEF&\
            nonce=blabla",
            openid_client.client_id
        ))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FOUND);
    let location = response
        .headers()
        .get("Location")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(location.starts_with(&fallback_url));
    assert!(location.contains("error=access_denied"));

    // test whitelisted uri, invalid scope
    let response = client
        .post(format!(
            "/api/v1/oauth/authorize?\
            response_type=code&\
            client_id={}&\
            redirect_uri=http://safe.net&\
            scope=profile&\
            state=ABCDEF&\
            allow=true&\
            nonce=blabla",
            openid_client.client_id
        ))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FOUND);
    let location = response
        .headers()
        .get("Location")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(location.starts_with("http://safe.net"));
    assert!(location.contains("error=invalid_scope"));

    // test wrong redirect uri
    let response = client
        .post(format!(
            "/api/v1/oauth/authorize?\
            response_type=code&\
            client_id={}&\
            redirect_uri=http%3A%2F%2Fexample%3A3000%3Fvalue1=one%26value2=two&\
            scope=openid&\
            state=ABCDEF&\
            allow=true&\
            nonce=blabla",
            openid_client.client_id
        ))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FOUND);
    let location = response
        .headers()
        .get("Location")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(location.starts_with(&fallback_url));
    assert!(location.contains("error=access_denied"));

    // test allow false
    let response = client
        .post(format!(
            "/api/v1/oauth/authorize?\
            response_type=code&\
            client_id={}&\
            redirect_uri=http%3A%2F%2Flocalhost%3A3000%2F&\
            scope=blabla&\
            state=ABCDEF&\
            allow=false&\
            nonce=blabla",
            openid_client.client_id
        ))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FOUND);
    let location = response
        .headers()
        .get("Location")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(location.starts_with(TEST_SERVER_URL));
    assert!(location.contains("error=access_denied"));
}

/// Helper function for translating HTTP communication from `HttpRequest` to `LocalClient`.
async fn http_client(
    request: HttpRequest,
    client: &TestClient,
) -> Result<HttpResponse, ToStrError> {
    let uri = request.uri().path();
    eprintln!("HTTP client request path: {uri}");
    if let Some(query) = request.uri().query() {
        eprintln!("HTTP client request query: {query}");
    }
    eprintln!("HTTP client request headers: {:#?}", request.headers());
    if let Ok(text) = String::from_utf8(request.body().clone()) {
        eprintln!("HTTP client body: {text}");
    }
    let mut test_request = match *request.method() {
        Method::GET => client.get(uri),
        Method::HEAD => client.head(uri),
        Method::POST => client.post(uri),
        Method::PUT => client.put(uri),
        Method::DELETE => client.delete(uri),
        _ => unimplemented!(),
    };
    for (key, value) in request.headers() {
        test_request = test_request.header(
            HeaderName::from_str(key.as_str()).unwrap(),
            value.to_str().unwrap(),
        );
    }
    let response = test_request.body(request.body().clone()).send().await;
    let status_code = response.status();
    let headers = response.headers().clone();
    let body = response.bytes().await.to_vec();

    let mut http_response = HttpResponse::new(body);
    *http_response.status_mut() = status_code;
    *http_response.headers_mut() = headers;

    Ok(http_response)
}

static FAKE_REDIRECT_URI: &str = "http://test.server.tnt:12345/";

#[sqlx::test]
async fn test_openid_authorization_code(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (client, _) = make_test_client(pool).await;

    let issuer_url = IssuerUrl::from_url(Settings::url().unwrap().clone());

    // discover OpenID service
    let provider_metadata =
        CoreProviderMetadata::discover_async(issuer_url, &|r| http_client(r, &client))
            .await
            .unwrap();

    // create OAuth2 client
    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let oauth2client = NewOpenIDClient {
        name: "My test client".into(),
        redirect_uri: vec![FAKE_REDIRECT_URI.into()],
        scope: vec!["openid".into()],
        enabled: true,
    };
    let response = client
        .post("/api/v1/oauth")
        .json(&oauth2client)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let oauth2client: OAuth2Client<Id> = response.json().await;
    assert_eq!(oauth2client.name, "My test client");
    assert_eq!(oauth2client.scope[0], "openid");
    assert_eq!(oauth2client.client_id.len(), 16);
    assert_eq!(oauth2client.client_secret.len(), 32);

    // start the Authorization Code Flow
    let client_id = ClientId::new(oauth2client.client_id);
    let client_secret = ClientSecret::new(oauth2client.client_secret);
    let core_client =
        CoreClient::from_provider_metadata(provider_metadata, client_id, Some(client_secret))
            .set_redirect_uri(RedirectUrl::new(FAKE_REDIRECT_URI.into()).unwrap());
    let (authorize_url, _csrf_state, nonce) = core_client
        .authorize_url(
            AuthenticationFlow::<CoreResponseType>::AuthorizationCode,
            CsrfToken::new_random,
            Nonce::new_random,
        )
        .add_scope(Scope::new("email".to_string()))
        .add_scope(Scope::new("profile".to_string()))
        .url();
    assert_eq!(authorize_url.scheme(), "http");
    assert_eq!(authorize_url.host_str(), Some("localhost"));
    assert_eq!(authorize_url.path(), "/api/v1/oauth/authorize");

    // obtain authorization code
    let uri = format!(
        "{}?allow=true&{}",
        authorize_url.path(),
        authorize_url.query().unwrap()
    );
    let response = client.post(uri).send().await;
    assert_eq!(response.status(), StatusCode::FOUND);
    let location = response
        .headers()
        .get("Location")
        .unwrap()
        .to_str()
        .unwrap();
    let (location, query) = location.split_once('?').unwrap();
    assert_eq!(location, FAKE_REDIRECT_URI);
    let auth_response: AuthenticationResponse = serde_qs::from_str(query).unwrap();

    // exchange authorization code for token
    let token_response = core_client
        .exchange_code(AuthorizationCode::new(auth_response.code.into()))
        .unwrap()
        .request_async(&|r| http_client(r, &client))
        .await
        .unwrap();

    // verify id token
    let id_token_verifier = core_client.id_token_verifier().allow_any_alg();
    let _id_token_claims = token_response
        .extra_fields()
        .id_token()
        .expect("Server did not return an ID token")
        .claims(&id_token_verifier, &nonce)
        .unwrap();

    // refresh token
    let refresh_token = token_response.refresh_token().unwrap();
    let refresh_response = core_client
        .exchange_refresh_token(refresh_token)
        .unwrap()
        .request_async(&|r| http_client(r, &client))
        .await
        .unwrap();
    assert!(refresh_response.refresh_token().is_some());
}

#[sqlx::test]
async fn dg25_20_test_openid_disabled_client_doesnt_generate_code(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;

    let (client, _) = make_test_client(pool).await;

    let issuer_url = IssuerUrl::from_url(Settings::url().unwrap().clone());

    // discover OpenID service
    let provider_metadata =
        CoreProviderMetadata::discover_async(issuer_url, &|r| http_client(r, &client))
            .await
            .unwrap();

    // create OAuth2 client (initially enabled)
    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let oauth2client = NewOpenIDClient {
        name: "My test client".into(),
        redirect_uri: vec![FAKE_REDIRECT_URI.into()],
        scope: vec!["openid".into()],
        enabled: true,
    };
    let response = client
        .post("/api/v1/oauth")
        .json(&oauth2client)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let oauth2client: OAuth2Client<Id> = response.json().await;
    assert_eq!(oauth2client.name, "My test client");
    assert_eq!(oauth2client.scope[0], "openid");
    assert_eq!(oauth2client.client_id.len(), 16);
    assert_eq!(oauth2client.client_secret.len(), 32);

    let client_id = ClientId::new(oauth2client.client_id.clone());
    let client_secret = ClientSecret::new(oauth2client.client_secret);
    let core_client =
        CoreClient::from_provider_metadata(provider_metadata, client_id, Some(client_secret))
            .set_redirect_uri(RedirectUrl::new(FAKE_REDIRECT_URI.into()).unwrap());
    let (authorize_url, _csrf_state, _nonce) = core_client
        .authorize_url(
            AuthenticationFlow::<CoreResponseType>::AuthorizationCode,
            CsrfToken::new_random,
            Nonce::new_random,
        )
        .add_scope(Scope::new("email".to_string()))
        .add_scope(Scope::new("profile".to_string()))
        .url();
    assert_eq!(authorize_url.scheme(), "http");
    assert_eq!(authorize_url.host_str(), Some("localhost"));
    assert_eq!(authorize_url.path(), "/api/v1/oauth/authorize");

    // verify that authorization works when client is enabled
    let uri = format!(
        "{}?allow=true&{}",
        authorize_url.path(),
        authorize_url.query().unwrap()
    );
    let response = client.post(uri.clone()).send().await;
    assert_eq!(response.status(), StatusCode::FOUND);
    let location = response
        .headers()
        .get("Location")
        .unwrap()
        .to_str()
        .unwrap();
    let (location, query) = location.split_once('?').unwrap();
    assert_eq!(location, FAKE_REDIRECT_URI);
    let auth_response: AuthenticationResponse = serde_qs::from_str(query).unwrap();
    // Verify we got a valid authorization code
    assert!(!auth_response.code.is_empty());

    // Now disable the OAuth2 client
    let disabled_oauth2client = NewOpenIDClient {
        name: "My test client".into(),
        redirect_uri: vec![FAKE_REDIRECT_URI.into()],
        scope: vec!["openid".into()],
        enabled: false,
    };
    let response = client
        .put(format!("/api/v1/oauth/{}", oauth2client.client_id))
        .json(&disabled_oauth2client)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    let response = client.post(uri).send().await;
    assert_eq!(response.status(), StatusCode::FOUND);
    let location = response
        .headers()
        .get("Location")
        .unwrap()
        .to_str()
        .unwrap();

    assert!(location.contains("error=unauthorized_client"));
}

#[sqlx::test]
async fn dg25_25_openid_disabled_client_userinfo_fails(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;

    let (client, state) = make_test_client(pool).await;
    let mut config = state.config;

    let mut rng = rand::thread_rng();
    config.openid_signing_key = RsaPrivateKey::new(&mut rng, 2048).ok();

    let issuer_url = IssuerUrl::from_url(Settings::url().unwrap().clone());

    // discover OpenID service
    let provider_metadata =
        CoreProviderMetadata::discover_async(issuer_url, &|r| http_client(r, &client))
            .await
            .unwrap();

    // create OAuth2 client (initially enabled)
    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let oauth2client = NewOpenIDClient {
        name: "My test client".into(),
        redirect_uri: vec![FAKE_REDIRECT_URI.into()],
        scope: vec!["openid".into(), "email".into(), "profile".into()],
        enabled: true,
    };
    let response = client
        .post("/api/v1/oauth")
        .json(&oauth2client)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let oauth2client: OAuth2Client<Id> = response.json().await;
    assert_eq!(oauth2client.name, "My test client");

    // start the Authorization Code Flow with PKCE
    let client_id = ClientId::new(oauth2client.client_id.clone());
    let client_secret = ClientSecret::new(oauth2client.client_secret);
    let core_client =
        CoreClient::from_provider_metadata(provider_metadata, client_id, Some(client_secret))
            .set_redirect_uri(RedirectUrl::new(FAKE_REDIRECT_URI.into()).unwrap());
    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
    let (authorize_url, _csrf_state, nonce) = core_client
        .authorize_url(
            AuthenticationFlow::<CoreResponseType>::AuthorizationCode,
            CsrfToken::new_random,
            Nonce::new_random,
        )
        .add_scope(Scope::new("email".to_string()))
        .add_scope(Scope::new("profile".to_string()))
        .set_pkce_challenge(pkce_challenge)
        .url();

    // obtain authorization code while client is enabled
    let uri = format!(
        "{}?allow=true&{}",
        authorize_url.path(),
        authorize_url.query().unwrap()
    );
    let response = client.post(uri).send().await;
    assert_eq!(response.status(), StatusCode::FOUND);
    let location = response
        .headers()
        .get("Location")
        .unwrap()
        .to_str()
        .unwrap();
    let (location, query) = location.split_once('?').unwrap();
    assert_eq!(location, FAKE_REDIRECT_URI);
    let auth_response: AuthenticationResponse = serde_qs::from_str(query).unwrap();

    // exchange authorization code for token while client is enabled
    let token_response = core_client
        .exchange_code(AuthorizationCode::new(auth_response.code.into()))
        .unwrap()
        .set_pkce_verifier(pkce_verifier)
        .request_async(&|r| http_client(r, &client))
        .await
        .unwrap();

    // verify id token works while client is enabled
    let id_token_verifier = core_client.id_token_verifier();
    let _id_token_claims = token_response
        .extra_fields()
        .id_token()
        .expect("Server did not return an ID token")
        .claims(&id_token_verifier, &nonce)
        .unwrap();

    // verify userinfo works while client is enabled
    let userinfo_claims: UserInfoClaims<EmptyAdditionalClaims, CoreGenderClaim> = core_client
        .user_info(token_response.access_token().clone(), None)
        .expect("Missing info endpoint")
        .request_async(&|r| http_client(r, &client))
        .await
        .unwrap();

    // Verify we got valid userinfo
    assert!(userinfo_claims.email().is_some());

    // Now disable the OAuth2 client
    let disabled_oauth2client = NewOpenIDClient {
        name: "My test client".into(),
        redirect_uri: vec![FAKE_REDIRECT_URI.into()],
        scope: vec!["openid".into(), "email".into(), "profile".into()],
        enabled: false,
    };
    let response = client
        .put(format!("/api/v1/oauth/{}", oauth2client.client_id))
        .json(&disabled_oauth2client)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    // Try to access userinfo with disabled client, should fail
    let userinfo_result: Result<UserInfoClaims<EmptyAdditionalClaims, CoreGenderClaim>, _> =
        core_client
            .user_info(token_response.access_token().clone(), None)
            .expect("Missing info endpoint")
            .request_async(&|r| http_client(r, &client))
            .await;

    // The userinfo request should fail when client is disabled
    assert!(
        userinfo_result.is_err(),
        "Userinfo should fail when client is disabled"
    );
}

#[sqlx::test]
async fn test_openid_authorization_code_with_pkce(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (client, state) = make_test_client(pool).await;
    let mut config = state.config;

    let mut rng = rand::thread_rng();
    config.openid_signing_key = RsaPrivateKey::new(&mut rng, 2048).ok();

    let issuer_url = IssuerUrl::from_url(Settings::url().unwrap().clone());

    // discover OpenID service
    let provider_metadata =
        CoreProviderMetadata::discover_async(issuer_url, &|r| http_client(r, &client))
            .await
            .unwrap();

    // create OAuth2 client/application
    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let oauth2client = NewOpenIDClient {
        name: "My test client".into(),
        redirect_uri: vec![FAKE_REDIRECT_URI.into()],
        scope: vec!["openid".into()],
        enabled: true,
    };
    let response = client
        .post("/api/v1/oauth")
        .json(&oauth2client)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let oauth2client: OAuth2Client<Id> = response.json().await;
    assert_eq!(oauth2client.name, "My test client");
    assert_eq!(oauth2client.scope[0], "openid");

    // start the Authorization Code Flow with PKCE
    let client_id = ClientId::new(oauth2client.client_id);
    let client_secret = ClientSecret::new(oauth2client.client_secret);
    let core_client =
        CoreClient::from_provider_metadata(provider_metadata, client_id, Some(client_secret))
            .set_redirect_uri(RedirectUrl::new(FAKE_REDIRECT_URI.into()).unwrap());
    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
    let (authorize_url, _csrf_state, nonce) = core_client
        .authorize_url(
            AuthenticationFlow::<CoreResponseType>::AuthorizationCode,
            CsrfToken::new_random,
            Nonce::new_random,
        )
        .add_scope(Scope::new("email".to_string()))
        .add_scope(Scope::new("profile".to_string()))
        .set_pkce_challenge(pkce_challenge)
        .url();
    assert_eq!(authorize_url.scheme(), "http");
    assert_eq!(authorize_url.host_str(), Some("localhost"));
    assert_eq!(authorize_url.path(), "/api/v1/oauth/authorize");

    // obtain authorization code
    let uri = format!(
        "{}?allow=true&{}",
        authorize_url.path(),
        authorize_url.query().unwrap()
    );
    let response = client.post(uri).send().await;
    assert_eq!(response.status(), StatusCode::FOUND);
    let location = response
        .headers()
        .get("Location")
        .unwrap()
        .to_str()
        .unwrap();
    let (location, query) = location.split_once('?').unwrap();
    assert_eq!(location, FAKE_REDIRECT_URI);
    let auth_response: AuthenticationResponse = serde_qs::from_str(query).unwrap();

    // exchange authorization code for token
    let token_response = core_client
        .exchange_code(AuthorizationCode::new(auth_response.code.into()))
        .unwrap()
        .set_pkce_verifier(pkce_verifier)
        .request_async(&|r| http_client(r, &client))
        .await
        .unwrap();

    // verify id token
    let id_token_verifier = core_client.id_token_verifier();
    let _id_token_claims = token_response
        .extra_fields()
        .id_token()
        .expect("Server did not return an ID token")
        .claims(&id_token_verifier, &nonce)
        .unwrap();

    // refresh token
    let refresh_token = token_response.refresh_token().unwrap();
    let refresh_response = core_client
        .exchange_refresh_token(refresh_token)
        .unwrap()
        .request_async(&|r| http_client(r, &client))
        .await
        .unwrap();
    assert!(refresh_response.refresh_token().is_some());

    // userinfo
    let _userinfo_claims: UserInfoClaims<EmptyAdditionalClaims, CoreGenderClaim> = core_client
        .user_info(token_response.access_token().clone(), None)
        .expect("Missing info endpoint")
        .request_async(&|r| http_client(r, &client))
        .await
        .unwrap();
}

#[sqlx::test]
async fn dg25_23_test_openid_client_scope_change_clears_authorizations(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let (client, state) = make_test_client(pool).await;
    let admin = User::find_by_username(&state.pool, "admin")
        .await
        .unwrap()
        .unwrap();

    // Authenticate admin
    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // Create OAuth2 client with initial scopes
    let oauth2client = NewOpenIDClient {
        name: "Test Client".into(),
        redirect_uri: vec![TEST_SERVER_URL.into()],
        scope: vec!["openid".into(), "email".into()],
        enabled: true,
    };

    let response = client
        .post("/api/v1/oauth")
        .json(&oauth2client)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let oauth2client: OAuth2Client<Id> = response.json().await;

    // Authorize the client - simulate user authorization
    let response = client
        .post(format!(
            "/api/v1/oauth/authorize?\
            response_type=code&\
            client_id={}&\
            redirect_uri=http%3A%2F%2Flocalhost%3A3000&\
            scope=openid email&\
            state=ABCDEF&\
            allow=true&\
            nonce=blabla",
            oauth2client.client_id
        ))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FOUND);

    // Verify that the authorization was created
    let authorized_app = OAuth2AuthorizedApp::find_by_user_and_oauth2client_id(
        &state.pool,
        admin.id,
        oauth2client.id,
    )
    .await
    .unwrap();
    assert!(
        authorized_app.is_some(),
        "Authorization should exist before scope change"
    );

    // Update the client with different scopes
    let updated_client = NewOpenIDClient {
        name: "Test Client".into(),
        redirect_uri: vec![TEST_SERVER_URL.into()],
        scope: vec!["openid".into(), "profile".into()], // Changed from email to profile
        enabled: true,
    };

    let response = client
        .put(format!("/api/v1/oauth/{}", oauth2client.client_id))
        .json(&updated_client)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    // Verify that the authorization was cleared after scope change
    let authorized_app_after = OAuth2AuthorizedApp::find_by_user_and_oauth2client_id(
        &state.pool,
        admin.id,
        oauth2client.id,
    )
    .await
    .unwrap();
    assert!(
        authorized_app_after.is_none(),
        "Authorization should be cleared after scope change"
    );

    // Test that updating without scope changes does NOT clear authorizations

    // Re-authorize the client
    let response = client
        .post(format!(
            "/api/v1/oauth/authorize?\
            response_type=code&\
            client_id={}&\
            redirect_uri=http%3A%2F%2Flocalhost%3A3000&\
            scope=openid profile&\
            state=ABCDEF2&\
            allow=true&\
            nonce=blabla2",
            oauth2client.client_id
        ))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FOUND);

    // Verify authorization exists again
    let authorized_app = OAuth2AuthorizedApp::find_by_user_and_oauth2client_id(
        &state.pool,
        admin.id,
        oauth2client.id,
    )
    .await
    .unwrap();
    assert!(
        authorized_app.is_some(),
        "Authorization should exist after re-authorization"
    );

    // Update the client without changing scopes (only name)
    let same_scope_update = NewOpenIDClient {
        name: "Test Client Updated Name".into(),
        redirect_uri: vec![TEST_SERVER_URL.into()],
        scope: vec!["openid".into(), "profile".into()], // Same scopes
        enabled: true,
    };

    let response = client
        .put(format!("/api/v1/oauth/{}", oauth2client.client_id))
        .json(&same_scope_update)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    // Verify that the authorization still exists when scopes haven't changed
    let authorized_app_preserved = OAuth2AuthorizedApp::find_by_user_and_oauth2client_id(
        &state.pool,
        admin.id,
        oauth2client.id,
    )
    .await
    .unwrap();
    assert!(
        authorized_app_preserved.is_some(),
        "Authorization should be preserved when scopes don't change"
    );
}

#[sqlx::test]
async fn dg25_17_test_openid_open_redirects(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (client, state) = make_test_client(pool).await;
    let _admin = User::find_by_username(&state.pool, "admin")
        .await
        .unwrap()
        .unwrap();

    // Authenticate admin
    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // Create OAuth2 client
    let oauth2client = NewOpenIDClient {
        name: "Test Client".into(),
        redirect_uri: vec![TEST_SERVER_URL.into(), "http://safe.net/".into()],
        scope: vec!["openid".into(), "email".into()],
        enabled: true,
    };

    let response = client
        .post("/api/v1/oauth")
        .json(&oauth2client)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let oauth2client: OAuth2Client<Id> = response.json().await;

    fn redirect_url(response: &TestResponse) -> String {
        Url::parse(response.headers().get(LOCATION).unwrap().to_str().unwrap())
            .unwrap()
            .origin()
            .ascii_serialization()
    }

    let fallback_url = Settings::url()
        .unwrap()
        .to_string()
        .trim_end_matches('/')
        .to_string();

    // Try to authorize with allowed redirect url - invalid client id
    let response = client
        .post(
            "/api/v1/oauth/authorize?\
            response_type=code&\
            client_id=xxx&\
            redirect_uri=http://localhost:3000&\
            scope=openid email&\
            state=ABCDEF&\
            allow=true&\
            nonce=blabla",
        )
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FOUND);
    assert_eq!(redirect_url(&response), fallback_url);

    let response = client
        .get(
            "/api/v1/oauth/authorize?\
            response_type=code&\
            client_id=xxx&\
            redirect_uri=http://localhost:3000&\
            scope=openid email&\
            state=ABCDEF&\
            allow=true&\
            nonce=blabla",
        )
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FOUND);
    assert_eq!(redirect_url(&response), fallback_url);

    // Try to authorize with forbidden redirect url - invalid client id
    let response = client
        .post(
            "/api/v1/oauth/authorize?\
            response_type=code&\
            client_id=xxx&\
            redirect_uri=http://isec.pl&\
            scope=openid email&\
            state=ABCDEF&\
            allow=true&\
            nonce=blabla",
        )
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FOUND);
    assert_eq!(redirect_url(&response), fallback_url);

    let response = client
        .get(
            "/api/v1/oauth/authorize?\
            response_type=code&\
            client_id=xxx&\
            redirect_uri=http://isec.pl&\
            scope=openid email&\
            state=ABCDEF&\
            allow=true&\
            nonce=blabla",
        )
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FOUND);
    assert_eq!(redirect_url(&response), fallback_url);

    // Try to authorize with forbidden redirect url - invalid scope
    let response = client
        .post(format!(
            "/api/v1/oauth/authorize?\
            response_type=code&\
            client_id={}&\
            redirect_uri=http://isec.pl&\
            scope=profile&\
            state=ABCDEF&\
            allow=true&\
            nonce=blabla",
            oauth2client.client_id
        ))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FOUND);
    assert_eq!(redirect_url(&response), fallback_url);

    let response = client
        .get(format!(
            "/api/v1/oauth/authorize?\
            response_type=code&\
            client_id={}&\
            redirect_uri=http://isec.pl&\
            scope=profile&\
            state=ABCDEF&\
            allow=true&\
            nonce=blabla",
            oauth2client.client_id
        ))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FOUND);
    assert_eq!(redirect_url(&response), fallback_url);

    // Same with allowed redirect_uri
    let response = client
        .post(format!(
            "/api/v1/oauth/authorize?\
            response_type=code&\
            client_id={}&\
            redirect_uri=http://safe.net&\
            scope=profile&\
            state=ABCDEF&\
            allow=true&\
            nonce=blabla",
            oauth2client.client_id
        ))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FOUND);
    assert_eq!(redirect_url(&response), "http://safe.net");

    let response = client
        .get(format!(
            "/api/v1/oauth/authorize?\
            response_type=code&\
            client_id={}&\
            redirect_uri=http://safe.net&\
            scope=profile&\
            state=ABCDEF&\
            allow=true&\
            nonce=blabla",
            oauth2client.client_id
        ))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FOUND);
    assert_eq!(redirect_url(&response), "http://safe.net");
}

#[sqlx::test]
async fn dg25_22_test_respect_openid_scope_in_userinfo(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;

    let (client, state) = make_test_client(pool).await;
    let mut config = state.config;

    let mut admin = User::find_by_username(&state.pool, "admin")
        .await
        .unwrap()
        .unwrap();

    admin.phone = Some("+123456789".into());
    admin.save(&state.pool).await.unwrap();

    let mut rng = rand::thread_rng();
    config.openid_signing_key = RsaPrivateKey::new(&mut rng, 2048).ok();

    let issuer_url = IssuerUrl::from_url(Settings::url().unwrap().clone());

    // discover OpenID service
    let provider_metadata =
        CoreProviderMetadata::discover_async(issuer_url, &|r| http_client(r, &client))
            .await
            .unwrap();

    // Create reusable closure for testing different scope configurations
    let get_user_claims = |client_scopes: Vec<String>, requested_scopes: Vec<String>| {
        let client = &client;
        let provider_metadata = provider_metadata.clone();
        async move {
            // Authenticate admin
            let auth = Auth::new("admin", "pass123");
            let response = client.post("/api/v1/auth").json(&auth).send().await;
            assert_eq!(response.status(), StatusCode::OK);

            // Create OAuth2 client with specified scopes
            let oauth2client = NewOpenIDClient {
                name: "Test client".into(),
                redirect_uri: vec![FAKE_REDIRECT_URI.into()],
                scope: client_scopes,
                enabled: true,
            };
            let response = client
                .post("/api/v1/oauth")
                .json(&oauth2client)
                .send()
                .await;
            assert_eq!(response.status(), StatusCode::CREATED);
            let oauth2client: OAuth2Client<Id> = response.json().await;

            // Store client_id for cleanup
            let client_id_for_cleanup = oauth2client.client_id.clone();

            // Create OpenID client
            let client_id = ClientId::new(oauth2client.client_id);
            let client_secret = ClientSecret::new(oauth2client.client_secret);
            let core_client = CoreClient::from_provider_metadata(
                provider_metadata,
                client_id,
                Some(client_secret),
            )
            .set_redirect_uri(RedirectUrl::new(FAKE_REDIRECT_URI.into()).unwrap());

            // Start Authorization Code Flow with PKCE
            let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
            let mut auth_request = core_client.authorize_url(
                AuthenticationFlow::<CoreResponseType>::AuthorizationCode,
                CsrfToken::new_random,
                Nonce::new_random,
            );

            // Add requested scopes
            for scope in requested_scopes {
                auth_request = auth_request.add_scope(Scope::new(scope));
            }

            let (authorize_url, _csrf_state, nonce) =
                auth_request.set_pkce_challenge(pkce_challenge).url();

            // Obtain authorization code
            let uri = format!(
                "{}?allow=true&{}",
                authorize_url.path(),
                authorize_url.query().unwrap()
            );
            let response = client.post(uri).send().await;
            assert_eq!(response.status(), StatusCode::FOUND);
            let location = response
                .headers()
                .get("Location")
                .unwrap()
                .to_str()
                .unwrap();
            let (location, query) = location.split_once('?').unwrap();
            assert_eq!(location, FAKE_REDIRECT_URI);
            let auth_response: AuthenticationResponse = serde_qs::from_str(query).unwrap();

            // Exchange authorization code for token
            let token_response = core_client
                .exchange_code(AuthorizationCode::new(auth_response.code.into()))
                .unwrap()
                .set_pkce_verifier(pkce_verifier)
                .request_async(&|r| http_client(r, client))
                .await
                .unwrap();

            // Verify id token
            let id_token_verifier = core_client.id_token_verifier();
            let _id_token_claims = token_response
                .extra_fields()
                .id_token()
                .expect("Server did not return an ID token")
                .claims(&id_token_verifier, &nonce)
                .unwrap();

            // Get userinfo claims
            let userinfo_claims: UserInfoClaims<EmptyAdditionalClaims, CoreGenderClaim> =
                core_client
                    .user_info(token_response.access_token().clone(), None)
                    .expect("Missing info endpoint")
                    .request_async(&|r| http_client(r, client))
                    .await
                    .unwrap();

            // Clean up - delete the OAuth client
            client
                .delete(format!("/api/v1/oauth/{client_id_for_cleanup}"))
                .send()
                .await;

            userinfo_claims
        }
    };

    // Client has phone and email scopes, request phone and email
    let claims = get_user_claims(
        vec![
            "openid".to_string(),
            "phone".to_string(),
            "email".to_string(),
        ],
        vec!["email".to_string(), "phone".to_string()],
    )
    .await;

    // Verify claims include both email and phone
    assert!(claims.email().is_some());
    assert!(claims.phone_number().is_some());

    // Client has phone and email scopes, but only request email
    let claims = get_user_claims(
        vec![
            "openid".to_string(),
            "phone".to_string(),
            "email".to_string(),
        ],
        vec!["email".to_string()],
    )
    .await;

    // Verify claims include only email, not phone
    assert!(claims.email().is_some());
    assert!(claims.phone_number().is_none());

    // Client has only email scope, request phone
    let claims = get_user_claims(
        vec!["openid".to_string(), "email".to_string()],
        vec!["email".to_string(), "phone".to_string()],
    )
    .await;

    // Verify claims include only email since client doesn't have phone scope
    assert!(claims.email().is_some());
    assert!(claims.phone_number().is_none());
}

#[sqlx::test]
async fn dg25_21_test_openid_html_injection(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let client = make_client(pool).await;
    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    let invalid_names = &[
        "Test <a href=\"isec.pl\">Click</a>",
        "Test <a href=\"isec.pl\">Click",
        "Test <script>alert('xss')</script>",
        "Test <script>alert('xss')",
        "Test <img src=x onerror=alert(1)>",
        "Test <a href=\"javascript:alert(1)\">here</a>",
        "Test <svg onload=\"alert(1)\"></svg>",
    ];

    // ensure creation of openid client with name containing HTML is forbidden
    for name in invalid_names {
        let openid_client = NewOpenIDClient {
            name: name.to_string(),
            redirect_uri: vec![TEST_SERVER_URL.into()],
            scope: vec!["openid".into()],
            enabled: true,
        };
        let response = client
            .post("/api/v1/oauth")
            .json(&openid_client)
            .send()
            .await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    // create valid openid client
    let openid_client = NewOpenIDClient {
        name: "Test".to_string(),
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
    let valid_openid_client: OAuth2Client<Id> = response.json().await;
    assert_eq!(valid_openid_client.name, "Test");

    // ensure edits of openid client with name containing HTML are forbidden
    for name in invalid_names {
        let openid_client = NewOpenIDClient {
            name: name.to_string(),
            redirect_uri: vec![TEST_SERVER_URL.into()],
            scope: vec!["openid".into()],
            enabled: true,
        };
        let response = client
            .put(format!("/api/v1/oauth/{}", valid_openid_client.client_id))
            .json(&openid_client)
            .send()
            .await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}

#[sqlx::test]
async fn test_openid_flow_new_login_mail(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (client, _) = make_test_client(pool).await;
    let user_agent_header = "Mozilla/5.0 (iPhone; CPU iPhone OS 17_1 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.1 Mobile/15E148 Safari/604.1";

    let auth = Auth::new("admin", "pass123");
    let response = client
        .post("/api/v1/auth")
        .header(USER_AGENT, user_agent_header)
        .json(&auth)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
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

    // all clients
    let response = client.get("/api/v1/oauth").send().await;
    assert_eq!(response.status(), StatusCode::OK);

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

    let location = response
        .headers()
        .get("Location")
        .unwrap()
        .to_str()
        .unwrap();
    let (location, query) = location.split_once('?').unwrap();
    assert_eq!(location, TEST_SERVER_URL);
    let auth_response: AuthenticationResponse = serde_qs::from_str(query).unwrap();
    assert_eq!(auth_response.state, "ABCDEF");

    // assert_eq!(mail.to(), "admin@defguard");
    // assert_eq!(
    //     mail.subject(),
    //     "New login to Test application with Defguard"
    // );
    // assert!(mail.content().contains("IP Address:</span> 127.0.0.1"));
    // assert!(
    //     mail.content()
    //         .contains("Device type:</span> iPhone, OS: iOS 17.1, Mobile Safari")
    // );

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
}
