use std::time::SystemTime;

use defguard_common::db::models::{
    Settings,
    user::{TOTP_CODE_DIGITS, TOTP_CODE_VALIDITY_PERIOD},
};
use defguard_core::handlers::{Auth, AuthCode, AuthTotp};
use reqwest::StatusCode;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use totp_lite::{Sha1, totp_custom};

use super::common::{
    X_FORWARDED_HOST, X_FORWARDED_URI, make_client, make_client_with_db, setup_pool,
};

#[sqlx::test]
async fn test_forward_auth(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let mut client = make_client(pool).await;

    // auth request from reverse proxy
    let response = client
        .get("/api/v1/forward_auth")
        .header(X_FORWARDED_HOST, "app.example.com")
        .header(X_FORWARDED_URI, "/test")
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::TEMPORARY_REDIRECT);
    let headers = response.headers();
    let url = Settings::url().unwrap();
    assert_eq!(
        headers.get("location").unwrap().to_str().unwrap(),
        format!("{}auth/login?r={}", url, "http://app.example.com/test")
    );

    // login
    let auth = Auth::new("hpotter", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // store auth cookie for later use
    let auth_cookie = response
        .cookies()
        .find(|c| c.name() == "defguard_session")
        .unwrap();

    // make another auth request after logging in
    client.set_cookie(&auth_cookie);
    let response = client
        .get("/api/v1/forward_auth")
        .header(X_FORWARDED_HOST, "app.example.com")
        .header(X_FORWARDED_URI, "/test")
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
}

fn totp_code(auth_totp: &AuthTotp) -> AuthCode {
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    let secret = base32::decode(
        base32::Alphabet::Rfc4648 { padding: false },
        &auth_totp.secret,
    )
    .unwrap();
    let code = totp_custom::<Sha1>(
        TOTP_CODE_VALIDITY_PERIOD,
        TOTP_CODE_DIGITS,
        &secret,
        timestamp.as_secs(),
    );
    AuthCode::new(code)
}

#[sqlx::test]
async fn test_forward_auth_mfa_not_completed(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (client, _pool) = make_client_with_db(pool).await;

    // login as hpotter
    let auth = Auth::new("hpotter", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // enable TOTP
    let response = client.post("/api/v1/auth/totp/init").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let auth_totp: AuthTotp = response.json().await;
    let code = totp_code(&auth_totp);
    let response = client.post("/api/v1/auth/totp").json(&code).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // enable MFA
    let response = client.put("/api/v1/auth/mfa").send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // logout
    let response = client.post("/api/v1/auth/logout").send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // login password-only — MFA now required, session is PasswordVerified
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // forward_auth with PasswordVerified session must redirect, not accept
    let response = client
        .get("/api/v1/forward_auth")
        .header(X_FORWARDED_HOST, "app.example.com")
        .header(X_FORWARDED_URI, "/test")
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::TEMPORARY_REDIRECT);
}

#[sqlx::test]
async fn test_forward_auth_mfa_completed(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (client, _pool) = make_client_with_db(pool).await;

    // login as hpotter
    let auth = Auth::new("hpotter", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // enable TOTP
    let response = client.post("/api/v1/auth/totp/init").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let auth_totp: AuthTotp = response.json().await;
    let code = totp_code(&auth_totp);
    let response = client.post("/api/v1/auth/totp").json(&code).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // enable MFA
    let response = client.put("/api/v1/auth/mfa").send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // logout
    let response = client.post("/api/v1/auth/logout").send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // login password-only — MFA required
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // complete MFA with valid TOTP code
    let code = totp_code(&auth_totp);
    let response = client
        .post("/api/v1/auth/totp/verify")
        .json(&code)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    // forward_auth with MultiFactorVerified session must accept
    let response = client
        .get("/api/v1/forward_auth")
        .header(X_FORWARDED_HOST, "app.example.com")
        .header(X_FORWARDED_URI, "/test")
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
}
