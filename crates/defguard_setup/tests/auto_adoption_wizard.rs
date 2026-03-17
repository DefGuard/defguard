use defguard_common::{
    config::DefGuardConfig,
    db::{
        Id,
        models::{
            Settings, WireguardNetwork,
            settings::initialize_current_settings,
            setup_auto_adoption::{AutoAdoptionWizardState, AutoAdoptionWizardStep},
            wireguard::{LocationMfaMode, ServiceLocationMode},
            wizard::{ActiveWizard, Wizard},
        },
        setup_pool,
    },
};
use defguard_setup::auto_adoption::attempt_auto_adoption;
use ipnetwork::IpNetwork;
use reqwest::{
    Client, StatusCode,
    header::{HeaderMap, HeaderValue, USER_AGENT},
};
use serde_json::json;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

mod common;
use common::make_setup_test_client;

const SESSION_COOKIE_NAME: &str = "defguard_session";

async fn assert_auto_adoption_step(pool: &sqlx::PgPool, expected: AutoAdoptionWizardStep) {
    let state = AutoAdoptionWizardState::get(pool)
        .await
        .expect("Failed to fetch auto adoption state")
        .unwrap_or_default();
    assert_eq!(
        state.step, expected,
        "Expected auto-adoption step {expected:?}, got {:?}",
        state.step
    );
}

/// Seed a minimal WireguardNetwork row required by the auto-adoption VPN/MFA steps.
async fn seed_wireguard_network(pool: &sqlx::PgPool) -> WireguardNetwork<Id> {
    let mut location = WireguardNetwork::new(
        "auto-net".to_string(),
        51820,
        "1.2.3.4".to_string(),
        None,
        ["0.0.0.0/0".parse().unwrap()],
        false,
        false,
        false,
        LocationMfaMode::Disabled,
        ServiceLocationMode::Disabled,
    );
    location.set_address(["10.0.0.0/24".parse::<IpNetwork>().unwrap()]);
    location.mtu = 1280;
    location
        .save(pool)
        .await
        .expect("Failed to save wireguard network")
}

#[sqlx::test]
async fn test_auto_adoption_full_flow(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool)
        .await
        .expect("Failed to initialize settings");

    // Auto-adoption requires a pre-existing network to configure
    let network = seed_wireguard_network(&pool).await;

    Wizard::init(&pool, true)
        .await
        .expect("Failed to init wizard");

    let (client, shutdown_rx) = make_setup_test_client(pool.clone()).await;

    assert_auto_adoption_step(&pool, AutoAdoptionWizardStep::Welcome).await;

    let resp = client
        .post("/api/v1/initial_setup/admin")
        .json(&json!({
            "first_name": "Admin",
            "last_name": "Admin",
            "username": "auto_admin",
            "email": "auto_admin@example.com",
            "password": "Passw0rd!"
        }))
        .send()
        .await
        .expect("Failed to create admin");
    assert_eq!(resp.status(), StatusCode::CREATED);
    let _session_cookie = resp
        .cookies()
        .find(|c| c.name() == SESSION_COOKIE_NAME)
        .expect("Session cookie not set after admin creation");

    assert_auto_adoption_step(&pool, AutoAdoptionWizardStep::UrlSettings).await;

    let user = defguard_common::db::models::User::find_by_username(&pool, "auto_admin")
        .await
        .expect("DB query failed")
        .expect("Admin user not found in DB");
    assert_eq!(user.email, "auto_admin@example.com");

    let resp = client
        .post("/api/v1/initial_setup/auto_wizard/url_settings")
        .json(&json!({
            "defguard_url": "https://auto.example.com",
            "public_proxy_url": "https://proxy.auto.example.com"
        }))
        .send()
        .await
        .expect("Failed to set URL settings");
    assert_eq!(resp.status(), StatusCode::CREATED);

    assert_auto_adoption_step(&pool, AutoAdoptionWizardStep::VpnSettings).await;

    let settings = Settings::get_current_settings();
    assert_eq!(settings.defguard_url, "https://auto.example.com");
    assert_eq!(settings.public_proxy_url, "https://proxy.auto.example.com");

    let resp = client
        .post("/api/v1/initial_setup/auto_wizard/vpn_settings")
        .json(&json!({
            "vpn_public_ip": "5.5.5.5",
            "vpn_wireguard_port": 51820,
            "vpn_gateway_address": "10.10.0.1/24",
            "vpn_allowed_ips": "0.0.0.0/0",
            "vpn_dns_server_ip": "8.8.8.8"
        }))
        .send()
        .await
        .expect("Failed to set VPN settings");
    assert_eq!(resp.status(), StatusCode::CREATED);

    assert_auto_adoption_step(&pool, AutoAdoptionWizardStep::MfaSettings).await;

    let updated_network = WireguardNetwork::find_by_id(&pool, network.id)
        .await
        .expect("DB query failed")
        .expect("Network not found after VPN settings update");
    assert_eq!(updated_network.endpoint, "5.5.5.5");
    assert_eq!(updated_network.port, 51820);
    assert_eq!(updated_network.dns, Some("8.8.8.8".to_string()));

    let resp = client
        .post("/api/v1/initial_setup/auto_wizard/mfa_settings")
        .json(&json!({ "vpn_mfa_mode": "disabled" }))
        .send()
        .await
        .expect("Failed to set MFA settings");
    assert_eq!(resp.status(), StatusCode::CREATED);

    assert_auto_adoption_step(&pool, AutoAdoptionWizardStep::Summary).await;

    let updated_network = WireguardNetwork::find_by_id(&pool, network.id)
        .await
        .expect("DB query failed")
        .expect("Network not found after MFA settings update");
    assert_eq!(updated_network.location_mfa_mode, LocationMfaMode::Disabled);

    let resp = client
        .get("/api/v1/initial_setup/auto_adoption")
        .send()
        .await
        .expect("Failed to get auto adoption result");
    assert_eq!(resp.status(), StatusCode::OK);
    let result: serde_json::Value = resp
        .json()
        .await
        .expect("Failed to parse auto adoption result");
    assert_eq!(result["step"], "summary");

    let resp = client
        .post("/api/v1/initial_setup/finish")
        .send()
        .await
        .expect("Failed to finish setup");
    assert_eq!(resp.status(), StatusCode::OK);

    let wizard = Wizard::get(&pool).await.expect("Failed to get wizard");
    assert!(wizard.completed);
    assert_eq!(wizard.active_wizard, ActiveWizard::None);

    let shutdown_signal =
        tokio::time::timeout(std::time::Duration::from_secs(1), shutdown_rx).await;
    assert!(
        matches!(shutdown_signal, Ok(Ok(()))),
        "Setup server should have sent shutdown signal after finish"
    );
}

