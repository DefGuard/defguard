use axum::http::{header::ToStrError, StatusCode};
use claims::assert_err;
use defguard::{
    config::DefGuardConfig,
    db::{
        models::{oauth2client::OAuth2Client, NewOpenIDClient},
        DbPool,
    },
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
use reqwest::header::USER_AGENT;
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

#[derive(Deserialize)]
pub struct AuthenticationResponse<'r> {
    pub code: &'r str,
    pub state: &'r str,
}

#[tokio::test]
async fn test_openid_client() {
    let client = make_client().await;

    let auth = Auth::new("admin".into(), "pass123".into());
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    let mut openid_client = NewOpenIDClient {
        name: "Test".into(),
        redirect_uri: vec!["http://localhost:3000/".into()],
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
    let openid_clients: Vec<OAuth2Client> = response.json().await;
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
    let fetched_client: OAuth2Client = response.json().await;
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

    let openid_clients: Vec<OAuth2Client> = response.json().await;
    assert!(openid_clients.is_empty());
}

#[tokio::test]
async fn test_openid_flow() {
    let client = make_client().await;
    let auth = Auth::new("admin".into(), "pass123".into());
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let openid_client = NewOpenIDClient {
        name: "Test".into(),
        redirect_uri: vec!["http://localhost:3000/".into()],
        scope: vec!["openid".into()],
        enabled: true,
    };

    let response = client
        .post("/api/v1/oauth")
        .json(&openid_client)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let openid_client: OAuth2Client = response.json().await;
    assert_eq!(openid_client.name, "Test");

    // all clients
    let response = client.get("/api/v1/oauth").send().await;
    assert_eq!(response.status(), StatusCode::OK);

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
    assert_eq!(location, "http://localhost:3000/");
    let auth_response: AuthenticationResponse = serde_qs::from_str(query).unwrap();
    assert_eq!(auth_response.state, "ABCDEF");

    // exchange wrong code for token should fail
    let response = client
        .post("/api/v1/oauth/token")
        .header("Content-Type", "application/x-www-form-urlencoded")
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

    // exchange correct code for token
    let response = client
        .post("/api/v1/oauth/token")
        .header("Content-Type", "application/x-www-form-urlencoded")
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
    assert_eq!(response.status(), StatusCode::OK);

    // make sure access token cannot be used to manage defguard server itself
    client.post("/api/v1/auth/logout").send().await;
    let token_response: CoreTokenResponse = response.json().await;
    let response = client
        .get("/api/v1/network")
        .header(
            "Authorization",
            format!("Bearer {}", token_response.access_token().secret()),
        )
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let response = client
        .get("/api/v1/user")
        .header(
            "Authorization",
            format!("Bearer {}", token_response.access_token().secret()),
        )
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // log back in
    let auth = Auth::new("admin".into(), "pass123".into());
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // check code cannot be reused
    let response = client
        .post("/api/v1/oauth/token")
        .header("Content-Type", "application/x-www-form-urlencoded")
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
    assert!(location.contains("error"));

    // test wrong invalid uri
    let response = client
        .post(
            "/api/v1/oauth/authorize?\
            response_type=code&\
            client_id=1&\
            redirect_uri=http%3A%2F%example%3A3000%2F&\
            scope=openid&\
            state=ABCDEF&\
            nonce=blabla",
        )
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // test wrong redirect uri
    let response = client
        .post(
            "/api/v1/oauth/authorize?\
            response_type=code&\
            client_id=1&\
            redirect_uri=http%3A%2F%2Fexample%3A3000%3Fvalue1=one%26value2=two&\
            scope=openid&\
            state=ABCDEF&\
            nonce=blabla",
        )
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FOUND);
    let location = response
        .headers()
        .get("Location")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(location.contains("error=access_denied"));
    assert!(location.contains("value1="));
    assert!(location.contains("value2="));

    // // test allow false
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
    assert!(location.contains("error=access_denied"));
}

