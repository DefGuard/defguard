use defguard_certs::{CertificateAuthority, Csr, PemLabel, der_to_pem, generate_key_pair};
use defguard_common::{
    db::{
        Id,
        models::{
            Certificates,
            certificates::{CoreCertSource, ProxyCertSource},
            settings::initialize_current_settings,
            setup_auto_adoption::{AutoAdoptionWizardState, AutoAdoptionWizardStep},
            wireguard::{LocationMfaMode, ServiceLocationMode},
            wizard::{ActiveWizard, Wizard},
            WireguardNetwork,
        },
        setup_pool,
    },
};
use ipnetwork::IpNetwork;
use rcgen::DnType;
use reqwest::{
    Client, StatusCode,
    header::{HeaderMap, HeaderValue, USER_AGENT},
};
use serde_json::json;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

mod common;
use common::make_setup_test_client;

const SESSION_COOKIE_NAME: &str = "defguard_session";

async fn bootstrap_wizard_to_url_settings(
    pool: &sqlx::PgPool,
) -> (common::TestClient, tokio::sync::oneshot::Receiver<()>) {
    Wizard::init(pool, true).await.expect("Failed to init wizard");
    let (client, shutdown_rx) = make_setup_test_client(pool.clone()).await;
    let resp = client
        .post("/api/v1/initial_setup/admin")
        .json(&json!({
            "first_name": "Admin",
            "last_name": "Admin",
            "username": "url_admin",
            "email": "url_admin@example.com",
            "password": "Passw0rd!"
        }))
        .send()
        .await
        .expect("Failed to create admin");
    assert_eq!(resp.status(), StatusCode::CREATED);
    assert!(resp.cookies().any(|c| c.name() == SESSION_COOKIE_NAME));
    (client, shutdown_rx)
}

fn generate_test_cert_pem(common_name: &str) -> (String, String) {
    let ca = CertificateAuthority::new("Test CA", "test@example.com", 365).unwrap();
    let key_pair = generate_key_pair().unwrap();
    let san = vec![common_name.to_string()];
    let dn = vec![(DnType::CommonName, common_name)];
    let csr = Csr::new(&key_pair, &san, dn).unwrap();
    let cert = ca.sign_csr(&csr).unwrap();
    let cert_pem = der_to_pem(&cert.der().to_vec(), PemLabel::Certificate).unwrap();
    let key_pem = der_to_pem(key_pair.serialize_der().as_slice(), PemLabel::PrivateKey).unwrap();
    (cert_pem, key_pem)
}

async fn seed_wireguard_network(pool: &sqlx::PgPool) -> WireguardNetwork<Id> {
    let mut location = WireguardNetwork::new(
        "url-settings-net".to_string(),
        51820,
        "1.2.3.4".to_string(),
        None,
        ["0.0.0.0/0".parse().unwrap()],
        false,
        false,
        false,
        LocationMfaMode::Disabled,
        ServiceLocationMode::Disabled,
    )
    .set_address(["10.0.0.1/24".parse::<IpNetwork>().unwrap()])
    .unwrap();
    location.mtu = 1280;
    location.save(pool).await.expect("Failed to save wireguard network")
}

