use std::time::SystemTime;

use chrono::DateTime;
use defguard_common::db::models::{
    MFAInfo, MFAMethod, User,
    user::{TOTP_CODE_DIGITS, TOTP_CODE_VALIDITY_PERIOD},
};
use defguard_core::{
    events::ApiEventType,
    handlers::{Auth, AuthCode, AuthResponse, AuthTotp, user::UserDetails},
};
use reqwest::{StatusCode, header::USER_AGENT};
use serde::Deserialize;
use serde_json::json;
use sqlx::{
    postgres::{PgConnectOptions, PgPoolOptions},
    query,
};
use totp_lite::{Sha1, totp_custom};
use webauthn_authenticator_rs::{WebauthnAuthenticator, prelude::Url, softpasskey::SoftPasskey};
use webauthn_rs::prelude::{CreationChallengeResponse, RequestChallengeResponse};

use super::common::{
    X_FORWARDED_FOR, fetch_user_details, make_client, make_client_with_db, make_test_client,
    setup_pool,
};
use crate::api::common::client::TestResponse;

static SESSION_COOKIE_NAME: &str = "defguard_session";

#[derive(Deserialize)]
pub struct RecoveryCodes {
    codes: Option<Vec<String>>,
}

#[sqlx::test]
async fn dg25_19_clickjacking_vulnerability(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let client = make_client(pool).await;

    let response = client.get("/").send().await;
    let headers = response.headers();
    let csp_header = headers.get("content-security-policy").unwrap();
    let csp_value = csp_header.to_str().unwrap();
    assert!(
        csp_value.contains("frame-ancestors 'none'"),
        "CSP header should block all iframes with 'none' directive"
    );
}