/// Helper function for translating HTTP communication from `HttpRequest` to `LocalClient`.
async fn http_client(
    request: HttpRequest,
    pool: DbPool,
    config: DefGuardConfig,
) -> Result<HttpResponse, ToStrError> {
    let client = make_client_v2(pool, config).await;

    let uri = request.url.path();
    eprintln!("HTTP client request path: {uri}");
    if let Some(query) = request.url.query() {
        eprintln!("HTTP client request query: {query}");
    }
    eprintln!("HTTP client request headers: {:#?}", request.headers);
    if let Ok(text) = String::from_utf8(request.body.clone()) {
        eprintln!("HTTP client body: {text}");
    }
    let mut test_request = match request.method {
        Method::GET => client.get(uri),
        Method::HEAD => client.head(uri),
        Method::POST => client.post(uri),
        Method::PUT => client.put(uri),
        Method::DELETE => client.delete(uri),
        _ => unimplemented!(),
    };
    for (key, value) in &request.headers {
        test_request = test_request.header(key.as_str(), value.to_str()?);
    }
    let response = test_request.body(request.body).send().await;

    Ok(HttpResponse {
        status_code: response.status(),
        headers: response.headers().clone(),
        body: response.bytes().await.to_vec(),
    })
}

#[tokio::test]
async fn test_openid_authorization_code() {
    let (pool, config) = init_test_db().await;

    let issuer_url = IssuerUrl::from_url(config.url.clone());
    let client = make_client_v2(pool.clone(), config.clone()).await;
    let pool_clone = pool.clone();
    let config_clone = config.clone();

    // discover OpenID service
    let provider_metadata = CoreProviderMetadata::discover_async(issuer_url, move |r| {
        http_client(r, pool_clone.clone(), config_clone.clone())
    })
    .await
    .unwrap();

    // create OAuth2 client
    let auth = Auth::new("admin".into(), "pass123".into());
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let oauth2client = NewOpenIDClient {
        name: "My test client".into(),
        redirect_uri: vec!["http://test.server.tnt:12345/".into()],
        scope: vec!["openid".into()],
        enabled: true,
    };
    let response = client
        .post("/api/v1/oauth")
        .json(&oauth2client)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let oauth2client: OAuth2Client = response.json().await;
    assert_eq!(oauth2client.name, "My test client");
    assert_eq!(oauth2client.scope[0], "openid");
    assert_eq!(oauth2client.client_id.len(), 16);
    assert_eq!(oauth2client.client_secret.len(), 32);

    // start the Authorization Code Flow
    let client_id = ClientId::new(oauth2client.client_id);
    let client_secret = ClientSecret::new(oauth2client.client_secret);
    let core_client =
        CoreClient::from_provider_metadata(provider_metadata, client_id, Some(client_secret))
            .set_redirect_uri(RedirectUrl::new("http://test.server.tnt:12345/".into()).unwrap());
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
    assert_eq!(location, "http://test.server.tnt:12345/");
    let auth_response: AuthenticationResponse = serde_qs::from_str(query).unwrap();

    // exchange authorization code for token
    let pool_clone_2 = pool.clone();
    let config_clone_2 = config.clone();
    let token_response = core_client
        .exchange_code(AuthorizationCode::new(auth_response.code.into()))
        .request_async(move |r| http_client(r, pool_clone_2, config_clone_2))
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
        .request_async(move |r| http_client(r, pool, config))
        .await
        .unwrap();
    assert!(refresh_response.refresh_token().is_some());
}

