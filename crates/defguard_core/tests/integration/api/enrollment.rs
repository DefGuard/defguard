use chrono::Duration;
use defguard_common::db::models::{Settings, User, settings::update_current_settings};
use defguard_core::{
    db::models::enrollment::Token,
    handlers::{AddUserData, Auth},
};
use reqwest::StatusCode;
use serde::Deserialize;
use serde_json::json;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

use super::common::{
    configure_ldap, configure_smtp, fetch_user_details, make_client_with_db, setup_pool,
};

#[sqlx::test]
async fn test_initialize_enrollment(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (client, pool) = make_client_with_db(pool).await;

    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // create user with password
    let new_user = AddUserData {
        username: "adumbledore".into(),
        last_name: "Dumbledore".into(),
        first_name: "Albus".into(),
        email: "a.dumbledore@hogwart.edu.uk".into(),
        phone: Some("1234".into()),
        password: Some("Password1234543$!".into()),
    };
    let response = client.post("/api/v1/user").json(&new_user).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // verify enrollment token was not created
    let enrollments = Token::fetch_all(&pool).await.unwrap();
    assert_eq!(enrollments.len(), 0);

    // try to start enrollment
    let response = client
        .post("/api/v1/user/adumbledore/start_enrollment")
        .json(&json!({}))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // create user without password
    #[derive(Deserialize)]
    struct StartEnrollmentResponse {
        enrollment_token: String,
    }
    let new_user = AddUserData {
        username: "adumbledore2".into(),
        last_name: "Dumbledore".into(),
        first_name: "Albus".into(),
        email: "a.dumbledore2@hogwart.edu.uk".into(),
        phone: Some("1234".into()),
        password: None,
    };
    let response = client.post("/api/v1/user").json(&new_user).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // verify enrollment token was not created
    let enrollments = Token::fetch_all(&pool).await.unwrap();
    assert_eq!(enrollments.len(), 0);

    // try to start enrollment
    let response = client
        .post("/api/v1/user/adumbledore2/start_enrollment")
        .json(&json!({}))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let response: StartEnrollmentResponse = response.json().await;

    // verify enrollment token was created
    let enrollment = Token::find_by_id(&pool, &response.enrollment_token)
        .await
        .unwrap();
    assert_eq!(enrollment.user_id, 4);
    assert_eq!(enrollment.admin_id, Some(1));
    assert_eq!(enrollment.used_at, None);
}

#[sqlx::test]
async fn test_enroll_disabled_user(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (client, _) = make_client_with_db(pool).await;

    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    let new_user = AddUserData {
        username: "adumbledore".into(),
        last_name: "Dumbledore".into(),
        first_name: "Albus".into(),
        email: "a.dumbledore@hogwart.edu.uk".into(),
        phone: Some("1234".into()),
        password: None,
    };
    let response = client.post("/api/v1/user").json(&new_user).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);

    let mut user_details = fetch_user_details(&client, "adumbledore").await;
    user_details.user.is_active = false;
    let response = client
        .put(format!("/api/v1/user/{}", "adumbledore"))
        .json(&user_details.user)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);

    // enrollment should fail, because user is disabled
    let response = client
        .post("/api/v1/user/adumbledore/start_enrollment")
        .json(&json!({}))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn test_enrollment_pending_unset_for_regular_user(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;

    let (client, pool) = make_client_with_db(pool).await;

    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // create user with password
    let new_user = AddUserData {
        username: "adumbledore".into(),
        last_name: "Dumbledore".into(),
        first_name: "Albus".into(),
        email: "a.dumbledore@hogwart.edu.uk".into(),
        phone: Some("1234".into()),
        password: Some("Password1234543$!".into()),
    };
    let response = client.post("/api/v1/user").json(&new_user).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);

    let user = User::find_by_username(&pool, &new_user.username)
        .await
        .unwrap()
        .unwrap();

    // verify enrollment_pending flag is not set
    assert!(!user.enrollment_pending);

    // verify user is considered enrolled
    assert!(user.is_enrolled());
}

