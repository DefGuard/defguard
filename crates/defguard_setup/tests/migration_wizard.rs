use defguard_common::{
    config::DefGuardConfig,
    db::{
        models::{
            Certificates, Settings,
            migration_wizard::MigrationWizardState,
            wizard::{ActiveWizard, Wizard},
        },
        setup_pool,
    },
};
use reqwest::{
    Client, StatusCode,
    header::{HeaderMap, HeaderValue, USER_AGENT},
};
use serde_json::json;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

mod common;
use common::{init_settings_with_secret_key, make_migration_test_client, seed_admin_user};

async fn assert_migration_step(pool: &sqlx::PgPool, expected_variant: &str) {
    let state = MigrationWizardState::get(pool)
        .await
        .expect("Failed to fetch migration wizard state")
        .unwrap_or_default();
    let serialized =
        serde_json::to_value(&state.current_step).expect("Failed to serialize migration step");
    assert_eq!(
        serialized,
        serde_json::Value::String(expected_variant.to_string()),
        "Expected migration step '{expected_variant}', got {serialized}"
    );
}

#[sqlx::test]
async fn test_migration_full_flow(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    init_settings_with_secret_key(&pool).await;

    seed_admin_user(&pool, "migration_admin", "Passw0rd!").await;

    Wizard::init(&pool, false, &DefGuardConfig::new_test_config())
        .await
        .expect("Failed to init wizard");

    let wizard = Wizard::get(&pool).await.expect("Failed to get wizard");
    assert_eq!(wizard.active_wizard, ActiveWizard::Migration);

    let (client, shutdown_rx, _webapp) = make_migration_test_client(pool.clone()).await;

    let resp = client
        .get("/api/v1/session-info")
        .send()
        .await
        .expect("Failed to get session-info");
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = resp.json().await.expect("Failed to parse session-info");
    assert_eq!(body["active_wizard"], "migration");
    assert_eq!(body["authorized"], false);

    let resp = client
        .post("/api/v1/auth")
        .json(&json!({
            "username": "migration_admin",
            "password": "Passw0rd!"
        }))
        .send()
        .await
        .expect("Failed to authenticate");
    assert_eq!(resp.status(), StatusCode::OK);

    let resp = client
        .get("/api/v1/migration/state")
        .send()
        .await
        .expect("Failed to GET /api/v1/migration/state");
    assert_eq!(resp.status(), StatusCode::OK);
    let state: serde_json::Value = resp.json().await.expect("Failed to parse migration state");
    assert_eq!(
        state["current_step"], "welcome",
        "Initial migration step should be 'welcome'"
    );
    assert!(
        state["location_state"].is_null(),
        "location_state should be null initially"
    );

    let resp = client
        .put("/api/v1/migration/state")
        .json(&json!({
            "current_step": "general",
            "location_state": null
        }))
        .send()
        .await
        .expect("Failed to PUT /api/v1/migration/state");
    assert_eq!(resp.status(), StatusCode::OK);

    assert_migration_step(&pool, "general").await;

    let resp = client
        .client
        .patch(format!("{}/api/v1/settings", client.base_url()))
        .json(&json!({
            "defguard_url": "https://migration.example.com",
            "authentication_period_days": 14,
            "mfa_code_timeout_seconds": 120
        }))
        .send()
        .await
        .expect("Failed to PATCH /api/v1/settings");
    assert_eq!(resp.status(), StatusCode::OK);

    let settings = Settings::get(&pool)
        .await
        .expect("Failed to fetch settings")
        .expect("Settings not found");
    assert_eq!(settings.defguard_url, "https://migration.example.com");
    assert_eq!(settings.authentication_period_days, 14);
    assert_eq!(settings.mfa_code_timeout_seconds, 120);

    let resp = client
        .post("/api/v1/migration/ca")
        .json(&json!({
            "common_name": "Migration CA",
            "email": "ca@migration.example.com",
            "validity_period_years": 1
        }))
        .send()
        .await
        .expect("Failed to POST /api/v1/migration/ca");
    assert_eq!(resp.status(), StatusCode::CREATED);

    let certs = Certificates::get_or_default(&pool)
        .await
        .expect("Failed to fetch certificates");
    assert!(certs.ca_cert_der.is_some(), "CA cert should be set");
    assert!(certs.ca_key_der.is_some(), "CA key should be set");
    assert!(certs.ca_expiry.is_some(), "CA expiry should be set");

    let resp = client
        .put("/api/v1/migration/state")
        .json(&json!({
            "current_step": "confirmation",
            "location_state": null
        }))
        .send()
        .await
        .expect("Failed to PUT migration state to confirmation");
    assert_eq!(resp.status(), StatusCode::OK);
    assert_migration_step(&pool, "confirmation").await;

    let resp = client
        .post("/api/v1/migration/finish")
        .send()
        .await
        .expect("Failed to POST /api/v1/migration/finish");
    assert_eq!(resp.status(), StatusCode::OK);

    let wizard = Wizard::get(&pool).await.expect("Failed to get wizard");
    assert!(wizard.completed, "Wizard should be completed after finish");
    assert_eq!(wizard.active_wizard, ActiveWizard::None);

    let migration_state = MigrationWizardState::get(&pool)
        .await
        .expect("Failed to get migration state");
    assert!(
        migration_state.is_none(),
        "Migration wizard state should be cleared after finish"
    );

    let shutdown_signal =
        tokio::time::timeout(std::time::Duration::from_secs(1), shutdown_rx).await;
    assert!(
        matches!(shutdown_signal, Ok(Ok(()))),
        "Migration server should have sent shutdown signal after finish"
    );
}

