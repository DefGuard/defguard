use defguard::{
    build_webapp,
    db::{AppEvent, GatewayEvent},
    enterprise::db::{
        openid::{AuthorizedApp, NewOpenIDClient},
        OAuth2Client,
    },
    handlers::Auth,
};
use openidconnect::{
    core::{CoreClient, CoreProviderMetadata, CoreResponseType},
    http::{HeaderMap, Method, StatusCode},
    AuthenticationFlow, ClientId, ClientSecret, CsrfToken, HttpRequest, HttpResponse, IssuerUrl,
    Nonce, RedirectUrl, Scope,
};
use rocket::{http, local::asynchronous::Client};
use tokio::sync::mpsc::unbounded_channel;

mod common;
use common::{init_test_db, LICENSE_ENTERPRISE};

async fn make_client() -> Client {
    let (pool, mut config) = init_test_db().await;
    config.license = LICENSE_ENTERPRISE.into();

    let (tx, rx) = unbounded_channel::<AppEvent>();
    let (wg_tx, _) = unbounded_channel::<GatewayEvent>();

    let webapp = build_webapp(config, tx, rx, wg_tx, pool).await;
    Client::tracked(webapp).await.unwrap()
}

// #[rocket::async_test]
// async fn test_openid_client() {
//     let client = make_client().await;

//     let auth = Auth::new("admin".into(), "pass123".into());
//     let response = client.post("/api/v1/auth").json(&auth).dispatch().await;
//     assert_eq!(response.status(), Status::Ok);

//     let mut openid_client = NewOpenIDClient {
//         name: "Test".into(),
//         redirect_uri: "http://localhost:3000/".into(),
//         enabled: true,
//     };

//     let response = client
//         .post("/api/v1/openid")
//         .json(&openid_client)
//         .dispatch()
//         .await;
//     assert_eq!(response.status(), Status::Created);

//     let response = client.get("/api/v1/openid").dispatch().await;
//     assert_eq!(response.status(), Status::Ok);
//     let openid_clients: Vec<OAuth2Client> = response.into_json().await.unwrap();
//     assert_eq!(openid_clients.len(), 1);

//     openid_client.name = "Test changed".into();
//     let response = client
//         .put(format!("/api/v1/openid/{}", openid_clients[0].client_id))
//         .json(&openid_client)
//         .dispatch()
//         .await;
//     assert_eq!(response.status(), Status::Ok);

//     let response = client
//         .get(format!("/api/v1/openid/{}", openid_clients[0].client_id))
//         .dispatch()
//         .await;
//     assert_eq!(response.status(), Status::Ok);
//     let fetched_client: OAuth2Client = response.into_json().await.unwrap();
//     assert_eq!(fetched_client.name, openid_client.name);

//     // Openid flow tests
//     // test unsupported response type
//     // Test client delete
//     let response = client
//         .delete(format!("/api/v1/openid/{}", openid_clients[0].client_id))
//         .dispatch()
//         .await;
//     assert_eq!(response.status(), Status::Ok);

//     let response = client.get("/api/v1/openid").dispatch().await;
//     assert_eq!(response.status(), Status::Ok);

//     let openid_clients: Vec<OAuth2Client> = response.into_json().await.unwrap();
//     assert!(openid_clients.is_empty());
// }

// #[rocket::async_test]
// async fn test_openid_flow() {
//     let client = make_client().await;
//     let auth = Auth::new("admin".into(), "pass123".into());
//     let response = client.post("/api/v1/auth").json(&auth).dispatch().await;
//     assert_eq!(response.status(), Status::Ok);
//     let openid_client = NewOpenIDClient {
//         name: "Test".into(),
//         redirect_uri: "http://localhost:3000/".into(),
//         enabled: true,
//     };

//     let response = client
//         .post("/api/v1/openid")
//         .json(&openid_client)
//         .dispatch()
//         .await;
//     assert_eq!(response.status(), Status::Created);
//     let openid_client: OAuth2Client = response.into_json().await.unwrap();
//     assert_eq!(openid_client.name, "Test");

