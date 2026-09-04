use chrono::{Duration, Utc};
use defguard_common::db::{
    Id,
    models::{oauth2client::OAuth2Client, settings::OpenIdUsernameHandling},
};
use defguard_core::{
    enterprise::{
        db::models::openid_provider::{
            DirectorySyncTarget, DirectorySyncUserBehavior, OpenIdProviderKind,
        },
        handlers::openid_providers::AddProviderData,
        license::{License, LicenseTier, SupportType, set_cached_license},
    },
    handlers::{Auth, openid_clients::NewOpenIDClient},
};
use reqwest::{StatusCode, Url};
use serde::Deserialize;
use serde_json::json;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

use super::common::{
    exceed_enterprise_limits, make_client, make_network, setup_pool, update_location_mfa_flows,
};
use crate::api::PaginatedApiResponse;

#[derive(Deserialize)]
struct UrlResponse {
    url: String,
}

#[derive(Deserialize)]
struct CurrentProviderResponse {
    provider: CurrentProvider,
}

#[derive(Deserialize)]
struct CurrentProvider {
    disable_password_management: bool,
}

#[sqlx::test]
async fn test_openid_providers(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let client = make_client(pool).await;

    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    exceed_enterprise_limits(&client).await;

    let provider_data = AddProviderData {
        name: "test".to_owned(),
        // FIXME: this won't work offline.
        base_url: "https://accounts.google.com".to_owned(),
        kind: OpenIdProviderKind::Google,
        client_id: "client_id".to_owned(),
        client_secret: "client_secret".to_owned(),
        display_name: Some("display_name".to_owned()),
        admin_email: None,
        google_service_account_email: None,
        google_service_account_key: None,
        directory_sync_enabled: false,
        directory_sync_interval: 100,
        directory_sync_user_behavior: DirectorySyncUserBehavior::Keep.to_string(),
        directory_sync_admin_behavior: DirectorySyncUserBehavior::Keep.to_string(),
        directory_sync_target: DirectorySyncTarget::All.to_string(),
        create_account: false,
        okta_dirsync_client_id: None,
        okta_private_jwk: None,
        directory_sync_group_match: None,
        username_handling: OpenIdUsernameHandling::PruneEmailDomain,
        jumpcloud_api_key: None,
        prefetch_users: false,
        disable_password_management: false,
        directory_sync_user_groups: None,
    };

    let response = client
        .post("/api/v1/openid/provider")
        .json(&provider_data)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::CREATED);

    let response = client.get("/api/v1/openid/auth_info").send().await;

    assert_eq!(response.status(), StatusCode::OK);

    let provider = response.json::<UrlResponse>().await;

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
        "test".to_owned(),
        false,
        Some(Utc::now() - Duration::days(1)),
        None,
        None,
        LicenseTier::Business,
        SupportType::Basic,
        vec![],
    );
    set_cached_license(Some(new_license));
    let response = client.get("/api/v1/openid/auth_info").send().await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[sqlx::test]
async fn test_modify_openid_provider_persists_disable_password_management(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let client = make_client(pool).await;

    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    exceed_enterprise_limits(&client).await;

    let mut provider_data = AddProviderData {
        name: "test".to_owned(),
        base_url: "https://accounts.google.com".to_owned(),
        kind: OpenIdProviderKind::Google,
        client_id: "client_id".to_owned(),
        client_secret: "client_secret".to_owned(),
        display_name: Some("display_name".to_owned()),
        admin_email: None,
        google_service_account_email: None,
        google_service_account_key: None,
        directory_sync_enabled: false,
        directory_sync_interval: 100,
        directory_sync_user_behavior: DirectorySyncUserBehavior::Keep.to_string(),
        directory_sync_admin_behavior: DirectorySyncUserBehavior::Keep.to_string(),
        directory_sync_target: DirectorySyncTarget::All.to_string(),
        create_account: false,
        okta_dirsync_client_id: None,
        okta_private_jwk: None,
        directory_sync_group_match: None,
        username_handling: OpenIdUsernameHandling::PruneEmailDomain,
        jumpcloud_api_key: None,
        prefetch_users: false,
        disable_password_management: false,
        directory_sync_user_groups: None,
    };

    let response = client
        .post("/api/v1/openid/provider")
        .json(&provider_data)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // Toggle the flag and update the provider via PUT.
    provider_data.disable_password_management = true;
    let response = client
        .put("/api/v1/openid/provider/test")
        .json(&provider_data)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    // Read back the current provider and assert the flag was persisted.
    let response = client.get("/api/v1/openid/provider/current").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let body: CurrentProviderResponse = response.json().await;
    assert!(
        body.provider.disable_password_management,
        "disable_password_management should be persisted as true after update"
    );
}

