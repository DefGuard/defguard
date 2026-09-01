use std::borrow::Cow;

use defguard_common::db::{
    Id,
    models::{
        OAuth2AuthorizedApp, OAuth2Token,
        oauth2client::{OAuth2Client, OAuth2ClientSafe},
    },
};
use defguard_core::handlers::{Auth, openid_clients::NewOpenIDClient};
use reqwest::{
    StatusCode, Url,
    header::{AUTHORIZATION, CONTENT_TYPE},
};
use serde_json::{Value, json};
use sqlx::{
    PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
};

use super::common::{client::TestClient, make_client_with_db, setup_pool};
use crate::api::PaginatedApiResponse;

#[sqlx::test]
async fn test_authorize(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (client, pool) = make_client_with_db(pool).await;

    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // create OAuth2 client
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
    let oauth_client: OAuth2Client<Id> = response.json().await;

    // authorize client for test user
    OAuth2AuthorizedApp::new(1, oauth_client.id)
        .save(&pool)
        .await
        .unwrap();

    // wrong response type
    let response = client
        .get(
            "/api/v1/oauth/authorize?\
            response_type=wrong&\
            client_id=MyClient&\
            redirect_uri=http%3A%2F%2Flocalhost%3A3000%2F&\
            scope=default-scope&\
            state=ABCDEF",
        )
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // error response
    let response = client
        .get(
            "/api/v1/oauth/authorize?\
            response_type=code&\
            client_id=MyClient&\
            redirect_uri=http%3A%2F%2Flocalhost%3A3000%2F&\
            scope=openid&\
            state=ABCDEF",
        )
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FOUND);
    let redirect_url = Url::parse(
        response
            .headers()
            .get("Location")
            .unwrap()
            .to_str()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(redirect_url.domain().unwrap(), "localhost");
    let mut pairs = redirect_url.query_pairs();
    assert_eq!(pairs.count(), 2);
    assert_eq!(
        pairs.next(),
        Some((Cow::Borrowed("error"), Cow::Borrowed("unauthorized_client")))
    );
    assert_eq!(
        pairs.next(),
        Some((Cow::Borrowed("state"), Cow::Borrowed("ABCDEF")))
    );

    // error response without state
    let response = client
        .get(format!(
            "/api/v1/oauth/authorize?\
            response_type=code&\
            client_id={}&\
            redirect_uri=http%3A%2F%2Flocalhost%3A3000%2F&\
            scope=invalid",
            oauth_client.client_id
        ))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FOUND);
    let redirect_url = Url::parse(
        response
            .headers()
            .get("Location")
            .unwrap()
            .to_str()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(redirect_url.domain().unwrap(), "localhost");
    let mut pairs = redirect_url.query_pairs();
    assert_eq!(pairs.count(), 1);
    assert_eq!(
        pairs.next(),
        Some((Cow::Borrowed("error"), Cow::Borrowed("invalid_scope")))
    );

    // successful response
    let response = client
        .get(format!(
            "/api/v1/oauth/authorize?\
            response_type=code&\
            client_id={}&\
            redirect_uri=http%3A%2F%2Ftest.server.tnt%3A12345%2F&\
            scope=openid&\
            state=ABCDEF",
            oauth_client.client_id
        ))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FOUND);
    let redirect_url = Url::parse(
        response
            .headers()
            .get("Location")
            .unwrap()
            .to_str()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(redirect_url.domain().unwrap(), "test.server.tnt");
    let mut pairs = redirect_url.query_pairs();
    assert_eq!(pairs.count(), 2);
    assert_eq!(pairs.next().unwrap().0, Cow::Borrowed("code"),);
    assert_eq!(
        pairs.next(),
        Some((Cow::Borrowed("state"), Cow::Borrowed("ABCDEF")))
    );
}