//     // all clients
//     let response = client.get("/api/v1/openid").dispatch().await;
//     assert_eq!(response.status(), Status::Ok);

//     let response = client
//         .post(format!(
//             "/api/v1/openid/authorize?\
//             response_type=code%20id_token%20token&\
//             client_id={}&\
//             redirect_uri=http%3A%2F%2Flocalhost%3A3000%2F&\
//             scope=openid&\
//             state=ABCDEF&\
//             allow=true&\
//             nonce=blabla",
//             openid_client.client_id
//         ))
//         .dispatch()
//         .await;
//     let location = response.headers().get_one("Location").unwrap();
//     assert!(location.contains("error=unsupported_response_type"));

//     // unsupported_response_type
//     let response = client
//         .post(format!(
//             "/api/v1/openid/authorize?\
//             response_type=code%20id_token%20token&\
//             client_id={}&\
//             redirect_uri=http%3A%2F%2Flocalhost%3A3000%2F&\
//             scope=openid&\
//             state=ABCDEF&\
//             allow=true&\
//             nonce=blabla",
//             openid_client.client_id
//         ))
//         .dispatch()
//         .await;
//     let location = response.headers().get_one("Location").unwrap();
//     assert!(location.contains("error=unsupported_response_type"));

//     let response = client
//         .post(format!(
//             "/api/v1/openid/authorize?\
//             response_type=id_token&\
//             client_id={}&\
//             redirect_uri=http%3A%2F%2Flocalhost%3A3000%2F&\
//             scope=openid&\
//             state=ABCDEF&\
//             allow=true&\
//             nonce=blabla",
//             openid_client.client_id
//         ))
//         .dispatch()
//         .await;
//     let location = response.headers().get_one("Location").unwrap();
//     assert!(location.contains("error=unsupported_response_type"));

//     // Obtain code
//     let response = client
//         .post(format!(
//             "/api/v1/openid/authorize?\
//             response_type=code&\
//             client_id={}&\
//             redirect_uri=http%3A%2F%2Flocalhost%3A3000%2F&\
//             scope=openid&\
//             state=ABCDEF&\
//             allow=true&\
//             nonce=blabla",
//             openid_client.client_id
//         ))
//         .dispatch()
//         .await;
//     assert_eq!(response.status(), Status::Found);

//     let location = response.headers().get_one("Location").unwrap();
//     assert!(location.starts_with("http://localhost:3000/?code="));

//     // check returned state
//     let index = location.find("&state").unwrap();
//     assert_eq!("&state=ABCDEF", location.get(index..).unwrap());
//     // exchange wrong code for token should fail
//     let response = client
//         .post("/api/v1/openid/token")
//         .header(ContentType::Form)
//         .body(
//             "grant_type=authorization_code&\
//             code=ncuoew2323&\
//             redirect_uri=http%3A%2F%2Flocalhost%3A3000%2F",
//         )
//         .dispatch()
//         .await;
//     assert_eq!(response.status(), Status::BadRequest);

//     // exchange code for token
//     let code = location.get(28..index).unwrap();
//     let response = client
//         .post("/api/v1/openid/token")
//         .header(ContentType::Form)
//         .body(format!(
//             "grant_type=authorization_code&\
//             code={}&\
//             redirect_uri=http%3A%2F%2Flocalhost%3A3000%2F",
//             code
//         ))
//         .dispatch()
//         .await;
//     assert_eq!(response.status(), Status::Ok);

//     // check used code
//     let response = client
//         .post("/api/v1/openid/token")
//         .header(ContentType::Form)
//         .body(format!(
//             "grant_type=authorization_code&\
//             code={}&\
//             redirect_uri=http%3A%2F%2Flocalhost%3A3000%2F",
//             code
//         ))
//         .dispatch()
//         .await;
//     assert_eq!(response.status(), Status::BadRequest);

