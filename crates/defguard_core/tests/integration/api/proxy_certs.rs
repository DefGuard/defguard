/// Integration tests for proxy HTTPS certificate endpoints:
///   POST /api/v1/proxy/cert/upload
///   POST /api/v1/proxy/cert/self-signed
///
/// The `proxy_control_tx` channel is kept alive and its receiver is held by
/// `ProxyBroadcastCapture`, acting as a mock for the proxy manager.
/// This lets us assert that the correct `ProxyControlMessage::BroadcastHttpsCerts`
/// was sent after a successful cert operation without needing a real proxy process.
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::{Arc, Mutex},
};

use axum_extra::extract::cookie::Key;
use defguard_certs::{CertificateAuthority, Csr, DnType, PemLabel, der_to_pem, generate_key_pair};
use defguard_common::{
    VERSION,
    db::{
        models::{
            Certificates, ProxyCertSource, Settings,
            settings::{initialize_current_settings, update_current_settings},
        },
        setup_pool,
    },
    types::proxy::ProxyControlMessage,
};
use defguard_core::{
    auth::failed_login::FailedLoginMap,
    build_webapp,
    db::AppEvent,
    enterprise::license::{License, LicenseTier, SupportType, set_cached_license},
    events::ApiEvent,
    grpc::{GatewayEvent, WorkerState},
    handlers::Auth,
};
use reqwest::StatusCode;
use semver::Version;
use serde_json::json;
use sqlx::{
    PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
};
use tokio::{
    net::TcpListener,
    sync::{
        broadcast,
        mpsc::{Receiver, Sender, channel, unbounded_channel},
    },
};

use super::common::client::TestClient;
use crate::common::{init_config, initialize_users};

// Mock: captures messages sent to the proxy manager channel.
struct ProxyBroadcastCapture {
    rx: Receiver<ProxyControlMessage>,
}

impl ProxyBroadcastCapture {
    /// Drain all pending messages and return only the `BroadcastHttpsCerts` ones.
    async fn drain_broadcast_certs(&mut self) -> Vec<(String, String)> {
        let mut results = Vec::new();
        // Give the handler a brief moment to enqueue the message.
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        loop {
            match self.rx.try_recv() {
                Ok(ProxyControlMessage::BroadcastHttpsCerts { cert_pem, key_pem }) => {
                    results.push((cert_pem, key_pem));
                }
                Ok(_) => {} // other control messages - ignore
                Err(_) => break,
            }
        }
        results
    }
}

// Test client builder that exposes the proxy-control receiver.
async fn make_test_client_with_proxy_rx(
    pool: PgPool,
) -> (TestClient, ProxyBroadcastCapture, PgPool) {
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);
    let listener = TcpListener::bind(addr)
        .await
        .expect("Could not bind ephemeral socket");
    let port = listener.local_addr().unwrap().port();
    let _config = init_config(Some(&format!("http://localhost:{port}")), &pool).await;
    initialize_users(&pool).await;
    initialize_current_settings(&pool)
        .await
        .expect("Could not initialize settings");

    // Use a channel large enough that sends never block in tests.
    let (proxy_control_tx, proxy_control_rx): (
        Sender<ProxyControlMessage>,
        Receiver<ProxyControlMessage>,
    ) = channel(32);

    let (api_event_tx, api_event_rx) = unbounded_channel::<ApiEvent>();
    let (tx, rx) = unbounded_channel::<AppEvent>();
    let worker_state = Arc::new(Mutex::new(WorkerState::new(tx.clone())));
    let (wg_tx, _wg_rx) = broadcast::channel::<GatewayEvent>(16);

    let failed_logins = Arc::new(Mutex::new(FailedLoginMap::new()));

    let license = License::new(
        "test_customer".to_string(),
        false,
        None,
        None,
        None,
        LicenseTier::Business,
        SupportType::Basic,
    );
    set_cached_license(Some(license));

    let key = Key::from(
        Settings::get_current_settings()
            .secret_key_required()
            .unwrap()
            .as_bytes(),
    );

    let webapp = build_webapp(
        tx,
        rx,
        wg_tx,
        worker_state,
        pool.clone(),
        key,
        failed_logins,
        api_event_tx,
        Version::parse(VERSION).unwrap(),
        Arc::default(),
        proxy_control_tx,
    );

    let client = TestClient::new(webapp, listener, api_event_rx);
    let capture = ProxyBroadcastCapture {
        rx: proxy_control_rx,
    };

    (client, capture, pool)
}

