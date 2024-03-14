mod common;

use std::{str::FromStr, time::SystemTime};

use chrono::NaiveDateTime;
use claims::assert_err;
use defguard::{
    db::{models::wallet::keccak256, DbPool, MFAInfo, MFAMethod, Settings, UserDetails, Wallet},
    handlers::{Auth, AuthCode, AuthResponse, AuthTotp, WalletChallenge},
    hex::to_lower_hex,
    secret::SecretString,
    SERVER_CONFIG,
};
use ethers_core::types::transaction::eip712::{Eip712, TypedData};
use otpauth::TOTP;
use reqwest::{header::USER_AGENT, StatusCode};
use secp256k1::{rand::rngs::OsRng, All, Message, Secp256k1, SecretKey};
use serde::Deserialize;
use serde_json::json;
use sqlx::query;
use webauthn_authenticator_rs::{prelude::Url, softpasskey::SoftPasskey, WebauthnAuthenticator};
use webauthn_rs::prelude::{CreationChallengeResponse, RequestChallengeResponse};

use self::common::{client::TestClient, make_test_client, ClientState, X_FORWARDED_FOR};

static SESSION_COOKIE_NAME: &str = "defguard_session";

#[derive(Deserialize)]
pub struct RecoveryCodes {
    codes: Option<Vec<String>>,
}

async fn make_client() -> TestClient {
    let (client, client_state) = make_test_client().await;

    let mut wallet = Wallet::new_for_user(
        client_state.test_user.id.unwrap(),
        "0x4aF8803CBAD86BA65ED347a3fbB3fb50e96eDD3e",
        "test",
        5,
        "",
    );
    wallet.save(&client_state.pool).await.unwrap();

    client
}

async fn make_client_with_db() -> (TestClient, DbPool) {
    let (client, client_state) = make_test_client().await;

    let mut wallet = Wallet::new_for_user(
        client_state.test_user.id.unwrap(),
        "0x4aF8803CBAD86BA65ED347a3fbB3fb50e96eDD3e",
        "test",
        5,
        "",
    );
    wallet.save(&client_state.pool).await.unwrap();

    (client, client_state.pool)
}

async fn make_client_with_state() -> (TestClient, ClientState) {
    let (client, client_state) = make_test_client().await;

    let mut wallet = Wallet::new_for_user(
        client_state.test_user.id.unwrap(),
        "0x4aF8803CBAD86BA65ED347a3fbB3fb50e96eDD3e",
        "test",
        5,
        "",
    );
    wallet.save(&client_state.pool).await.unwrap();

    (client, client_state)
}

async fn make_client_with_wallet(address: &str) -> TestClient {
    let (client, client_state) = make_test_client().await;

    let mut wallet =
        Wallet::new_for_user(client_state.test_user.id.unwrap(), address, "test", 5, "");
    wallet.save(&client_state.pool).await.unwrap();

    client
}