#[sqlx::test]
async fn test_auto_adoption_auth_enforcement(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool)
        .await
        .expect("Failed to initialize settings");
    seed_wireguard_network(&pool).await;
    Wizard::init(&pool, true)
        .await
        .expect("Failed to init wizard");

    // Use a fresh client (no cookie jar state) to simulate unauthenticated access
    let unauthenticated_client = {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static("test/0.0"));
        Client::builder()
            .default_headers(headers)
            .build()
            .expect("Failed to build unauthenticated reqwest client")
    };

    let (client_with_session, _shutdown_rx) = make_setup_test_client(pool.clone()).await;
    let base_url = client_with_session.base_url();

    let resp = unauthenticated_client
        .post(format!("{base_url}/api/v1/initial_setup/admin"))
        .json(&json!({
            "first_name": "Admin",
            "last_name": "Admin",
            "username": "auth_test_admin",
            "email": "auth_test@example.com",
            "password": "Passw0rd!"
        }))
        .header(USER_AGENT, "test/0.0")
        .send()
        .await
        .expect("Failed to POST admin");
    assert_eq!(
        resp.status(),
        StatusCode::CREATED,
        "Admin creation should be allowed without auth at Welcome step"
    );

    let resp = unauthenticated_client
        .post(format!(
            "{base_url}/api/v1/initial_setup/auto_wizard/url_settings"
        ))
        .json(&json!({
            "defguard_url": "https://example.com",
            "public_proxy_url": "https://proxy.example.com"
        }))
        .header(USER_AGENT, "test/0.0")
        .send()
        .await
        .expect("Failed to POST url_settings");
    assert_eq!(
        resp.status(),
        StatusCode::UNAUTHORIZED,
        "url_settings should require auth after admin has been created"
    );

    // vpn_settings also blocked
    let resp = unauthenticated_client
        .post(format!(
            "{base_url}/api/v1/initial_setup/auto_wizard/vpn_settings"
        ))
        .json(&json!({
            "vpn_public_ip": "1.2.3.4",
            "vpn_wireguard_port": 51820,
            "vpn_gateway_address": "10.0.0.1/24",
            "vpn_allowed_ips": "",
            "vpn_dns_server_ip": ""
        }))
        .header(USER_AGENT, "test/0.0")
        .send()
        .await
        .expect("Failed to POST vpn_settings");
    assert_eq!(
        resp.status(),
        StatusCode::UNAUTHORIZED,
        "vpn_settings should require auth"
    );

    // mfa_settings also blocked
    let resp = unauthenticated_client
        .post(format!(
            "{base_url}/api/v1/initial_setup/auto_wizard/mfa_settings"
        ))
        .json(&json!({ "vpn_mfa_mode": "disabled" }))
        .header(USER_AGENT, "test/0.0")
        .send()
        .await
        .expect("Failed to POST mfa_settings");
    assert_eq!(
        resp.status(),
        StatusCode::UNAUTHORIZED,
        "mfa_settings should require auth"
    );

    let resp = client_with_session
        .post("/api/v1/initial_setup/login")
        .json(&json!({
            "username": "auth_test_admin",
            "password": "Passw0rd!"
        }))
        .send()
        .await
        .expect("Failed to login");
    assert_eq!(resp.status(), StatusCode::OK);

    let resp = client_with_session
        .post("/api/v1/initial_setup/auto_wizard/url_settings")
        .json(&json!({
            "defguard_url": "https://example.com",
            "public_proxy_url": "https://proxy.example.com"
        }))
        .send()
        .await
        .expect("Failed to set URL settings after login");
    assert_eq!(
        resp.status(),
        StatusCode::CREATED,
        "url_settings should succeed after login"
    );
}