#[sqlx::test]
async fn test_migration_auth_enforcement(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    init_settings_with_secret_key(&pool).await;

    seed_admin_user(&pool, "auth_migration_admin", "Passw0rd!").await;
    Wizard::init(&pool, false, &DefGuardConfig::new_test_config())
        .await
        .expect("Failed to init wizard");

    let (client, _shutdown_rx, _webapp) = make_migration_test_client(pool.clone()).await;

    let unauth = {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static("test/0.0"));
        Client::builder()
            .default_headers(headers)
            .build()
            .expect("Failed to build unauthenticated client")
    };
    let base = client.base_url();

    let resp = unauth
        .get(format!("{base}/api/v1/migration/state"))
        .header(USER_AGENT, "test/0.0")
        .send()
        .await
        .expect("Failed GET migration/state");
    assert_eq!(
        resp.status(),
        StatusCode::UNAUTHORIZED,
        "GET /migration/state should require auth"
    );

    let resp = unauth
        .put(format!("{base}/api/v1/migration/state"))
        .header(USER_AGENT, "test/0.0")
        .json(&json!({"current_step": "general", "location_state": null}))
        .send()
        .await
        .expect("Failed PUT migration/state");
    assert_eq!(
        resp.status(),
        StatusCode::UNAUTHORIZED,
        "PUT /migration/state should require auth"
    );

    let resp = unauth
        .patch(format!("{base}/api/v1/settings"))
        .header(USER_AGENT, "test/0.0")
        .json(&json!({
            "defguard_url": "https://x.example.com",
            "authentication_period_days": 14,
            "mfa_code_timeout_seconds": 120
        }))
        .send()
        .await
        .expect("Failed PATCH settings");
    assert_eq!(
        resp.status(),
        StatusCode::UNAUTHORIZED,
        "PATCH /settings should require auth"
    );

    let resp = unauth
        .post(format!("{base}/api/v1/migration/finish"))
        .header(USER_AGENT, "test/0.0")
        .send()
        .await
        .expect("Failed POST migration/finish");
    assert_eq!(
        resp.status(),
        StatusCode::UNAUTHORIZED,
        "POST /migration/finish should require auth"
    );

    let resp = client
        .post("/api/v1/auth")
        .json(&json!({
            "username": "auth_migration_admin",
            "password": "Passw0rd!"
        }))
        .send()
        .await
        .expect("Failed to authenticate");
    assert_eq!(resp.status(), StatusCode::OK);

    let resp = client
        .get("/api/v1/migration/state")
        .send()
        .await
        .expect("Failed GET migration/state after login");
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "GET /migration/state should succeed after login"
    );

    let resp = client
        .put("/api/v1/migration/state")
        .json(&json!({"current_step": "general", "location_state": null}))
        .send()
        .await
        .expect("Failed PUT migration/state after login");
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "PUT /migration/state should succeed after login"
    );
}
