use defguard_common::db::{Id, models::proxy::Proxy};
use defguard_core::handlers::{Auth, proxy::ProxyUpdateData};
use reqwest::StatusCode;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

use super::common::{make_test_client, setup_pool};


#[sqlx::test]
async fn test_proxy_details(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (client, _) = make_test_client(pool.clone()).await;

    // Authorize as an administrator.
    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // Create new proxy.
    let proxy = Proxy::new("test", "localhost", 50051, "public.net")
        .save(&pool)
        .await
        .unwrap();

	// Get proxy via API
    let response = client
        .get(format!("/api/v1/proxy/{}", proxy.id))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    // Verify proxy is correct
    let proxy_from_api: Proxy<Id> = response.json().await;
    assert_eq!(proxy, proxy_from_api);
}

#[sqlx::test]
async fn test_proxy_update(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (client, _) = make_test_client(pool.clone()).await;

    // Authorize as an administrator.
    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // Create new proxy.
    let mut proxy = Proxy::new("test", "localhost", 50051, "public.net")
        .save(&pool)
        .await
        .unwrap();

    // Modify name
    let data = ProxyUpdateData {
        name: "modified".to_string(),
    };
    let response = client
        .put(format!("/api/v1/proxy/{}", proxy.id))
        .json(&data)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    // Verify proxy is modified correctly
    let proxy_updated: Proxy<Id> = response.json().await;
    assert_eq!(proxy_updated.name, "modified");
    proxy.name = "modified".to_string();
    assert_eq!(proxy, proxy_updated);

    // Try to modify other fields
    let proxy_before_mods = proxy.clone();
    proxy.address = "otherhost".to_string();
    proxy.port = 50052;
    proxy.public_address = "otherpublichost.net".to_string();
    let response = client
        .put(format!("/api/v1/proxy/{}", proxy.id))
        .json(&proxy)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let proxy_updated: Proxy<Id> = response.json().await;
    assert_eq!(proxy_before_mods, proxy_updated);
}

#[sqlx::test]
async fn test_delete_proxy(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (client, _) = make_test_client(pool.clone()).await;

    // Authorize as an administrator.
    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // Create new proxy.
    let proxy = Proxy::new("test", "localhost", 50051, "public.net")
        .save(&pool)
        .await
        .unwrap();

	// Delete proxy via API
    let response = client
        .delete(format!("/api/v1/proxy/{}", proxy.id))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

	// Verify proxy is deleted
    let response = client
        .get(format!("/api/v1/proxy/{}", proxy.id))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
	assert_eq!(Proxy::all(&pool).await.unwrap().len(), 0);
}
