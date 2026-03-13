use defguard_common::db::{
    models::{
        settings::initialize_current_settings,
        setup_auto_adoption::AutoAdoptionWizardStep,
        wireguard::{LocationMfaMode, ServiceLocationMode, WireguardNetwork},
        wizard::{ActiveWizard, Wizard},
    },
    setup_pool,
};
use reqwest::StatusCode;
use serde_json::json;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

mod common;
use common::make_setup_test_client;

#[sqlx::test]
async fn test_wizard_state_initial(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool)
        .await
        .expect("Failed to initialize settings");
    Wizard::init(&pool, false)
        .await
        .expect("Failed to init wizard");

    let (client, _shutdown_rx) = make_setup_test_client(pool.clone()).await;

    let resp = client
        .get("/api/v1/wizard")
        .send()
        .await
        .expect("Failed to GET /api/v1/wizard");
    assert_eq!(resp.status(), StatusCode::OK);
    let state: serde_json::Value = resp.json().await.expect("Failed to parse wizard state");

    assert_eq!(state["active_wizard"], "initial");
    assert_eq!(state["completed"], false);
    assert_eq!(
        state["initial_setup_state"]["step"], "welcome",
        "Initial step should be 'welcome' before any action"
    );
    assert!(
        state["auto_adoption_state"].is_null(),
        "auto_adoption_state should be null for Initial wizard"
    );

    let resp = client
        .post("/api/v1/initial_setup/admin")
        .json(&json!({
            "first_name": "Admin",
            "last_name": "Admin",
            "username": "admin1",
            "email": "admin1@example.com",
            "password": "Passw0rd!"
        }))
        .send()
        .await
        .expect("Failed to create admin");
    assert_eq!(resp.status(), StatusCode::CREATED);

    let state: serde_json::Value = client
        .get("/api/v1/wizard")
        .send()
        .await
        .expect("Failed to GET /api/v1/wizard")
        .json()
        .await
        .expect("Failed to parse wizard state");
    assert_eq!(
        state["initial_setup_state"]["step"],
        "general_configuration"
    );

    let resp = client
        .post("/api/v1/initial_setup/general_config")
        .json(&json!({
            "defguard_url": "https://example.com",
            "default_admin_group_name": "admins",
            "default_authentication": 14,
            "default_mfa_code_lifetime": 120,
            "public_proxy_url": "https://proxy.example.com",
            "admin_username": "admin1"
        }))
        .send()
        .await
        .expect("Failed to set general config");
    assert_eq!(resp.status(), StatusCode::CREATED);

    let state: serde_json::Value = client
        .get("/api/v1/wizard")
        .send()
        .await
        .expect("Failed to GET /api/v1/wizard")
        .json()
        .await
        .expect("Failed to parse wizard state");
    assert_eq!(state["initial_setup_state"]["step"], "ca");

    let resp = client
        .post("/api/v1/initial_setup/ca")
        .json(&json!({
            "common_name": "Test CA",
            "email": "ca@example.com",
            "validity_period_years": 1
        }))
        .send()
        .await
        .expect("Failed to create CA");
    assert_eq!(resp.status(), StatusCode::CREATED);

    let state: serde_json::Value = client
        .get("/api/v1/wizard")
        .send()
        .await
        .expect("Failed to GET /api/v1/wizard")
        .json()
        .await
        .expect("Failed to parse wizard state");
    assert_eq!(state["initial_setup_state"]["step"], "ca_summary");

    let resp = client
        .post("/api/v1/initial_setup/finish")
        .send()
        .await
        .expect("Failed to finish setup");
    assert_eq!(resp.status(), StatusCode::OK);

    let wizard = Wizard::get(&pool).await.expect("Failed to get wizard");
    assert!(wizard.completed);
    assert_eq!(wizard.active_wizard, ActiveWizard::None);
}

