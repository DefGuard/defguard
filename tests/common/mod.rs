#[cfg(test)]
use defguard::{
    auth::failed_login::FailedLoginMap,
    build_webapp,
    config::DefGuardConfig,
    db::{init_db, AppEvent, DbPool, GatewayEvent, User},
    grpc::{GatewayMap, WorkerState},
};
use defguard::{db::UserDetails, mail::Mail, SERVER_CONFIG};
use rocket::http::Status;
use rocket::local::asynchronous::Client;
use secrecy::ExposeSecret;
use sqlx::{postgres::PgConnectOptions, query, types::Uuid};
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;
use tokio::sync::broadcast::Receiver;
use tokio::sync::mpsc::unbounded_channel;

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
    query(&format!("CREATE DATABASE \"{}\"", db_name))
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
        "hpotter".into(),
        Some("pass123"),
        "Potter".into(),
        "Harry".into(),
        "h.potter@hogwart.edu.uk".into(),
        None,
    );
    test_user.save(pool).await.unwrap();
}

pub struct ClientState {
    pub pool: DbPool,
    pub worker_state: Arc<Mutex<WorkerState>>,
    pub wireguard_rx: Receiver<GatewayEvent>,
    pub failed_logins: Arc<Mutex<FailedLoginMap>>,
    pub test_user: User,
    pub config: DefGuardConfig,
}

impl ClientState {
    pub fn new(
        pool: DbPool,
        worker_state: Arc<Mutex<WorkerState>>,
        wireguard_rx: Receiver<GatewayEvent>,
        failed_logins: Arc<Mutex<FailedLoginMap>>,
        test_user: User,
        config: DefGuardConfig,
    ) -> Self {
        Self {
            pool,
            worker_state,
            wireguard_rx,
            failed_logins,
            test_user,
            config,
        }
    }
}

pub async fn make_base_client(pool: DbPool, config: DefGuardConfig) -> (Client, ClientState) {
    let (tx, rx) = unbounded_channel::<AppEvent>();
    let worker_state = Arc::new(Mutex::new(WorkerState::new(tx.clone())));
    let (wg_tx, wg_rx) = broadcast::channel::<GatewayEvent>(16);
    let (mail_tx, _) = unbounded_channel::<Mail>();
    let gateway_state = Arc::new(Mutex::new(GatewayMap::new()));

    let failed_logins = FailedLoginMap::new();
    let failed_logins = Arc::new(Mutex::new(failed_logins));

    let client_state = ClientState::new(
        pool.clone(),
        worker_state.clone(),
        wg_rx,
        failed_logins.clone(),
        User::find_by_username(&pool, "hpotter")
            .await
            .unwrap()
            .unwrap(),
        config.clone(),
    );

    let webapp = build_webapp(
        config,
        tx,
        rx,
        wg_tx,
        mail_tx,
        worker_state,
        gateway_state,
        pool,
        failed_logins,
    )
    .await;
    (Client::tracked(webapp).await.unwrap(), client_state)
}

#[allow(dead_code)]
pub async fn make_test_client() -> (Client, ClientState) {
    let (pool, config) = init_test_db().await;
    make_base_client(pool, config).await
}

#[allow(dead_code)]
pub async fn make_license_test_client(license: &str) -> (Client, ClientState) {
    let (pool, mut config) = init_test_db().await;
    config.license = license.into();
    make_base_client(pool, config).await
}

#[allow(dead_code)]
pub async fn make_enterprise_test_client() -> (Client, ClientState) {
    make_license_test_client(LICENSE_ENTERPRISE).await
}

#[allow(dead_code)]
pub async fn fetch_user_details(client: &Client, username: &str) -> UserDetails {
    let response = client
        .get(format!("/api/v1/user/{}", username))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);
    response.into_json().await.unwrap()
}

#[allow(dead_code)]
#[cfg(feature = "openid")]
pub(super) static LICENSE_ENTERPRISE: &str = "BwAAAAAAAAB0ZW9uaXRlCgAAAAAAAAAyMDUwLTEwLTEwAAAAAAFiayfBptq8pZXjPo4FV3VnmmwR/ipZHLriVPTW3AFyRq4c2wR+DzWC4BUACu3YMS27kX116JVKWB3/edYKNELFSiqYc6vsfoOrXnnQQJDI8RoyAQB6MpLv/EcgRZh47iI4L+tp44jKFQZ+EqqvMNt3G41u13P72HdkUv8yzQ7dmm3BrYQGJSCh/xiLna+mtQ9IQdqXOmYVInPXiWtIvi157Utfnow3gS0Ak45jci0DhtH+RWmFfiMOQCc4Qx0kEF9PsHl6Hn9Ay4oRTAnSYEPdWfQlVh5Rp276bLqnHDdyJ3/o2RSNK+QUXR7V2iuN1M3sWyW1rCGXtV5miHGI97CS";
#[allow(dead_code)]
pub(super) static LICENSE_EXPIRED: &str = "BwAAAAAAAAB0ZW9uaXRlCgAAAAAAAAAyMDE5LTEwLTEwAAAAAAFuZ7Xm9M20ds/U/PQgVmz4uViCRTJbyAPVLtYRBGvE0i+czH4mxPl4mCyAO1cAOPXNxqh9sAVVr/GzToOix4DfK0aLrYG9FqV5jW13CH+UKTFBqQvN9gGLmnl9+b3pH10gxpGKRZ5fn73fsZsO0SKrJvQ8SAHEQ2+r+VCdZphZ2r9cFR6MC39Ixk4lCki8mz9A4FHZyW4YWWr6k+bxu9RjG/0imh+6OBeddKBpU3HnK96B4rjhiEhrKpfJo6dzib/Mfk+UNZHQA2dAjlocKKxa2+acUaEJQmnaIv4FyFZHl2OzGKkweqDBo0E+Ai7m1g07+pXdXGYb9ykVfoCBEgEX";
#[allow(dead_code)]
#[cfg(feature = "openid")]
pub(super) static LICENSE_WITHOUT_OPENID: &str = "BwAAAAAAAAB0ZW9uaXRlCgAAAAAAAAAyMDUwLTEwLTIyAQABAQCCpzpcqi+8jRX+QTuVjyK0ZmdKa8j+SrA53qSY4rAxjZyt6hgVLlcqTqIbbA7uds5ACa1oBWvQbbPIlGGTpNnG+gQzTm9hAc3CmEd2zMdQXOXzWN8jJHTflsr1dYMxA+tK1el+An+jOY85j0WaRNJma7desF6HEasgEEPktV5P5y3Yh1fULS1scDjEbOJS3pvI07BmSA0/Z+swPMqRzSoyt6NaOUDbR53HR2mMjBSsaZBLsrTQ9Ai16A8fo6pqt2XpSfy/1ImC3mq2q6TG/ABnFw1j65UW0Mx261Bn9184zyLdKPycFUWfyOmOpk/46JZX/PMBHERXeFbmN6YE3KpO";