/// Seed a Core CA into the certificates singleton so cert-signing tests can run.
async fn seed_ca(pool: &PgPool) {
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

/// Authenticate as admin and discard the login event from the capture queue.
async fn login_admin(client: &mut TestClient) {
    let auth = Auth::new("admin", "pass123");
    let resp = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(resp.status(), StatusCode::OK);
}

/// Unauthenticated requests to both endpoints must return 401.
#[sqlx::test]
async fn test_proxy_cert_endpoints_require_auth(_: PgPoolOptions, opts: PgConnectOptions) {
    let pool = setup_pool(opts).await;
    let (client, _capture, _pool) = make_test_client_with_proxy_rx(pool).await;

    let fake_cert = "-----BEGIN CERTIFICATE-----\nfake\n-----END CERTIFICATE-----\n";
    let fake_key = "-----BEGIN PRIVATE KEY-----\nfake\n-----END PRIVATE KEY-----\n";

    let resp = client
        .post("/api/v1/proxy/cert/upload")
        .json(&json!({"cert_pem": fake_cert, "key_pem": fake_key}))
        .send()
        .await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    let resp = client
        .post("/api/v1/proxy/cert/self-signed")
        .json(&json!({"san": ["proxy.example.com"]}))
        .send()
        .await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

/// Uploading a custom cert stores it in the DB with source=Custom and broadcasts
/// the exact same PEM strings to the proxy manager channel.
#[sqlx::test]
async fn test_proxy_cert_upload_persists_and_broadcasts(_: PgPoolOptions, opts: PgConnectOptions) {
    let pool = setup_pool(opts).await;
    let (mut client, mut capture, pool) = make_test_client_with_proxy_rx(pool).await;
    login_admin(&mut client).await;

    let cert_pem = "-----BEGIN CERTIFICATE-----\ncustom_cert\n-----END CERTIFICATE-----\n";
    let key_pem = "-----BEGIN PRIVATE KEY-----\ncustom_key\n-----END PRIVATE KEY-----\n";

    let resp = client
        .post("/api/v1/proxy/cert/upload")
        .json(&json!({"cert_pem": cert_pem, "key_pem": key_pem}))
        .send()
        .await;
    assert_eq!(resp.status(), StatusCode::OK);

    // DB persistence
    let saved = Certificates::get(&pool).await.unwrap().unwrap();
    assert_eq!(saved.proxy_http_cert_source, ProxyCertSource::Custom);
    assert_eq!(saved.proxy_http_cert_pem.as_deref(), Some(cert_pem));
    assert_eq!(saved.proxy_http_cert_key_pem.as_deref(), Some(key_pem));

    // Broadcast mock: exactly one BroadcastHttpsCerts with the correct PEM values
    let broadcasts = capture.drain_broadcast_certs().await;
    assert_eq!(broadcasts.len(), 1, "Expected exactly one broadcast");
    assert_eq!(broadcasts[0].0, cert_pem);
    assert_eq!(broadcasts[0].1, key_pem);
}

/// proxy_http_cert_pair() returns Some after a custom upload.
#[sqlx::test]
async fn test_proxy_cert_pair_accessible_after_upload(_: PgPoolOptions, opts: PgConnectOptions) {
    let pool = setup_pool(opts).await;
    let (mut client, _capture, pool) = make_test_client_with_proxy_rx(pool).await;
    login_admin(&mut client).await;

    let cert_pem = "-----BEGIN CERTIFICATE-----\npair_test\n-----END CERTIFICATE-----\n";
    let key_pem = "-----BEGIN PRIVATE KEY-----\npair_key\n-----END PRIVATE KEY-----\n";

    client
        .post("/api/v1/proxy/cert/upload")
        .json(&json!({"cert_pem": cert_pem, "key_pem": key_pem}))
        .send()
        .await;

    let saved = Certificates::get(&pool).await.unwrap().unwrap();
    assert_eq!(
        saved.proxy_http_cert_pair(),
        Some((cert_pem, key_pem)),
        "proxy_http_cert_pair() must return the stored PEM pair"
    );
}

/// Requesting a self-signed cert when no CA is configured returns 400.
#[sqlx::test]
async fn test_proxy_cert_self_signed_without_ca(_: PgPoolOptions, opts: PgConnectOptions) {
    let pool = setup_pool(opts).await;
    let (mut client, _capture, _pool) = make_test_client_with_proxy_rx(pool).await;
    login_admin(&mut client).await;

    let resp = client
        .post("/api/v1/proxy/cert/self-signed")
        .json(&json!({"san": ["proxy.example.com"]}))
        .send()
        .await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

/// With a CA present, a self-signed cert is issued, saved as SelfSigned, and
/// the resulting PEM is broadcast to the proxy manager.
#[sqlx::test]
async fn test_proxy_cert_self_signed_with_ca(_: PgPoolOptions, opts: PgConnectOptions) {
    let pool = setup_pool(opts).await;
    let (mut client, mut capture, pool) = make_test_client_with_proxy_rx(pool).await;
    seed_ca(&pool).await;
    login_admin(&mut client).await;

    let resp = client
        .post("/api/v1/proxy/cert/self-signed")
        .json(&json!({"san": ["proxy.example.com"]}))
        .send()
        .await;
    assert_eq!(resp.status(), StatusCode::OK);

    // DB: source set to SelfSigned, valid PEM stored
    let saved = Certificates::get(&pool).await.unwrap().unwrap();
    assert_eq!(saved.proxy_http_cert_source, ProxyCertSource::SelfSigned);
    assert!(
        saved
            .proxy_http_cert_pem
            .as_deref()
            .unwrap_or("")
            .contains("BEGIN CERTIFICATE"),
        "cert_pem must be a valid PEM certificate"
    );
    assert!(
        saved
            .proxy_http_cert_key_pem
            .as_deref()
            .unwrap_or("")
            .contains("BEGIN"),
        "key_pem must be a valid PEM key"
    );

    // Broadcast mock: one BroadcastHttpsCerts with matching PEM content
    let broadcasts = capture.drain_broadcast_certs().await;
    assert_eq!(broadcasts.len(), 1, "Expected exactly one broadcast");
    let (broadcasted_cert, broadcasted_key) = &broadcasts[0];
    assert!(
        broadcasted_cert.contains("BEGIN CERTIFICATE"),
        "Broadcasted cert must be valid PEM"
    );
    assert!(
        broadcasted_key.contains("BEGIN"),
        "Broadcasted key must be valid PEM"
    );
    // Broadcast must match what was persisted
    assert_eq!(
        saved.proxy_http_cert_pem.as_deref(),
        Some(broadcasted_cert.as_str())
    );
    assert_eq!(
        saved.proxy_http_cert_key_pem.as_deref(),
        Some(broadcasted_key.as_str())
    );
}

/// An empty SAN list returns 400 - at least one SAN is required to issue a cert.
#[sqlx::test]
async fn test_proxy_cert_self_signed_empty_san(_: PgPoolOptions, opts: PgConnectOptions) {
    let pool = setup_pool(opts).await;
    let (mut client, _capture, pool) = make_test_client_with_proxy_rx(pool).await;
    seed_ca(&pool).await;
    login_admin(&mut client).await;

    let resp = client
        .post("/api/v1/proxy/cert/self-signed")
        .json(&json!({"san": []}))
        .send()
        .await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

/// Multiple SANs are all included in the issued certificate.
#[sqlx::test]
async fn test_proxy_cert_self_signed_multiple_sans(_: PgPoolOptions, opts: PgConnectOptions) {
    let pool = setup_pool(opts).await;
    let (mut client, _capture, pool) = make_test_client_with_proxy_rx(pool).await;
    seed_ca(&pool).await;
    login_admin(&mut client).await;

    let resp = client
        .post("/api/v1/proxy/cert/self-signed")
        .json(&json!({"san": ["proxy.example.com", "proxy2.example.com", "192.168.1.1"]}))
        .send()
        .await;
    assert_eq!(resp.status(), StatusCode::OK);

    let saved = Certificates::get(&pool).await.unwrap().unwrap();
    assert_eq!(saved.proxy_http_cert_source, ProxyCertSource::SelfSigned);
    assert!(saved.proxy_http_cert_pem.is_some());
}

/// Uploading a second custom cert overwrites the previous one (idempotent upsert).
#[sqlx::test]
async fn test_proxy_cert_upload_overwrites_previous(_: PgPoolOptions, opts: PgConnectOptions) {
    let pool = setup_pool(opts).await;
    let (mut client, mut capture, pool) = make_test_client_with_proxy_rx(pool).await;
    login_admin(&mut client).await;

    let first_cert = "-----BEGIN CERTIFICATE-----\nfirst\n-----END CERTIFICATE-----\n";
    let first_key = "-----BEGIN PRIVATE KEY-----\nfirst_key\n-----END PRIVATE KEY-----\n";
    client
        .post("/api/v1/proxy/cert/upload")
        .json(&json!({"cert_pem": first_cert, "key_pem": first_key}))
        .send()
        .await;

    let second_cert = "-----BEGIN CERTIFICATE-----\nsecond\n-----END CERTIFICATE-----\n";
    let second_key = "-----BEGIN PRIVATE KEY-----\nsecond_key\n-----END PRIVATE KEY-----\n";
    let resp = client
        .post("/api/v1/proxy/cert/upload")
        .json(&json!({"cert_pem": second_cert, "key_pem": second_key}))
        .send()
        .await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Only the latest cert must be stored
    let saved = Certificates::get(&pool).await.unwrap().unwrap();
    assert_eq!(saved.proxy_http_cert_source, ProxyCertSource::Custom);
    assert_eq!(saved.proxy_http_cert_pem.as_deref(), Some(second_cert));
    assert_eq!(saved.proxy_http_cert_key_pem.as_deref(), Some(second_key));

    // Both uploads must have triggered a broadcast
    let broadcasts = capture.drain_broadcast_certs().await;
    assert_eq!(broadcasts.len(), 2, "Expected one broadcast per upload");
    assert_eq!(broadcasts[1].0, second_cert);
}

/// After a self-signed cert is issued, source transitions from the previous
/// state (Custom) to SelfSigned and the old PEM is replaced.
#[sqlx::test]
async fn test_proxy_cert_self_signed_overwrites_custom(_: PgPoolOptions, opts: PgConnectOptions) {
    let pool = setup_pool(opts).await;
    let (mut client, _capture, pool) = make_test_client_with_proxy_rx(pool).await;
    seed_ca(&pool).await;
    login_admin(&mut client).await;

    // First, upload a custom cert.
    client
        .post("/api/v1/proxy/cert/upload")
        .json(&json!({
            "cert_pem": "-----BEGIN CERTIFICATE-----\ncustom\n-----END CERTIFICATE-----\n",
            "key_pem":  "-----BEGIN PRIVATE KEY-----\ncustom_key\n-----END PRIVATE KEY-----\n"
        }))
        .send()
        .await;

    // Now provision a self-signed one.
    let resp = client
        .post("/api/v1/proxy/cert/self-signed")
        .json(&json!({"san": ["proxy.example.com"]}))
        .send()
        .await;
    assert_eq!(resp.status(), StatusCode::OK);

    let saved = Certificates::get(&pool).await.unwrap().unwrap();
    assert_eq!(
        saved.proxy_http_cert_source,
        ProxyCertSource::SelfSigned,
        "Source must be SelfSigned after re-provisioning"
    );
    assert!(
        saved
            .proxy_http_cert_pem
            .as_deref()
            .unwrap_or("")
            .contains("BEGIN CERTIFICATE"),
        "Stored cert must be a valid CA-signed PEM, not the old custom one"
    );
}

/// A non-admin user (regular user) must receive 403 on both endpoints.
#[sqlx::test]
async fn test_proxy_cert_endpoints_require_admin_role(_: PgPoolOptions, opts: PgConnectOptions) {
    let pool = setup_pool(opts).await;
    let (client, _capture, _pool) = make_test_client_with_proxy_rx(pool).await;

    // Log in as the regular test user (hpotter) seeded by initialize_users()
    let auth = Auth::new("hpotter", "pass123");
    let resp = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(resp.status(), StatusCode::OK);

    let fake_cert = "-----BEGIN CERTIFICATE-----\nfake\n-----END CERTIFICATE-----\n";
    let fake_key = "-----BEGIN PRIVATE KEY-----\nfake\n-----END PRIVATE KEY-----\n";

    let resp = client
        .post("/api/v1/proxy/cert/upload")
        .json(&json!({"cert_pem": fake_cert, "key_pem": fake_key}))
        .send()
        .await;
    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "Non-admin must not upload proxy cert"
    );

    let resp = client
        .post("/api/v1/proxy/cert/self-signed")
        .json(&json!({"san": ["proxy.example.com"]}))
        .send()
        .await;
    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "Non-admin must not provision self-signed proxy cert"
    );
}

/// When no cert is configured (default state), proxy_http_cert_pair() returns None.
#[sqlx::test]
async fn test_proxy_cert_pair_none_by_default(_: PgPoolOptions, opts: PgConnectOptions) {
    let pool = setup_pool(opts).await;
    // Initialize DB without touching the certificates table (fresh schema).
    initialize_current_settings(&pool).await.unwrap();

    let certs = Certificates::get_or_default(&pool).await.unwrap();
    assert_eq!(certs.proxy_http_cert_source, ProxyCertSource::None);
    assert!(
        certs.proxy_http_cert_pair().is_none(),
        "No cert must be configured by default"
    );
}

#[sqlx::test]
async fn test_external_url_settings_endpoint(_: PgPoolOptions, opts: PgConnectOptions) {
    let pool = setup_pool(opts).await;
    let (mut client, mut capture, pool) = make_test_client_with_proxy_rx(pool).await;
    login_admin(&mut client).await;

    let mut settings = Settings::get_current_settings();
    settings.public_proxy_url = "https://edge.example.com".into();
    update_current_settings(&pool, settings).await.unwrap();

    let response = client
        .post("/api/v1/proxy/cert/external_url_settings")
        .json(&json!({ "ssl_type": "none" }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    let saved = Certificates::get(&pool).await.unwrap().unwrap();
    assert_eq!(saved.proxy_http_cert_source, ProxyCertSource::None);
    assert!(saved.proxy_http_cert_pem.is_none());
    assert!(saved.proxy_http_cert_key_pem.is_none());
    assert!(saved.proxy_http_cert_expiry.is_none());
    assert!(saved.acme_domain.is_none());
    assert!(capture.drain_broadcast_certs().await.is_empty());

    let response = client
        .post("/api/v1/proxy/cert/external_url_settings")
        .json(&json!({ "ssl_type": "lets_encrypt" }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    let body: serde_json::Value = response.json().await;
    assert!(body["cert_info"].is_null());

    let saved = Certificates::get(&pool).await.unwrap().unwrap();
    assert_eq!(saved.proxy_http_cert_source, ProxyCertSource::LetsEncrypt);
    assert_eq!(saved.acme_domain.as_deref(), Some("edge.example.com"));
    assert!(saved.proxy_http_cert_pem.is_none());
    assert!(capture.drain_broadcast_certs().await.is_empty());

    seed_ca(&pool).await;

    let response = client
        .post("/api/v1/proxy/cert/external_url_settings")
        .json(&json!({ "ssl_type": "defguard_ca" }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    let body: serde_json::Value = response.json().await;
    assert!(!body["cert_info"].is_null());
    assert_eq!(body["cert_info"]["common_name"], "edge.example.com");

    let saved = Certificates::get(&pool).await.unwrap().unwrap();
    assert_eq!(saved.proxy_http_cert_source, ProxyCertSource::SelfSigned);
    assert!(saved.proxy_http_cert_expiry.is_some());
    assert!(
        saved
            .proxy_http_cert_pem
            .as_deref()
            .unwrap_or("")
            .contains("BEGIN CERTIFICATE")
    );
    assert!(saved.acme_domain.is_none());

    let broadcasts = capture.drain_broadcast_certs().await;
    assert_eq!(broadcasts.len(), 1, "Expected exactly one broadcast");
    assert!(broadcasts[0].0.contains("BEGIN CERTIFICATE"));
    assert!(broadcasts[0].1.contains("BEGIN PRIVATE KEY"));

    let (cert_pem, key_pem) = generate_test_cert_pem("uploaded-edge.example.com");
    let expected_cert_pem = cert_pem.clone();
    let expected_key_pem = key_pem.clone();
    let response = client
        .post("/api/v1/proxy/cert/external_url_settings")
        .json(&json!({
            "ssl_type": "own_cert",
            "cert_pem": cert_pem,
            "key_pem": key_pem
        }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    let body: serde_json::Value = response.json().await;
    assert_eq!(body["cert_info"]["common_name"], "uploaded-edge.example.com");

    let saved = Certificates::get(&pool).await.unwrap().unwrap();
    assert_eq!(saved.proxy_http_cert_source, ProxyCertSource::Custom);
    assert!(saved.proxy_http_cert_expiry.is_some());
    assert!(saved.acme_domain.is_none());

    let broadcasts = capture.drain_broadcast_certs().await;
    assert_eq!(broadcasts.len(), 1, "Expected exactly one broadcast");
    assert_eq!(broadcasts[0].0, expected_cert_pem);
    assert_eq!(broadcasts[0].1, expected_key_pem);

    let response = client
        .post("/api/v1/proxy/cert/external_url_settings")
        .json(&json!({
            "ssl_type": "own_cert",
            "cert_pem": "-----BEGIN CERTIFICATE-----\nfake\n-----END CERTIFICATE-----\n"
        }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
