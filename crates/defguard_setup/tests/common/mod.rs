use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
};

use axum::serve;
use defguard_common::{
    VERSION,
    config::{DefGuardConfig, SERVER_CONFIG},
    db::{
        Id,
        models::{
            Settings, User,
            group::Group,
            settings::{initialize_current_settings, update_current_settings},
        },
    },
};
use defguard_setup::{migration::build_migration_webapp, setup_server::build_setup_webapp};
use reqwest::{
    Client,
    cookie::Jar,
    header::{HeaderMap, HeaderValue, USER_AGENT},
};
use semver::Version;
use sqlx::PgPool;
use tokio::{net::TcpListener, sync::oneshot, task::JoinHandle};

#[allow(dead_code)]
pub const TEST_SECRET_KEY: &str =
    "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

pub struct TestClient {
    pub client: Client,
    pub _jar: Arc<Jar>,
    pub port: u16,
    pub _task: JoinHandle<()>,
}

impl TestClient {
    pub fn new(router: axum::Router, listener: TcpListener) -> Self {
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

    pub fn base_url(&self) -> String {
        format!("http://localhost:{}", self.port)
    }

    pub fn get(&self, path: &str) -> reqwest::RequestBuilder {
        self.client.get(format!("{}{}", self.base_url(), path))
    }

    pub fn post(&self, path: &str) -> reqwest::RequestBuilder {
        self.client.post(format!("{}{}", self.base_url(), path))
    }

    #[allow(dead_code)]
    pub fn put(&self, path: &str) -> reqwest::RequestBuilder {
        self.client.put(format!("{}{}", self.base_url(), path))
    }
}

#[allow(dead_code)]
pub async fn make_setup_test_client(pool: PgPool) -> (TestClient, oneshot::Receiver<()>) {
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

#[allow(dead_code)]
pub async fn make_migration_test_client(
    pool: PgPool,
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

/// Initialise settings with a known secret key so `build_migration_webapp` can
/// call `secret_key_required()` without panicking. Also initialises SERVER_CONFIG
/// so the auth handler can call `server_config()`.
#[allow(dead_code)]
pub async fn init_settings_with_secret_key(pool: &PgPool) {
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

/// Creates an admin group + admin user and returns the user.
/// `User::is_admin()` checks group membership, not a column flag.
#[allow(dead_code)]
pub async fn seed_admin_user(pool: &PgPool, username: &str, password: &str) -> User<Id> {
    let mut admin_group = Group::new("admins");
    admin_group.is_admin = true;
    let admin_group = admin_group
        .save(pool)
        .await
        .expect("Failed to save admin group");

    let user = User::new(
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