#[tokio::test]
async fn test_logout() {
    let mut client = make_client().await;

    let auth = Auth::new("hpotter", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

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

    // try reusing auth cookie
    client.set_cookie(&auth_cookie);
    let response = client.get("/api/v1/me").send().await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_login_bruteforce() {
    let client = make_client().await;

    let invalid_auth = Auth::new("hpotter", "invalid");

    // fail login 5 times in a row
    for i in 0..6 {
        let response = client.post("/api/v1/auth").json(&invalid_auth).send().await;
        if i == 5 {
            assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        } else {
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        }
    }
}

#[tokio::test]
async fn test_cannot_enable_mfa() {
    let client = make_client().await;

    let auth = Auth::new("hpotter", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // enable MFA
    let response = client.put("/api/v1/auth/mfa").send().await;
    assert_eq!(response.status(), StatusCode::NOT_MODIFIED);
}

fn totp_code(auth_totp: &AuthTotp) -> AuthCode {
    let auth = TOTP::from_base32(auth_totp.secret.clone()).unwrap();
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    AuthCode::new(auth.generate(
        SERVER_CONFIG.get().unwrap().totp_code_timeout.as_secs(),
        timestamp,
    ))
}

#[tokio::test]
async fn test_totp() {
    let client = make_client().await;

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
    let code = AuthCode::new(0);
    let response = client
        .post("/api/v1/auth/totp/verify")
        .json(&code)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // provide recovery code
    let code = recovery_codes.codes.unwrap().first().unwrap().to_string();
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

static EMAIL_CODE_REGEX: &str = r"<b>(?<code>\d{6})</b>";
fn extract_email_code(content: &str) -> u32 {
    let re = regex::Regex::new(EMAIL_CODE_REGEX).unwrap();
    let code = re.captures(content).unwrap().name("code").unwrap().as_str();
    code.parse().unwrap()
}

#[tokio::test]
async fn test_email_mfa() {
    let (client, state) = make_client_with_state().await;
    let pool = state.pool;
    let mut mail_rx = state.mail_rx;

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
    let mut settings = Settings::get_settings(&pool).await.unwrap();
    settings.smtp_server = Some("smtp_server".into());
    settings.smtp_port = Some(587);
    settings.smtp_user = Some("dummy_user".into());
    settings.smtp_password = Some(SecretString::from_str("dummy_password").unwrap());
    settings.smtp_sender = Some("smtp@sender.pl".into());
    settings.save(&pool).await.unwrap();

    // initialize email MFA setup
    let response = client.post("/api/v1/auth/email/init").send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // check email was sent
    let mail = mail_rx.try_recv().unwrap();
    assert_err!(mail_rx.try_recv());
    assert_eq!(mail.to, "h.potter@hogwart.edu.uk");
    assert_eq!(mail.subject, "Your Multi-Factor Authentication Activation");

    // resend setup email
    let response = client.post("/api/v1/auth/email/init").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let mail = mail_rx.try_recv().unwrap();
    assert_err!(mail_rx.try_recv());
    assert_eq!(mail.to, "h.potter@hogwart.edu.uk");
    assert_eq!(mail.subject, "Your Multi-Factor Authentication Activation");
    let code = extract_email_code(&mail.content);

    // finish setup
    let code = AuthCode::new(code);
    let response = client.post("/api/v1/auth/email").json(&code).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // check that confirmation email was sent
    let mail = mail_rx.try_recv().unwrap();
    assert_err!(mail_rx.try_recv());
    assert_eq!(mail.to, "h.potter@hogwart.edu.uk");
    assert_eq!(
        mail.subject,
        "MFA method Email was activated on your account"
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
    let code = AuthCode::new(0);
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
    assert_err!(mail_rx.try_recv());
    assert_eq!(mail.to, "h.potter@hogwart.edu.uk");
    assert_eq!(
        mail.subject,
        "Your Multi-Factor Authentication Code for Login"
    );

    // resend code
    let response = client.get("/api/v1/auth/email").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let mail = mail_rx.try_recv().unwrap();
    assert_err!(mail_rx.try_recv());
    assert_eq!(mail.to, "h.potter@hogwart.edu.uk");
    assert_eq!(
        mail.subject,
        "Your Multi-Factor Authentication Code for Login"
    );
    let code = extract_email_code(&mail.content);

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

#[tokio::test]
async fn test_webauthn() {
    let (client, pool) = make_client_with_db().await;

    let mut authenticator = WebauthnAuthenticator::new(SoftPasskey::new());
    let origin = Url::parse("http://localhost:8000").unwrap();

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

#[tokio::test]
async fn test_cannot_skip_otp_by_adding_yubikey() {
    let client = make_client().await;

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

#[tokio::test]
async fn test_cannot_skip_security_key_by_adding_yubikey() {
    let client = make_client().await;

    let mut authenticator = WebauthnAuthenticator::new(SoftPasskey::new());
    let origin = Url::parse("http://localhost:8000").unwrap();

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

#[tokio::test]
async fn test_mfa_method_is_updated_when_removing_last_webauthn_passkey() {
    let client = make_client().await;

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
    let mut authenticator = WebauthnAuthenticator::new(SoftPasskey::new());
    let origin = Url::parse("http://localhost:8000").unwrap();

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

#[derive(Deserialize)]
struct Challenge {
    challenge: String,
}

// helper to perform login using a wallet
async fn wallet_login(
    client: &TestClient,
    wallet_address: String,
    secp: &Secp256k1<All>,
    secret_key: SecretKey,
) {
    let wallet_address_request = json!({
        "address": wallet_address.clone(),
    });

    // obtain challenge message
    let response = client
        .post("/api/v1/auth/web3/start")
        .json(&wallet_address_request)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let data: Challenge = response.json().await;

    let parsed_data: TypedData = serde_json::from_str(&data.challenge).unwrap();
    let parsed_message = parsed_data.message;

    let challenge_message = "Please read this carefully:

Click to sign to prove you are in possesion of your private key to the account.
This request will not trigger a blockchain transaction or cost any gas fees.";
    let message: String = format!(
        r#"{{
            "domain": {{ "name": "Defguard", "version": "1" }},
            "types": {{
            "EIP712Domain": [
                {{ "name": "name", "type": "string" }},
                {{ "name": "version", "type": "string" }}
            ],
            "ProofOfOwnership": [
                {{ "name": "wallet", "type": "address" }},
                {{ "name": "content", "type": "string" }},
                {{ "name": "nonce", "type": "string" }}
            ]
        }},
        "primaryType": "ProofOfOwnership",
        "message": {{
            "wallet": "{wallet_address}",
            "content": "{challenge_message}",
            "nonce": {}
        }}}}
        "#,
        parsed_message.get("nonce").unwrap(),
    )
    .chars()
    .filter(|c| *c != ' ' && *c != '\r' && *c != '\n' && *c != '\t')
    .collect::<String>();
    let challenge = data
        .challenge
        .chars()
        .filter(|c| *c != ' ' && *c != '\r' && *c != '\n' && *c != '\t')
        .collect::<String>();
    assert_eq!(challenge, message);

    // Sign message
    let signature = sign_message(&data.challenge, secp, secret_key);

    // Check if invalid signature results into 401
    let invalid_request_response = client
        .post("/api/v1/auth/web3")
        .json(&json!({
            "address": wallet_address.clone(),
            "signature": "0x00"
        }))
        .send()
        .await;

    assert_eq!(invalid_request_response.status(), StatusCode::UNAUTHORIZED);

    // Web3 authentication
    let response = client
        .post("/api/v1/auth/web3")
        .json(&json!({
            "address": wallet_address.clone(),
            "signature": signature,
        }))
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);
}

fn sign_message(message: &str, secp: &Secp256k1<All>, secret_key: SecretKey) -> String {
    let typed_data: TypedData = serde_json::from_str(message).unwrap();
    let hash_msg = typed_data.encode_eip712().unwrap();
    let message = Message::from_digest_slice(&hash_msg).unwrap();
    let sig_r = secp.sign_ecdsa_recoverable(&message, &secret_key);
    let (rec_id, sig) = sig_r.serialize_compact();

    // Create recoverable_signature array
    let mut sig_arr = [0; 65];
    sig_arr[0..64].copy_from_slice(&sig[0..64]);
    sig_arr[64] = rec_id.to_i32() as u8;

    to_lower_hex(&sig_arr)
}

#[tokio::test]
async fn test_web3() {
    let secp = Secp256k1::new();
    let (secret_key, public_key) = secp.generate_keypair(&mut OsRng);

    // create eth wallet address
    let public_key = public_key.serialize_uncompressed();
    let hash = keccak256(&public_key[1..]);
    let addr = &hash[hash.len() - 20..];
    let wallet_address = to_lower_hex(addr);

    // create client
    let client = make_client_with_wallet(&wallet_address).await;

    // login
    let auth = Auth::new("hpotter", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // set wallet for MFA
    let response = client
        .put(format!("/api/v1/user/hpotter/wallet/{wallet_address}"))
        .json(&json!({
            "use_for_mfa": true
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

    // login with wallet
    let auth = Auth::new("hpotter", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);
    wallet_login(&client, wallet_address, &secp, secret_key).await;

    // disable MFA
    let response = client.delete("/api/v1/auth/mfa").send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // login again
    let auth = Auth::new("hpotter", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_re_adding_wallet() {
    let secp = Secp256k1::new();
    let (secret_key, public_key) = secp.generate_keypair(&mut OsRng);

    // create eth wallet address
    let public_key = public_key.serialize_uncompressed();
    let hash = keccak256(&public_key[1..]);
    let addr = &hash[hash.len() - 20..];
    let wallet_address = to_lower_hex(addr);

    // create client
    let client = make_client().await;

    // login
    let auth = Auth::new("hpotter", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // add wallet
    let response = client
        .get(format!(
            "/api/v1/user/hpotter/challenge?address={}&name=TestWallet&chain_id=1",
            &wallet_address
        ))
        .send()
        .await;
    let challenge: WalletChallenge = response.json().await;
    let signature = sign_message(&challenge.message, &secp, secret_key);
    let response = client
        .put("/api/v1/user/hpotter/wallet")
        .json(&json!({
            "address": wallet_address,
            "chain_id": 1,
            "name": "TestWallet",
            "signature": signature
        }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    // enable wallet for MFA
    let response = client
        .put(format!("/api/v1/user/hpotter/wallet/{}", &wallet_address))
        .json(&json!({
            "use_for_mfa": true
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

    // login with wallet
    let auth = Auth::new("hpotter", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);
    wallet_login(&client, wallet_address.clone(), &secp, secret_key).await;

    // remove wallet
    let response = client
        .delete(format!("/api/v1/user/hpotter/wallet/{}", &wallet_address))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    // logout
    let response = client.post("/api/v1/auth/logout").send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // login without MFA
    let auth = Auth::new("hpotter", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // add the same wallet and enable MFA
    let response = client
        .get(format!(
            "/api/v1/user/hpotter/challenge?address={}&name=TestWallet&chain_id=1",
            &wallet_address
        ))
        .send()
        .await;
    let challenge: WalletChallenge = response.json().await;
    let signature = sign_message(&challenge.message, &secp, secret_key);
    let response = client
        .put("/api/v1/user/hpotter/wallet")
        .json(&json!({
            "address": wallet_address,
            "chain_id": 1,
            "name": "TestWallet",
            "signature": signature
        }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let response = client
        .put(format!("/api/v1/user/hpotter/wallet/{}", &wallet_address))
        .json(&json!({
            "use_for_mfa": true
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

    // login with wallet
    let auth = Auth::new("hpotter", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);
    wallet_login(&client, wallet_address.clone(), &secp, secret_key).await;
}

#[tokio::test]
async fn test_mfa_method_totp_enabled_mail() {
    let (client, state) = make_test_client().await;
    let mut mail_rx = state.mail_rx;
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
    assert_eq!(mail.to, "h.potter@hogwart.edu.uk");
    assert_eq!(
        mail.subject,
        "MFA method TOTP was activated on your account"
    );
    assert!(mail.content.contains("IP Address:</span> 127.0.0.1"));
    assert!(mail
        .content
        .contains("Device type:</span> iPhone, OS: iOS 17.1, Mobile Safari"));
}

#[tokio::test]
async fn test_new_device_login() {
    let (client, state) = make_test_client().await;
    let mut mail_rx = state.mail_rx;
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

    let mail = mail_rx.try_recv().unwrap();
    assert_eq!(mail.to, "h.potter@hogwart.edu.uk");
    assert_eq!(
        mail.subject,
        "Defguard: new device logged in to your account"
    );
    assert!(mail.content.contains("IP Address:</span> 127.0.0.1"));
    assert!(mail
        .content
        .contains("Device type:</span> iPhone, OS: iOS 17.1, Mobile Safari"));

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

    assert_err!(mail_rx.try_recv());

    // login using a different device
    let auth = Auth::new("hpotter", "pass123");
    let response = client
        .post("/api/v1/auth")
        .header(USER_AGENT, user_agent_header_android)
        .json(&auth)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    let mail = mail_rx.try_recv().unwrap();
    assert_eq!(
        mail.subject,
        "Defguard: new device logged in to your account"
    );
    assert!(mail.content.contains("IP Address:</span> 127.0.0.1"));
    assert!(mail
        .content
        .contains("Device type:</span> SM-G930VC, OS: Android 7.0, Chrome Mobile WebView"));
}

#[tokio::test]
async fn test_login_ip_headers() {
    let (client, state) = make_test_client().await;
    let mut mail_rx = state.mail_rx;
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

    let mail = mail_rx.try_recv().unwrap();
    assert_eq!(mail.to, "h.potter@hogwart.edu.uk");
    assert_eq!(
        mail.subject,
        "Defguard: new device logged in to your account"
    );
    assert!(mail.content.contains("IP Address:</span> 10.0.0.20"));
}

#[tokio::test]
async fn test_session_cookie() {
    let (client, pool) = make_client_with_db().await;

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
        NaiveDateTime::UNIX_EPOCH,
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
