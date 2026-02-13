pub(crate) mod client;

use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::{Arc, Mutex},
};

pub use defguard_common::db::setup_pool;
use defguard_common::{
    VERSION,
    config::DefGuardConfig,
    db::{
        Id,
        models::{Device, User, WireguardNetwork, settings::initialize_current_settings},
    },
};
use defguard_core::{
    auth::failed_login::FailedLoginMap,
    build_webapp,
    db::AppEvent,
    enterprise::license::{License, LicenseTier, set_cached_license},
    events::ApiEvent,
    grpc::{WorkerState, gateway::events::GatewayEvent},
    handlers::{Auth, user::UserDetails},
};
use reqwest::{StatusCode, header::HeaderName};
use semver::Version;
use serde_json::json;
use sqlx::PgPool;
use tokio::{
    net::TcpListener,
    sync::{
        broadcast::{self, Receiver},
        mpsc::{channel, unbounded_channel},
    },
};

use self::client::TestClient;
use crate::{
    api::common::client::TestResponse,
    common::{init_config, initialize_users},
};

#[allow(clippy::declare_interior_mutable_const)]
pub const X_FORWARDED_HOST: HeaderName = HeaderName::from_static("x-forwarded-host");
#[allow(clippy::declare_interior_mutable_const)]
pub const X_FORWARDED_FOR: HeaderName = HeaderName::from_static("x-forwarded-for");
#[allow(clippy::declare_interior_mutable_const)]
pub const X_FORWARDED_URI: HeaderName = HeaderName::from_static("x-forwarded-uri");

pub(crate) struct ClientState {
    pub pool: PgPool,
    pub worker_state: Arc<Mutex<WorkerState>>,
    pub wireguard_rx: Receiver<GatewayEvent>,
    pub test_user: User<Id>,
    pub config: DefGuardConfig,
}

impl ClientState {
    pub fn new(
        pool: PgPool,
        worker_state: Arc<Mutex<WorkerState>>,
        wireguard_rx: Receiver<GatewayEvent>,
        test_user: User<Id>,
        config: DefGuardConfig,
    ) -> Self {
        Self {
            pool,
            worker_state,
            wireguard_rx,
            test_user,
            config,
        }
    }
}

pub(crate) async fn make_base_client(
    pool: PgPool,
    config: DefGuardConfig,
    listener: TcpListener,
) -> (TestClient, ClientState) {
    let (api_event_tx, api_event_rx) = unbounded_channel::<ApiEvent>();
    let (tx, rx) = unbounded_channel::<AppEvent>();
    let worker_state = Arc::new(Mutex::new(WorkerState::new(tx.clone())));
    let (wg_tx, wg_rx) = broadcast::channel::<GatewayEvent>(16);

    let failed_logins = FailedLoginMap::new();
    let failed_logins = Arc::new(Mutex::new(failed_logins));

    let license = License::new(
        "test_customer".to_string(),
        false,
        // Permanent license
        None,
        None,
        None,
        LicenseTier::Business,
    );

    set_cached_license(Some(license));

    let client_state = ClientState::new(
        pool.clone(),
        worker_state.clone(),
        wg_rx,
        User::find_by_username(&pool, "hpotter")
            .await
            .unwrap()
            .unwrap(),
        config.clone(),
    );

    let (proxy_control_tx, _proxy_control_rx) = channel(10);

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
        worker_state,
        pool,
        failed_logins,
        api_event_tx,
        Version::parse(VERSION).unwrap(),
        Arc::default(),
        proxy_control_tx,
    );

    (
        TestClient::new(webapp, listener, api_event_rx),
        client_state,
    )
}

pub(crate) async fn make_test_client(pool: PgPool) -> (TestClient, ClientState) {
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);
    let listener = TcpListener::bind(addr)
        .await
        .expect("Could not bind ephemeral socket");
    let port = listener.local_addr().unwrap().port();
    let config = init_config(Some(&format!("http://localhost:{port}")), &pool).await;
    initialize_users(&pool, &config).await;
    initialize_current_settings(&pool)
        .await
        .expect("Could not initialize settings");
    make_base_client(pool, config, listener).await
}

pub(crate) async fn fetch_user_details(client: &TestClient, username: &str) -> UserDetails {
    let response = client.get(format!("/api/v1/user/{username}")).send().await;
    assert_eq!(response.status(), StatusCode::OK);
    response.json().await
}

/// Exceeds enterprise free version limits by creating more than 1 network
pub(crate) async fn exceed_enterprise_limits(client: &TestClient) {
    let auth = Auth::new("admin", "pass123");
    client.post("/api/v1/auth").json(&auth).send().await;

    make_network(client, "network1").await;
    make_network(client, "network2").await;
}

/// Create test network with a given name.
pub(crate) async fn make_network(client: &TestClient, name: &str) -> TestResponse {
    let response = client
        .post("/api/v1/network")
        .json(&json!({
            "name": name,
            "address": "10.1.1.1/24",
            "port": 55555,
            "endpoint": "192.168.4.14",
            "allowed_ips": "10.1.1.0/24",
            "dns": "1.1.1.1",
            "mtu": 1420,
            "fwmark": 0,
            "allowed_groups": [],
            "keepalive_interval": 25,
            "peer_disconnect_threshold": 300,
            "acl_enabled": false,
            "acl_default_allow": false,
            "location_mfa_mode": "disabled",
            "service_location_mode": "disabled"
        }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    response
}

pub(crate) async fn make_client(pool: PgPool) -> TestClient {
    let (client, _) = make_test_client(pool).await;
    client
}

pub(crate) async fn make_client_with_db(pool: PgPool) -> (TestClient, PgPool) {
    let (client, client_state) = make_test_client(pool).await;
    (client, client_state.pool)
}

pub(crate) async fn authenticate_admin(client: &mut TestClient) {
    client.login_user("admin", "pass123").await;
}

// Helper to fetch current user state from DB by username
pub(crate) async fn get_db_user(pool: &PgPool, username: &str) -> User<Id> {
    User::find_by_username(pool, username)
        .await
        .unwrap()
        .unwrap()
}

// Helper to fetch current location state from DB by ID
pub(crate) async fn get_db_location(pool: &PgPool, location_id: Id) -> WireguardNetwork<Id> {
    WireguardNetwork::find_by_id(pool, location_id)
        .await
        .unwrap()
        .unwrap()
}

// Helper to fetch current user device state from DB by device ID
pub(crate) async fn get_db_device(pool: &PgPool, device_id: Id) -> Device<Id> {
    Device::find_by_id(pool, device_id).await.unwrap().unwrap()
}
