pub(crate) mod client;

use std::sync::{Arc, Mutex};

use defguard::appstate::AppState;
use defguard::{
    auth::failed_login::FailedLoginMap,
    build_webapp,
    config::DefGuardConfig,
    db::{init_db, AppEvent, DbPool, GatewayEvent, User, UserDetails},
    grpc::{GatewayMap, WorkerState},
    headers::create_user_agent_parser,
    mail::Mail,
    SERVER_CONFIG,
};
use reqwest::{header::HeaderName, StatusCode};
use secrecy::ExposeSecret;
use sqlx::{postgres::PgConnectOptions, query, types::Uuid};
use tokio::sync::{
    broadcast::{self, Receiver},
    mpsc::{unbounded_channel, UnboundedReceiver},
};

use self::client::TestClient;

#[allow(dead_code)]
pub const X_FORWARDED_HOST: HeaderName = HeaderName::from_static("x-forwarded-host");
#[allow(dead_code)]
pub const X_FORWARDED_FOR: HeaderName = HeaderName::from_static("x-forwarded-for");
#[allow(dead_code)]
pub const X_FORWARDED_URI: HeaderName = HeaderName::from_static("x-forwarded-uri");

pub async fn init_test_db() -> (DbPool, DefGuardConfig) {
    let config = DefGuardConfig::new_test_config();
    let _ = SERVER_CONFIG.set(config.clone());
    let opts = PgConnectOptions::new()
        .host(&config.database_host)
        .port(config.database_port)
        .username(&config.database_user)
        .password(config.database_password.expose_secret())
        .database(&config.database_name);
    let pool = DbPool::connect_with(opts)
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

    initialize_users(&pool, config.clone()).await;

    (pool, config)
}

async fn initialize_users(pool: &DbPool, config: DefGuardConfig) {
    User::init_admin_user(pool, config.default_admin_password.expose_secret())
        .await
        .unwrap();

    let mut test_user = User::new(
        "hpotter",
        Some("pass123"),
        "Potter",
        "Harry",
        "h.potter@hogwart.edu.uk",
        None,
    );
    test_user.save(pool).await.unwrap();
}

pub struct ClientState {
    pub pool: DbPool,
    pub worker_state: Arc<Mutex<WorkerState>>,
    pub wireguard_rx: Receiver<GatewayEvent>,
    pub mail_rx: UnboundedReceiver<Mail>,
    pub failed_logins: Arc<Mutex<FailedLoginMap>>,
    pub test_user: User,
    pub config: DefGuardConfig,
}

impl ClientState {
    pub fn new(
        pool: DbPool,
        worker_state: Arc<Mutex<WorkerState>>,
        wireguard_rx: Receiver<GatewayEvent>,
        mail_rx: UnboundedReceiver<Mail>,
        failed_logins: Arc<Mutex<FailedLoginMap>>,
        test_user: User,
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

pub async fn make_base_client(pool: DbPool, config: DefGuardConfig) -> (TestClient, ClientState) {
    let (tx, rx) = unbounded_channel::<AppEvent>();
    let worker_state = Arc::new(Mutex::new(WorkerState::new(tx.clone())));
    let (wg_tx, wg_rx) = broadcast::channel::<GatewayEvent>(16);
    let (mail_tx, mail_rx) = unbounded_channel::<Mail>();
    let gateway_state = Arc::new(Mutex::new(GatewayMap::new()));

    let failed_logins = FailedLoginMap::new();
    let failed_logins = Arc::new(Mutex::new(failed_logins));

    let user_agent_parser = create_user_agent_parser();

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

    let app_state = AppState::new(
        config.clone(),
        pool,
        tx,
        rx,
        wg_tx,
        mail_tx,
        user_agent_parser.clone(),
        failed_logins.clone(),
    );

    let webapp = build_webapp(worker_state, gateway_state, app_state);
    (TestClient::new(webapp).await, client_state)
}

#[allow(dead_code)]
pub async fn make_test_client() -> (TestClient, ClientState) {
    let (pool, config) = init_test_db().await;
    make_base_client(pool, config).await
}

#[allow(dead_code)]
pub async fn fetch_user_details(client: &TestClient, username: &str) -> UserDetails {
    let response = client.get(&format!("/api/v1/user/{username}")).send().await;
    assert_eq!(response.status(), StatusCode::OK);
    response.json().await
}
