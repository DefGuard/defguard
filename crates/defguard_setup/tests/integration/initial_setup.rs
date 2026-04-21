use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
};

use axum::serve;
use defguard_certs::{CertificateAuthority, PemLabel, der_to_pem};
use defguard_common::{
    VERSION,
    config::DefGuardConfig,
    db::{
        models::{
            Certificates, Session, Settings, User,
            group::Group,
            initial_setup_wizard::{InitialSetupState, InitialSetupStep},
            settings::initialize_current_settings,
            wizard::Wizard,
        },
        setup_pool,
    },
};
use defguard_setup::setup_server::build_setup_webapp;
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
    time::timeout,
};

use super::common::{SHUTDOWN_TIMEOUT, make_setup_test_client};
use crate::common::SESSION_COOKIE_NAME;

async fn assert_setup_step(pool: &sqlx::PgPool, expected: InitialSetupStep) {
    let step = InitialSetupState::get(pool)
        .await
        .expect("Failed to fetch initial setup state")
        .map_or(InitialSetupStep::Welcome, |s| s.step);
    assert_eq!(step, expected);
}

#[sqlx::test]
async fn test_create_admin(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool)
        .await
        .expect("Failed to initialize settings");
    Wizard::init(&pool, false, &DefGuardConfig::new_test_config())
        .await
        .expect("Failed to initialize wizard");

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
async fn test_create_admin_with_automatic_group_assignment(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool)
        .await
        .expect("Failed to initialize settings");
    Wizard::init(&pool, false, &DefGuardConfig::new_test_config())
        .await
        .expect("Failed to initialize wizard");

    let (client, _shutdown_rx) = make_setup_test_client(pool.clone()).await;
    let default_admin_group_name = Settings::get_current_settings().default_admin_group_name;

    let payload = json!({
        "first_name": "Admin",
        "last_name": "Admin",
        "username": "admin1",
        "email": "admin1@example.com",
        "password": "Passw0rd!",
        "automatically_assign_group": true
    });

    let response = client
        .post("/api/v1/initial_setup/admin")
        .json(&payload)
        .send()
        .await
        .expect("Failed to create admin user");
    assert_eq!(response.status(), StatusCode::CREATED);

    let group = Group::find_by_name(&pool, &default_admin_group_name)
        .await
        .expect("Failed to fetch group")
        .expect("Default admin group not created");
    assert!(group.is_admin);

    let admin = User::find_by_username(&pool, "admin1")
        .await
        .expect("Failed to fetch admin")
        .expect("Admin user missing");
    let groups = admin
        .member_of_names(&pool)
        .await
        .expect("Failed to fetch group membership");
    assert!(groups.contains(&default_admin_group_name));
}

#[sqlx::test]
async fn test_setup_login_too_many_attempts(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool)
        .await
        .expect("Failed to initialize settings");
    Wizard::init(&pool, false, &DefGuardConfig::new_test_config())
        .await
        .expect("Failed to initialize wizard");

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
    Wizard::init(&pool, false, &DefGuardConfig::new_test_config())
        .await
        .expect("Failed to initialize wizard");

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
        "default_admin_group_name": "admins",
        "default_authentication": 14,
        "default_mfa_code_lifetime": 120,
        "admin_username": "admin1"
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
    assert_eq!(settings.defguard_url, "http://localhost:8000");
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
    Wizard::init(&pool, false, &DefGuardConfig::new_test_config())
        .await
        .expect("Failed to initialize wizard");

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

    let certs = Certificates::get_or_default(&pool)
        .await
        .expect("Failed to fetch certificates");
    assert!(certs.ca_cert_der.is_some());
    assert!(certs.ca_key_der.is_some());
    assert!(certs.ca_expiry.is_some());

    assert_setup_step(&pool, InitialSetupStep::CaSummary).await;
}

#[sqlx::test]
async fn test_upload_ca(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool)
        .await
        .expect("Failed to initialize settings");
    Wizard::init(&pool, false, &DefGuardConfig::new_test_config())
        .await
        .expect("Failed to initialize wizard");

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

    let certs = Certificates::get_or_default(&pool)
        .await
        .expect("Failed to fetch certificates");
    assert!(certs.ca_cert_der.is_some());
    assert!(certs.ca_key_der.is_none());
    assert!(certs.ca_expiry.is_some());

    assert_setup_step(&pool, InitialSetupStep::CaSummary).await;
}

#[sqlx::test]
async fn test_get_ca(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool)
        .await
        .expect("Failed to initialize settings");
    Wizard::init(&pool, false, &DefGuardConfig::new_test_config())
        .await
        .expect("Failed to initialize wizard");

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
    Wizard::init(&pool, false, &DefGuardConfig::new_test_config())
        .await
        .expect("Failed to initialize wizard");

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

    let wizard = Wizard::get(&pool)
        .await
        .expect("Failed to fetch wizard state");
    assert!(wizard.completed);
    let setup_state = InitialSetupState::get(&pool)
        .await
        .expect("Failed to fetch initial setup state");
    assert_eq!(
        setup_state.as_ref().map(|s| s.step),
        Some(InitialSetupStep::Finished)
    );

    assert_setup_step(&pool, InitialSetupStep::Finished).await;

    let shutdown_signal = timeout(SHUTDOWN_TIMEOUT, shutdown_rx).await;
    assert!(matches!(shutdown_signal, Ok(Ok(()))));
}

#[sqlx::test]
async fn test_setup_flow(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool)
        .await
        .expect("Failed to initialize settings");
    Wizard::init(&pool, false, &DefGuardConfig::new_test_config())
        .await
        .expect("Failed to initialize wizard");

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
            "default_admin_group_name": "admins",
            "default_authentication": 14,
            "default_mfa_code_lifetime": 120,
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
    assert_eq!(settings.default_admin_group_name, "admins");
    assert_eq!(settings.authentication_period_days, 14);
    assert_eq!(settings.mfa_code_timeout_seconds, 120);

    let certs = Certificates::get_or_default(&pool)
        .await
        .expect("Failed to fetch certificates");
    assert!(certs.ca_cert_der.is_some());
    assert!(certs.ca_key_der.is_some());
    assert!(certs.ca_expiry.is_some());

    let wizard = Wizard::get(&pool)
        .await
        .expect("Failed to fetch wizard state");
    assert!(wizard.completed);
    let setup_state = InitialSetupState::get(&pool)
        .await
        .expect("Failed to fetch initial setup state");
    assert_eq!(
        setup_state.as_ref().map(|s| s.step),
        Some(InitialSetupStep::Finished)
    );

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

    let shutdown_signal = timeout(SHUTDOWN_TIMEOUT, shutdown_notify.notified()).await;
    assert!(shutdown_signal.is_ok());

    let server_result = timeout(SHUTDOWN_TIMEOUT, server_task).await;
    assert!(matches!(server_result, Ok(Ok(()))));
}
