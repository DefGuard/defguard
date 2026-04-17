use defguard_certs::CertificateAuthority;
use defguard_common::db::models::{
    Certificates, CoreCertSource, Settings, settings::update_current_settings,
};
use defguard_core::handlers::Auth;
use reqwest::StatusCode;
use serde_json::json;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

use super::common::{
    generate_expired_test_cert_pem, generate_test_cert_pem, make_test_client, setup_pool,
};

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
async fn test_internal_url_settings_endpoint(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (client, _) = make_test_client(pool.clone()).await;

    let response = client
        .post("/api/v1/core/cert/internal_url_settings")
        .json(&json!({ "ssl_type": "none" }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    let mut settings = Settings::get_current_settings();
    settings.defguard_url = "https://defguard.example.com".into();
    update_current_settings(&pool, settings).await.unwrap();

    let response = client
        .post("/api/v1/core/cert/internal_url_settings")
        .json(&json!({ "ssl_type": "none" }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let mut settings = Settings::get(&pool).await.unwrap().unwrap();
    // Don't touch the URL if setting no cert
    assert_eq!(settings.defguard_url, "https://defguard.example.com");

    let saved = Certificates::get(&pool).await.unwrap().unwrap();
    assert_eq!(saved.core_http_cert_source, CoreCertSource::None);
    assert!(saved.core_http_cert_pem.is_none());
    assert!(saved.core_http_cert_key_pem.is_none());
    assert!(saved.core_http_cert_expiry.is_none());

    seed_ca(&pool).await;

    settings.defguard_url = "http://defguard.example.com".to_string();
    update_current_settings(&pool, settings).await.unwrap();
    let response = client
        .post("/api/v1/core/cert/internal_url_settings")
        .json(&json!({ "ssl_type": "defguard_ca" }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let mut settings = Settings::get(&pool).await.unwrap().unwrap();
    // Url schema changed to https
    assert_eq!(settings.defguard_url, "https://defguard.example.com");

    let body: serde_json::Value = response.json::<serde_json::Value>().await;
    assert!(!body["cert_info"].is_null());
    assert_eq!(body["cert_info"]["common_name"], "defguard.example.com");

    let saved = Certificates::get(&pool).await.unwrap().unwrap();
    assert_eq!(saved.core_http_cert_source, CoreCertSource::SelfSigned);
    assert!(saved.core_http_cert_expiry.is_some());
    assert!(
        saved
            .core_http_cert_pem
            .as_deref()
            .unwrap_or("")
            .contains("BEGIN CERTIFICATE")
    );

    settings.defguard_url = "http://defguard.example.com".to_string();
    update_current_settings(&pool, settings).await.unwrap();
    let (cert_pem, key_pem) = generate_test_cert_pem("uploaded.example.com");
    let response = client
        .post("/api/v1/core/cert/internal_url_settings")
        .json(&json!({
            "ssl_type": "own_cert",
            "cert_pem": cert_pem,
            "key_pem": key_pem
        }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let mut settings = Settings::get(&pool).await.unwrap().unwrap();
    // Url schema changed to https
    assert_eq!(settings.defguard_url, "https://defguard.example.com");

    let body: serde_json::Value = response.json::<serde_json::Value>().await;
    assert_eq!(body["cert_info"]["common_name"], "uploaded.example.com");

    let saved = Certificates::get(&pool).await.unwrap().unwrap();
    assert_eq!(saved.core_http_cert_source, CoreCertSource::Custom);
    assert!(saved.core_http_cert_expiry.is_some());

    settings.defguard_url = "http://defguard.example.com".to_string();
    update_current_settings(&pool, settings).await.unwrap();
    let (_, mismatched_key_pem) = generate_test_cert_pem("different.example.com");
    let response = client
        .post("/api/v1/core/cert/internal_url_settings")
        .json(&json!({
            "ssl_type": "own_cert",
            "cert_pem": cert_pem,
            "key_pem": mismatched_key_pem
        }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let mut settings = Settings::get(&pool).await.unwrap().unwrap();
    // Url schema unchanged on errors
    assert_eq!(settings.defguard_url, "http://defguard.example.com");

    let response = client
        .post("/api/v1/core/cert/internal_url_settings")
        .json(&json!({
            "ssl_type": "own_cert",
            "cert_pem": "-----BEGIN CERTIFICATE-----\nfake\n-----END CERTIFICATE-----\n"
        }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    settings.defguard_url = "http://defguard.example.com".to_string();
    update_current_settings(&pool, settings).await.unwrap();
    let (expired_cert_pem, expired_key_pem) = generate_expired_test_cert_pem("expired.example.com");
    let response = client
        .post("/api/v1/core/cert/internal_url_settings")
        .json(&json!({
            "ssl_type": "own_cert",
            "cert_pem": expired_cert_pem,
            "key_pem": expired_key_pem
        }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let settings = Settings::get(&pool).await.unwrap().unwrap();
    // Url schema unchanged on errors
    assert_eq!(settings.defguard_url, "http://defguard.example.com");
    let body: serde_json::Value = response.json().await;
    assert_eq!(body["msg"], "Certificate has expired");
}
