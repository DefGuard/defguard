use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
};

use axum::serve;
use defguard_certs::{CertificateAuthority, PemLabel, der_to_pem};
use defguard_common::{
    VERSION,
    config::{DefGuardConfig, SERVER_CONFIG},
    db::{
        models::{
            Certificates, Session, Settings, User,
            group::Group,
            initial_setup_wizard::{InitialSetupState, InitialSetupStep},
            settings::{initialize_current_settings, set_settings},
            wizard::Wizard,
        },
        setup_pool,
    },
};
use defguard_setup::setup_server::build_setup_webapp;
use reqwest::{
    Client, StatusCode,
    cookie::Jar,
    header::{HeaderMap, USER_AGENT},
};
use semver::Version;
use serde_json::json;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use tokio::{
    net::TcpListener,
    sync::{Notify, oneshot, oneshot::error::TryRecvError},
    time::timeout,
};

use super::common::{SHUTDOWN_TIMEOUT, make_setup_test_client};
use crate::common::{SESSION_COOKIE_NAME, TEST_USER_AGENT};

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
async fn test_create_admin_sets_secure_cookie_for_forwarded_https(
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

    let response = client
        .post("/api/v1/initial_setup/admin")
        .header("x-forwarded-proto", "https")
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

    let session_cookie = response
        .cookies()
        .find(|cookie| cookie.name() == SESSION_COOKIE_NAME)
        .expect("Session cookie not set");
    assert!(
        session_cookie.secure(),
        "Session cookie must be Secure for forwarded HTTPS"
    );
    assert_setup_step(&pool, InitialSetupStep::GeneralConfiguration).await;
}