// FIXME: this test sometimes fails because of test_openid_providers.
// The license state is possibly preserved between those two. This requires further research.
#[sqlx::test]
async fn test_openid_login(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let client = make_client(pool).await;
    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let mut url = client.base_url();
    url.push('/'); // `CoreProviderMetadata::discover_async` expects the slash.

    // Add an OpenID client
    let redirect_uri = format!("{url}/auth/callback");
    // let redirect_uri = String::from("http://localhost:8000/auth/callback");
    let openid_client = NewOpenIDClient {
        name: "Defguard".into(),
        redirect_uri: vec![redirect_uri],
        scope: vec!["openid".into(), "email".into(), "profile".into()],
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
    let openid_clients = response
        .json::<PaginatedApiResponse<OAuth2Client<Id>>>()
        .await
        .data;
    assert_eq!(openid_clients.len(), 1);
    let openid_client = openid_clients.first().unwrap();
    assert_eq!(openid_client.name, "Defguard");

    // Add the provider (ourselves)
    let provider_data = AddProviderData {
        name: "Custom".into(),
        base_url: url,
        kind: OpenIdProviderKind::Custom,
        client_id: openid_client.client_id.clone(),
        client_secret: openid_client.client_secret.clone(),
        display_name: Some("Defguard".to_owned()),
        admin_email: None,
        google_service_account_email: None,
        google_service_account_key: None,
        directory_sync_enabled: false,
        directory_sync_interval: 100,
        directory_sync_user_behavior: DirectorySyncUserBehavior::Keep.to_string(),
        directory_sync_admin_behavior: DirectorySyncUserBehavior::Keep.to_string(),
        directory_sync_target: DirectorySyncTarget::All.to_string(),
        create_account: false,
        okta_dirsync_client_id: None,
        okta_private_jwk: None,
        directory_sync_group_match: None,
        username_handling: OpenIdUsernameHandling::PruneEmailDomain,
        jumpcloud_api_key: None,
        prefetch_users: false,
        disable_password_management: false,
        directory_sync_user_groups: None,
    };
    let response = client
        .post("/api/v1/openid/provider")
        .json(&provider_data)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // Logout to make sure we start from a clean slate
    client.post("/api/v1/auth/logout").send().await;

    // Get the provider's authorization endpoint (and button display name)
    let response = client.get("/api/v1/openid/auth_info").send().await;
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "{}",
        response.text().await
    );

    #[derive(Deserialize)]
    struct AuthInfoResponse {
        button_display_name: String,
        url: Url,
    }
    let response_body = response.json::<AuthInfoResponse>().await;
    assert_eq!(response_body.button_display_name, "Defguard");

    // Begin OIDC login at the provider's authorization endpoint.
    let url = format!(
        "{}?{}",
        response_body.url.path(),
        response_body.url.query().unwrap()
    );
    let response = client.get(&url).send().await;
    assert_eq!(response.status(), StatusCode::FOUND);

    // A user should now be redirected to the login page
    // let response = client.post("/api/v1/auth").json(&auth).send().await;
    // let login_response = response.json::<UrlResponse>().await;

    // // During the flow, the user may be first redirected to a consent page, simualte that here
    // let url = Url::parse(&login_response.url).unwrap();
    // let path = url.path();
    // let query = url.query().unwrap();
    // let url = format!("{}?{}", path, query);
    // let response = client.get(&url).send().await;
    // assert_eq!(response.status(), StatusCode::FOUND);
    // let location = response.headers().get("location").unwrap();
    // let location = location.to_str().unwrap();
    // assert!(location.starts_with("/consent"));

    // // Consent to everything by adding the allow=true query parameter and sending a post request this time
    // let url = Url::parse(&login_response.url).unwrap();
    // let mut query_pairs = url
    //     .query_pairs()
    //     .into_owned()
    //     .collect::<Vec<(String, String)>>();
    // query_pairs.push(("allow".to_string(), "true".to_string()));
    // let pairs = query_pairs
    //     .iter()
    //     .map(|(key, value)| format!("{key}={value}"))
    //     .collect::<Vec<String>>()
    //     .join("&");
    // let path = format!("{}?{pairs}", url.path());
    // let response = client.post(&path).send().await;
    // assert_eq!(response.status(), StatusCode::FOUND);

    // // logout to make sure the session won't be carried over after the callback later
    // client.post("/api/v1/auth/logout").send().await;

    // // Extract callback data from the response's location header
    // let location = response.headers().get("location").unwrap();
    // let location = location.to_str().unwrap();
    // let url = Url::parse(location).unwrap();
    // let query_pairs = url
    //     .query_pairs()
    //     .into_owned()
    //     .collect::<Vec<(String, String)>>();
    // let code = query_pairs
    //     .iter()
    //     .find(|(key, _)| key == "code")
    //     .unwrap()
    //     .1
    //     .clone();
    // let state = query_pairs
    //     .iter()
    //     .find(|(key, _)| key == "state")
    //     .unwrap()
    //     .1
    //     .clone();

    // // Post the callback with the data inside a json payload
    // #[derive(Serialize)]
    // struct AuthResponse {
    //     code: String,
    //     state: String,
    // }
    // let auth_response = AuthResponse { code, state };
    // let response = client
    //     .post("/api/v1/openid/callback")
    //     .json(&auth_response)
    //     .send()
    //     .await;
    // assert_eq!(response.status(), StatusCode::OK);

    // // Am I logged in?
    // let response = client.get("/api/v1/me").send().await;
    // assert_eq!(response.status(), StatusCode::OK);
}

