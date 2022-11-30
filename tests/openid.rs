use defguard::{
    build_webapp,
    config::DefGuardConfig,
    db::{AppEvent, DbPool, GatewayEvent},
    enterprise::{
        db::{NewOpenIDClient, OAuth2Client},
        handlers::openid_flow::AuthenticationResponse,
    },
    grpc::GatewayState,
    handlers::Auth,
};
use openidconnect::{
    core::{CoreClient, CoreGenderClaim, CoreProviderMetadata, CoreResponseType},
    http::{
        header::{HeaderName, HeaderValue},
        HeaderMap, Method, StatusCode,
    },
    url::Url,
    AuthenticationFlow, AuthorizationCode, ClientId, ClientSecret, CsrfToken,
    EmptyAdditionalClaims, HttpRequest, HttpResponse, IssuerUrl, Nonce, OAuth2TokenResponse,
    PkceCodeChallenge, RedirectUrl, Scope, UserInfoClaims,
};
use rocket::{
    http::{ContentType, Header, Status},
    local::asynchronous::Client,
};
use std::{
    str::FromStr,
    sync::{Arc, Mutex},
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

async fn make_client_v2(pool: DbPool, config: DefGuardConfig) -> Client {
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
        redirect_uri: vec!["http://localhost:3000/".into()],
        scope: vec!["openid".into()],
        enabled: true,
    };

    let response = client
        .post("/api/v1/oauth")
        .json(&openid_client)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Created);

    let response = client.get("/api/v1/oauth").dispatch().await;
    assert_eq!(response.status(), Status::Ok);
    let openid_clients: Vec<OAuth2Client> = response.into_json().await.unwrap();
    assert_eq!(openid_clients.len(), 1);

    openid_client.name = "Test changed".into();
    let response = client
        .put(format!("/api/v1/oauth/{}", openid_clients[0].client_id))
        .json(&openid_client)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);

    let response = client
        .get(format!("/api/v1/oauth/{}", openid_clients[0].client_id))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);
    let fetched_client: OAuth2Client = response.into_json().await.unwrap();
    assert_eq!(fetched_client.name, openid_client.name);

    // OpenID flow tests
    // test unsupported response type
    // Test client delete
    let response = client
        .delete(format!("/api/v1/oauth/{}", openid_clients[0].client_id))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);

    let response = client.get("/api/v1/oauth").dispatch().await;
    assert_eq!(response.status(), Status::Ok);

    let openid_clients: Vec<OAuth2Client> = response.into_json().await.unwrap();
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
        redirect_uri: vec!["http://localhost:3000/".into()],
        scope: vec!["openid".into()],
        enabled: true,
    };

    let response = client
        .post("/api/v1/oauth")
        .json(&openid_client)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Created);
    let openid_client: OAuth2Client = response.into_json().await.unwrap();
    assert_eq!(openid_client.name, "Test");

    // all clients
    let response = client.get("/api/v1/oauth").dispatch().await;
    assert_eq!(response.status(), Status::Ok);

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
        .dispatch()
        .await;
    let location = response.headers().get_one("Location").unwrap();
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
        .dispatch()
        .await;
    let location = response.headers().get_one("Location").unwrap();
    assert!(location.contains("error=invalid_request"));

    // obtain authentication code
    let response = client
        .post(format!(
            "/api/v1/oauth/authorize?\
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
    let (location, query) = location.split_once('?').unwrap();
    assert_eq!(location, "http://localhost:3000/");
    let auth_response: AuthenticationResponse = serde_qs::from_str(query).unwrap();
    assert_eq!(auth_response.state, "ABCDEF");

    // exchange wrong code for token should fail
    let response = client
        .post("/api/v1/oauth/token")
        .header(ContentType::Form)
        .body(format!(
            "grant_type=authorization_code&\
            code=ncuoew2323&\
            redirect_uri=http%3A%2F%2Flocalhost%3A3000%2F&\
            client_id={}&\
            client_secret={}",
            openid_client.client_id, openid_client.client_secret
        ))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::BadRequest);

    // exchange correct code for token
    let response = client
        .post("/api/v1/oauth/token")
        .header(ContentType::Form)
        .body(format!(
            "grant_type=authorization_code&\
            code={}&\
            redirect_uri=http%3A%2F%2Flocalhost%3A3000%2F&\
            client_id={}&\
            client_secret={}",
            auth_response.code, openid_client.client_id, openid_client.client_secret
        ))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);

    // check used code
    let response = client
        .post("/api/v1/oauth/token")
        .header(ContentType::Form)
        .body(format!(
            "grant_type=authorization_code&\
            code={}&\
            redirect_uri=http%3A%2F%2Flocalhost%3A3000%2F",
            auth_response.code
        ))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::BadRequest);

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
        .dispatch()
        .await;
    let location = response.headers().get_one("Location").unwrap();
    assert!(location.contains("error"));

    // test wrong redirect uri
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
        .dispatch()
        .await;
    let location = response.headers().get_one("Location").unwrap();
    assert!(location.contains("error"));

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
        .dispatch()
        .await;
    let location = response.headers().get_one("Location").unwrap();
    assert!(location.contains("error=unauthorized_client"));
}

