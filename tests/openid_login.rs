use chrono::{Duration, Utc};
use common::{exceed_enterprise_limits, make_test_client};
use defguard::enterprise::db::models::openid_provider::DirectorySyncTarget;
use defguard::enterprise::db::models::openid_provider::DirectorySyncUserBehavior;
use defguard::{
    enterprise::{
        handlers::openid_providers::AddProviderData,
        license::{set_cached_license, License},
    },
    handlers::Auth,
};
use reqwest::{StatusCode, Url};
use serde::Deserialize;

pub mod common;
use self::common::client::TestClient;

async fn make_client() -> TestClient {
    let (client, _) = make_test_client().await;
    client
}

// Temporarily disabled because of the issue with test_openid_login
// async fn make_client_with_real_url() -> TestClient {
//     let (client, _) = make_test_client_with_real_url().await;
//     client
// }

#[tokio::test]
async fn test_openid_providers() {
    let client = make_client().await;

    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    exceed_enterprise_limits(&client).await;

    let provider_data = AddProviderData {
        name: "test".to_string(),
        base_url: "https://accounts.google.com".to_string(),
        client_id: "client_id".to_string(),
        client_secret: "client_secret".to_string(),
        display_name: Some("display_name".to_string()),
        admin_email: None,
        google_service_account_email: None,
        google_service_account_key: None,
        directory_sync_enabled: false,
        directory_sync_interval: 100,
        directory_sync_user_behavior: DirectorySyncUserBehavior::Keep.to_string(),
        directory_sync_admin_behavior: DirectorySyncUserBehavior::Keep.to_string(),
        directory_sync_target: DirectorySyncTarget::All.to_string(),
    };

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

    let provider: UrlResponse = response.json().await;

    let url = Url::parse(&provider.url).unwrap();

    let client_id = url
        .query_pairs()
        .find(|(key, _)| key == "client_id")
        .unwrap();
    assert_eq!(client_id.1, "client_id");

    let mut query_pairs = url.query_pairs();
    let nonce = query_pairs.clone().find(|(key, _)| key == "nonce");
    assert!(nonce.is_some());
    let state = query_pairs.clone().find(|(key, _)| key == "state");
    assert!(state.is_some());
    let redirect_uri = query_pairs.find(|(key, _)| key == "redirect_uri");
    assert!(redirect_uri.is_some());

    // Test that the endpoint is forbidden when the license is expired
    let new_license = License::new(
        "test".to_string(),
        false,
        Some(Utc::now() - Duration::days(1)),
        None,
    );
    set_cached_license(Some(new_license));
    let response = client.get("/api/v1/openid/auth_info").send().await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

// FIXME: tihs test sometimes fails because of test_openid_providers.
// The license state is possibly preserved between those two. This requires further research.
// #[tokio::test]
// async fn test_openid_login() {
//     // Test setup
//     let client = make_client_with_real_url().await;
//     let auth = Auth::new("admin", "pass123");
//     let response = client.post("/api/v1/auth").json(&auth).send().await;
//     assert_eq!(response.status(), StatusCode::OK);
//     let url = client.base_url();

//     // Add an OpenID client
//     let redirect_uri = format!("{}/auth/callback", &url);
//     let openid_client = NewOpenIDClient {
//         name: "Defguard".into(),
//         redirect_uri: vec![redirect_uri],
//         scope: vec!["openid".into(), "email".into(), "profile".into()],
//         enabled: true,
//     };
//     let response = client
//         .post("/api/v1/oauth")
//         .json(&openid_client)
//         .send()
//         .await;
//     assert_eq!(response.status(), StatusCode::CREATED);
//     let response = client.get("/api/v1/oauth").send().await;
//     assert_eq!(response.status(), StatusCode::OK);
//     let openid_clients: Vec<OAuth2Client<Id>> = response.json().await;
//     assert_eq!(openid_clients.len(), 1);
//     let openid_client = openid_clients.first().unwrap();
//     assert_eq!(openid_client.name, "Defguard");

//     // Add the provider (ourselves)
//     let (secret, id) = (
//         openid_client.client_secret.clone(),
//         openid_client.client_id.clone(),
//     );
//     let provider_data = AddProviderData::new(
//         "Custom",
//         format!("{}/", &url).as_str(),
//         id.to_string().as_str(),
//         &secret,
//         Some("Defguard"),
//     );
//     let response = client
//         .post("/api/v1/openid/provider")
//         .json(&provider_data)
//         .send()
//         .await;
//     assert_eq!(response.status(), StatusCode::CREATED);

//     // Logout to make sure we start from a clean slate
//     client.post("/api/v1/auth/logout").send().await;

//     // Get the provider's authorization endpoint (and button display name)
//     let response = client.get("/api/v1/openid/auth_info").send().await;
//     assert_eq!(response.status(), StatusCode::OK);
//     #[derive(Deserialize, Debug)]
//     struct AuthInfoResponse {
//         button_display_name: String,
//         url: Url,
//     }
//     let response_body: AuthInfoResponse = response.json().await;
//     assert_eq!(response_body.button_display_name, "Defguard");

//     // Begin OIDC login at the provider's authorization endpoint
//     let url = format!(
//         "{}?{}",
//         response_body.url.path(),
//         response_body.url.query().unwrap()
//     );
//     let response = client.get(&url).send().await;
//     assert_eq!(response.status(), StatusCode::FOUND);

//     // A user should now be redirected to the login page
//     #[derive(Deserialize, Debug)]
//     struct LoginResponse {
//         url: String,
//     }
//     let response = client.post("/api/v1/auth").json(&auth).send().await;
//     let login_response: LoginResponse = response.json().await;

//     // During the flow, the user may be first redirected to a consent page, simualte that here
//     let url = Url::parse(&login_response.url).unwrap();
//     let path = url.path();
//     let query = url.query().unwrap();
//     let url = format!("{}?{}", path, query);
//     let response = client.get(&url).send().await;
//     assert_eq!(response.status(), StatusCode::FOUND);
//     let location = response.headers().get("location").unwrap();
//     let location = location.to_str().unwrap();
//     assert!(location.starts_with("/consent"));

//     // Consent to everything by adding the allow=true query parameter and sending a post request this time
//     let url = Url::parse(&login_response.url).unwrap();
//     let mut query_pairs = url
//         .query_pairs()
//         .into_owned()
//         .collect::<Vec<(String, String)>>();
//     query_pairs.push(("allow".to_string(), "true".to_string()));
//     let pairs = query_pairs
//         .iter()
//         .map(|(key, value)| format!("{}={}", key, value))
//         .collect::<Vec<String>>()
//         .join("&");
//     let path = format!("{}?{}", url.path(), pairs);
//     let response = client.post(&path).send().await;
//     assert_eq!(response.status(), StatusCode::FOUND);

//     // logout to make sure the session won't be carried over after the callback later
//     client.post("/api/v1/auth/logout").send().await;

//     // Extract callback data from the response's location header
//     let location = response.headers().get("location").unwrap();
//     let location = location.to_str().unwrap();
//     let url = Url::parse(location).unwrap();
//     let query_pairs = url
//         .query_pairs()
//         .into_owned()
//         .collect::<Vec<(String, String)>>();
//     let code = query_pairs
//         .iter()
//         .find(|(key, _)| key == "code")
//         .unwrap()
//         .1
//         .clone();
//     let state = query_pairs
//         .iter()
//         .find(|(key, _)| key == "state")
//         .unwrap()
//         .1
//         .clone();

//     // Post the callback with the data inside a json payload
//     #[derive(Serialize, Debug)]
//     struct AuthResponse {
//         code: String,
//         state: String,
//     }
//     let auth_response = AuthResponse { code, state };
//     let response = client
//         .post("/api/v1/openid/callback")
//         .json(&auth_response)
//         .send()
//         .await;
//     assert_eq!(response.status(), StatusCode::OK);

//     // Am I logged in?
//     let response = client.get("/api/v1/me").send().await;
//     assert_eq!(response.status(), StatusCode::OK);
// }
