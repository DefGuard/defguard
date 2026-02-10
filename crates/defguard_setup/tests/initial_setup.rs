use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
};

use axum::serve;
use defguard_certs::{CertificateAuthority, PemLabel, der_to_pem};
use defguard_common::{
    VERSION,
    db::{
        models::{
            Session, Settings, User,
            group::Group,
            settings::{InitialSetupStep, initialize_current_settings},
        },
        setup_pool,
    },
};
use defguard_setup::setup::build_setup_webapp;
use reqwest::{
    Client, StatusCode,
    cookie::Jar,
    header::{HeaderMap, HeaderValue, USER_AGENT},
};
use semver::Version;
use serde_json::json;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use tokio::{
    net::TcpListener,
    sync::{Notify, oneshot},
    task::JoinHandle,
};

const SESSION_COOKIE_NAME: &str = "defguard_session";

async fn assert_setup_step(pool: &sqlx::PgPool, expected: InitialSetupStep) {
    let settings = Settings::get(pool)
        .await
        .expect("Failed to fetch settings")
        .expect("Settings not found");
    assert_eq!(settings.initial_setup_step, expected);

    let current_settings = Settings::get_current_settings();
    assert_eq!(current_settings.initial_setup_step, expected);
}

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
            let server = serve(
                listener,
                app.into_make_service_with_connect_info::<SocketAddr>(),
            );
            server.await.expect("server error");
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

#[sqlx::test]
async fn test_create_admin(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool)
        .await
        .expect("Failed to initialize settings");

    let (client, _shutdown_rx) = make_setup_test_client(pool.clone()).await;

    let payload = json!({
        "first_name": "Admin",
        "last_name": "Admin",
        "username": "admin1",
        "email": "admin1@example.com",
        "password": "Passw0rd!"
    });

    let response = client
        .post("/api/v1/initial_setup/admin")
        .json(&payload)
        .send()
        .await
        .expect("Failed to create admin user");
    assert_eq!(response.status(), StatusCode::CREATED);

    let session_cookie = response
        .cookies()
        .find(|cookie| cookie.name() == SESSION_COOKIE_NAME)
        .expect("Session cookie not set");

    let user = User::find_by_username(&pool, "admin1")
        .await
        .expect("Failed to fetch user")
        .expect("Admin user not created");
    assert_eq!(user.email, "admin1@example.com");

    let session = Session::find_by_id(&pool, session_cookie.value())
        .await
        .expect("Failed to fetch session")
        .expect("Session not created");
    assert_eq!(session.user_id, user.id);

    let settings = Settings::get(&pool)
        .await
        .expect("Failed to fetch settings")
        .expect("Settings not found");
    assert_eq!(settings.default_admin_id, Some(user.id));

    assert_setup_step(&pool, InitialSetupStep::GeneralConfiguration).await;
}

#[sqlx::test]
async fn test_setup_login_too_many_attempts(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool)
        .await
        .expect("Failed to initialize settings");

    let (client, _shutdown_rx) = make_setup_test_client(pool.clone()).await;

    let response = client
        .post("/api/v1/initial_setup/admin")
        .json(&json!({
            "first_name": "Admin",
            "last_name": "Admin",
            "username": "admin1",
            "email": "admin1@example.com",
            "password": "Passw0rd!"
        }))
        .send()
        .await
        .expect("Failed to create admin user");
    assert_eq!(response.status(), StatusCode::CREATED);

    let payload = json!({
        "username": "admin1",
        "password": "WrongPass"
    });

    for _ in 0..5 {
        let response = client
            .post("/api/v1/initial_setup/login")
            .json(&payload)
            .send()
            .await
            .expect("Failed to login during setup");
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    let response = client
        .post("/api/v1/initial_setup/login")
        .json(&payload)
        .send()
        .await
        .expect("Failed to login during setup");
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
}

