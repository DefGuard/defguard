use defguard_core::{
    enterprise::db::models::activity_log_stream::ActivityLogStream, handlers::Auth,
};
use reqwest::StatusCode;
use serde_json::{Value, json};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

use super::common::{client::TestClient, make_client, setup_pool};

const PASSWORD: &str = "super-secret-stream-password";

fn stream_request(name: &str, password: Option<&str>) -> Value {
    json!({
        "name": name,
        "stream_type": "vector_http",
        "stream_config": {
            "url": "http://localhost:8686",
            "username": "user",
            "password": password,
            "cert": null,
        },
    })
}

async fn create_stream(client: &TestClient) -> i64 {
    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    let response = client
        .post("/api/v1/activity_log_stream")
        .json(&stream_request("vector", Some(PASSWORD)))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    let response = client.get("/api/v1/activity_log_stream").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = response.json().await;
    body[0]["id"].as_i64().unwrap()
}

#[sqlx::test]
async fn test_get_activity_log_stream_does_not_expose_password(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let client = make_client(pool).await;
    create_stream(&client).await;

    let response = client.get("/api/v1/activity_log_stream").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = response.json().await;

    assert!(body[0]["config"].get("password").is_none());
    assert_eq!(body[0]["password_set"], true);
    assert!(!body.to_string().contains(PASSWORD));
}

#[sqlx::test]
async fn test_modify_activity_log_stream_without_password_keeps_it(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let client = make_client(pool.clone()).await;
    let id = create_stream(&client).await;

    let response = client
        .put(format!("/api/v1/activity_log_stream/{id}"))
        .json(&stream_request("renamed", None))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    let stream = ActivityLogStream::find_by_id(&pool, id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stream.name, "renamed");
    assert_eq!(stream.config["password"], PASSWORD);
}

#[sqlx::test]
async fn test_modify_activity_log_stream_with_new_password_replaces_it(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let client = make_client(pool.clone()).await;
    let id = create_stream(&client).await;

    let response = client
        .put(format!("/api/v1/activity_log_stream/{id}"))
        .json(&stream_request("vector", Some("new-password")))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    let stream = ActivityLogStream::find_by_id(&pool, id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stream.config["password"], "new-password");
}
