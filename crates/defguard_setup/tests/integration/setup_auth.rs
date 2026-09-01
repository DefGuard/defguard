use defguard_common::{
    config::DefGuardConfig,
    db::{
        models::{
            settings::initialize_current_settings,
            setup_auto_adoption::{AutoAdoptionWizardState, AutoAdoptionWizardStep},
            wizard::Wizard,
        },
        setup_pool,
    },
};
use reqwest::StatusCode;
use serde_json::json;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

use super::common::make_setup_test_client;

#[sqlx::test]
async fn dg2608_16_test_acme_stream_rejects_anonymous_before_admin_exists(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool).await.unwrap();
    Wizard::init(&pool, true, &DefGuardConfig::new_test_config())
        .await
        .expect("Failed to init wizard");

    // The wizard is at the Welcome step and no admin exists yet.
    let (client, _shutdown_rx) = make_setup_test_client(pool.clone()).await;
    let resp = client
        .post("/api/v1/proxy/acme/stream")
        .json(&json!({}))
        .send()
        .await
        .expect("Request failed");
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn dg2608_16_test_wizard_settings_reject_anonymous_before_admin_exists(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool).await.unwrap();
    Wizard::init(&pool, true, &DefGuardConfig::new_test_config())
        .await
        .expect("Failed to init wizard");

    let (client, _shutdown_rx) = make_setup_test_client(pool.clone()).await;
    let resp = client
        .post("/api/v1/initial_setup/auto_wizard/external_url_settings")
        .json(&json!({
            "public_proxy_url": "https://proxy.example.com",
            "ssl_type": "none"
        }))
        .send()
        .await
        .expect("Request failed");
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // The anonymous request must not advance the wizard step either.
    let state = AutoAdoptionWizardState::get(&pool)
        .await
        .unwrap()
        .unwrap_or_default();
    assert_eq!(state.step, AutoAdoptionWizardStep::Welcome);
}