/// Deleting an OIDC provider must actually remove it, not merely report success.
///
/// Regression test: the OIDC-flow conflict check added for multi-step MFA replaced the
/// `provider.delete(...)` call, so the handler committed an empty transaction, logged and
/// audited a deletion, and returned 200 while the provider row survived. An admin removing
/// a compromised IdP would have been told it was gone while it stayed live for SSO and for
/// OIDC MFA steps.
#[sqlx::test]
async fn test_delete_openid_provider_removes_it(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let client = make_client(pool).await;

    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    exceed_enterprise_limits(&client).await;

    let provider_data = AddProviderData {
        name: "to-delete".to_owned(),
        base_url: "https://example.com".to_owned(),
        kind: OpenIdProviderKind::Custom,
        client_id: "client_id".to_owned(),
        client_secret: "client_secret".to_owned(),
        display_name: Some("display_name".to_owned()),
        admin_email: None,
        google_service_account_email: None,
        google_service_account_key: None,
        directory_sync_enabled: false,
        directory_sync_interval: 100,
        directory_sync_user_behavior: DirectorySyncUserBehavior::Keep.to_string(),
        directory_sync_admin_behavior: DirectorySyncUserBehavior::Keep.to_string(),
        directory_sync_target: DirectorySyncTarget::All.to_string(),
        create_account: false,
        okta_dirsync_client_id: None,
        okta_private_jwk: None,
        directory_sync_group_match: None,
        username_handling: OpenIdUsernameHandling::PruneEmailDomain,
        jumpcloud_api_key: None,
        prefetch_users: false,
        disable_password_management: false,
        directory_sync_user_groups: None,
    };

    let response = client
        .post("/api/v1/openid/provider")
        .json(&provider_data)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // The provider exists before deletion.
    let response = client.get("/api/v1/openid/provider/to-delete").send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // No location has OIDC in an MFA flow, so deletion is unobstructed.
    let response = client
        .delete("/api/v1/openid/provider/to-delete")
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    // The provider must actually be gone. `get_openid_provider` answers 204 when
    // no provider carries the requested name.
    let response = client.get("/api/v1/openid/provider/to-delete").send().await;
    assert_eq!(
        response.status(),
        StatusCode::NO_CONTENT,
        "provider still exists after a successful DELETE"
    );
}

