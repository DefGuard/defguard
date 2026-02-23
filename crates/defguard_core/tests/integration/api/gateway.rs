use defguard_common::db::{
    Id,
    models::{WireguardNetwork, gateway::Gateway},
};
use defguard_core::handlers::Auth;
use reqwest::StatusCode;
use serde_json::json;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

use super::common::{make_network, make_test_client, setup_pool};

#[sqlx::test]
async fn test_gateway_crud(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (mut client, client_state) = make_test_client(pool).await;

    client.login_user("admin", "pass123").await;

    let response = make_network(&client, "network").await;
    let network: WireguardNetwork<Id> = response.json().await;
    client.drain_all_events();
    client.drain_all_events();

    let gateway_1 = Gateway::new(network.id, "gateway1", "127.0.0.1", 50051, 1)
        .save(&client_state.pool)
        .await
        .unwrap();
    let gateway_2 = Gateway::new(network.id, "gateway2", "1.2.3.1", 55555, 1)
        .save(&client_state.pool)
        .await
        .unwrap();

    let response = client.get("/api/v1/gateway").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let gateways: Vec<Gateway<Id>> = response.json().await;
    assert_eq!(gateways.len(), 2);
    let gateway_from_list = &gateways[0];
    assert_eq!(gateway_from_list, &gateway_1);
    let gateway_from_list = &gateways[1];
    assert_eq!(gateway_from_list, &gateway_2);

    let response = client
        .get(format!("/api/v1/gateway/{}", gateway_1.id))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let gateway_details: Gateway<Id> = response.json().await;
    assert_eq!(gateway_details, gateway_1);

    let response = client
        .put(format!("/api/v1/gateway/{}", gateway_1.id))
        .json(&json!({
            "name": "gateway-updated",
        }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let updated_gateway: Gateway<Id> = response.json().await;
    assert_eq!(updated_gateway.name, "gateway-updated");
    assert_eq!(updated_gateway.address, gateway_1.address);
    assert_eq!(updated_gateway.port, gateway_1.port);

    let response = client
        .delete(format!("/api/v1/gateway/{}", gateway_1.id))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    let response = client
        .get(format!("/api/v1/gateway/{}", gateway_1.id))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let response = client.get("/api/v1/gateway").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let gateways: Vec<Gateway<Id>> = response.json().await;
    assert_eq!(gateways.len(), 1);
}

#[sqlx::test]
async fn test_gateway_endpoints_require_admin(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (mut client, client_state) = make_test_client(pool).await;

    client.login_user("admin", "pass123").await;

    let response = make_network(&client, "network").await;
    let network: WireguardNetwork<Id> = response.json().await;

    let gateway = Gateway::new(network.id, "gateway", "127.0.0.1", 50051, 1)
        .save(&client_state.pool)
        .await
        .unwrap();

    let auth = Auth::new("hpotter", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    let response = client.get("/api/v1/gateway").send().await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let response = client
        .get(format!("/api/v1/gateway/{}", gateway.id))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let response = client
        .put(format!("/api/v1/gateway/{}", gateway.id))
        .json(&json!({
            "name": "gateway-updated",
        }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let response = client
        .delete(format!("/api/v1/gateway/{}", gateway.id))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[sqlx::test]
async fn test_gateway_update_rejects_unknown_fields(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (mut client, client_state) = make_test_client(pool).await;

    client.login_user("admin", "pass123").await;

    let response = make_network(&client, "network").await;
    let network: WireguardNetwork<Id> = response.json().await;

    let gateway = Gateway::new(network.id, "gateway", "127.0.0.1", 50051, 1)
        .save(&client_state.pool)
        .await
        .unwrap();

    let response = client
        .put(format!("/api/v1/gateway/{}", gateway.id))
        .json(&json!({
            "name": "gateway-updated",
            "address": "127.0.0.2",
            "port": 50052,
            "location_id": 999,
        }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