/// Helper function for translating HTTP communication from `HttpRequest` to `LocalClient`.
async fn http_client(
    request: HttpRequest,
    pool: DbPool,
    config: DefGuardConfig,
) -> Result<HttpResponse, rocket::Error> {
    let client = make_client_v2(pool, config).await;

    let uri = request.url.path();
    eprintln!("HTTP client request path: {}", uri);
    if let Some(query) = request.url.query() {
        eprintln!("HTTP client request query: {}", query);
    }
    eprintln!("HTTP client request headers: {:#?}", request.headers);
    if let Ok(text) = String::from_utf8(request.body.clone()) {
        eprintln!("HTTP client body: {}", text);
    }
    let mut rocket_request = match request.method {
        Method::GET => client.get(uri),
        Method::POST => client.post(uri),
        Method::PUT => client.put(uri),
        Method::DELETE => client.delete(uri),
        _ => unimplemented!(),
    };
    for (key, value) in request.headers.iter() {
        let header = Header::new(key.as_str().to_owned(), value.to_str().unwrap().to_owned());
        rocket_request = rocket_request.header(header);
    }
    let response = rocket_request.body(request.body).dispatch().await;

    let mut headers = HeaderMap::new();
    for header in response.headers().iter() {
        if let (Ok(key), Ok(value)) = (
            HeaderName::from_str(header.name.as_ref()),
            HeaderValue::from_str(header.value()),
        ) {
            headers.append(key, value);
        }
    }

    Ok(HttpResponse {
        status_code: StatusCode::from_u16(response.status().code).unwrap(),
        headers,
        body: response.into_bytes().await.unwrap_or_default(),
    })
}

#[rocket::async_test]
async fn test_openid_authorization_code() {
    let (pool, mut config) = init_test_db().await;
    config.license = LICENSE_ENTERPRISE.into();

    let issuer_url = IssuerUrl::from_url(Url::parse(&config.url).expect("Invalid issuer URL"));
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
    let response = client.post("/api/v1/auth").json(&auth).dispatch().await;
    assert_eq!(response.status(), Status::Ok);
    let oauth2client = NewOpenIDClient {
        name: "My test client".into(),
        redirect_uri: vec!["http://test.server.tnt:12345/".into()],
        scope: vec!["openid".into()],
        enabled: true,
    };
    let response = client
        .post("/api/v1/oauth")
        .json(&oauth2client)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Created);
    let oauth2client: OAuth2Client = response.into_json().await.unwrap();
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
    let response = client.post(uri).dispatch().await;
    assert_eq!(response.status(), Status::Found);
    let location = response.headers().get_one("Location").unwrap();
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

#[rocket::async_test]
async fn test_openid_authorization_code_with_pkce() {
    let (pool, mut config) = init_test_db().await;
    config.license = LICENSE_ENTERPRISE.into();

    let issuer_url = IssuerUrl::from_url(Url::parse(&config.url).expect("Invalid issuer URL"));
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
    let response = client.post("/api/v1/auth").json(&auth).dispatch().await;
    assert_eq!(response.status(), Status::Ok);
    let oauth2client = NewOpenIDClient {
        name: "My test client".into(),
        redirect_uri: vec!["http://test.server.tnt:12345/".into()],
        scope: vec!["openid".into()],
        enabled: true,
    };
    let response = client
        .post("/api/v1/oauth")
        .json(&oauth2client)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Created);
    let oauth2client: OAuth2Client = response.into_json().await.unwrap();
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
    let response = client.post(uri).dispatch().await;
    assert_eq!(response.status(), Status::Found);
    let location = response.headers().get_one("Location").unwrap();
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
    let id_token_verifier = core_client.id_token_verifier().allow_any_alg();
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
        .user_info(token_response.access_token().to_owned(), None)
        .expect("Missing info endpoint")
        .request_async(move |r| http_client(r, pool, config))
        .await
        .unwrap();
}