#[sqlx::test]
async fn test_request_enrollment(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (client, pool) = make_client_with_db(pool).await;

    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // create user without password
    let new_user = AddUserData {
        username: "adumbledore".into(),
        last_name: "Dumbledore".into(),
        first_name: "Albus".into(),
        email: "a.dumbledore@hogwart.edu.uk".into(),
        phone: Some("1234".into()),
        password: None,
    };
    let response = client.post("/api/v1/user").json(&new_user).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);

    let user = User::find_by_username(&pool, &new_user.username)
        .await
        .unwrap()
        .unwrap();

    // verify enrollment token was not created
    let tokens = Token::fetch_all(&pool).await.unwrap();
    assert_eq!(tokens.len(), 0);

    // verify enrollment variables
    assert!(!user.enrollment_pending);
    assert!(!user.is_enrolled());

    // request enrollment
    let response = client
        .post(format!("/api/v1/user/{}/start_enrollment", user.username))
        .json(&json!({"email": user.email, "send_enrollment_notification": false}))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // re-fetch the user
    let user = User::find_by_username(&pool, &new_user.username)
        .await
        .unwrap()
        .unwrap();

    // verify enrollment variables
    assert!(user.enrollment_pending);
    assert!(!user.is_enrolled());

    // verify enrollment token was created correctly
    let tokens = Token::fetch_all(&pool).await.unwrap();
    assert_eq!(tokens.len(), 1);
    let token = tokens.first().unwrap();
    assert!(token.used_at.is_none());
}

#[sqlx::test]
async fn test_enrollment_token_expiration_time(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (client, pool) = make_client_with_db(pool).await;

    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // create user without password
    let new_user = AddUserData {
        username: "adumbledore".into(),
        last_name: "Dumbledore".into(),
        first_name: "Albus".into(),
        email: "a.dumbledore@hogwart.edu.uk".into(),
        phone: Some("1234".into()),
        password: None,
    };
    let response = client.post("/api/v1/user").json(&new_user).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);

    let user = User::find_by_username(&pool, &new_user.username)
        .await
        .unwrap()
        .unwrap();

    // verify enrollment token was not created
    let tokens = Token::fetch_all(&pool).await.unwrap();
    assert_eq!(tokens.len(), 0);

    // request enrollment
    let response = client
        .post(format!("/api/v1/user/{}/start_enrollment", user.username))
        .json(&json!({"email": user.email, "send_enrollment_notification": false}))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // verify enrollment token was created with default expiration time (24h)
    let tokens = Token::fetch_all(&pool).await.unwrap();
    assert_eq!(tokens.len(), 1);
    let token = tokens.first().unwrap();
    assert_eq!(token.expires_at, token.created_at + Duration::hours(24));

    // request enrollment with different expiration time
    let response = client
        .post(format!("/api/v1/user/{}/start_enrollment", user.username))
        .json(&json!({"email": user.email, "send_enrollment_notification": false, "token_expiration_time": "3d"}))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // verify enrollment token was created with default expiration time (24h)
    let tokens = Token::fetch_all(&pool).await.unwrap();
    assert_eq!(tokens.len(), 1);
    let token = tokens.first().unwrap();
    assert_eq!(token.expires_at, token.created_at + Duration::hours(72));

    // request enrollment with different expiration time
    let response = client
        .post(format!("/api/v1/user/{}/start_enrollment", user.username))
        .json(&json!({"email": user.email, "send_enrollment_notification": false, "token_expiration_time": "1w"}))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // verify enrollment token was created with default expiration time (24h)
    let tokens = Token::fetch_all(&pool).await.unwrap();
    assert_eq!(tokens.len(), 1);
    let token = tokens.first().unwrap();
    assert_eq!(token.expires_at, token.created_at + Duration::days(7));

    // request enrollment with different expiration time
    let response = client
        .post(format!("/api/v1/user/{}/start_enrollment", user.username))
        .json(&json!({"email": user.email, "send_enrollment_notification": false, "token_expiration_time": "2h"}))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // verify enrollment token was created with default expiration time (24h)
    let tokens = Token::fetch_all(&pool).await.unwrap();
    assert_eq!(tokens.len(), 1);
    let token = tokens.first().unwrap();
    assert_eq!(token.expires_at, token.created_at + Duration::hours(2));
}

