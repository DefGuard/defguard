pub(crate) mod client;

use std::sync::{Arc, Mutex};

use defguard::{
    auth::failed_login::FailedLoginMap,
    build_webapp,
    config::DefGuardConfig,
    db::{init_db, AppEvent, GatewayEvent, Id, User, UserDetails},
    enterprise::license::{set_cached_license, License},
    grpc::{GatewayMap, WorkerState},
    handlers::Auth,
    headers::create_user_agent_parser,
    mail::Mail,
    SERVER_CONFIG,
};
use reqwest::{header::HeaderName, StatusCode};
use secrecy::ExposeSecret;
use serde_json::json;
use sqlx::{postgres::PgConnectOptions, query, types::Uuid, PgPool};
use tokio::sync::{
    broadcast::{self, Receiver},
    mpsc::{unbounded_channel, UnboundedReceiver},
};

use self::client::TestClient;

#[allow(dead_code, clippy::declare_interior_mutable_const)]
pub const X_FORWARDED_HOST: HeaderName = HeaderName::from_static("x-forwarded-host");
#[allow(dead_code, clippy::declare_interior_mutable_const)]
pub const X_FORWARDED_FOR: HeaderName = HeaderName::from_static("x-forwarded-for");
#[allow(dead_code, clippy::declare_interior_mutable_const)]
pub const X_FORWARDED_URI: HeaderName = HeaderName::from_static("x-forwarded-uri");

pub async fn init_test_db() -> (PgPool, DefGuardConfig) {
    let config = DefGuardConfig::new_test_config();
    let _ = SERVER_CONFIG.set(config.clone());
    let opts = PgConnectOptions::new()
        .host(&config.database_host)
        .port(config.database_port)
        .username(&config.database_user)
        .password(config.database_password.expose_secret())
        .database(&config.database_name);
    let pool = PgPool::connect_with(opts)
        .await
        .expect("Failed to connect to Postgres");
    let db_name = Uuid::new_v4().to_string();
    query(&format!("CREATE DATABASE \"{db_name}\""))
        .execute(&pool)
        .await
        .expect("Failed to create test database");
    let pool = init_db(
        &config.database_host,
        config.database_port,
        &db_name,
        &config.database_user,
        config.database_password.expose_secret(),
    )
    .await;

    initialize_users(&pool, &config).await;

    (pool, config)
}

async fn initialize_users(pool: &PgPool, config: &DefGuardConfig) {
    User::init_admin_user(pool, config.default_admin_password.expose_secret())
        .await
        .unwrap();

    User::new(
        "hpotter",
        Some("pass123"),
        "Potter",
        "Harry",
        "h.potter@hogwart.edu.uk",
        None,
    )
    .save(pool)
    .await
    .unwrap();
}

pub struct ClientState {
    pub pool: PgPool,
    pub worker_state: Arc<Mutex<WorkerState>>,
    pub wireguard_rx: Receiver<GatewayEvent>,
    pub mail_rx: UnboundedReceiver<Mail>,
    pub failed_logins: Arc<Mutex<FailedLoginMap>>,
    pub test_user: User<Id>,
    pub config: DefGuardConfig,
}

impl ClientState {
    pub fn new(
        pool: PgPool,
        worker_state: Arc<Mutex<WorkerState>>,
        wireguard_rx: Receiver<GatewayEvent>,
        mail_rx: UnboundedReceiver<Mail>,
        failed_logins: Arc<Mutex<FailedLoginMap>>,
        test_user: User<Id>,
        config: DefGuardConfig,
    ) -> Self {
        Self {
            pool,
            worker_state,
            wireguard_rx,
            mail_rx,
            failed_logins,
            test_user,
            config,
        }
    }
}

pub async fn make_base_client(pool: PgPool, config: DefGuardConfig) -> (TestClient, ClientState) {
    let (tx, rx) = unbounded_channel::<AppEvent>();
    let worker_state = Arc::new(Mutex::new(WorkerState::new(tx.clone())));
    let (wg_tx, wg_rx) = broadcast::channel::<GatewayEvent>(16);
    let (mail_tx, mail_rx) = unbounded_channel::<Mail>();
    let gateway_state = Arc::new(Mutex::new(GatewayMap::new()));

    let failed_logins = FailedLoginMap::new();
    let failed_logins = Arc::new(Mutex::new(failed_logins));

    let license = License::new(
        "test_customer".to_string(),
        true,
        // Some(Utc.with_ymd_and_hms(2030, 1, 1, 0, 0, 0).unwrap()),
        // Permanent license
        None,
    );

    set_cached_license(Some(license));

    let client_state = ClientState::new(
        pool.clone(),
        worker_state.clone(),
        wg_rx,
        mail_rx,
        failed_logins.clone(),
        User::find_by_username(&pool, "hpotter")
            .await
            .unwrap()
            .unwrap(),
        config.clone(),
    );

    // Uncomment this to enable tracing in tests.
    // It only works for running a single test, so leave it commented out for running all tests.
    // use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
    // tracing_subscriber::registry()
    //     .with(
    //         tracing_subscriber::EnvFilter::try_from_default_env()
    //             .unwrap_or_else(|_| "defguard=debug,tower_http=debug,axum::rejection=trace".into()),
    //     )
    //     .with(tracing_subscriber::fmt::layer())
    //     .init();

    let webapp = build_webapp(
        tx,
        rx,
        wg_tx,
        mail_tx,
        worker_state,
        gateway_state,
        pool,
        failed_logins,
    );

    (TestClient::new(webapp).await, client_state)
}

#[allow(dead_code)]
pub async fn make_test_client() -> (TestClient, ClientState) {
    let (pool, config) = init_test_db().await;
    make_base_client(pool, config).await
}

#[allow(dead_code)]
pub async fn fetch_user_details(client: &TestClient, username: &str) -> UserDetails {
    let response = client.get(format!("/api/v1/user/{username}")).send().await;
    assert_eq!(response.status(), StatusCode::OK);
    response.json().await
}

pub async fn exceed_enterprise_limits(client: &TestClient) {
    let auth = Auth::new("admin", "pass123");
    client.post("/api/v1/auth").json(&auth).send().await;
    client
        .post("/api/v1/network")
        .json(&json!({
            "name": "network1",
            "address": "10.1.1.1/24",
            "port": 55555,
            "endpoint": "192.168.4.14",
            "allowed_ips": "10.1.1.0/24",
            "dns": "1.1.1.1",
            "allowed_groups": [],
            "mfa_enabled": false,
            "keepalive_interval": 25,
            "peer_disconnect_threshold": 180
        }))
        .send()
        .await;

    client
        .post("/api/v1/network")
        .json(&json!({
            "name": "network2",
            "address": "10.1.1.1/24",
            "port": 55555,
            "endpoint": "192.168.4.14",
            "allowed_ips": "10.1.1.0/24",
            "dns": "1.1.1.1",
            "allowed_groups": [],
            "mfa_enabled": false,
            "keepalive_interval": 25,
            "peer_disconnect_threshold": 180
        }))
        .send()
        .await;
}
