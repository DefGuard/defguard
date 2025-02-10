use chrono::Utc;
use common::{client::TestClient, make_test_client, ClientState};
use defguard::{
    db::UserInfo,
    enterprise::{
        db::models::api_tokens::{ApiToken, ApiTokenInfo},
        handlers::api_tokens::{AddApiTokenData, RenameRequest},
    },
    handlers::Auth,
};
use reqwest::{header::HeaderName, StatusCode};
use serde::Deserialize;

pub mod common;

async fn make_client() -> TestClient {
    let (client, _) = make_test_client().await;
    client
}

async fn make_client_with_state() -> (TestClient, ClientState) {
    let (client, client_state) = make_test_client().await;
    (client, client_state)
}

#[tokio::test]
async fn test_normal_user_cannot_access_token_endpoints() {
    let client = make_client().await;

    // log in as normal user
    let auth = Auth::new("hpotter", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // normal user cannot access API token endpoints
    let response = client.get("/api/v1/user/hpotter/api_token").send().await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let response = client
        .post("/api/v1/user/hpotter/api_token")
        .json(&AddApiTokenData {
            name: "dummy token".into(),
        })
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let response = client
        .delete("/api/v1/user/hpotter/api_token/1")
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let response = client
        .post("/api/v1/user/hpotter/api_token/1/rename")
        .json(&RenameRequest {
            name: "dummy token".into(),
        })
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_normal_user_cannot_use_token_auth() {
    let (client, state) = make_client_with_state().await;

    // sidestep API access restrictions by creating a token manually
    let token_string = "test-token-string";
    let token = ApiToken::new(
        state.test_user.id,
        Utc::now().naive_utc(),
        "dummy token".into(),
        token_string,
    );
    token.save(&state.pool).await.unwrap();

    // normal user cannot access API token endpoints
    let response = client
        .get("/api/v1/me")
        .header(
            HeaderName::from_static("authorization"),
            &format!("Bearer {token_string}"),
        )
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_admin_user_can_manage_api_tokens() {
    let client = make_client().await;

    // log in as admin user
    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // create some API tokens
    #[derive(Deserialize)]
    struct NewTokenResponse {
        token: String,
    }
    let response = client
        .post("/api/v1/user/admin/api_token")
        .json(&AddApiTokenData {
            name: "dummy token 1".into(),
        })
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let _token_1 = response
        .into_inner()
        .json::<NewTokenResponse>()
        .await
        .unwrap()
        .token;
    let response = client
        .post("/api/v1/user/admin/api_token")
        .json(&AddApiTokenData {
            name: "dummy token 2".into(),
        })
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let _token_2 = response
        .into_inner()
        .json::<NewTokenResponse>()
        .await
        .unwrap()
        .token;
    let response = client
        .post("/api/v1/user/admin/api_token")
        .json(&AddApiTokenData {
            name: "dummy token 3".into(),
        })
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let _token_3 = response
        .into_inner()
        .json::<NewTokenResponse>()
        .await
        .unwrap()
        .token;

    // cannot add tokens for a normal user
    let response = client
        .post("/api/v1/user/hpotter/api_token")
        .json(&AddApiTokenData {
            name: "nope".into(),
        })
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    // list tokens
    let response = client.get("/api/v1/user/admin/api_token").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let tokens: Vec<ApiTokenInfo> = response.json().await;
    assert_eq!(tokens.len(), 3);
    let first_token = tokens.first().unwrap();
    assert_eq!(first_token.name, "dummy token 1");

    // rename token
    let response = client
        .post(format!(
            "/api/v1/user/admin/api_token/{}/rename",
            first_token.id
        ))
        .json(&RenameRequest {
            name: "renamed token".into(),
        })
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let response = client.get("/api/v1/user/admin/api_token").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let tokens: Vec<ApiTokenInfo> = response.json().await;
    assert_eq!(tokens.len(), 3);
    let first_token = tokens.first().unwrap();
    assert_eq!(first_token.name, "renamed token");

    // delete token
    let response = client
        .delete(format!("/api/v1/user/admin/api_token/{}", first_token.id))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let response = client.get("/api/v1/user/admin/api_token").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let tokens: Vec<ApiTokenInfo> = response.json().await;
    assert_eq!(tokens.len(), 2);
    let first_token = tokens.first().unwrap();
    assert_eq!(first_token.name, "dummy token 2");
}

#[tokio::test]
async fn test_admin_user_can_use_api_tokens_to_authenticate() {
    let client = make_client().await;

    // log in as admin user
    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // create API token
    #[derive(Deserialize)]
    struct NewTokenResponse {
        token: String,
    }
    let response = client
        .post("/api/v1/user/admin/api_token")
        .json(&AddApiTokenData {
            name: "dummy token 1".into(),
        })
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let token = response
        .into_inner()
        .json::<NewTokenResponse>()
        .await
        .unwrap()
        .token;

    // logout
    let response = client.post("/api/v1/auth/logout").send().await;
    assert_eq!(response.status(), StatusCode::OK);

    let response = client.get("/api/v1/me").send().await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // authorize request with API token
    let response = client
        .get("/api/v1/me")
        .header(
            HeaderName::from_static("authorization"),
            &format!("Bearer {token}"),
        )
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    // invalid header name
    let response = client
        .get("/api/v1/me")
        .header(
            HeaderName::from_static("not_actually_authorization"),
            &format!("Bearer {token}"),
        )
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // invalid header value
    let response = client
        .get("/api/v1/me")
        .header(
            HeaderName::from_static("authorization"),
            &format!("Bear {token}"),
        )
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // use the same token again
    let response = client
        .get("/api/v1/me")
        .header(
            HeaderName::from_static("authorization"),
            &format!("Bearer {token}"),
        )
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let user: UserInfo = response.json().await;
    assert_eq!(user.username, "admin");
}