#[sqlx::test]
async fn test_openid_app_management_access(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (client, _) = make_client_with_db(pool).await;

    // login as admin
    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // add app
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

    // list apps
    let response = client.get("/api/v1/oauth").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let apps = response
        .json::<PaginatedApiResponse<OAuth2Client<Id>>>()
        .await
        .data;
    assert_eq!(apps.len(), 1);
    let test_app = &apps[0];
    assert_eq!(test_app.name, oauth2client.name);

    // fetch app details
    let response = client
        .get(format!("/api/v1/oauth/{}", test_app.client_id))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let app: OAuth2Client<Id> = response.json().await;
    assert_eq!(app.name, oauth2client.name);

    // edit app
    let oauth2client = NewOpenIDClient {
        name: "Changed test client".into(),
        redirect_uri: vec!["http://test.server.tnt:12345/".into()],
        scope: vec!["openid email".into()],
        enabled: true,
    };
    let response = client
        .put(format!("/api/v1/oauth/{}", test_app.client_id))
        .json(&oauth2client)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    // change app state
    let data = json!(
        {"enabled": false}
    );
    let response = client
        .post(format!("/api/v1/oauth/{}", test_app.client_id))
        .json(&data)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    // fetch changed app details
    let response = client
        .get(format!("/api/v1/oauth/{}", test_app.client_id))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let app: OAuth2Client<Id> = response.json().await;
    assert_eq!(app.name, oauth2client.name);
    assert!(!app.enabled);

    // delete app
    let response = client
        .delete(format!("/api/v1/oauth/{}", test_app.client_id))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    // list apps
    let response = client.get("/api/v1/oauth").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let apps = response
        .json::<PaginatedApiResponse<OAuth2Client<Id>>>()
        .await
        .data;
    assert_eq!(apps.len(), 0);

    // add another app for further testing
    let oauth2client = NewOpenIDClient {
        name: "New test client".into(),
        redirect_uri: vec!["http://test.server.tnt:12345/".into()],
        scope: vec!["openid phone".into()],
        enabled: true,
    };
    let response = client
        .post("/api/v1/oauth")
        .json(&oauth2client)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let response = client.get("/api/v1/oauth").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let apps = response
        .json::<PaginatedApiResponse<OAuth2Client<Id>>>()
        .await
        .data;
    let test_app = &apps[0];

    // login as standard user
    let auth = Auth::new("hpotter", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // standard user cannot list apps
    let response = client.get("/api/v1/oauth").send().await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    // standard user cannot get sensitive app details
    let response = client
        .get(format!("/api/v1/oauth/{}", test_app.client_id))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let _response_details: OAuth2ClientSafe = response.json().await;

    // standard user cannot add apps
    let oauth2client = NewOpenIDClient {
        name: "Another test client".into(),
        redirect_uri: vec!["http://test.com/redirect".into()],
        scope: vec!["openid profile".into()],
        enabled: true,
    };
    let response = client
        .post("/api/v1/oauth")
        .json(&oauth2client)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    // standard user cannot edit apps
    let response = client
        .put(format!("/api/v1/oauth/{}", test_app.client_id))
        .json(&oauth2client)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    // standard user cannot change app status
    let data = json!(
        {"enabled": false}
    );
    let response = client
        .post(format!("/api/v1/oauth/{}", test_app.client_id))
        .json(&data)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    // standard user cannot delete apps
    let response = client
        .delete(format!("/api/v1/oauth/{}", test_app.client_id))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[sqlx::test]
async fn test_authorize_consent(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (client, _) = make_client_with_db(pool).await;

    // Establish a session.
    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // Create an OAuth2 client.
    let oauth2client = NewOpenIDClient {
        name: "Consent test client".into(),
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
    let oauth_client: OAuth2Client<Id> = response.json().await;

    // User consents via the POST /authorize handler (secure_authorization).
    let response = client
        .post(format!(
            "/api/v1/oauth/authorize?\
            allow=true&\
            response_type=code&\
            client_id={}&\
            redirect_uri=http%3A%2F%2Ftest.server.tnt%3A12345%2F&\
            scope=openid&\
            state=ABCDEF",
            oauth_client.client_id
        ))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FOUND);

    // Extract the authorization code and verify state is echoed back.
    let redirect_url = Url::parse(
        response
            .headers()
            .get("Location")
            .unwrap()
            .to_str()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(redirect_url.domain().unwrap(), "test.server.tnt");
    let code = redirect_url
        .query_pairs()
        .find(|(k, _)| k == "code")
        .unwrap()
        .1
        .into_owned();
    assert!(
        redirect_url
            .query_pairs()
            .any(|(k, v)| k == "state" && v == "ABCDEF")
    );

    // Exchange the authorization code for a token.
    // Credentials are passed as form fields; the token endpoint accepts either
    // Basic auth or client_id/client_secret in the body.
    let response = client
        .post("/api/v1/oauth/token")
        .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(format!(
            "grant_type=authorization_code&\
            code={code}&\
            redirect_uri=http%3A%2F%2Ftest.server.tnt%3A12345%2F&\
            client_id={}&\
            client_secret={}",
            oauth_client.client_id, oauth_client.client_secret
        ))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
}

#[sqlx::test]
async fn test_authorize_consent_wrong_client(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (client, _) = make_client_with_db(pool).await;

    // Establish a session - secure_authorization requires SessionInfo.
    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // POST with a non-existent client_id. The handler cannot validate the
    // redirect_uri (is_redirect_allowed stays false), so it redirects to the
    // defguard base URL with error=unauthorized_client instead of the provided
    // redirect_uri. This prevents open redirects (DG25-17).
    let response = client
        .post(
            "/api/v1/oauth/authorize?\
            allow=true&\
            response_type=code&\
            client_id=NonExistentClient&\
            redirect_uri=http%3A%2F%2Fattacker.example.com%2F&\
            scope=openid&\
            state=ABCDEF",
        )
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FOUND);
    let redirect_url = Url::parse(
        response
            .headers()
            .get("Location")
            .unwrap()
            .to_str()
            .unwrap(),
    )
    .unwrap();
    // Must NOT redirect to the caller-supplied redirect_uri.
    assert_ne!(redirect_url.domain().unwrap(), "attacker.example.com");
    let mut pairs = redirect_url.query_pairs();
    assert_eq!(
        pairs.next(),
        Some((Cow::Borrowed("error"), Cow::Borrowed("unauthorized_client")))
    );
    assert_eq!(
        pairs.next(),
        Some((Cow::Borrowed("state"), Cow::Borrowed("ABCDEF")))
    );
}

#[sqlx::test]
async fn test_token_client_credentials(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (client, _) = make_client_with_db(pool).await;

    let response = client
        .post("/api/v1/oauth/token")
        .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body("client_id=WrongClient&client_secret=WrongSecret&grant_type=code")
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[sqlx::test]
async fn dg26_7_test_state_parameter_validation(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (client, pool) = make_client_with_db(pool).await;

    // Authenticate as admin.
    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // Create an OAuth2 client.
    let oauth2client = NewOpenIDClient {
        name: "State test client".into(),
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
    let oauth_client: OAuth2Client<Id> = response.json().await;

    // Pre-authorise the app for the admin user (id=1).
    OAuth2AuthorizedApp::new(1, oauth_client.id)
        .save(&pool)
        .await
        .unwrap();

    // A numeric-only state value (e.g. "123456") must be accepted: all digits are
    // within VSCHAR (%x30-39) so the backend should echo the state back in the redirect.
    let response = client
        .get(format!(
            "/api/v1/oauth/authorize?\
            response_type=code&\
            client_id={}&\
            redirect_uri=http%3A%2F%2Ftest.server.tnt%3A12345%2F&\
            scope=openid&\
            state=123456",
            oauth_client.client_id
        ))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FOUND);
    let redirect_url = Url::parse(
        response
            .headers()
            .get("Location")
            .unwrap()
            .to_str()
            .unwrap(),
    )
    .unwrap();
    // Redirect must carry the auth code and echo the state back unchanged.
    assert!(
        redirect_url
            .query_pairs()
            .any(|(k, v)| k == "state" && v == "123456")
    );

    // VSCHAR boundary: space (0x20) is the lowest valid character - must be accepted.
    let response = client
        .get(format!(
            "/api/v1/oauth/authorize?\
            response_type=code&\
            client_id={}&\
            redirect_uri=http%3A%2F%2Ftest.server.tnt%3A12345%2F&\
            scope=openid&\
            state=%20",
            oauth_client.client_id
        ))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FOUND);
    assert!(
        Url::parse(
            response
                .headers()
                .get("Location")
                .unwrap()
                .to_str()
                .unwrap(),
        )
        .unwrap()
        .query_pairs()
        .any(|(k, v)| k == "state" && v == " ")
    );

    // VSCHAR boundary: tilde (0x7E) is the highest valid character - must be accepted.
    let response = client
        .get(format!(
            "/api/v1/oauth/authorize?\
            response_type=code&\
            client_id={}&\
            redirect_uri=http%3A%2F%2Ftest.server.tnt%3A12345%2F&\
            scope=openid&\
            state=~",
            oauth_client.client_id
        ))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FOUND);
    assert!(
        Url::parse(
            response
                .headers()
                .get("Location")
                .unwrap()
                .to_str()
                .unwrap(),
        )
        .unwrap()
        .query_pairs()
        .any(|(k, v)| k == "state" && v == "~")
    );

    // VSCHAR boundary: DEL (0x7F) is one above the valid range - must be rejected with 400.
    let response = client
        .get(format!(
            "/api/v1/oauth/authorize?\
            response_type=code&\
            client_id={}&\
            redirect_uri=http%3A%2F%2Ftest.server.tnt%3A12345%2F&\
            scope=openid&\
            state=%7F",
            oauth_client.client_id
        ))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // VSCHAR boundary: US (0x1F) is one below the valid range - must be rejected with 400.
    let response = client
        .get(format!(
            "/api/v1/oauth/authorize?\
            response_type=code&\
            client_id={}&\
            redirect_uri=http%3A%2F%2Ftest.server.tnt%3A12345%2F&\
            scope=openid&\
            state=%1F",
            oauth_client.client_id
        ))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // An empty state (state= present but empty) must be rejected with 400.
    // RFC 6749 Appendix A.5 requires 1*VSCHAR - at least one character.
    let response = client
        .get(format!(
            "/api/v1/oauth/authorize?\
            response_type=code&\
            client_id={}&\
            redirect_uri=http%3A%2F%2Ftest.server.tnt%3A12345%2F&\
            scope=openid&\
            state=",
            oauth_client.client_id
        ))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // A state containing bytes outside VSCHAR (%x20-7E) must be rejected with 400.
    // The raw bytes \xEE\xFF\x02\x03 are percent-encoded below.
    let response = client
        .get(format!(
            "/api/v1/oauth/authorize?\
            response_type=code&\
            client_id={}&\
            redirect_uri=http%3A%2F%2Ftest.server.tnt%3A12345%2F&\
            scope=openid&\
            state=%EE%FF%02%03",
            oauth_client.client_id
        ))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

/// Regression test for DG26-6: scope parameter must be validated per individual element, not as a
/// whole. Sending scope=openid%20email when the client only allows ["openid"] must be rejected
/// with invalid_scope.
#[sqlx::test]
async fn dg26_6_test_authorize_scope_validation(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (client, pool) = make_client_with_db(pool).await;

    // authenticate
    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // create client with only "openid" scope
    let oauth2client = NewOpenIDClient {
        name: "Scope test client".into(),
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
    let oauth2client: OAuth2Client<Id> = response.json().await;

    // authorize client for the test user so session-based redirect returns a code, not consent
    OAuth2AuthorizedApp::new(1, oauth2client.id)
        .save(&pool)
        .await
        .unwrap();

    // valid request: scope=openid (the only allowed scope) - must succeed
    let response = client
        .get(format!(
            "/api/v1/oauth/authorize?\
            response_type=code&\
            client_id={}&\
            redirect_uri=http%3A%2F%2Ftest.server.tnt%3A12345%2F&\
            scope=openid&\
            state=valid",
            oauth2client.client_id
        ))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FOUND);
    let location = Url::parse(
        response
            .headers()
            .get("Location")
            .unwrap()
            .to_str()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(location.domain().unwrap(), "test.server.tnt");
    assert!(location.query().unwrap().contains("code="));

    // forbidden single scope: scope=email - must be rejected
    let response = client
        .get(format!(
            "/api/v1/oauth/authorize?\
            response_type=code&\
            client_id={}&\
            redirect_uri=http%3A%2F%2Ftest.server.tnt%3A12345%2F&\
            scope=email&\
            state=forbidden_single",
            oauth2client.client_id
        ))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FOUND);
    let location = Url::parse(
        response
            .headers()
            .get("Location")
            .unwrap()
            .to_str()
            .unwrap(),
    )
    .unwrap();
    let mut pairs = location.query_pairs();
    assert_eq!(
        pairs.next(),
        Some((Cow::Borrowed("error"), Cow::Borrowed("invalid_scope")))
    );
    assert_eq!(
        pairs.next(),
        Some((Cow::Borrowed("state"), Cow::Borrowed("forbidden_single")))
    );

    // mixed scope=openid%20email - second token must not be accepted
    let response = client
        .get(format!(
            "/api/v1/oauth/authorize?\
            response_type=code&\
            client_id={}&\
            redirect_uri=http%3A%2F%2Ftest.server.tnt%3A12345%2F&\
            scope=openid%20email&\
            state=mixed",
            oauth2client.client_id
        ))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FOUND);
    let location = Url::parse(
        response
            .headers()
            .get("Location")
            .unwrap()
            .to_str()
            .unwrap(),
    )
    .unwrap();
    let mut pairs = location.query_pairs();
    assert_eq!(
        pairs.next(),
        Some((Cow::Borrowed("error"), Cow::Borrowed("invalid_scope")))
    );
    assert_eq!(
        pairs.next(),
        Some((Cow::Borrowed("state"), Cow::Borrowed("mixed")))
    );
}

#[sqlx::test]
async fn dg26_7_test_state_parameter_secure_authorization(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let (client, pool) = make_client_with_db(pool).await;

    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // Create an OAuth2 client
    let oauth2client = NewOpenIDClient {
        name: "State POST test client".into(),
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
    let oauth_client: OAuth2Client<Id> = response.json().await;

    // Pre-authorise app
    OAuth2AuthorizedApp::new(1, oauth_client.id)
        .save(&pool)
        .await
        .unwrap();

    // Non-VSCHAR state on POST must be rejected with 400
    let response = client
        .post(format!(
            "/api/v1/oauth/authorize?\
            response_type=code&\
            client_id={}&\
            redirect_uri=http%3A%2F%2Ftest.server.tnt%3A12345%2F&\
            scope=openid&\
            allow=true&\
            state=%EE%FF%02%03",
            oauth_client.client_id
        ))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // Empty state on POST must also be rejected with 400
    let response = client
        .post(format!(
            "/api/v1/oauth/authorize?\
            response_type=code&\
            client_id={}&\
            redirect_uri=http%3A%2F%2Ftest.server.tnt%3A12345%2F&\
            scope=openid&\
            allow=true&\
            state=",
            oauth_client.client_id
        ))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

struct TokenPair {
    access_token: String,
    refresh_token: String,
}

async fn authorize_and_get_code(client: &TestClient, oauth_client: &OAuth2Client<Id>) -> String {
    let response = client
        .get(format!(
            "/api/v1/oauth/authorize?\
            response_type=code&\
            client_id={}&\
            redirect_uri=http%3A%2F%2Ftest.server.tnt%3A12345%2F&\
            scope=openid&\
            state=ABCDEF",
            oauth_client.client_id
        ))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FOUND);
    let redirect_url = Url::parse(
        response
            .headers()
            .get("Location")
            .unwrap()
            .to_str()
            .unwrap(),
    )
    .unwrap();
    redirect_url
        .query_pairs()
        .find(|(key, _)| key == "code")
        .expect("authorize endpoint did not return a code")
        .1
        .into_owned()
}

/// Run a full authorization_code flow and return the OAuth2 client, the id of its authorized app,
/// and the issued access and refresh tokens.
async fn issue_token_pair(client: &TestClient, pool: &PgPool) -> (OAuth2Client<Id>, Id, TokenPair) {
    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    let oauth2client = NewOpenIDClient {
        name: "test client".into(),
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
    let oauth_client: OAuth2Client<Id> = response.json().await;

    // authorize the client for the admin user so the authorize endpoint returns a code directly
    let authorized_app = OAuth2AuthorizedApp::new(1, oauth_client.id)
        .save(pool)
        .await
        .unwrap();
    let code = authorize_and_get_code(client, &oauth_client).await;

    let response = client
        .post("/api/v1/oauth/token")
        .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(format!(
            "grant_type=authorization_code&\
            code={code}&\
            redirect_uri=http%3A%2F%2Ftest.server.tnt%3A12345%2F&\
            client_id={}&\
            client_secret={}",
            oauth_client.client_id, oauth_client.client_secret
        ))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let tokens: Value = response.json().await;
    let token_pair = TokenPair {
        access_token: tokens["access_token"]
            .as_str()
            .expect("no access_token in token response")
            .to_owned(),
        refresh_token: tokens["refresh_token"]
            .as_str()
            .expect("no refresh_token in token response")
            .to_owned(),
    };

    (oauth_client, authorized_app.id, token_pair)
}

/// Count the token rows currently stored for an authorized app.
async fn token_row_count(pool: &PgPool, authorized_app_id: Id) -> i64 {
    sqlx::query_scalar!(
        "SELECT count(*) AS \"count!\" FROM oauth2token WHERE oauth2authorizedapp_id = $1",
        authorized_app_id
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

/// Regression test for DG2608-3: the refresh_token grant must authenticate the OAuth2 client.
#[sqlx::test]
async fn dg2608_3_test_refresh_token_requires_client_authentication(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let (client, pool) = make_client_with_db(pool).await;

    let (_, _, TokenPair { refresh_token, .. }) = issue_token_pair(&client, &pool).await;

    // No Authorization header, no client_id, no client_secret - only the stolen refresh token.
    let response = client
        .post("/api/v1/oauth/token")
        .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(format!(
            "grant_type=refresh_token&refresh_token={refresh_token}"
        ))
        .send()
        .await;

    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "an unauthenticated refresh_token grant must be rejected"
    );
    let body: Value = response.json().await;
    assert_eq!(body["error"], "invalid_client");
}

/// Regression test for DG2608-3: wrong client credentials must be rejected outright.
#[sqlx::test]
async fn dg2608_3_test_refresh_token_rejects_wrong_client_credentials(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let (client, pool) = make_client_with_db(pool).await;

    let (_, _, TokenPair { refresh_token, .. }) = issue_token_pair(&client, &pool).await;

    // "isec:isec", base64-encoded.
    let response = client
        .post("/api/v1/oauth/token")
        .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
        .header(AUTHORIZATION, "Basic aXNlYzppc2Vj")
        .body(format!(
            "grant_type=refresh_token&refresh_token={refresh_token}"
        ))
        .send()
        .await;
    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "a refresh_token grant with a wrong Basic header must be rejected"
    );
    let body: Value = response.json().await;
    assert!(
        body["msg"].is_string(),
        "expected the extractor to reject the request, got {body}"
    );

    // The same credentials in the form body.
    let response = client
        .post("/api/v1/oauth/token")
        .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(format!(
            "grant_type=refresh_token&\
            refresh_token={refresh_token}&\
            client_id=isec&\
            client_secret=isec"
        ))
        .send()
        .await;
    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "a refresh_token grant with wrong form credentials must be rejected"
    );
    let body: Value = response.json().await;
    assert_eq!(body["error"], "invalid_client");
}

/// Regression test for DG2608-3: a disabled client must be rejected as an invalid client.
#[sqlx::test]
async fn dg2608_3_test_disabled_client_refresh_returns_invalid_client(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let (client, pool) = make_client_with_db(pool).await;

    let (oauth_client, _, TokenPair { refresh_token, .. }) = issue_token_pair(&client, &pool).await;

    let response = client
        .post(format!("/api/v1/oauth/{}", oauth_client.client_id))
        .json(&json!({"enabled": false}))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    let response = client
        .post("/api/v1/oauth/token")
        .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(format!(
            "grant_type=refresh_token&\
            refresh_token={refresh_token}&\
            client_id={}&\
            client_secret={}",
            oauth_client.client_id, oauth_client.client_secret
        ))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let body: Value = response.json().await;
    assert_eq!(body["error"], "invalid_client");
}

/// Regression test for DG2608-3: redeeming a refresh token must rotate the pair and must not leave
/// stale rows behind.
#[sqlx::test]
async fn dg2608_3_test_refresh_token_is_rotated(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (client, pool) = make_client_with_db(pool).await;

    let (
        oauth_client,
        authorized_app_id,
        TokenPair {
            access_token,
            refresh_token,
        },
    ) = issue_token_pair(&client, &pool).await;

    assert_eq!(
        token_row_count(&pool, authorized_app_id).await,
        1,
        "the authorization_code flow should store exactly one token row"
    );

    let mut current_access_token = access_token;
    let mut current_refresh_token = refresh_token;

    for round in 1..=3 {
        let previous_access_token = current_access_token.clone();
        let previous_refresh_token = current_refresh_token.clone();

        let response = client
            .post("/api/v1/oauth/token")
            .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(format!(
                "grant_type=refresh_token&\
                refresh_token={previous_refresh_token}&\
                client_id={}&\
                client_secret={}",
                oauth_client.client_id, oauth_client.client_secret
            ))
            .send()
            .await;
        assert_eq!(
            response.status(),
            StatusCode::OK,
            "round {round}: an authenticated refresh must succeed"
        );
        let tokens: Value = response.json().await;
        current_access_token = tokens["access_token"]
            .as_str()
            .expect("no access_token in refresh response")
            .to_owned();
        current_refresh_token = tokens["refresh_token"]
            .as_str()
            .expect("no refresh_token in refresh response")
            .to_owned();

        assert_ne!(
            current_access_token, previous_access_token,
            "round {round}: the access token was not rotated"
        );
        assert_ne!(
            current_refresh_token, previous_refresh_token,
            "round {round}: the refresh token was not rotated"
        );

        // The consumed refresh token must be dead, otherwise a leaked credential never expires.
        let response = client
            .post("/api/v1/oauth/token")
            .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(format!(
                "grant_type=refresh_token&\
                refresh_token={previous_refresh_token}&\
                client_id={}&\
                client_secret={}",
                oauth_client.client_id, oauth_client.client_secret
            ))
            .send()
            .await;
        assert_ne!(
            response.status(),
            StatusCode::OK,
            "round {round}: a consumed refresh token must not be redeemable again"
        );

        // Refreshing must replace the stored token, not add a row.
        let rows = token_row_count(&pool, authorized_app_id).await;
        assert_eq!(
            rows, 1,
            "round {round}: expected exactly one token row for the authorized app, found {rows}"
        );
    }
}

/// Regression test for DG2608-3: concurrent refreshes must rotate a pair only once.
#[sqlx::test]
async fn dg2608_3_test_concurrent_refresh_allows_single_rotation(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let (client, pool) = make_client_with_db(pool).await;

    let (oauth_client, _, TokenPair { refresh_token, .. }) = issue_token_pair(&client, &pool).await;

    sqlx::query(
        r#"
        CREATE OR REPLACE FUNCTION dg2608_3_pause_oauth2token_update()
        RETURNS trigger
        LANGUAGE plpgsql
        AS $$
        BEGIN
            PERFORM pg_sleep(1);
            RETURN NEW;
        END;
        $$;
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "CREATE TRIGGER dg2608_3_pause_oauth2token_update_trigger
         BEFORE UPDATE ON oauth2token
         FOR EACH ROW
         EXECUTE FUNCTION dg2608_3_pause_oauth2token_update()",
    )
    .execute(&pool)
    .await
    .unwrap();

    let request_body = format!(
        "grant_type=refresh_token&\
        refresh_token={refresh_token}&\
        client_id={}&\
        client_secret={}",
        oauth_client.client_id, oauth_client.client_secret
    );
    let first_request = client
        .post("/api/v1/oauth/token")
        .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(request_body.clone())
        .send();
    let second_request = client
        .post("/api/v1/oauth/token")
        .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(request_body)
        .send();
    let (first, second) = tokio::join!(first_request, second_request);

    let first_status = first.status();
    let second_status = second.status();
    let (winner, loser) = if first_status == StatusCode::OK
        && second_status == StatusCode::BAD_REQUEST
    {
        (first, second)
    } else if first_status == StatusCode::BAD_REQUEST && second_status == StatusCode::OK {
        (second, first)
    } else {
        panic!(
            "expected one successful refresh and one invalid grant, got {first_status} and {second_status}"
        );
    };

    let winner_tokens: Value = winner.json().await;
    let loser_body: Value = loser.json().await;
    assert_eq!(loser_body["error"], "invalid_grant");

    let access_token = winner_tokens["access_token"]
        .as_str()
        .expect("winner did not return an access token");
    let bearer = format!("Bearer {access_token}");
    let response = client
        .get("/api/v1/oauth/userinfo")
        .header(AUTHORIZATION, &bearer)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
}

/// Regression test for DG2608-3: reauthorization must clear leftover token rows.
#[sqlx::test]
async fn dg2608_3_test_reauthorization_clears_leftover_rows(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let (client, pool) = make_client_with_db(pool).await;

    let (oauth_client, authorized_app_id, _) = issue_token_pair(&client, &pool).await;

    for _ in 0..2 {
        OAuth2Token::new(
            authorized_app_id,
            "http://test.server.tnt:12345/".into(),
            "openid".into(),
        )
        .save(&pool)
        .await
        .unwrap();
    }
    assert_eq!(token_row_count(&pool, authorized_app_id).await, 3);

    let code = authorize_and_get_code(&client, &oauth_client).await;

    let response = client
        .post("/api/v1/oauth/token")
        .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(format!(
            "grant_type=authorization_code&\
            code={code}&\
            redirect_uri=http%3A%2F%2Ftest.server.tnt%3A12345%2F&\
            client_id={}&\
            client_secret={}",
            oauth_client.client_id, oauth_client.client_secret
        ))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(token_row_count(&pool, authorized_app_id).await, 1);
}