#[sqlx::test]
async fn test_internal_url_settings_all_ssl_types(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool).await.unwrap();
    let (client, _shutdown_rx) = bootstrap_wizard_to_url_settings(&pool).await;

    // ssl_type = none
    let resp = client
        .post("/api/v1/initial_setup/auto_wizard/internal_url_settings")
        .json(&json!({ "defguard_url": "http://defguard.example.com", "ssl_type": "none" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    let state = AutoAdoptionWizardState::get(&pool).await.unwrap().unwrap_or_default();
    assert_eq!(state.step, AutoAdoptionWizardStep::InternalUrlSslConfig);

    let certs = Certificates::get_or_default(&pool).await.unwrap();
    assert_eq!(certs.core_http_cert_source, CoreCertSource::None);
    assert!(certs.core_http_cert_pem.is_none());
    assert!(certs.core_http_cert_key_pem.is_none());

    // ssl_type = defguard_ca
    let resp = client
        .post("/api/v1/initial_setup/auto_wizard/internal_url_settings")
        .json(&json!({ "defguard_url": "https://defguard.example.com", "ssl_type": "defguard_ca" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(!body["cert_info"].is_null());
    assert!(body["cert_info"]["valid_for_days"].as_i64().unwrap_or(0) > 0);

    let certs = Certificates::get_or_default(&pool).await.unwrap();
    assert_eq!(certs.core_http_cert_source, CoreCertSource::SelfSigned);
    assert!(certs.core_http_cert_pem.as_deref().unwrap_or("").contains("BEGIN CERTIFICATE"));
    assert!(certs.core_http_cert_key_pem.is_some());
    assert!(certs.ca_cert_der.is_some());
    assert!(certs.ca_key_der.is_some());

    // ssl_type = own_cert
    let (cert_pem, key_pem) = generate_test_cert_pem("defguard.example.com");
    let resp = client
        .post("/api/v1/initial_setup/auto_wizard/internal_url_settings")
        .json(&json!({
            "defguard_url": "https://defguard.example.com",
            "ssl_type": "own_cert",
            "cert_pem": cert_pem,
            "key_pem": key_pem
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(!body["cert_info"].is_null());

    let certs = Certificates::get_or_default(&pool).await.unwrap();
    assert_eq!(certs.core_http_cert_source, CoreCertSource::Custom);
    assert_eq!(certs.core_http_cert_pem.as_deref(), Some(cert_pem.as_str()));
    assert_eq!(certs.core_http_cert_key_pem.as_deref(), Some(key_pem.as_str()));

    // own_cert without key_pem
    let (cert_pem_only, _) = generate_test_cert_pem("defguard.example.com");
    let resp = client
        .post("/api/v1/initial_setup/auto_wizard/internal_url_settings")
        .json(&json!({
            "defguard_url": "https://defguard.example.com",
            "ssl_type": "own_cert",
            "cert_pem": cert_pem_only
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[sqlx::test]
async fn test_get_internal_ssl_info(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool).await.unwrap();
    let (client, _shutdown_rx) = bootstrap_wizard_to_url_settings(&pool).await;

    let resp = client
        .get("/api/v1/initial_setup/auto_wizard/internal_url_settings")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body["ca_cert_pem"].is_null());

    client
        .post("/api/v1/initial_setup/auto_wizard/internal_url_settings")
        .json(&json!({ "defguard_url": "https://defguard.example.com", "ssl_type": "defguard_ca" }))
        .send()
        .await
        .unwrap();

    let resp = client
        .get("/api/v1/initial_setup/auto_wizard/internal_url_settings")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body["ca_cert_pem"].as_str().unwrap_or("").contains("BEGIN CERTIFICATE"));
}

#[sqlx::test]
async fn test_external_url_settings_all_ssl_types(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool).await.unwrap();
    let (client, _shutdown_rx) = bootstrap_wizard_to_url_settings(&pool).await;

    // ssl_type = none
    let resp = client
        .post("/api/v1/initial_setup/auto_wizard/external_url_settings")
        .json(&json!({ "public_proxy_url": "https://proxy.example.com", "ssl_type": "none" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    let state = AutoAdoptionWizardState::get(&pool).await.unwrap().unwrap_or_default();
    assert_eq!(state.step, AutoAdoptionWizardStep::ExternalUrlSslConfig);

    let certs = Certificates::get_or_default(&pool).await.unwrap();
    assert_eq!(certs.proxy_http_cert_source, ProxyCertSource::None);
    assert!(certs.proxy_http_cert_pem.is_none());

    // ssl_type = lets_encrypt: stores ACME domain, does not issue cert yet
    let resp = client
        .post("/api/v1/initial_setup/auto_wizard/external_url_settings")
        .json(&json!({ "public_proxy_url": "https://proxy.example.com", "ssl_type": "lets_encrypt" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    let certs = Certificates::get_or_default(&pool).await.unwrap();
    assert_eq!(certs.proxy_http_cert_source, ProxyCertSource::LetsEncrypt);
    assert_eq!(certs.acme_domain.as_deref(), Some("proxy.example.com"));
    assert!(certs.proxy_http_cert_pem.is_none());
    assert!(certs.proxy_http_cert_key_pem.is_none());

    // ssl_type = defguard_ca
    let resp = client
        .post("/api/v1/initial_setup/auto_wizard/external_url_settings")
        .json(&json!({ "public_proxy_url": "https://proxy.example.com", "ssl_type": "defguard_ca" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(!body["cert_info"].is_null());

    let certs = Certificates::get_or_default(&pool).await.unwrap();
    assert_eq!(certs.proxy_http_cert_source, ProxyCertSource::SelfSigned);
    assert!(certs.proxy_http_cert_pem.as_deref().unwrap_or("").contains("BEGIN CERTIFICATE"));
    assert!(certs.proxy_http_cert_key_pem.is_some());
    assert!(certs.ca_cert_der.is_some());

    // ssl_type = own_cert
    let (cert_pem, key_pem) = generate_test_cert_pem("proxy.example.com");
    let resp = client
        .post("/api/v1/initial_setup/auto_wizard/external_url_settings")
        .json(&json!({
            "public_proxy_url": "https://proxy.example.com",
            "ssl_type": "own_cert",
            "cert_pem": cert_pem,
            "key_pem": key_pem
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    let certs = Certificates::get_or_default(&pool).await.unwrap();
    assert_eq!(certs.proxy_http_cert_source, ProxyCertSource::Custom);
    assert_eq!(certs.proxy_http_cert_pem.as_deref(), Some(cert_pem.as_str()));
    assert_eq!(certs.proxy_http_cert_key_pem.as_deref(), Some(key_pem.as_str()));
}

#[sqlx::test]
async fn test_get_external_ssl_info(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool).await.unwrap();
    let (client, _shutdown_rx) = bootstrap_wizard_to_url_settings(&pool).await;

    let resp = client
        .get("/api/v1/initial_setup/auto_wizard/external_url_settings")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body["ca_cert_pem"].is_null());

    client
        .post("/api/v1/initial_setup/auto_wizard/external_url_settings")
        .json(&json!({ "public_proxy_url": "https://proxy.example.com", "ssl_type": "defguard_ca" }))
        .send()
        .await
        .unwrap();

    let resp = client
        .get("/api/v1/initial_setup/auto_wizard/external_url_settings")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body["ca_cert_pem"].as_str().unwrap_or("").contains("BEGIN CERTIFICATE"));
}

#[sqlx::test]
async fn test_url_settings_endpoints_require_auth(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool).await.unwrap();
    Wizard::init(&pool, true).await.unwrap();

    let (authed_client, _shutdown_rx) = make_setup_test_client(pool.clone()).await;
    let base_url = authed_client.base_url();

    authed_client
        .post("/api/v1/initial_setup/admin")
        .json(&json!({
            "first_name": "Admin", "last_name": "Admin",
            "username": "auth_url_admin", "email": "auth_url@example.com",
            "password": "Passw0rd!"
        }))
        .send()
        .await
        .unwrap();

    let unauth = Client::builder()
        .default_headers({
            let mut h = HeaderMap::new();
            h.insert(USER_AGENT, HeaderValue::from_static("test/0.0"));
            h
        })
        .build()
        .unwrap();

    for path in [
        "/api/v1/initial_setup/auto_wizard/internal_url_settings",
        "/api/v1/initial_setup/auto_wizard/external_url_settings",
    ] {
        let resp = unauth
            .post(format!("{base_url}{path}"))
            .json(&json!({ "defguard_url": "https://x.com", "public_proxy_url": "https://x.com", "ssl_type": "none" }))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED, "POST {path}");

        let resp = unauth.get(format!("{base_url}{path}")).send().await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED, "GET {path}");
    }
}

#[sqlx::test]
async fn test_auto_adoption_full_flow_new_url_steps(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool).await.unwrap();
    seed_wireguard_network(&pool).await;
    Wizard::init(&pool, true).await.unwrap();
    let (client, shutdown_rx) = make_setup_test_client(pool.clone()).await;

    let resp = client
        .post("/api/v1/initial_setup/admin")
        .json(&json!({
            "first_name": "Admin", "last_name": "Admin",
            "username": "new_flow_admin", "email": "new_flow@example.com",
            "password": "Passw0rd!"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let state = AutoAdoptionWizardState::get(&pool).await.unwrap().unwrap_or_default();
    assert_eq!(state.step, AutoAdoptionWizardStep::UrlSettings);

    let resp = client
        .post("/api/v1/initial_setup/auto_wizard/internal_url_settings")
        .json(&json!({ "defguard_url": "https://defguard.new.example.com", "ssl_type": "none" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let state = AutoAdoptionWizardState::get(&pool).await.unwrap().unwrap_or_default();
    assert_eq!(state.step, AutoAdoptionWizardStep::InternalUrlSslConfig);

    let resp = client
        .post("/api/v1/initial_setup/auto_wizard/external_url_settings")
        .json(&json!({ "public_proxy_url": "https://proxy.new.example.com", "ssl_type": "none" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let state = AutoAdoptionWizardState::get(&pool).await.unwrap().unwrap_or_default();
    assert_eq!(state.step, AutoAdoptionWizardStep::ExternalUrlSslConfig);

    let resp = client
        .post("/api/v1/initial_setup/auto_wizard/vpn_settings")
        .json(&json!({
            "vpn_public_ip": "10.20.30.40",
            "vpn_wireguard_port": 51820,
            "vpn_gateway_address": "10.10.0.1/24",
            "vpn_allowed_ips": "0.0.0.0/0",
            "vpn_dns_server_ip": "1.1.1.1"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let state = AutoAdoptionWizardState::get(&pool).await.unwrap().unwrap_or_default();
    assert_eq!(state.step, AutoAdoptionWizardStep::MfaSettings);

    let resp = client
        .post("/api/v1/initial_setup/auto_wizard/mfa_settings")
        .json(&json!({ "vpn_mfa_mode": "disabled" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let state = AutoAdoptionWizardState::get(&pool).await.unwrap().unwrap_or_default();
    assert_eq!(state.step, AutoAdoptionWizardStep::Summary);

    let resp = client
        .post("/api/v1/initial_setup/finish")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let wizard = Wizard::get(&pool).await.unwrap();
    assert!(wizard.completed);
    assert_eq!(wizard.active_wizard, ActiveWizard::None);

    let shutdown = tokio::time::timeout(std::time::Duration::from_secs(1), shutdown_rx).await;
    assert!(matches!(shutdown, Ok(Ok(()))));
}