#[sqlx::test]
async fn test_auto_adoption_vpn_settings_missing_network(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool)
        .await
        .expect("Failed to initialize settings");

    Wizard::init(&pool, true)
        .await
        .expect("Failed to init wizard");

    let (client, _shutdown_rx) = make_setup_test_client(pool.clone()).await;

    // Create admin (no auth required yet)
    let resp = client
        .post("/api/v1/initial_setup/admin")
        .json(&json!({
            "first_name": "Admin",
            "last_name": "Admin",
            "username": "no_net_admin",
            "email": "no_net@example.com",
            "password": "Passw0rd!"
        }))
        .send()
        .await
        .expect("Failed to create admin");
    assert_eq!(resp.status(), StatusCode::CREATED);

    // Set URL settings (requires auth — cookie jar carries session)
    let resp = client
        .post("/api/v1/initial_setup/auto_wizard/url_settings")
        .json(&json!({
            "defguard_url": "https://example.com",
            "public_proxy_url": "https://proxy.example.com"
        }))
        .send()
        .await
        .expect("Failed to set URL settings");
    assert_eq!(resp.status(), StatusCode::CREATED);

    // VPN settings must fail because no network exists
    let resp = client
        .post("/api/v1/initial_setup/auto_wizard/vpn_settings")
        .json(&json!({
            "vpn_public_ip": "1.2.3.4",
            "vpn_wireguard_port": 51820,
            "vpn_gateway_address": "10.0.0.1/24",
            "vpn_allowed_ips": "",
            "vpn_dns_server_ip": ""
        }))
        .send()
        .await
        .expect("Failed to POST vpn_settings");
    assert_eq!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "Should return 404 when no network exists to configure"
    );

    // Step must NOT have advanced past VpnSettings
    assert_auto_adoption_step(&pool, AutoAdoptionWizardStep::VpnSettings).await;
}

fn config_with_flags(adopt_edge: Option<&str>, adopt_gateway: Option<&str>) -> DefGuardConfig {
    let mut config = DefGuardConfig::new_test_config();
    config.adopt_edge = adopt_edge.map(str::to_string);
    config.adopt_gateway = adopt_gateway.map(str::to_string);
    config
}

/// attempt_auto_adoption must fail immediately when fewer than both flags are set.
#[sqlx::test]
async fn test_attempt_auto_adoption_requires_both_flags(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = defguard_common::db::setup_pool(options).await;
    initialize_current_settings(&pool)
        .await
        .expect("Failed to initialize settings");

    // only adopt_edge
    assert!(
        attempt_auto_adoption(
            &pool,
            &config_with_flags(Some("edge.example.com:8080"), None)
        )
        .await
        .is_err()
    );

    // only adopt_gateway
    assert!(
        attempt_auto_adoption(&pool, &config_with_flags(None, Some("gw.example.com:8080")))
            .await
            .is_err()
    );

    // neither flag
    assert!(
        attempt_auto_adoption(&pool, &config_with_flags(None, None))
            .await
            .is_err()
    );
}