//     // test non-existing client
//     let response = client
//         .post(
//             "/api/v1/openid/authorize?\
//             response_type=code&\
//             client_id=666&\
//             redirect_uri=http%3A%2F%2Flocalhost%3A3000%2F&\
//             scope=openid&\
//             state=ABCDEF&\
//             nonce=blabla",
//         )
//         .dispatch()
//         .await;
//     let location = response.headers().get_one("Location").unwrap();
//     assert!(location.contains("error"));

//     // test wrong redirect uri
//     let response = client
//         .post(
//             "/api/v1/openid/authorize?\
//             response_type=code&\
//             client_id=1&\
//             redirect_uri=http%3A%2F%example%3A3000%2F&\
//             scope=openid&\
//             state=ABCDEF&\
//             nonce=blabla",
//         )
//         .dispatch()
//         .await;
//     let location = response.headers().get_one("Location").unwrap();
//     assert!(location.contains("error"));

//     // test scope doesnt contain openid
//     let response = client
//         .post(format!(
//             "/api/v1/openid/authorize?\
//             response_type=code&\
//             client_id={}&\
//             redirect_uri=http%3A%2F%2Flocalhost%3A3000%2F&\
//             scope=blabla&\
//             state=ABCDEF&\
//             allow=true&\
//             nonce=blabla",
//             openid_client.client_id
//         ))
//         .dispatch()
//         .await;
//     let location = response.headers().get_one("Location").unwrap();
//     assert!(location.contains("error=wrong_scope&error_description=scope_must_contain_openid"));

//     // test allow false
//     let response = client
//         .post(format!(
//             "/api/v1/openid/authorize?\
//             response_type=code&\
//             client_id={}&\
//             redirect_uri=http%3A%2F%2Flocalhost%3A3000%2F&\
//             scope=blabla&\
//             state=ABCDEF&\
//             allow=false&\
//             nonce=blabla",
//             openid_client.client_id
//         ))
//         .dispatch()
//         .await;
//     let location = response.headers().get_one("Location").unwrap();
//     assert!(location.contains("error=user_unauthorized"));
// }

// #[rocket::async_test]
// async fn test_openid_apps() {
//     let client = make_client().await;

//     let auth = Auth::new("admin".into(), "pass123".into());
//     let response = client.post("/api/v1/auth").json(&auth).dispatch().await;
//     assert_eq!(response.status(), Status::Ok);

//     let openid_client = NewOpenIDClient {
//         name: "Test".into(),
//         redirect_uri: "http://localhost:3000/".into(),
//         enabled: true,
//     };
//     let response = client
//         .post("/api/v1/openid")
//         .json(&openid_client)
//         .dispatch()
//         .await;
//     assert_eq!(response.status(), Status::Created);
//     let fetched_client: OAuth2Client = response.into_json().await.unwrap();
//     assert_eq!(fetched_client.name, "Test");

//     let response = client
//         .post(format!(
//             "/api/v1/openid/authorize?\
//             response_type=code&\
//             client_id={}&\
//             redirect_uri=http%3A%2F%2Flocalhost%3A3000%2F&\
//             scope=openid&\
//             state=ABCDEF&\
//             allow=true&\
//             nonce=blabla",
//             fetched_client.client_id
//         ))
//         .dispatch()
//         .await;
//     assert_eq!(response.status(), Status::Found);

//     let location = response.headers().get_one("Location").unwrap();
//     let index = location.find("&state").unwrap();
//     let code = location.get(28..index).unwrap();
//     let response = client
//         .post("/api/v1/openid/token")
//         .header(ContentType::Form)
//         .body(format!(
//             "grant_type=authorization_code&\
//             code={}&\
//             redirect_uri=http%3A%2F%2Flocalhost%3A3000%2F",
//             code
//         ))
//         .dispatch()
//         .await;
//     assert_eq!(response.status(), Status::Ok);

//     // fetch applications
//     let response = client.get("/api/v1/openid/apps/admin").dispatch().await;
//     assert_eq!(response.status(), Status::Ok);
//     let mut apps: Vec<AuthorizedApp> = response.into_json().await.unwrap();
//     assert_eq!(apps.len(), 1);

//     let mut app = apps.pop().unwrap();
//     assert_eq!(app.name, "Test");

