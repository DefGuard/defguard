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
    sync::{Arc, Mutex, atomic::AtomicBool},
    time::Duration,
};

use axum_extra::extract::cookie::Key;
use defguard_certs::CertificateAuthority;
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
    time::sleep,
};

use super::common::{client::TestClient, generate_expired_test_cert_pem, generate_test_cert_pem};
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
        sleep(Duration::from_millis(50)).await;
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

    async fn drain_clear_https_certs(&mut self) -> usize {
        let mut results = 0;
        sleep(Duration::from_millis(50)).await;
        loop {
            match self.rx.try_recv() {
                Ok(ProxyControlMessage::ClearHttpsCerts) => {
                    results += 1;
                }
                Ok(_) => {}
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
    let (web_reload_tx, _web_reload_rx) = broadcast::channel::<()>(8);

    let webapp = build_webapp(
        tx,
        rx,
        wg_tx,
        web_reload_tx,
        worker_state,
        pool.clone(),
        key,
        failed_logins,
        api_event_tx,
        Version::parse(VERSION).unwrap(),
        Arc::default(),
        proxy_control_tx,
        Arc::new(AtomicBool::new(false)),
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

/// Authenticate as admin and discard the login event from the capture queue.
async fn login_admin(client: &mut TestClient) {
    let auth = Auth::new("admin", "pass123");
    let resp = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(resp.status(), StatusCode::OK);
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

    let response = client
        .post("/api/v1/proxy/cert/external_url_settings")
        .json(&json!({ "ssl_type": "none" }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

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
    let mut settings = Settings::get(&pool).await.unwrap().unwrap();
    // Don't touch the URL if setting no cert
    assert_eq!(settings.public_proxy_url, "https://edge.example.com");

    let saved = Certificates::get(&pool).await.unwrap().unwrap();
    assert_eq!(saved.proxy_http_cert_source, ProxyCertSource::None);
    assert!(saved.proxy_http_cert_pem.is_none());
    assert!(saved.proxy_http_cert_key_pem.is_none());
    assert!(saved.proxy_http_cert_expiry.is_none());
    assert!(saved.acme_domain.is_none());
    assert_eq!(capture.drain_clear_https_certs().await, 1);

    settings.public_proxy_url = "http://edge.example.com".to_string();
    update_current_settings(&pool, settings).await.unwrap();
    let response = client
        .post("/api/v1/proxy/cert/external_url_settings")
        .json(&json!({ "ssl_type": "lets_encrypt" }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    // Url schema changed to https
    let mut settings = Settings::get(&pool).await.unwrap().unwrap();
    assert_eq!(settings.public_proxy_url, "https://edge.example.com");

    let body: serde_json::Value = response.json().await;
    assert!(body["cert_info"].is_null());

    let saved = Certificates::get(&pool).await.unwrap().unwrap();
    assert_eq!(saved.proxy_http_cert_source, ProxyCertSource::None);
    assert!(saved.acme_domain.is_none());
    assert!(saved.proxy_http_cert_pem.is_none());
    assert!(capture.drain_broadcast_certs().await.is_empty());

    seed_ca(&pool).await;

    settings.public_proxy_url = "http://edge.example.com".to_string();
    update_current_settings(&pool, settings).await.unwrap();
    let response = client
        .post("/api/v1/proxy/cert/external_url_settings")
        .json(&json!({ "ssl_type": "defguard_ca" }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    // Url schema changed to https
    let mut settings = Settings::get(&pool).await.unwrap().unwrap();
    assert_eq!(settings.public_proxy_url, "https://edge.example.com");

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

    settings.public_proxy_url = "http://edge.example.com".to_string();
    update_current_settings(&pool, settings).await.unwrap();
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
    // Url schema changed to https
    let mut settings = Settings::get(&pool).await.unwrap().unwrap();
    assert_eq!(settings.public_proxy_url, "https://edge.example.com");

    let body: serde_json::Value = response.json().await;
    assert_eq!(
        body["cert_info"]["common_name"],
        "uploaded-edge.example.com"
    );

    let saved = Certificates::get(&pool).await.unwrap().unwrap();
    assert_eq!(saved.proxy_http_cert_source, ProxyCertSource::Custom);
    assert!(saved.proxy_http_cert_expiry.is_some());
    assert!(saved.acme_domain.is_none());

    settings.public_proxy_url = "http://edge.example.com".to_string();
    update_current_settings(&pool, settings).await.unwrap();
    let (_, mismatched_key_pem) = generate_test_cert_pem("different-edge.example.com");
    let response = client
        .post("/api/v1/proxy/cert/external_url_settings")
        .json(&json!({
            "ssl_type": "own_cert",
            "cert_pem": expected_cert_pem,
            "key_pem": mismatched_key_pem
        }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    // Url schema unchanged on errors
    let mut settings = Settings::get(&pool).await.unwrap().unwrap();
    assert_eq!(settings.public_proxy_url, "http://edge.example.com");

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

    settings.public_proxy_url = "http://edge.example.com".to_string();
    update_current_settings(&pool, settings).await.unwrap();
    let (expired_cert_pem, expired_key_pem) =
        generate_expired_test_cert_pem("expired-edge.example.com");
    let response = client
        .post("/api/v1/proxy/cert/external_url_settings")
        .json(&json!({
            "ssl_type": "own_cert",
            "cert_pem": expired_cert_pem,
            "key_pem": expired_key_pem
        }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    // Url schema unchanged on errors
    let settings = Settings::get(&pool).await.unwrap().unwrap();
    assert_eq!(settings.public_proxy_url, "http://edge.example.com");
    let body: serde_json::Value = response.json().await;
    assert_eq!(body["msg"], "Certificate has expired");
}
