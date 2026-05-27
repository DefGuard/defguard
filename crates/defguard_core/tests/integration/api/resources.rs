use defguard_core::handlers::Auth;
use reqwest::StatusCode;
use serde_json::Value;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

use super::common::{make_test_client, setup_pool};

#[sqlx::test]
async fn test_check_username_available(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (client, _) = make_test_client(pool).await;

    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    let response = client
        .get("/api/v1/reserved?resource=username&value=newuser")
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    let body: Value = response.json().await;
    assert_eq!(body["available"], true);
}

#[sqlx::test]
async fn test_check_username_taken(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (client, _) = make_test_client(pool).await;

    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    let response = client
        .get("/api/v1/reserved?resource=username&value=hpotter")
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CONFLICT);

    let body: Value = response.json().await;
    assert!(
        body["msg"]
            .as_str()
            .unwrap_or("")
            .contains("hpotter is already taken")
    );
}

#[sqlx::test]
async fn test_check_email_available(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (client, _) = make_test_client(pool).await;

    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    let response = client
        .get("/api/v1/reserved?resource=email&value=new.user%40example.com")
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    let body: Value = response.json().await;
    assert_eq!(body["available"], true);
}

#[sqlx::test]
async fn test_check_email_taken(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (client, _) = make_test_client(pool).await;

    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    let response = client
        .get("/api/v1/reserved?resource=email&value=h.potter%40hogwart.edu.uk")
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CONFLICT);

    let body: Value = response.json().await;
    assert!(
        body["msg"]
            .as_str()
            .unwrap_or("")
            .contains("is already taken")
    );
}

#[sqlx::test]
async fn test_check_email_taken_case_insensitive(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (client, _) = make_test_client(pool).await;

    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // h.potter@hogwart.edu.uk exists in the DB; send it upper-cased
    let response = client
        .get("/api/v1/reserved?resource=email&value=H.POTTER%40HOGWART.EDU.UK")
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CONFLICT);

    let body: Value = response.json().await;
    assert!(
        body["msg"]
            .as_str()
            .unwrap_or("")
            .contains("is already taken")
    );
}

#[sqlx::test]
async fn test_check_email_taken_mixed_case(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (client, _) = make_test_client(pool).await;

    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // Mixed-case variant of an existing address
    let response = client
        .get("/api/v1/reserved?resource=email&value=H.Potter%40Hogwart.Edu.Uk")
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CONFLICT);

    let body: Value = response.json().await;
    assert!(
        body["msg"]
            .as_str()
            .unwrap_or("")
            .contains("is already taken")
    );
}

#[sqlx::test]
async fn test_check_reserved_unauthenticated(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (client, _) = make_test_client(pool).await;

    let response = client
        .get("/api/v1/reserved?resource=username&value=anyone")
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn test_check_reserved_non_admin(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (client, _) = make_test_client(pool).await;

    let auth = Auth::new("hpotter", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    let response = client
        .get("/api/v1/reserved?resource=username&value=anyone")
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[sqlx::test]
async fn test_check_admin_username_taken(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (client, _) = make_test_client(pool).await;

    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    let response = client
        .get("/api/v1/reserved?resource=username&value=admin")
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CONFLICT);
}