#[sqlx::test]
async fn test_set_general_config(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool)
        .await
        .expect("Failed to initialize settings");

    let (client, _shutdown_rx) = make_setup_test_client(pool.clone()).await;

    let response = client
        .post("/api/v1/initial_setup/admin")
        .json(&json!({
            "first_name": "Admin",
            "last_name": "Admin",
            "username": "admin1",
            "email": "admin1@example.com",
            "password": "Passw0rd!"
        }))
        .send()
        .await
        .expect("Failed to create admin user");
    assert_eq!(response.status(), StatusCode::CREATED);

    let payload = json!({
        "defguard_url": "https://example.com",
        "default_admin_group_name": "admins",
        "default_authentication": 14,
        "default_mfa_code_lifetime": 120,
        "admin_username": "admin1",
        "public_proxy_url": "https://proxy.example.com"
    });

    let response = client
        .post("/api/v1/initial_setup/general_config")
        .json(&payload)
        .send()
        .await
        .expect("Failed to set general config");
    assert_eq!(response.status(), StatusCode::CREATED);

    let settings = Settings::get(&pool)
        .await
        .expect("Failed to fetch settings")
        .expect("Settings not found");
    assert_eq!(settings.defguard_url, "https://example.com");
    assert_eq!(settings.default_admin_group_name, "admins");
    assert_eq!(settings.authentication_period_days, 14);
    assert_eq!(settings.mfa_code_timeout_seconds, 120);

    let group = Group::find_by_name(&pool, "admins")
        .await
        .expect("Failed to fetch group")
        .expect("Admin group not created");
    assert!(group.is_admin);

    let admin = User::find_by_username(&pool, "admin1")
        .await
        .expect("Failed to fetch admin")
        .expect("Admin user missing");
    let groups = admin
        .member_of_names(&pool)
        .await
        .expect("Failed to fetch group membership");
    assert!(groups.contains(&"admins".to_string()));

    assert_setup_step(&pool, InitialSetupStep::Ca).await;
}

#[sqlx::test]
async fn test_create_ca(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool)
        .await
        .expect("Failed to initialize settings");

    let (client, _shutdown_rx) = make_setup_test_client(pool.clone()).await;

    let response = client
        .post("/api/v1/initial_setup/admin")
        .json(&json!({
            "first_name": "Admin",
            "last_name": "Admin",
            "username": "admin1",
            "email": "admin1@example.com",
            "password": "Passw0rd!"
        }))
        .send()
        .await
        .expect("Failed to create admin user");
    assert_eq!(response.status(), StatusCode::CREATED);

    let payload = json!({
        "common_name": "Test CA",
        "email": "ca@example.com",
        "validity_period_years": 1
    });

    let response = client
        .post("/api/v1/initial_setup/ca")
        .json(&payload)
        .send()
        .await
        .expect("Failed to create CA");
    assert_eq!(response.status(), StatusCode::CREATED);

    let settings = Settings::get(&pool)
        .await
        .expect("Failed to fetch settings")
        .expect("Settings not found");
    assert!(settings.ca_cert_der.is_some());
    assert!(settings.ca_key_der.is_some());
    assert!(settings.ca_expiry.is_some());

    assert_setup_step(&pool, InitialSetupStep::CaSummary).await;
}

#[sqlx::test]
async fn test_upload_ca(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool)
        .await
        .expect("Failed to initialize settings");

    let (client, _shutdown_rx) = make_setup_test_client(pool.clone()).await;

    let response = client
        .post("/api/v1/initial_setup/admin")
        .json(&json!({
            "first_name": "Admin",
            "last_name": "Admin",
            "username": "admin1",
            "email": "admin1@example.com",
            "password": "Passw0rd!"
        }))
        .send()
        .await
        .expect("Failed to create admin user");
    assert_eq!(response.status(), StatusCode::CREATED);

    let ca = CertificateAuthority::new("CA", "ca@example.com", 365).expect("Failed to create CA");
    let cert_pem =
        der_to_pem(ca.cert_der(), PemLabel::Certificate).expect("Failed to convert cert to PEM");

    let response = client
        .post("/api/v1/initial_setup/ca/upload")
        .json(&json!({ "cert_file": cert_pem }))
        .send()
        .await
        .expect("Failed to upload CA");
    assert_eq!(response.status(), StatusCode::CREATED);

    let settings = Settings::get(&pool)
        .await
        .expect("Failed to fetch settings")
        .expect("Settings not found");
    assert!(settings.ca_cert_der.is_some());
    assert!(settings.ca_key_der.is_none());
    assert!(settings.ca_expiry.is_some());

    assert_setup_step(&pool, InitialSetupStep::CaSummary).await;
}

