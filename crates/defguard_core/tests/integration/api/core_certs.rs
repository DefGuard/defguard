use defguard_certs::{CertificateAuthority, Csr, DnType, PemLabel, der_to_pem, generate_key_pair};
use defguard_common::db::models::{
    Certificates, CoreCertSource, Settings, settings::update_current_settings,
};
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

fn generate_test_cert_pem(common_name: &str) -> (String, String) {
    let ca = CertificateAuthority::new("Test CA", "test@example.com", 365).unwrap();
    let key_pair = generate_key_pair().unwrap();
    let san = vec![common_name.to_string()];
    let dn = vec![(DnType::CommonName, common_name)];
    let csr = Csr::new(&key_pair, &san, dn).unwrap();
    let cert = ca.sign_csr(&csr).unwrap();
    let cert_pem = der_to_pem(cert.der(), PemLabel::Certificate).unwrap();
    let key_pem = der_to_pem(key_pair.serialize_der().as_slice(), PemLabel::PrivateKey).unwrap();
    (cert_pem, key_pem)
}

#[sqlx::test]
async fn test_core_cert_endpoints(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (client, _) = make_test_client(pool.clone()).await;

    // unauthenticated requests return 401
    let response = client
        .post("/api/v1/core/cert/upload")
        .json(&json!({"cert_pem": "c", "key_pem": "k"}))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let response = client
        .post("/api/v1/core/cert/self-signed")
        .json(&json!({"san": ["localhost"]}))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // self-signed without CA returns 400
    let response = client
        .post("/api/v1/core/cert/self-signed")
        .json(&json!({"san": ["localhost"]}))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // upload custom cert
    let cert_pem = "-----BEGIN CERTIFICATE-----\nfake\n-----END CERTIFICATE-----\n";
    let key_pem = "-----BEGIN PRIVATE KEY-----\nfake\n-----END PRIVATE KEY-----\n";

    let response = client
        .post("/api/v1/core/cert/upload")
        .json(&json!({"cert_pem": cert_pem, "key_pem": key_pem}))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    let saved = Certificates::get(&pool).await.unwrap().unwrap();
    assert_eq!(saved.core_http_cert_source, CoreCertSource::Custom);
    assert_eq!(saved.core_http_cert_pem.as_deref(), Some(cert_pem));
    assert_eq!(saved.core_http_cert_key_pem.as_deref(), Some(key_pem));

    // self-signed with CA present
    seed_ca(&pool).await;

    let response = client
        .post("/api/v1/core/cert/self-signed")
        .json(&json!({"san": ["localhost"]}))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    let saved = Certificates::get(&pool).await.unwrap().unwrap();
    assert_eq!(saved.core_http_cert_source, CoreCertSource::SelfSigned);
    assert!(
        saved
            .core_http_cert_pem
            .as_deref()
            .unwrap_or("")
            .contains("BEGIN CERTIFICATE")
    );
    assert!(
        saved
            .core_http_cert_key_pem
            .as_deref()
            .unwrap_or("")
            .contains("BEGIN")
    );
}

#[sqlx::test]
async fn test_internal_url_settings_endpoint(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (client, _) = make_test_client(pool.clone()).await;

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

    let saved = Certificates::get(&pool).await.unwrap().unwrap();
    assert_eq!(saved.core_http_cert_source, CoreCertSource::None);
    assert!(saved.core_http_cert_pem.is_none());
    assert!(saved.core_http_cert_key_pem.is_none());
    assert!(saved.core_http_cert_expiry.is_none());

    seed_ca(&pool).await;

    let response = client
        .post("/api/v1/core/cert/internal_url_settings")
        .json(&json!({ "ssl_type": "defguard_ca" }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);

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

    let body: serde_json::Value = response.json::<serde_json::Value>().await;
    assert_eq!(body["cert_info"]["common_name"], "uploaded.example.com");

    let saved = Certificates::get(&pool).await.unwrap().unwrap();
    assert_eq!(saved.core_http_cert_source, CoreCertSource::Custom);
    assert!(saved.core_http_cert_expiry.is_some());

    let response = client
        .post("/api/v1/core/cert/internal_url_settings")
        .json(&json!({
            "ssl_type": "own_cert",
            "cert_pem": "-----BEGIN CERTIFICATE-----\nfake\n-----END CERTIFICATE-----\n"
        }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
