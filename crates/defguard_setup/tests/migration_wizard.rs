use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
};

use axum::serve;
use defguard_common::{
    VERSION,
    config::{DefGuardConfig, SERVER_CONFIG},
    db::{
        models::{
            Settings, User,
            group::Group,
            migration_wizard::MigrationWizardState,
            settings::{initialize_current_settings, update_current_settings},
            wizard::{ActiveWizard, Wizard},
        },
        setup_pool,
    },
};
use defguard_setup::migration::build_migration_webapp;
use reqwest::{
    Client, StatusCode,
    cookie::Jar,
    header::{HeaderMap, HeaderValue, USER_AGENT},
};
use semver::Version;
use serde_json::json;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use tokio::{net::TcpListener, sync::oneshot, task::JoinHandle};

const TEST_SECRET_KEY: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

struct TestClient {
    client: Client,
    _jar: Arc<Jar>,
    port: u16,
    _task: JoinHandle<()>,
}

impl TestClient {
    fn new(router: axum::Router, listener: TcpListener) -> Self {
        let port = listener.local_addr().unwrap().port();
        let task = tokio::spawn(async move {
            serve(
                listener,
                router.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .await
            .expect("server error");
        });

        let jar = Arc::new(Jar::default());
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static("test/0.0"));
        let client = Client::builder()
            .default_headers(headers)
            .cookie_provider(jar.clone())
            .build()
            .expect("Failed to build reqwest client");

        Self {
            client,
            _jar: jar,
            port,
            _task: task,
        }
    }

    fn base_url(&self) -> String {
        format!("http://localhost:{}", self.port)
    }

    fn get(&self, path: &str) -> reqwest::RequestBuilder {
        self.client.get(format!("{}{}", self.base_url(), path))
    }

    fn post(&self, path: &str) -> reqwest::RequestBuilder {
        self.client.post(format!("{}{}", self.base_url(), path))
    }

    fn put(&self, path: &str) -> reqwest::RequestBuilder {
        self.client.put(format!("{}{}", self.base_url(), path))
    }
}

/// Initialise settings with a known secret key + URL so `build_migration_webapp`
/// can call `secret_key_required()` without panicking. Also initialises SERVER_CONFIG
/// so the auth handler can call `server_config()`.
async fn init_settings_with_secret_key(pool: &sqlx::PgPool) {
    initialize_current_settings(pool)
        .await
        .expect("Failed to initialize settings");
    let mut settings = Settings::get_current_settings();
    settings.secret_key = Some(TEST_SECRET_KEY.to_string());
    settings.defguard_url = "http://localhost:8000".to_string();
    settings.webauthn_rp_id = Some("localhost".to_string());
    update_current_settings(pool, settings)
        .await
        .expect("Failed to update settings with secret key");

    let mut config = DefGuardConfig::new_test_config();
    config.cookie_insecure = true;
    config.initialize_post_settings();
    let _ = SERVER_CONFIG.set(config);
}

/// Creates an admin group + admin user and returns the user.
/// `User::is_admin()` checks group membership, not a column flag.
async fn seed_admin_user(
    pool: &sqlx::PgPool,
    username: &str,
    password: &str,
) -> User<defguard_common::db::Id> {
    let mut admin_group = Group::new("admins");
    admin_group.is_admin = true;
    let admin_group = admin_group
        .save(pool)
        .await
        .expect("Failed to save admin group");

    let mut user = User::new(
        username,
        Some(password),
        "Admin",
        "Migration",
        &format!("{username}@example.com"),
        None,
    )
    .save(pool)
    .await
    .expect("Failed to save admin user");

    user.add_to_group(pool, &admin_group)
        .await
        .expect("Failed to add user to admin group");

    user
}

async fn make_migration_test_client(
    pool: sqlx::PgPool,
) -> (
    TestClient,
    oneshot::Receiver<()>,
    defguard_setup::migration::MigrationWebapp,
) {
    let (setup_shutdown_tx, setup_shutdown_rx) = oneshot::channel::<()>();
    let webapp = build_migration_webapp(
        pool,
        Version::parse(VERSION).expect("Invalid version"),
        setup_shutdown_tx,
    );
    // We must keep `webapp` alive to prevent its event receiver channels from
    // being dropped — if they are dropped the `emit_event` call in the auth
    // handler will fail with "channel closed".
    let router = webapp.router.clone();
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);
    let listener = TcpListener::bind(addr)
        .await
        .expect("Could not bind ephemeral socket");
    (TestClient::new(router, listener), setup_shutdown_rx, webapp)
}

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

    Wizard::init(&pool, false)
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
        .post("/api/v1/migration/general_config")
        .json(&json!({
            "defguard_url": "https://migration.example.com",
            "default_admin_group_name": "admins",
            "default_authentication": 14,
            "default_mfa_code_lifetime": 120,
            "public_proxy_url": "https://proxy.migration.example.com"
        }))
        .send()
        .await
        .expect("Failed to POST /api/v1/migration/general_config");
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

    let settings = Settings::get(&pool)
        .await
        .expect("Failed to fetch settings")
        .expect("Settings not found");
    assert!(settings.ca_cert_der.is_some(), "CA cert should be set");
    assert!(settings.ca_key_der.is_some(), "CA key should be set");
    assert!(settings.ca_expiry.is_some(), "CA expiry should be set");

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
    Wizard::init(&pool, false)
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
        .post(format!("{base}/api/v1/migration/general_config"))
        .header(USER_AGENT, "test/0.0")
        .json(&json!({
            "defguard_url": "https://x.example.com",
            "default_admin_group_name": "admins",
            "default_authentication": 14,
            "default_mfa_code_lifetime": 120,
            "public_proxy_url": "https://px.example.com"
        }))
        .send()
        .await
        .expect("Failed POST migration/general_config");
    assert_eq!(
        resp.status(),
        StatusCode::UNAUTHORIZED,
        "POST /migration/general_config should require auth"
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