//     // rename application
//     app.name = "My app".into();
//     let response = client
//         .put(format!("/api/v1/openid/apps/{}", app.id.unwrap()))
//         .json(&app)
//         .dispatch()
//         .await;
//     assert_eq!(response.status(), Status::Ok);

//     // fetch again to check if the name has been changed
//     let response = client.get("/api/v1/openid/apps/admin").dispatch().await;
//     assert_eq!(response.status(), Status::Ok);
//     let apps: Vec<AuthorizedApp> = response.into_json().await.unwrap();
//     assert_eq!(apps.len(), 1);
//     assert_eq!(apps[0].name, "My app");

//     // delete application
//     let response = client
//         .delete(format!("/api/v1/openid/apps/{}", app.id.unwrap()))
//         .dispatch()
//         .await;
//     assert_eq!(response.status(), Status::Ok);

//     // fetch once more to check if the application has been deleted
//     let response = client.get("/api/v1/openid/apps/admin").dispatch().await;
//     assert_eq!(response.status(), Status::Ok);
//     let apps: Vec<AuthorizedApp> = response.into_json().await.unwrap();
//     assert_eq!(apps.len(), 0);
// }

// Helper function for translating HTTP communication from `openidconnect` to `LocalClient`.
async fn http_client(request: HttpRequest) -> Result<HttpResponse, rocket::Error> {
    let client = make_client().await;
    let mut uri = request.url.path().to_string();
    if let Some(query) = request.url.query() {
        uri += "?";
        uri += query;
    }
    let rocket_request = match request.method {
        Method::GET => client.get(uri),
        Method::POST => client.post(uri),
        Method::PUT => client.put(uri),
        Method::DELETE => client.delete(uri),
        _ => unimplemented!(),
    };
    // TODO: build headers
    // for (key, value) in request.headers.iter() {
    //     let header = Header::new(key.as_str(), value.to_str().unwrap());
    //     rocket_request.add_header(header);
    // }
    let response = rocket_request.body(request.body).dispatch().await;

    let headers = HeaderMap::new();
    // TODO: deal with headers and fix lifetime
    // for header in response.headers().iter() {
    //     headers.insert(header.name().as_str(), header.value().parse().unwrap());
    // }

    Ok(HttpResponse {
        status_code: StatusCode::from_u16(response.status().code).unwrap(),
        headers,
        body: response.into_bytes().await.unwrap_or_default(),
    })
}

#[rocket::async_test]
async fn test_openid_authorization_code() {
    let issuer_url =
        IssuerUrl::new("http://localhost:8000/".to_string()).expect("Invalid issuer URL");

    // discover OpenID service
    let provider_metadata = CoreProviderMetadata::discover_async(issuer_url, http_client)
        .await
        .unwrap();

    let client_id = ClientId::new("CLIENT_ID".into());
    let client_secret = ClientSecret::new("CLIENT_SECRET".into());
    let client =
        CoreClient::from_provider_metadata(provider_metadata, client_id, Some(client_secret))
            .set_redirect_uri(
                RedirectUrl::new("http://test.server.tnt:12345/".to_string()).unwrap(),
            );
    let (authorize_url, _csrf_state, _nonce) = client
        .authorize_url(
            AuthenticationFlow::<CoreResponseType>::AuthorizationCode,
            CsrfToken::new_random,
            Nonce::new_random,
        )
        // This example is requesting access to the the user's profile including email.
        .add_scope(Scope::new("email".to_string()))
        .add_scope(Scope::new("profile".to_string()))
        .url();
    assert_eq!(authorize_url.scheme(), "http");
    assert_eq!(authorize_url.host_str(), Some("localhost"));
    assert_eq!(authorize_url.path(), "/api/v1/openid/authorize");

    let response = http_client(HttpRequest {
        url: authorize_url,
        method: Method::GET,
        headers: HeaderMap::new(),
        body: Vec::new(),
    })
    .await
    .unwrap();
    assert_eq!(response.status_code, StatusCode::FOUND);
}