/// Deleting an OIDC provider proceeds even while locations still reference OIDC in their MFA
/// flows, and reports those locations so an admin can be warned.
///
/// Removing a provider is frequently incident response (a compromised or decommissioned IdP), so
/// revocation is deliberately not blocked. Access still fails closed: the affected flows become
/// unsatisfiable, so connect-time MFA refuses rather than letting anyone through.
#[sqlx::test]
async fn test_delete_openid_provider_reports_affected_locations(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;

    let client = make_client(pool).await;

    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    exceed_enterprise_limits(&client).await;

    // Set the licence explicitly: a sibling test in this file leaves an expired one cached, and
    // saving an OIDC flow step requires an active business licence.
    set_cached_license(Some(License::new(
        "test".to_owned(),
        false,
        Some(Utc::now() + Duration::days(365)),
        None,
        None,
        LicenseTier::Business,
        SupportType::Basic,
        vec![],
    )));

    // The provider has to exist first: an OIDC flow cannot be saved without one.
    let provider_data = AddProviderData {
        name: "to-delete".to_owned(),
        base_url: "https://example.com".to_owned(),
        kind: OpenIdProviderKind::Custom,
        client_id: "client_id".to_owned(),
        client_secret: "client_secret".to_owned(),
        display_name: Some("display_name".to_owned()),
        admin_email: None,
        google_service_account_email: None,
        google_service_account_key: None,
        directory_sync_enabled: false,
        directory_sync_interval: 100,
        directory_sync_user_behavior: DirectorySyncUserBehavior::Keep.to_string(),
        directory_sync_admin_behavior: DirectorySyncUserBehavior::Keep.to_string(),
        directory_sync_target: DirectorySyncTarget::All.to_string(),
        create_account: false,
        okta_dirsync_client_id: None,
        okta_private_jwk: None,
        directory_sync_group_match: None,
        username_handling: OpenIdUsernameHandling::PruneEmailDomain,
        jumpcloud_api_key: None,
        prefetch_users: false,
        disable_password_management: false,
        directory_sync_user_groups: None,
    };
    let response = client
        .post("/api/v1/openid/provider")
        .json(&provider_data)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    let location_id = make_network(&client, "oidc-location")
        .await
        .json::<serde_json::Value>()
        .await["id"]
        .as_i64()
        .unwrap();

    let response = client
        .post("/api/v1/mfa-flow")
        .json(&json!({
            "title": "OIDC Flow",
            "steps": [{ "methods": ["oidc"] }]
        }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let flow_id = response.json::<serde_json::Value>().await["id"]
        .as_i64()
        .unwrap();

    // Assign the OIDC flow as the location's default, so the location genuinely depends on it.
    let response = update_location_mfa_flows(
        &client,
        location_id,
        json!([{ "flow_id": flow_id, "is_default": true, "group_ids": [] }]),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);

    // Deletion proceeds and names the location whose flows are now unsatisfiable.
    let response = client
        .delete("/api/v1/openid/provider/to-delete")
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await;
    assert_eq!(
        body["affected_locations"],
        json!(["oidc-location"]),
        "the location depending on OIDC must be reported back to the caller"
    );

    // The provider is gone despite the outstanding OIDC reference: revocation is not blocked.
    let response = client.get("/api/v1/openid/provider/to-delete").send().await;
    assert_eq!(
        response.status(),
        StatusCode::NO_CONTENT,
        "provider still exists after a successful DELETE"
    );
}
