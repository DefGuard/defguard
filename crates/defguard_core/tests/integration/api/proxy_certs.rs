use defguard_certs::CertificateAuthority;
use defguard_common::db::models::{Certificates, ProxyCertSource, proxy::Proxy};
use defguard_core::handlers::Auth;
use reqwest::StatusCode;
use serde_json::json;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

use super::common::{make_test_client, setup_pool};

async fn seed_ca(pool: &sqlx::PgPool) {
    let ca = CertificateAuthority::new("Test CA", "test@example.com", 365).unwrap();
    let certs = Certificates {
        ca_cert_der: Some(ca.cert_der().to_vec()),
        ca_key_der: Some(ca.key_pair_der().to_vec()),
        ca_expiry: Some(ca.expiry().unwrap()),
        ..Default::default()
    };
    certs.save(pool).await.unwrap();
}

#[sqlx::test]
async fn test_proxy_cert_endpoints(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (client, _) = make_test_client(pool.clone()).await;

    let proxy = Proxy::new("test-proxy", "localhost", 50051, "admin")
        .save(&pool)
        .await
        .unwrap();

    // unauthenticated requests return 401
    let response = client
        .post("/api/v1/proxy/cert/upload")
        .json(&json!({"cert_pem": "c", "key_pem": "k"}))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let response = client
        .post("/api/v1/proxy/cert/self-signed")
        .json(&json!({"san": ["localhost"]}))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let response = client
        .post(format!("/api/v1/proxy/{}/acme/issue", proxy.id))
        .json(&json!({"domain": "example.com", "use_staging": true}))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // self-signed without CA returns 400
    let response = client
        .post("/api/v1/proxy/cert/self-signed")
        .json(&json!({"san": ["localhost"]}))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // upload custom cert
    let cert_pem = "-----BEGIN CERTIFICATE-----\nfake\n-----END CERTIFICATE-----\n";
    let key_pem = "-----BEGIN PRIVATE KEY-----\nfake\n-----END PRIVATE KEY-----\n";

    let response = client
        .post("/api/v1/proxy/cert/upload")
        .json(&json!({"cert_pem": cert_pem, "key_pem": key_pem}))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    let saved = Certificates::get(&pool).await.unwrap().unwrap();
    assert_eq!(saved.proxy_http_cert_source, ProxyCertSource::Custom);
    assert_eq!(saved.proxy_http_cert_pem.as_deref(), Some(cert_pem));
    assert_eq!(saved.proxy_http_cert_key_pem.as_deref(), Some(key_pem));

    // self-signed with CA present
    seed_ca(&pool).await;

    let response = client
        .post("/api/v1/proxy/cert/self-signed")
        .json(&json!({"san": ["localhost"]}))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    let saved = Certificates::get(&pool).await.unwrap().unwrap();
    assert_eq!(saved.proxy_http_cert_source, ProxyCertSource::SelfSigned);
    assert!(saved.proxy_http_cert_pem.as_deref().unwrap_or("").contains("BEGIN CERTIFICATE"));
    assert!(saved.proxy_http_cert_key_pem.as_deref().unwrap_or("").contains("BEGIN"));

    // ACME issue: existing proxy -> 202, missing proxy -> 404
    let response = client
        .post(format!("/api/v1/proxy/{}/acme/issue", proxy.id))
        .json(&json!({"domain": "example.com", "use_staging": true}))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::ACCEPTED);

    let response = client
        .post("/api/v1/proxy/99999/acme/issue")
        .json(&json!({"domain": "example.com", "use_staging": false}))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