#[sqlx::test]
async fn test_setup_login_sets_insecure_cookie_without_forwarded_proto(
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
        .post("/api/v1/initial_setup/login")
        .json(&json!({
            "username": "admin1",
            "password": "Passw0rd!"
        }))
        .send()
        .await
        .expect("Failed to log in during setup");
    assert_eq!(response.status(), StatusCode::OK);

    let session_cookie = response
        .cookies()
        .find(|cookie| cookie.name() == SESSION_COOKIE_NAME)
        .expect("Session cookie not set");
    assert!(
        !session_cookie.secure(),
        "Session cookie must not be Secure without forwarded HTTPS"
    );
    assert_setup_step(&pool, InitialSetupStep::GeneralConfiguration).await;
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
    assert!(groups.contains(&"admins".to_owned()));

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
async fn test_finish_setup_rejects_invalid_admin_configuration(
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

    let (client, mut shutdown_rx) = make_setup_test_client(pool.clone()).await;
    assert_eq!(
        Settings::get_current_settings().default_admin_id,
        None,
        "Test must start without a default admin ID"
    );

    let response = client
        .post("/api/v1/initial_setup/finish")
        .send()
        .await
        .expect("Failed to finish setup without an admin ID");
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let body: serde_json::Value = response
        .json()
        .await
        .expect("Failed to parse missing-admin response");
    assert_eq!(body["msg"], "Internal Server Error");

    let wizard = Wizard::get(&pool)
        .await
        .expect("Failed to fetch wizard state");
    assert!(!wizard.completed);
    assert_setup_step(&pool, InitialSetupStep::Welcome).await;
    assert!(matches!(shutdown_rx.try_recv(), Err(TryRecvError::Empty)));

    let dangling_admin_id = i64::MAX;
    let original_settings = Settings::get_current_settings();
    let mut settings = original_settings.clone();
    settings.default_admin_id = Some(dangling_admin_id);
    set_settings(Some(settings));

    let response = client.post("/api/v1/initial_setup/finish").send().await;
    set_settings(Some(original_settings));
    let response = response.expect("Failed to finish setup with a dangling admin ID");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let body: serde_json::Value = response
        .json()
        .await
        .expect("Failed to parse dangling-admin response");
    let message = body["msg"]
        .as_str()
        .expect("Dangling-admin response message is not a string");
    assert_eq!(message, "Default admin user not found");
    assert!(!message.contains(&dangling_admin_id.to_string()));

    let wizard = Wizard::get(&pool)
        .await
        .expect("Failed to fetch wizard state");
    assert!(!wizard.completed);
    assert_setup_step(&pool, InitialSetupStep::Welcome).await;
    assert!(matches!(shutdown_rx.try_recv(), Err(TryRecvError::Empty)));
}

#[sqlx::test]
async fn test_finish_setup_rolls_back_when_session_invalidation_fails(
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

    let (client, mut shutdown_rx) = make_setup_test_client(pool.clone()).await;

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
    let session_id = response
        .cookies()
        .find(|cookie| cookie.name() == SESSION_COOKIE_NAME)
        .expect("Initial session cookie not set")
        .value()
        .to_owned();

    let wizard_before = Wizard::get(&pool)
        .await
        .expect("Failed to fetch wizard state before finish");
    let setup_state_before = InitialSetupState::get(&pool)
        .await
        .expect("Failed to fetch initial setup state before finish");

    sqlx::query(
        r#"
        CREATE FUNCTION fail_session_delete() RETURNS trigger
        LANGUAGE plpgsql AS $$
        BEGIN
            RAISE EXCEPTION 'session deletion blocked';
        END;
        $$;
        "#,
    )
    .execute(&pool)
    .await
    .expect("Failed to create session delete trigger function");
    sqlx::query(
        r#"
        CREATE TRIGGER fail_session_delete
        BEFORE DELETE ON session
        FOR EACH ROW
        EXECUTE FUNCTION fail_session_delete();
        "#,
    )
    .execute(&pool)
    .await
    .expect("Failed to create session delete trigger");

    let response = client
        .post("/api/v1/initial_setup/finish")
        .send()
        .await
        .expect("Failed to finish setup");
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

    let wizard_after = Wizard::get(&pool)
        .await
        .expect("Failed to fetch wizard state after failed finish");
    assert_eq!(wizard_after.active_wizard, wizard_before.active_wizard);
    assert_eq!(wizard_after.completed, wizard_before.completed);
    assert_eq!(
        wizard_after.last_version_migrated_to,
        wizard_before.last_version_migrated_to
    );
    let setup_state_after = InitialSetupState::get(&pool)
        .await
        .expect("Failed to fetch initial setup state after failed finish");
    assert_eq!(
        setup_state_after.map(|state| state.step),
        setup_state_before.map(|state| state.step)
    );
    assert!(
        Session::find_by_id(&pool, &session_id)
            .await
            .expect("Failed to fetch session after failed finish")
            .is_some()
    );
    assert!(matches!(shutdown_rx.try_recv(), Err(TryRecvError::Empty)));
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
async fn test_finish_setup_clears_session_cookie(_: PgPoolOptions, options: PgConnectOptions) {
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

    let response = client
        .post("/api/v1/initial_setup/finish")
        .send()
        .await
        .expect("Failed to finish setup");
    assert_eq!(response.status(), StatusCode::OK);

    let session_cookie = response
        .cookies()
        .find(|cookie| cookie.name() == SESSION_COOKIE_NAME)
        .expect("Session cookie was not cleared");
    assert_eq!(session_cookie.value(), "");
    assert_eq!(session_cookie.path(), Some("/"));
}

#[sqlx::test]
async fn test_finish_setup_deletes_all_admin_sessions(_: PgPoolOptions, options: PgConnectOptions) {
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
    let first_session_id = response
        .cookies()
        .find(|cookie| cookie.name() == SESSION_COOKIE_NAME)
        .expect("Initial session cookie not set")
        .value()
        .to_owned();

    let response = client
        .post("/api/v1/initial_setup/login")
        .json(&json!({
            "username": "admin1",
            "password": "Passw0rd!"
        }))
        .send()
        .await
        .expect("Failed to log in during setup");
    assert_eq!(response.status(), StatusCode::OK);
    let second_session_id = response
        .cookies()
        .find(|cookie| cookie.name() == SESSION_COOKIE_NAME)
        .expect("Relogin session cookie not set")
        .value()
        .to_owned();
    assert_ne!(first_session_id, second_session_id);

    let admin_user = User::find_by_username(&pool, "admin1")
        .await
        .expect("Failed to fetch admin user")
        .expect("Admin user not found");
    let session_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM session WHERE user_id = $1")
        .bind(admin_user.id)
        .fetch_one(&pool)
        .await
        .expect("Failed to count admin sessions");
    assert_eq!(session_count, 2);

    let response = client
        .post("/api/v1/initial_setup/finish")
        .send()
        .await
        .expect("Failed to finish setup");
    assert_eq!(response.status(), StatusCode::OK);

    let session_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM session WHERE user_id = $1")
        .bind(admin_user.id)
        .fetch_one(&pool)
        .await
        .expect("Failed to count admin sessions");
    assert_eq!(session_count, 0);
}

#[sqlx::test]
async fn test_setup_flow(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool)
        .await
        .expect("Failed to initialize settings");
    let mut config = DefGuardConfig::new_test_config();
    config.cookie_insecure = None;
    let _ = SERVER_CONFIG.set(config);
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
    headers.insert(USER_AGENT, TEST_USER_AGENT);
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
        .to_owned();
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
    assert!(groups.contains(&"admins".to_owned()));

    let session = Session::find_by_id(&pool, &session_cookie_value)
        .await
        .expect("Failed to fetch session");
    assert!(
        session.is_none(),
        "Session still exists after setup finished"
    );

    let shutdown_signal = timeout(SHUTDOWN_TIMEOUT, shutdown_notify.notified()).await;
    assert!(shutdown_signal.is_ok());

    let server_result = timeout(SHUTDOWN_TIMEOUT, server_task).await;
    assert!(matches!(server_result, Ok(Ok(()))));
}