#[sqlx::test]
async fn test_wizard_state_auto_adoption(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool)
        .await
        .expect("Failed to initialize settings");

    WireguardNetwork::new(
        "auto-net".to_string(),
        vec!["10.0.0.0/24".parse().unwrap()],
        51820,
        "1.2.3.4".to_string(),
        None,
        1280,
        0,
        vec!["0.0.0.0/0".parse().unwrap()],
        25,
        180,
        false,
        false,
        LocationMfaMode::Disabled,
        ServiceLocationMode::Disabled,
    )
    .save(&pool)
    .await
    .expect("Failed to seed wireguard network");

    Wizard::init(&pool, true)
        .await
        .expect("Failed to init wizard");

    let (client, _shutdown_rx) = make_setup_test_client(pool.clone()).await;

    let state: serde_json::Value = client
        .get("/api/v1/wizard")
        .send()
        .await
        .expect("Failed to GET /api/v1/wizard")
        .json()
        .await
        .expect("Failed to parse wizard state");

    assert_eq!(state["active_wizard"], "auto_adoption");
    assert_eq!(state["completed"], false);
    assert_eq!(
        state["auto_adoption_state"]["step"], "welcome",
        "Initial auto-adoption step should be 'welcome'"
    );
    assert!(
        state["initial_setup_state"].is_null(),
        "initial_setup_state should be null for AutoAdoption wizard"
    );

    let resp = client
        .post("/api/v1/initial_setup/admin")
        .json(&json!({
            "first_name": "Admin",
            "last_name": "Admin",
            "username": "admin1",
            "email": "admin1@example.com",
            "password": "Passw0rd!"
        }))
        .send()
        .await
        .expect("Failed to create admin");
    assert_eq!(resp.status(), StatusCode::CREATED);

    let state: serde_json::Value = client
        .get("/api/v1/wizard")
        .send()
        .await
        .expect("Failed to GET /api/v1/wizard")
        .json()
        .await
        .expect("Failed to parse wizard state");
    assert_eq!(state["active_wizard"], "auto_adoption");

    let auto_state =
        defguard_common::db::models::setup_auto_adoption::AutoAdoptionWizardState::get(&pool)
            .await
            .expect("Failed to get auto adoption state")
            .unwrap_or_default();
    assert_eq!(auto_state.step, AutoAdoptionWizardStep::UrlSettings);

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

    let auto_state =
        defguard_common::db::models::setup_auto_adoption::AutoAdoptionWizardState::get(&pool)
            .await
            .expect("Failed to get auto adoption state")
            .expect("Auto adoption state should be set");
    assert_eq!(auto_state.step, AutoAdoptionWizardStep::VpnSettings);

    let resp = client
        .post("/api/v1/initial_setup/auto_wizard/vpn_settings")
        .json(&json!({
            "vpn_public_ip": "1.2.3.4",
            "vpn_wireguard_port": 51820,
            "vpn_gateway_address": "10.0.0.1/24",
            "vpn_allowed_ips": "0.0.0.0/0",
            "vpn_dns_server_ip": "8.8.8.8"
        }))
        .send()
        .await
        .expect("Failed to set VPN settings");
    assert_eq!(resp.status(), StatusCode::CREATED);

    let auto_state =
        defguard_common::db::models::setup_auto_adoption::AutoAdoptionWizardState::get(&pool)
            .await
            .expect("Failed to get auto adoption state")
            .expect("Auto adoption state should be set");
    assert_eq!(auto_state.step, AutoAdoptionWizardStep::MfaSettings);

    let resp = client
        .post("/api/v1/initial_setup/auto_wizard/mfa_settings")
        .json(&json!({ "vpn_mfa_mode": "disabled" }))
        .send()
        .await
        .expect("Failed to set MFA settings");
    assert_eq!(resp.status(), StatusCode::CREATED);

    let auto_state =
        defguard_common::db::models::setup_auto_adoption::AutoAdoptionWizardState::get(&pool)
            .await
            .expect("Failed to get auto adoption state")
            .expect("Auto adoption state should be set");
    assert_eq!(auto_state.step, AutoAdoptionWizardStep::Summary);

    let resp = client
        .post("/api/v1/initial_setup/finish")
        .send()
        .await
        .expect("Failed to finish setup");
    assert_eq!(resp.status(), StatusCode::OK);

    let wizard = Wizard::get(&pool).await.expect("Failed to get wizard");
    assert!(wizard.completed);
    assert_eq!(wizard.active_wizard, ActiveWizard::None);
}