#[sqlx::test]
async fn test_enrollment_pending_unset_for_desktop_client(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;

    let (client, pool) = make_client_with_db(pool).await;

    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // create user with password
    let new_user = AddUserData {
        username: "adumbledore".into(),
        last_name: "Dumbledore".into(),
        first_name: "Albus".into(),
        email: "a.dumbledore@hogwart.edu.uk".into(),
        phone: Some("1234".into()),
        password: Some("Password1234543$!".into()),
    };
    let response = client.post("/api/v1/user").json(&new_user).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // verify enrollment token was not created
    let tokens = Token::fetch_all(&pool).await.unwrap();
    assert_eq!(tokens.len(), 0);

    let user = User::find_by_username(&pool, &new_user.username)
        .await
        .unwrap()
        .unwrap();

    // verify enrollment variables
    assert!(!user.enrollment_pending);
    assert!(user.is_enrolled());

    // request device configuration
    let response = client
        .post("/api/v1/user/adumbledore/start_desktop")
        .json(&json!({
            "username": user.username,
            "email": user.email,
            "send_enrollment_notification": false
        }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // verify enrollment token was created correctly
    let tokens = Token::fetch_all(&pool).await.unwrap();
    assert_eq!(tokens.len(), 1);
    let token = tokens.first().unwrap();
    assert!(token.used_at.is_none());

    // verify enrollment variables
    assert!(!user.enrollment_pending);
    assert!(user.is_enrolled());
}

/// An LDAP-synced user (from_ldap=true, ldap_remote_enrollment_completed=false) is
/// listed as **enrolled** by the API when `ldap_remote_enrollment_enabled` is disabled
/// (the default / legacy behaviour).
#[sqlx::test]
async fn test_ldap_user_enrolled_via_api_when_remote_enrollment_disabled(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let (client, pool) = make_client_with_db(pool).await;

    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // Create user without a password (simulates an LDAP-synced account).
    let new_user = AddUserData {
        username: "ldapuser_b1".into(),
        last_name: "User".into(),
        first_name: "Ldap".into(),
        email: "ldapuser_b1@example.com".into(),
        phone: None,
        password: None,
    };
    let response = client.post("/api/v1/user").json(&new_user).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // Mark the user as LDAP-sourced directly in the DB (the API doesn't expose from_ldap).
    let mut user = User::find_by_username(&pool, &new_user.username)
        .await
        .unwrap()
        .unwrap();
    user.from_ldap = true;
    user.ldap_remote_enrollment_completed = false;
    user.save(&pool).await.unwrap();

    // ldap_remote_enrollment_enabled is false by default — no settings change needed.
    let details = fetch_user_details(&client, &new_user.username).await;
    assert!(
        details.user.enrolled,
        "LDAP user should be reported as enrolled by the API when remote enrollment is disabled"
    );
}

/// An LDAP-synced user (from_ldap=true, ldap_remote_enrollment_completed=false) is
/// listed as **not enrolled** by the API when `ldap_remote_enrollment_enabled` is enabled.
#[sqlx::test]
async fn test_ldap_user_not_enrolled_via_api_when_remote_enrollment_enabled(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let (client, pool) = make_client_with_db(pool).await;

    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // Create user without a password (simulates an LDAP-synced account).
    let new_user = AddUserData {
        username: "ldapuser_b2".into(),
        last_name: "User".into(),
        first_name: "Ldap".into(),
        email: "ldapuser_b2@example.com".into(),
        phone: None,
        password: None,
    };
    let response = client.post("/api/v1/user").json(&new_user).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // Mark the user as LDAP-sourced directly in the DB.
    let mut user = User::find_by_username(&pool, &new_user.username)
        .await
        .unwrap()
        .unwrap();
    user.from_ldap = true;
    user.ldap_remote_enrollment_completed = false;
    user.save(&pool).await.unwrap();

    // Enable LDAP remote enrollment directly in the DB, bypassing HTTP API
    // validation (which would require LDAP and SMTP to be fully configured).
    let mut settings = Settings::get_current_settings();
    configure_smtp(&mut settings);
    configure_ldap(&mut settings);
    settings.ldap_remote_enrollment_enabled = true;
    update_current_settings(&pool, settings).await.unwrap();

    let details = fetch_user_details(&client, &new_user.username).await;
    assert!(
        !details.user.enrolled,
        "LDAP user should not be reported as enrolled by the API when remote enrollment is enabled but not completed"
    );
}
