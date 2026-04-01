use defguard_common::{
    config::DefGuardConfig,
    db::{
        models::{
            User,
            group::Group,
            settings::initialize_current_settings,
            wizard::{ActiveWizard, Wizard},
        },
        setup_pool,
    },
};
use reqwest::StatusCode;
use serde_json::json;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

mod common;
use common::{init_settings_with_secret_key, make_migration_test_client, make_setup_test_client};

#[sqlx::test]
async fn test_session_info_setup_server(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool)
        .await
        .expect("Failed to initialize settings");
    Wizard::init(&pool, false, &DefGuardConfig::new_test_config())
        .await
        .expect("Failed to initialize wizard");

    let (client, _shutdown_rx) = make_setup_test_client(pool.clone()).await;

    let resp = client
        .get("/api/v1/session-info")
        .send()
        .await
        .expect("Failed to get session-info");
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = resp.json().await.expect("Failed to parse session-info");
    assert_eq!(body["active_wizard"], "initial");
    assert_eq!(body["authorized"], false);
    assert_eq!(body["is_admin"], false);

    let resp = client
        .post("/api/v1/initial_setup/admin")
        .json(&json!({
            "first_name": "Admin",
            "last_name": "Admin",
            "username": "admin1",
            "email": "admin1@example.com",
            "password": "Passw0rd!",
            "automatically_assign_group": true
        }))
        .send()
        .await
        .expect("Failed to create admin");
    assert_eq!(resp.status(), StatusCode::CREATED);

    let resp = client
        .get("/api/v1/session-info")
        .send()
        .await
        .expect("Failed to get session-info after admin creation");
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = resp.json().await.expect("Failed to parse session-info");
    assert_eq!(
        body["active_wizard"], "initial",
        "Wizard should still be 'initial' mid-flow"
    );
    assert_eq!(body["authorized"], true);
    assert_eq!(body["is_admin"], true);

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
async fn test_session_info_auto_adoption_wizard(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool)
        .await
        .expect("Failed to initialize settings");
    // has_auto_adopt_flags = true (both flags provided): AutoAdoption wizard
    Wizard::init(&pool, true, &DefGuardConfig::new_test_config())
        .await
        .expect("Failed to initialize wizard");

    let (client, _shutdown_rx) = make_setup_test_client(pool.clone()).await;

    let resp = client
        .get("/api/v1/session-info")
        .send()
        .await
        .expect("Failed to get session-info");
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = resp.json().await.expect("Failed to parse session-info");
    assert_eq!(
        body["active_wizard"], "auto_adoption",
        "Should report auto_adoption wizard"
    );
    assert_eq!(body["authorized"], false);
}

#[sqlx::test]
async fn test_session_info_migration_server(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    init_settings_with_secret_key(&pool).await;

    let user = User::new(
        "migrating_admin",
        Some("Passw0rd!"),
        "Admin",
        "Migrating",
        "migrating_admin@example.com",
        None,
    )
    .save(&pool)
    .await
    .expect("Failed to save admin user");

    // Make that user an admin via group membership (is_admin is group-based, not a column)
    let mut admin_group = Group::new("admins");
    admin_group.is_admin = true;
    let admin_group = admin_group
        .save(&pool)
        .await
        .expect("Failed to save admin group");
    user.add_to_group(&pool, &admin_group)
        .await
        .expect("Failed to add user to admin group");

    Wizard::init(&pool, false, &DefGuardConfig::new_test_config())
        .await
        .expect("Failed to initialize wizard");

    let (client, _shutdown_rx, _webapp) = make_migration_test_client(pool.clone()).await;

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
            "username": "migrating_admin",
            "password": "Passw0rd!"
        }))
        .send()
        .await
        .expect("Failed to authenticate");
    assert_eq!(resp.status(), StatusCode::OK);

    let resp = client
        .get("/api/v1/session-info")
        .send()
        .await
        .expect("Failed to get session-info after login");
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = resp.json().await.expect("Failed to parse session-info");
    assert_eq!(body["active_wizard"], "migration");
    assert_eq!(body["authorized"], true);
    assert_eq!(body["is_admin"], true);

    let resp = client
        .post("/api/v1/migration/finish")
        .send()
        .await
        .expect("Failed to finish migration");
    assert_eq!(resp.status(), StatusCode::OK);

    let wizard = Wizard::get(&pool).await.expect("Failed to get wizard");
    assert!(wizard.completed);
    assert_eq!(wizard.active_wizard, ActiveWizard::None);
}
