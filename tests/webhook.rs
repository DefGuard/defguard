pub mod common;

use common::{make_client, setup_pool};
use defguard::{
    db::{Id, NoId, WebHook},
    handlers::Auth,
};
use reqwest::StatusCode;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

#[sqlx::test]
async fn test_webhooks(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let client = make_client(pool).await;

    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    let mut webhook = WebHook {
        id: NoId,
        url: "http://localhost:3000/trigger-happy".into(),
        description: "Test".into(),
        token: "1234567890".into(),
        enabled: false,
        on_user_created: true,
        on_user_deleted: false,
        on_user_modified: true,
        on_hwkey_provision: false,
    };

    let response = client.post("/api/v1/webhook").json(&webhook).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);

    let response = client.get("/api/v1/webhook").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let webhooks: Vec<WebHook<Id>> = response.json().await;
    assert_eq!(webhooks.len(), 1);

    webhook.description = "Changed".into();
    webhook.on_user_modified = false;
    let response = client
        .put(format!("/api/v1/webhook/{}", webhooks[0].id))
        .json(&webhook)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    let response = client
        .get(format!("/api/v1/webhook/{}", webhooks[0].id))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let fetched_webhook: WebHook<Id> = response.json().await;
    assert_eq!(fetched_webhook.url, webhook.url);
    assert_eq!(fetched_webhook.description, webhook.description);
    assert_eq!(fetched_webhook.on_user_modified, webhook.on_user_modified);

    let response = client
        .delete(format!("/api/v1/webhook/{}", webhooks[0].id))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    let response = client.get("/api/v1/webhook").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let webhooks: Vec<WebHook<Id>> = response.json().await;
    assert!(webhooks.is_empty());
}
