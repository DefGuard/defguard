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
            settings::{initialize_current_settings, update_current_settings},
            wizard::{ActiveWizard, Wizard},
        },
        setup_pool,
    },
};
use defguard_setup::{migration::build_migration_webapp, setup_server::build_setup_webapp};
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
    fn new(app: axum::Router, listener: TcpListener) -> Self {
        let port = listener.local_addr().unwrap().port();
        let task = tokio::spawn(async move {
            serve(
                listener,
                app.into_make_service_with_connect_info::<SocketAddr>(),
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
}

async fn make_setup_test_client(pool: sqlx::PgPool) -> (TestClient, oneshot::Receiver<()>) {
    let (setup_shutdown_tx, setup_shutdown_rx) = oneshot::channel::<()>();
    let app = build_setup_webapp(
        pool,
        Version::parse(VERSION).expect("Invalid version"),
        setup_shutdown_tx,
    );
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);
    let listener = TcpListener::bind(addr)
        .await
        .expect("Could not bind ephemeral socket");
    (TestClient::new(app, listener), setup_shutdown_rx)
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
    let router = webapp.router.clone();
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);
    let listener = TcpListener::bind(addr)
        .await
        .expect("Could not bind ephemeral socket");
    (TestClient::new(router, listener), setup_shutdown_rx, webapp)
}

/// Initialise settings with a known secret key so build_migration_webapp can
/// call `secret_key_required()` without panicking. Also initialises SERVER_CONFIG
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
        .expect("Failed to update settings");

    let mut config = DefGuardConfig::new_test_config();
    config.cookie_insecure = true;
    config.initialize_post_settings();
    let _ = SERVER_CONFIG.set(config);
}

#[sqlx::test]
async fn test_session_info_setup_server(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool)
        .await
        .expect("Failed to initialize settings");
    Wizard::init(&pool, false)
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
    // has_auto_adopt_flags = true: AutoAdoption wizard
    Wizard::init(&pool, true)
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

    Wizard::init(&pool, false)
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
