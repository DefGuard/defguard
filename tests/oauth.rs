pub mod common;

use std::borrow::Cow;

use defguard::{
    db::{
        models::{
            oauth2client::{OAuth2Client, OAuth2ClientSafe},
            NewOpenIDClient,
        },
        Id, OAuth2AuthorizedApp,
    },
    handlers::Auth,
};
use reqwest::{header::CONTENT_TYPE, StatusCode, Url};
use serde_json::json;
use sqlx::PgPool;

use self::common::{client::TestClient, make_test_client};

async fn make_client() -> (TestClient, PgPool) {
    let (client, client_state) = make_test_client().await;
    (client, client_state.pool)
}

#[tokio::test]
async fn test_authorize() {
    let (client, pool) = make_client().await;

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

#[tokio::test]
async fn test_openid_app_management_access() {
    let (client, _) = make_client().await;

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
    let apps: Vec<OAuth2Client<Id>> = response.json().await;
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
    let apps: Vec<OAuth2Client<Id>> = response.json().await;
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
    let apps: Vec<OAuth2Client<Id>> = response.json().await;
    let test_app = &apps[0];

    // // login as standard user
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

// FIXME: revive these tests
// #[tokio::test]
// async fn test_authorize_consent() {
//     let client = make_client().await;

//     let auth = Auth::new("admin".into(), "pass123".into());
//     let response = client.post("/api/v1/auth").json(&auth).send().await;
//     assert_eq!(response.status(), StatusCode::OK);

//     let response = client
//         .post("/api/v1/user/admin/oauth2client")
//         .json(&json!({
//             "client_id": "MyClient",
//             "client_secret": "secret",
//             "redirect_uri": "http://localhost:3000/",
//             "scope": ["default-scope"],
//             "name": "Test",
//             "enabled": true,
//         }))
//         .send()
//         .await;
//     assert_eq!(response.status(), StatusCode::OK);

//     let response = client
//         .post(
//             "/api/v1/oauth/authorize?\
//             allow=true&\
//             response_type=code&\
//             client_id=MyClient&\
//             redirect_uri=http%3A%2F%2Flocalhost%3A3000%2F&\
//             scope=default-scope&\
//             state=ABCDEF",
//         )
//         .send()
//         .await;
//     assert_eq!(response.status(), StatusCode::FOUND);

//     let localtion = response.headers().get("Location").unwrap().to_str().unwrap();
//     assert!(localtion.starts_with("http://localhost:3000/?code="));

//     // extract code
//     let index = localtion.find("&state").unwrap();
//     let code = localtion.get(28..index).unwrap();

//     let response = client
//         .post("/api/v1/oauth/token")
//         .header(ContentType::Form)
//         .header(Header::new(
//             "Authorization",
//             // echo -n 'LocalClient:secret' | base64
//             "Basic TG9jYWxDbGllbnQ6c2VjcmV0",
//         ))
//         .body(format!(
//             "grant_type=authorization_code&\
//             code={}&\
//             redirect_uri=http%3A%2F%2Flocalhost%3A3000%2F",
//             code
//         ))
//         .send()
//         .await;
//     assert_eq!(response.status(), StatusCode::OK);
// }

// #[tokio::test]
// async fn test_authorize_consent_wrong_client() {
//     let client = make_client().await;

//     let response = client
//         .post(
//             "/api/v1/oauth/authorize?\
//             allow=true&\
//             response_type=code&\
//             client_id=NonExistentClient&\
//             redirect_uri=http%3A%2F%2Flocalhost%3A3000%2F&\
//             scope=default-scope&\
//             state=ABCDEF",
//         )
//         .send()
//         .await;
//     assert_eq!(response.status(), StatusCode::BAD_REQUEST);
// }

#[tokio::test]
async fn test_token_client_credentials() {
    let (client, _) = make_client().await;

    let response = client
        .post("/api/v1/oauth/token")
        .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body("client_id=WrongClient&client_secret=WrongSecret&grant_type=code")
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