#[tokio::test]
async fn test_openid_authorization_code_with_pkce() {
    let (pool, mut config) = init_test_db().await;
    let mut rng = rand::thread_rng();
    config.openid_signing_key = RsaPrivateKey::new(&mut rng, 2048).ok();

    let issuer_url = IssuerUrl::from_url(config.url.clone());
    let client = make_client_v2(pool.clone(), config.clone()).await;
    let pool_clone = pool.clone();
    let config_clone = config.clone();

    // discover OpenID service
    let provider_metadata = CoreProviderMetadata::discover_async(issuer_url, move |r| {
        http_client(r, pool_clone.clone(), config_clone.clone())
    })
    .await
    .unwrap();

    // create OAuth2 client/application
    let auth = Auth::new("admin".into(), "pass123".into());
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let oauth2client = NewOpenIDClient {
        name: "My test client".into(),
        redirect_uri: vec!["http://test.server.tnt:12345/".into()],
        scope: vec!["openid".into()],
        enabled: true,
    };
    let response = client
        .post("/api/v1/oauth")
        .json(&oauth2client)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let oauth2client: OAuth2Client = response.json().await;
    assert_eq!(oauth2client.name, "My test client");
    assert_eq!(oauth2client.scope[0], "openid");

    // start the Authorization Code Flow with PKCE
    let client_id = ClientId::new(oauth2client.client_id);
    let client_secret = ClientSecret::new(oauth2client.client_secret);
    let core_client =
        CoreClient::from_provider_metadata(provider_metadata, client_id, Some(client_secret))
            .set_redirect_uri(RedirectUrl::new("http://test.server.tnt:12345/".into()).unwrap());
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
    assert_eq!(location, "http://test.server.tnt:12345/");
    let auth_response: AuthenticationResponse = serde_qs::from_str(query).unwrap();

    // exchange authorization code for token
    let pool_clone_2 = pool.clone();
    let config_clone_2 = config.clone();
    let token_response = core_client
        .exchange_code(AuthorizationCode::new(auth_response.code.into()))
        .set_pkce_verifier(pkce_verifier)
        .request_async(move |r| http_client(r, pool_clone_2, config_clone_2))
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
    let pool_clone_3 = pool.clone();
    let config_clone_3 = config.clone();
    let refresh_token = token_response.refresh_token().unwrap();
    let refresh_response = core_client
        .exchange_refresh_token(refresh_token)
        .request_async(move |r| http_client(r, pool_clone_3, config_clone_3))
        .await
        .unwrap();
    assert!(refresh_response.refresh_token().is_some());

    // userinfo
    let _userinfo_claims: UserInfoClaims<EmptyAdditionalClaims, CoreGenderClaim> = core_client
        .user_info(token_response.access_token().clone(), None)
        .expect("Missing info endpoint")
        .request_async(move |r| http_client(r, pool, config))
        .await
        .unwrap();
}

#[tokio::test]
async fn test_openid_flow_new_login_mail() {
    let (client, state) = make_test_client().await;
    let mut mail_rx = state.mail_rx;
    let user_agent_header = "Mozilla/5.0 (iPhone; CPU iPhone OS 17_1 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.1 Mobile/15E148 Safari/604.1";

    let auth = Auth::new("admin".into(), "pass123".into());
    let response = client
        .post("/api/v1/auth")
        .header(USER_AGENT, user_agent_header)
        .json(&auth)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let openid_client = NewOpenIDClient {
        name: "Test".into(),
        redirect_uri: vec!["http://localhost:3000/".into()],
        scope: vec!["openid".into()],
        enabled: true,
    };

    let response = client
        .post("/api/v1/oauth")
        .json(&openid_client)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let openid_client: OAuth2Client = response.json().await;
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
    assert_eq!(location, "http://localhost:3000/");
    let auth_response: AuthenticationResponse = serde_qs::from_str(query).unwrap();
    assert_eq!(auth_response.state, "ABCDEF");

    mail_rx.try_recv().unwrap();
    let mail = mail_rx.try_recv().unwrap();
    assert_eq!(mail.to, "admin@defguard");
    assert_eq!(mail.subject, "New login to Test application with defguard");
    assert_eq!(mail.content.contains("IP Address: 127.0.0.1"), true);
    assert_eq!(
        mail.content
            .contains("Device type: iPhone, OS: iOS 17.1, Mobile Safari"),
        true
    );

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

    // No new mail recevied
    assert_err!(mail_rx.try_recv());
}
