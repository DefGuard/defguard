use defguard::{
    build_webapp,
    db::{AppEvent, GatewayEvent, WebHook},
    grpc::{GatewayState, WorkerState},
    handlers::Auth,
};
use rocket::{http::Status, local::asynchronous::Client};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::unbounded_channel;

mod common;
use common::init_test_db;
use defguard::db::User;

#[rocket::async_test]
async fn test_webhooks() {
    let (pool, config) = init_test_db().await;

    let (tx, rx) = unbounded_channel::<AppEvent>();
    let worker_state = Arc::new(Mutex::new(WorkerState::new(tx.clone())));
    let (wg_tx, wg_rx) = unbounded_channel::<GatewayEvent>();
    let gateway_state = Arc::new(Mutex::new(GatewayState::new(wg_rx)));

    User::init_admin_user(&pool, &config.default_admin_password)
        .await
        .unwrap();

    let webapp = build_webapp(config, tx, rx, wg_tx, worker_state, gateway_state, pool).await;
    let client = Client::tracked(webapp).await.unwrap();

    let auth = Auth::new("admin".into(), "pass123".into());
    let response = client.post("/api/v1/auth").json(&auth).dispatch().await;
    assert_eq!(response.status(), Status::Ok);

    let mut webhook = WebHook {
        id: None,
        url: "http://localhost:3000/trigger-happy".into(),
        description: "Test".into(),
        token: "1234567890".into(),
        enabled: false,
        on_user_created: true,
        on_user_deleted: false,
        on_user_modified: true,
        on_hwkey_provision: false,
    };

    let response = client
        .post("/api/v1/webhook")
        .json(&webhook)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Created);

    let response = client.get("/api/v1/webhook").dispatch().await;
    assert_eq!(response.status(), Status::Ok);
    let webhooks: Vec<WebHook> = response.into_json().await.unwrap();
    assert_eq!(webhooks.len(), 1);

    webhook.description = "Changed".into();
    webhook.on_user_modified = false;
    let response = client
        .put(format!("/api/v1/webhook/{}", webhooks[0].id.unwrap()))
        .json(&webhook)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);

    let response = client
        .get(format!("/api/v1/webhook/{}", webhooks[0].id.unwrap()))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);
    let fetched_webhook: WebHook = response.into_json().await.unwrap();
    assert_eq!(fetched_webhook.url, webhook.url);
    assert_eq!(fetched_webhook.description, webhook.description);
    assert_eq!(fetched_webhook.on_user_modified, webhook.on_user_modified);

    let response = client
        .delete(format!("/api/v1/webhook/{}", webhooks[0].id.unwrap()))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);

    let response = client.get("/api/v1/webhook").dispatch().await;
    assert_eq!(response.status(), Status::Ok);
    let webhooks: Vec<WebHook> = response.into_json().await.unwrap();
    assert!(webhooks.is_empty());
}