#[sqlx::test]
async fn test_get_ca(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool)
        .await
        .expect("Failed to initialize settings");

    let (client, _shutdown_rx) = make_setup_test_client(pool.clone()).await;

    let response = client
        .post("/api/v1/initial_setup/admin")
        .json(&json!({
            "first_name": "Admin",
            "last_name": "Admin",
            "username": "admin1",
            "email": "admin1@example.com",
            "password": "Passw0rd!"
        }))
        .send()
        .await
        .expect("Failed to create admin user");
    assert_eq!(response.status(), StatusCode::CREATED);

    let payload = json!({
        "common_name": "CA",
        "email": "ca@example.com",
        "validity_period_years": 1
    });
    let response = client
        .post("/api/v1/initial_setup/ca")
        .json(&payload)
        .send()
        .await
        .expect("Failed to create CA");
    assert_eq!(response.status(), StatusCode::CREATED);

    let response = client
        .get("/api/v1/initial_setup/ca")
        .send()
        .await
        .expect("Failed to fetch CA");
    assert_eq!(response.status(), StatusCode::OK);

    let body: serde_json::Value = response.json().await.expect("Failed to parse CA response");
    assert_eq!(body["subject_common_name"], "CA");
    let pem = body["ca_cert_pem"].as_str().expect("Missing ca_cert_pem");
    assert!(pem.contains("BEGIN CERTIFICATE"));

    assert_setup_step(&pool, InitialSetupStep::EdgeComponent).await;
}

#[sqlx::test]
async fn test_finish_setup(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool)
        .await
        .expect("Failed to initialize settings");

    let (client, shutdown_rx) = make_setup_test_client(pool.clone()).await;

    let response = client
        .post("/api/v1/initial_setup/admin")
        .json(&json!({
            "first_name": "Admin",
            "last_name": "Admin",
            "username": "admin1",
            "email": "admin1@example.com",
            "password": "Passw0rd!"
        }))
        .send()
        .await
        .expect("Failed to create admin user");
    assert_eq!(response.status(), StatusCode::CREATED);

    let response = client
        .post("/api/v1/initial_setup/finish")
        .send()
        .await
        .expect("Failed to finish setup");
    assert_eq!(response.status(), StatusCode::OK);

    let settings = Settings::get(&pool)
        .await
        .expect("Failed to fetch settings")
        .expect("Settings not found");
    assert!(settings.initial_setup_completed);
    assert_eq!(settings.initial_setup_step, InitialSetupStep::Finished);

    assert_setup_step(&pool, InitialSetupStep::Finished).await;

    let shutdown_signal =
        tokio::time::timeout(std::time::Duration::from_secs(1), shutdown_rx).await;
    assert!(matches!(shutdown_signal, Ok(Ok(()))));
}