#[sqlx::test]
async fn test_logout(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let mut client = make_client(pool).await;

    let auth = Auth::new("hpotter", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    client.verify_api_events_with_user(&[(ApiEventType::UserLogin, 2, "hpotter")]);

    // store auth cookie for later use
    let auth_cookie = response
        .cookies()
        .find(|c| c.name() == SESSION_COOKIE_NAME)
        .unwrap();

    let response = client.get("/api/v1/me").send().await;
    assert_eq!(response.status(), StatusCode::OK);

    let response = client.post("/api/v1/auth/logout").send().await;
    assert_eq!(response.status(), StatusCode::OK);

    let response = client.get("/api/v1/me").send().await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    client.verify_api_events_with_user(&[(ApiEventType::UserLogout, 2, "hpotter")]);

    // try reusing auth cookie
    client.set_cookie(&auth_cookie);
    let response = client.get("/api/v1/me").send().await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn test_login_bruteforce(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let mut client = make_client(pool).await;

    let invalid_auth = Auth::new("hpotter", "invalid");

    // fail login 5 times in a row
    for i in 0..6 {
        let response = client.post("/api/v1/auth").json(&invalid_auth).send().await;
        if i == 5 {
            assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
            client.assert_event_queue_is_empty();
        } else {
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
            client.verify_api_events(&[ApiEventType::UserLoginFailed {
                message: "Authentication for hpotter failed: invalid password".into(),
            }]);
        }
    }
}

async fn responses_eq(response1: TestResponse, response2: TestResponse) -> bool {
    // omit date header
    let mut headers1 = response1.headers().clone();
    headers1.remove("date");
    let mut headers2 = response2.headers().clone();
    headers2.remove("date");
    let headers = headers1 == headers2;

    let status = response1.status() == response2.status();
    let body = response1.bytes().await == response2.bytes().await;

    status && headers && body
}

#[sqlx::test]
async fn dg25_21_test_login_enumeration(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let client = make_client(pool).await;

    let user_auth = Auth::new("hpotter", "pass123");
    let admin_auth = Auth::new("admin", "pass123");

    let response = client.post("/api/v1/auth").json(&admin_auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    let response = client.post("/api/v1/auth").json(&user_auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // regular user
    let user_auth = Auth::new("hpotter", "invalid");
    let response_existing_user = client.post("/api/v1/auth").json(&user_auth).send().await;
    let user_auth = Auth::new("nothpotter", "invalid");
    let response_nonexisting_user = client.post("/api/v1/auth").json(&user_auth).send().await;
    assert!(responses_eq(response_existing_user, response_nonexisting_user).await);

    // admin user
    let user_auth = Auth::new("admin", "invalid");
    let response_existing_user = client.post("/api/v1/auth").json(&user_auth).send().await;
    let user_auth = Auth::new("notadmin", "invalid");
    let response_nonexisting_user = client.post("/api/v1/auth").json(&user_auth).send().await;
    assert!(responses_eq(response_existing_user, response_nonexisting_user).await);

    // response for admin = response for regular user
    let user_auth = Auth::new("admin", "invalid");
    let response_existing_user = client.post("/api/v1/auth").json(&user_auth).send().await;
    let user_auth = Auth::new("hpotter", "invalid");
    let response_nonexisting_user = client.post("/api/v1/auth").json(&user_auth).send().await;
    assert!(responses_eq(response_existing_user, response_nonexisting_user).await);
}

#[sqlx::test]
async fn test_login_disabled(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let client = make_client(pool).await;

    let user_auth = Auth::new("hpotter", "pass123");
    let admin_auth = Auth::new("admin", "pass123");

    let response = client.post("/api/v1/auth").json(&admin_auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    let mut user_details = fetch_user_details(&client, "hpotter").await;
    user_details.user.is_active = false;
    let response = client
        .put("/api/v1/user/hpotter")
        .json(&user_details.user)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);

    let response = client.post("/api/v1/auth").json(&user_auth).send().await;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    client.post("/api/v1/auth").json(&admin_auth).send().await;
    let mut user_details = fetch_user_details(&client, "hpotter").await;
    user_details.user.is_active = true;
    let response = client
        .put("/api/v1/user/hpotter")
        .json(&user_details.user)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);

    let response = client.post("/api/v1/auth").json(&user_auth).send().await;

    assert_eq!(response.status(), StatusCode::OK);
}

#[sqlx::test]
async fn test_cannot_enable_mfa(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let client = make_client(pool).await;

    let auth = Auth::new("hpotter", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // enable MFA
    let response = client.put("/api/v1/auth/mfa").send().await;
    assert_eq!(response.status(), StatusCode::NOT_MODIFIED);
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
async fn test_totp(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let client = make_client(pool).await;

    // login
    let auth = Auth::new("hpotter", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // new TOTP secret
    let response = client.post("/api/v1/auth/totp/init").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let auth_totp: AuthTotp = response.json().await;

    // enable TOTP
    let code = totp_code(&auth_totp);
    let response = client.post("/api/v1/auth/totp").json(&code).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // check recovery codes
    let recovery_codes: RecoveryCodes = response.json().await;
    assert_eq!(recovery_codes.codes.as_ref().unwrap().len(), 8); // RECOVERY_CODES_COUNT

    // enable MFA
    let response = client.put("/api/v1/auth/mfa").send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // login again, this time a different status code is returned
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // still unauthorized
    let response = client.get("/api/v1/me").send().await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // provide wrong TOTP code
    let code = AuthCode::new("0");
    let response = client
        .post("/api/v1/auth/totp/verify")
        .json(&code)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // provide recovery code
    let code = recovery_codes.codes.unwrap().first().unwrap().clone();
    let response = client
        .post("/api/v1/auth/recovery")
        .json(&json!({ "code": code }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.json::<AuthResponse>().await.user.username,
        "hpotter"
    );

    // authorized
    let response = client.get("/api/v1/me").send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // logout
    let response = client.post("/api/v1/auth/logout").send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // login
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // reuse the same recovery code - shouldn't work
    let response = client
        .post("/api/v1/auth/recovery")
        .json(&json!({ "code": code }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // logout
    let response = client.post("/api/v1/auth/logout").send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // login
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // provide correct TOTP code
    let code = totp_code(&auth_totp);
    let response = client
        .post("/api/v1/auth/totp/verify")
        .json(&code)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    // authorized
    let response = client.get("/api/v1/me").send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // disable MFA
    let response = client.delete("/api/v1/auth/mfa").send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // login again
    let auth = Auth::new("hpotter", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);
}

#[sqlx::test]
async fn dg25_15_test_totp_brute_force(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let client = make_client(pool).await;

    // login
    let auth = Auth::new("hpotter", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // new TOTP secret
    let response = client.post("/api/v1/auth/totp/init").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let auth_totp: AuthTotp = response.json().await;

    // enable TOTP
    let code = totp_code(&auth_totp);
    let response = client.post("/api/v1/auth/totp").json(&code).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // enable MFA
    let response = client.put("/api/v1/auth/mfa").send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // login again, this time a different status code is returned
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // provide wrong TOTP code more than 5 times in a row
    let code = AuthCode::new("0");
    for i in 0..10 {
        let response = client
            .post("/api/v1/auth/totp/verify")
            .json(&code)
            .send()
            .await;
        if i >= 5 {
            assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        } else {
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        }
    }
}

// static EMAIL_CODE_REGEX: &str = r"<b>(?<code>\d{6})</b>";
// fn extract_email_code(content: &str) -> &str {
//     let re = regex::Regex::new(EMAIL_CODE_REGEX).unwrap();
//     re.captures(content).unwrap().name("code").unwrap().as_str()
// }

/*
#[sqlx::test]
async fn test_email_mfa(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (client, state) = make_test_client(pool).await;
    let pool = state.pool;

    // try to initialize email MFA setup before logging in
    let response = client.post("/api/v1/auth/email/init").send().await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // login
    let auth = Auth::new("hpotter", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // try to initialize email MFA setup without SMTP settings configured
    let response = client.post("/api/v1/auth/email/init").send().await;
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

    // add dummy SMTP settings
    let mut settings = Settings::get_current_settings();
    settings.smtp_server = Some("smtp_server".into());
    settings.smtp_port = Some(587);
    settings.smtp_sender = Some("smtp@sender.pl".into());
    update_current_settings(&pool, settings).await.unwrap();

    // initialize email MFA setup
    let response = client.post("/api/v1/auth/email/init").send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // check email was sent
    let mail = mail_rx.try_recv().unwrap();
    assert_ok!(mail_rx.try_recv());
    assert_eq!(mail.to(), "h.potter@hogwart.edu.uk");
    assert_eq!(
        mail.subject(),
        "Defguard: new device logged in to your account"
    );

    // resend setup email
    let response = client.post("/api/v1/auth/email/init").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let mail = mail_rx.try_recv().unwrap();
    assert_err!(mail_rx.try_recv());
    assert_eq!(mail.to(), "h.potter@hogwart.edu.uk");
    assert_eq!(
        mail.subject(),
        "Defguard: Multi-Factor Authentication activation"
    );
    let code = extract_email_code(mail.content());

    // finish setup
    let code = AuthCode::new(code);
    let response = client.post("/api/v1/auth/email").json(&code).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // check that confirmation email was sent
    let mail = mail_rx.try_recv().unwrap();
    assert_err!(mail_rx.try_recv());
    assert_eq!(mail.to(), "h.potter@hogwart.edu.uk");
    assert_eq!(
        mail.subject(),
        "MFA method Email has been activated on your account"
    );

    // check recovery codes
    let recovery_codes: RecoveryCodes = response.json().await;
    assert_eq!(recovery_codes.codes.as_ref().unwrap().len(), 8); // RECOVERY_CODES_COUNT

    // enable MFA
    let response = client.put("/api/v1/auth/mfa").send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // login again, this time a different status code is returned
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // still unauthorized
    let response = client.get("/api/v1/me").send().await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // provide wrong code
    let code = AuthCode::new("0");
    let response = client
        .post("/api/v1/auth/email/verify")
        .json(&code)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // still unauthorized
    let response = client.get("/api/v1/me").send().await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // request code
    let response = client.get("/api/v1/auth/email").send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // check that code email was sent
    let mail = mail_rx.try_recv().unwrap();
    assert_ok!(mail_rx.try_recv());
    assert_eq!(mail.to(), "h.potter@hogwart.edu.uk");
    assert_eq!(
        mail.subject(),
        "Defguard: new device logged in to your account"
    );

    // resend code
    let response = client.get("/api/v1/auth/email").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let mail = mail_rx.try_recv().unwrap();
    assert_err!(mail_rx.try_recv());
    assert_eq!(mail.to(), "h.potter@hogwart.edu.uk");
    assert_eq!(
        mail.subject(),
        "Defguard: Multi-Factor Authentication code for login"
    );
    let code = extract_email_code(mail.content());

    // login
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // provide correct code
    let code = AuthCode::new(code);
    let response = client
        .post("/api/v1/auth/email/verify")
        .json(&code)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    // authorized
    let response = client.get("/api/v1/me").send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // disable MFA
    let response = client.delete("/api/v1/auth/mfa").send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // login again
    let auth = Auth::new("hpotter", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);
}
*/

/*
#[sqlx::test]
async fn dg25_15_test_email_mfa_brute_force(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (client, state) = make_test_client(pool).await;
    let pool = state.pool;

    // try to initialize email MFA setup before logging in
    let response = client.post("/api/v1/auth/email/init").send().await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // login
    let auth = Auth::new("hpotter", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);
    // remove login confirmation email from queue
    let _mail = mail_rx.try_recv().unwrap();

    // add dummy SMTP settings
    let mut settings = Settings::get_current_settings();
    settings.smtp_server = Some("smtp_server".into());
    settings.smtp_port = Some(587);
    settings.smtp_sender = Some("smtp@sender.pl".into());
    update_current_settings(&pool, settings).await.unwrap();

    // initialize email MFA setup
    let response = client.post("/api/v1/auth/email/init").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let mail = mail_rx.try_recv().unwrap();
    assert_eq!(mail.to(), "h.potter@hogwart.edu.uk");
    assert_eq!(
        mail.subject(),
        "Defguard: Multi-Factor Authentication activation"
    );
    let code = extract_email_code(mail.content());

    // finish setup
    let code = AuthCode::new(code);
    let response = client.post("/api/v1/auth/email").json(&code).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // enable MFA
    let response = client.put("/api/v1/auth/mfa").send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // login again, this time a different status code is returned
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // provide wrong code more than 5 times in a row
    let code = AuthCode::new("0");
    for i in 0..10 {
        let response = client
            .post("/api/v1/auth/email/verify")
            .json(&code)
            .send()
            .await;
        if i >= 5 {
            assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        } else {
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        }
    }
}
*/

#[sqlx::test]
async fn test_webauthn(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (client, pool) = make_client_with_db(pool).await;

    let mut authenticator = WebauthnAuthenticator::new(SoftPasskey::new(true));
    let origin = Url::parse(&client.base_url()).unwrap();

    // login
    let auth = Auth::new("hpotter", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // WebAuthn registration
    let response = client.post("/api/v1/auth/webauthn/init").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let ccr: CreationChallengeResponse = response.json().await;
    let rpkc = authenticator.do_registration(origin.clone(), ccr).unwrap();
    let response = client
        .post("/api/v1/auth/webauthn/finish")
        .json(&json!({
            "name": "My security key",
            "rpkc": &rpkc
        }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    // check recovery codes
    let recovery_codes: RecoveryCodes = response.json().await;
    assert_eq!(recovery_codes.codes.unwrap().len(), 8); // RECOVERY_CODES_COUNT

    // enable MFA
    let response = client.put("/api/v1/auth/mfa").send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // login again
    let auth = Auth::new("hpotter", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // WebAuthn authentication
    let response = client.post("/api/v1/auth/webauthn/start").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let rcr: RequestChallengeResponse = response.json().await;
    let pkc = authenticator.do_authentication(origin, rcr).unwrap();
    let response = client.post("/api/v1/auth/webauthn").json(&pkc).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // get security keys
    let response = client.get("/api/v1/user/hpotter").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let user_info: UserDetails = response.json().await;
    assert_eq!(user_info.security_keys.len(), 1);

    // delete security key
    let response = client
        .delete(format!(
            "/api/v1/user/hpotter/security_key/{}",
            user_info.security_keys[0].id
        ))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    // login again
    let auth = Auth::new("hpotter", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // check that recovery codes were cleared since last MFA method was removed
    let record = query!(
        "SELECT recovery_codes FROM \"user\" WHERE id = $1",
        user_info.user.id,
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(record.recovery_codes.len(), 0);
}

#[sqlx::test]
async fn test_cannot_skip_otp_by_adding_yubikey(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let client = make_client(pool).await;

    // login
    let auth = Auth::new("hpotter", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // new TOTP secret
    let response = client.post("/api/v1/auth/totp/init").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let auth_totp: AuthTotp = response.json().await;

    // enable TOTP
    let code = totp_code(&auth_totp);
    let response = client.post("/api/v1/auth/totp").json(&code).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // enable MFA
    let response = client.put("/api/v1/auth/mfa").send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // login again, this time a different status code is returned
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // instead of continuing TOTP login try to add a new YubiKey
    let response = client.post("/api/v1/auth/webauthn/init").send().await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn test_cannot_skip_security_key_by_adding_yubikey(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;

    let client = make_client(pool).await;

    let mut authenticator = WebauthnAuthenticator::new(SoftPasskey::new(true));
    let origin = Url::parse(&client.base_url()).unwrap();

    // login
    let auth = Auth::new("hpotter", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // WebAuthn registration
    let response = client.post("/api/v1/auth/webauthn/init").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let ccr: CreationChallengeResponse = response.json().await;
    let rpkc = authenticator.do_registration(origin.clone(), ccr).unwrap();
    let response = client
        .post("/api/v1/auth/webauthn/finish")
        .json(&json!({
            "name": "My security key",
            "rpkc": &rpkc
        }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    // enable MFA
    let response = client.put("/api/v1/auth/mfa").send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // login again
    let auth = Auth::new("hpotter", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // instead of continuing TOTP login try to add a new YubiKey
    let response = client.post("/api/v1/auth/webauthn/init").send().await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn test_mfa_method_is_updated_when_removing_last_webauthn_passkey(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;

    let client = make_client(pool).await;

    // login
    let auth = Auth::new("hpotter", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // new TOTP secret
    let response = client.post("/api/v1/auth/totp/init").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let auth_totp: AuthTotp = response.json().await;

    // enable TOTP
    let code = totp_code(&auth_totp);
    let response = client.post("/api/v1/auth/totp").json(&code).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // check recovery codes
    let recovery_codes: RecoveryCodes = response.json().await;
    assert_eq!(recovery_codes.codes.as_ref().unwrap().len(), 8); // RECOVERY_CODES_COUNT

    // enable MFA
    let response = client.put("/api/v1/auth/mfa").send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // login again, this time a different status code is returned
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // provide correct TOTP code
    let code = totp_code(&auth_totp);
    let response = client
        .post("/api/v1/auth/totp/verify")
        .json(&code)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    // WebAuthn registration
    let mut authenticator = WebauthnAuthenticator::new(SoftPasskey::new(true));
    let origin = Url::parse(&client.base_url()).unwrap();

    let response = client.post("/api/v1/auth/webauthn/init").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let ccr: CreationChallengeResponse = response.json().await;
    let rpkc = authenticator.do_registration(origin.clone(), ccr).unwrap();
    let response = client
        .post("/api/v1/auth/webauthn/finish")
        .json(&json!({
            "name": "My security key",
            "rpkc": &rpkc
        }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    // get user info
    let response = client.get("/api/v1/user/hpotter").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let mut user_info: UserDetails = response.json().await;

    // set default MFA method
    user_info.user.mfa_method = MFAMethod::Webauthn;
    let response = client
        .put("/api/v1/user/hpotter")
        .json(&user_info.user)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    // delete security key
    let response = client
        .delete(format!(
            "/api/v1/user/hpotter/security_key/{}",
            user_info.security_keys[0].id
        ))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    // login again
    let auth = Auth::new("hpotter", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // verify that MFA method was updated
    let mfa_info: MFAInfo = response.json().await;
    assert_eq!(mfa_info.current_mfa_method(), &MFAMethod::OneTimePassword);
}

/*
#[sqlx::test]
async fn test_mfa_method_totp_enabled_mail(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (client, state) = make_test_client(pool).await;
    let user_agent_header = "Mozilla/5.0 (iPhone; CPU iPhone OS 17_1 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.1 Mobile/15E148 Safari/604.1";

    // login
    let auth = Auth::new("hpotter", "pass123");
    let response = client
        .post("/api/v1/auth")
        .header(USER_AGENT, user_agent_header)
        .json(&auth)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    // new TOTP secret
    let response = client.post("/api/v1/auth/totp/init").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let auth_totp: AuthTotp = response.json().await;

    // enable TOTP
    let code = totp_code(&auth_totp);
    let response = client.post("/api/v1/auth/totp").json(&code).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    mail_rx.try_recv().unwrap();
    let mail = mail_rx.try_recv().unwrap();
    assert_eq!(mail.to(), "h.potter@hogwart.edu.uk");
    assert_eq!(
        mail.subject(),
        "MFA method TOTP has been activated on your account"
    );
    assert!(mail.content().contains("IP Address:</span> 127.0.0.1"));
    assert!(
        mail.content()
            .contains("Device type:</span> iPhone, OS: iOS 17.1, Mobile Safari")
    );
}
*/

#[sqlx::test]
async fn test_new_device_login(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (client, _) = make_test_client(pool).await;
    let user_agent_header_iphone = "Mozilla/5.0 (iPhone; CPU iPhone OS 17_1 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.1 Mobile/15E148 Safari/604.1";
    let user_agent_header_android = "Mozilla/5.0 (Linux; Android 7.0; SM-G930VC Build/NRD90M; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/58.0.3029.83 Mobile Safari/537.36";

    // login
    let auth = Auth::new("hpotter", "pass123");
    let response = client
        .post("/api/v1/auth")
        .header(USER_AGENT, user_agent_header_iphone)
        .json(&auth)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    // assert_eq!(mail.to(), "h.potter@hogwart.edu.uk");
    // assert_eq!(
    //     mail.subject(),
    //     "Defguard: new device logged in to your account"
    // );
    // assert!(mail.content().contains("IP Address:</span> 127.0.0.1"));
    // assert!(
    //     mail.content()
    //         .contains("Device type:</span> iPhone, OS: iOS 17.1, Mobile Safari")
    // );

    let response = client.post("/api/v1/auth/logout").send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // login using the same device
    let auth = Auth::new("hpotter", "pass123");
    let response = client
        .post("/api/v1/auth")
        .header(USER_AGENT, user_agent_header_iphone)
        .json(&auth)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    // login using a different device
    let auth = Auth::new("hpotter", "pass123");
    let response = client
        .post("/api/v1/auth")
        .header(USER_AGENT, user_agent_header_android)
        .json(&auth)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    // assert_eq!(
    //     mail.subject(),
    //     "Defguard: new device logged in to your account"
    // );
    // assert!(mail.content().contains("IP Address:</span> 127.0.0.1"));
    // assert!(
    //     mail.content()
    //         .contains("Device type:</span> SM-G930VC, OS: Android 7.0, Chrome Mobile WebView")
    // );
}

#[sqlx::test]
async fn test_login_ip_headers(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (client, _) = make_test_client(pool).await;
    let user_agent_header_iphone = "Mozilla/5.0 (iPhone; CPU iPhone OS 17_1 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.1 Mobile/15E148 Safari/604.1";

    // Works with X-Forwarded-For header
    let auth = Auth::new("hpotter", "pass123");
    let response = client
        .post("/api/v1/auth")
        .header(USER_AGENT, user_agent_header_iphone)
        .header(X_FORWARDED_FOR, "10.0.0.20, 10.1.1.10")
        .json(&auth)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    // assert_eq!(mail.to(), "h.potter@hogwart.edu.uk");
    // assert_eq!(
    //     mail.subject(),
    //     "Defguard: new device logged in to your account"
    // );
    // assert!(mail.content().contains("IP Address:</span> 10.0.0.20"));
}

#[sqlx::test]
async fn test_session_cookie(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (client, pool) = make_client_with_db(pool).await;

    let auth = Auth::new("hpotter", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    let auth_cookie = response
        .cookies()
        .find(|c| c.name() == SESSION_COOKIE_NAME)
        .unwrap();

    let session_id = auth_cookie.value();

    // Forcibly expire the session
    query!(
        "UPDATE session SET expires = $1 WHERE id = $2",
        DateTime::UNIX_EPOCH.naive_utc(),
        session_id
    )
    .execute(&pool)
    .await
    .unwrap();

    let response = client.get("/api/v1/me").send().await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let auth_cookie = response.cookies().find(|c| c.name() == SESSION_COOKIE_NAME);
    assert!(auth_cookie.is_none());
}

#[sqlx::test]
async fn test_all_session_logout(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (client, pool) = make_client_with_db(pool).await;

    let auth = Auth::new("hpotter", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // Disable the user, effectively logging them out
    let user = User::find_by_username(&pool, "hpotter")
        .await
        .unwrap()
        .unwrap();

    user.logout_all_sessions(&pool).await.unwrap();

    let response = client.get("/api/v1/me").send().await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let auth_cookie = response.cookies().find(|c| c.name() == SESSION_COOKIE_NAME);
    assert!(auth_cookie.is_none());
}