#[sqlx::test]
async fn test_setup_flow(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool)
        .await
        .expect("Failed to initialize settings");

    let (setup_shutdown_tx, setup_shutdown_rx) = oneshot::channel::<()>();
    let shutdown_notify = Arc::new(Notify::new());
    let shutdown_notify_server = shutdown_notify.clone();

    let app = build_setup_webapp(
        pool.clone(),
        Version::parse(VERSION).expect("Invalid version"),
        setup_shutdown_tx,
    );

    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);
    let listener = TcpListener::bind(addr)
        .await
        .expect("Could not bind ephemeral socket");
    let port = listener.local_addr().unwrap().port();

    let server_task = tokio::spawn(async move {
        let server = serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .with_graceful_shutdown(async move {
            let _ = setup_shutdown_rx.await;
            shutdown_notify_server.notify_one();
        });
        server.await.expect("server error");
    });

    let jar = Arc::new(Jar::default());
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static("test/0.0"));
    let client = Client::builder()
        .default_headers(headers)
        .cookie_provider(jar)
        .build()
        .expect("Failed to build reqwest client");
    let base_url = format!("http://localhost:{port}");

    assert_setup_step(&pool, InitialSetupStep::Welcome).await;

    let response = client
        .post(format!("{base_url}/api/v1/initial_setup/admin"))
        .json(&json!({
            "first_name": "Admin",
            "last_name": "Admin",
            "username": "admin1",
            "email": "admin1@example.com",
            "password": "Passw0rd!"
        }))
        .send()
        .await
        .expect("Failed to create admin user");
    assert_eq!(response.status(), StatusCode::CREATED);
    let session_cookie_value = response
        .cookies()
        .find(|cookie| cookie.name() == SESSION_COOKIE_NAME)
        .expect("Session cookie not set")
        .value()
        .to_string();
    assert_setup_step(&pool, InitialSetupStep::GeneralConfiguration).await;

    let response = client
        .post(format!("{base_url}/api/v1/initial_setup/general_config"))
        .json(&json!({
            "defguard_url": "https://example.com",
            "default_admin_group_name": "admins",
            "default_authentication": 14,
            "default_mfa_code_lifetime": 120,
            "public_proxy_url": "https://proxy.example.com",
            "admin_username": "admin1"
        }))
        .send()
        .await
        .expect("Failed to set general config");
    assert_eq!(response.status(), StatusCode::CREATED);
    assert_setup_step(&pool, InitialSetupStep::Ca).await;

    let response = client
        .post(format!("{base_url}/api/v1/initial_setup/ca"))
        .json(&json!({
            "common_name": "CA",
            "email": "ca@example.com",
            "validity_period_years": 1
        }))
        .send()
        .await
        .expect("Failed to create CA");
    assert_eq!(response.status(), StatusCode::CREATED);
    assert_setup_step(&pool, InitialSetupStep::CaSummary).await;

    let response = client
        .post(format!("{base_url}/api/v1/initial_setup/finish"))
        .send()
        .await
        .expect("Failed to finish setup");
    assert_eq!(response.status(), StatusCode::OK);
    assert_setup_step(&pool, InitialSetupStep::Finished).await;

    let settings = Settings::get(&pool)
        .await
        .expect("Failed to fetch settings")
        .expect("Settings not found");
    assert!(settings.initial_setup_completed);
    assert_eq!(settings.defguard_url, "https://example.com");
    assert_eq!(settings.default_admin_group_name, "admins");
    assert_eq!(settings.authentication_period_days, 14);
    assert_eq!(settings.mfa_code_timeout_seconds, 120);
    assert!(settings.ca_cert_der.is_some());
    assert!(settings.ca_key_der.is_some());
    assert!(settings.ca_expiry.is_some());
    assert_eq!(settings.initial_setup_step, InitialSetupStep::Finished);

    let admin_group = Group::find_by_name(&pool, "admins")
        .await
        .expect("Failed to fetch admin group")
        .expect("Admin group not created");
    assert!(admin_group.is_admin);

    let admin_user = User::find_by_username(&pool, "admin1")
        .await
        .expect("Failed to fetch admin user")
        .expect("Admin user not found");
    let groups = admin_user
        .member_of_names(&pool)
        .await
        .expect("Failed to fetch group membership");
    assert!(groups.contains(&"admins".to_string()));

    let session = Session::find_by_id(&pool, &session_cookie_value)
        .await
        .expect("Failed to fetch session")
        .expect("Session not created");
    assert_eq!(session.user_id, admin_user.id);

    let shutdown_signal = tokio::time::timeout(
        std::time::Duration::from_secs(1),
        shutdown_notify.notified(),
    )
    .await;
    assert!(shutdown_signal.is_ok());

    let server_result = tokio::time::timeout(std::time::Duration::from_secs(1), server_task).await;
    assert!(matches!(server_result, Ok(Ok(()))));
}
