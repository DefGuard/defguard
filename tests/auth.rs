use defguard::{
    auth::TOTP_CODE_VALIDITY_PERIOD,
    build_webapp,
    db::{AppEvent, GatewayEvent, User, UserInfo, Wallet},
    handlers::{Auth, AuthCode, AuthTotp},
};
use otpauth::TOTP;
use rocket::{http::Status, local::asynchronous::Client, serde::json::serde_json::json};
use serde::Deserialize;
use std::time::SystemTime;
use tokio::sync::mpsc::unbounded_channel;
use webauthn_authenticator_rs::{prelude::Url, softpasskey::SoftPasskey, WebauthnAuthenticator};
use webauthn_rs::prelude::{CreationChallengeResponse, RequestChallengeResponse};

mod common;
use common::init_test_db;

#[derive(Deserialize)]
pub struct RecoveryCodes {
    codes: Option<Vec<String>>,
}

async fn make_client() -> Client {
    let (pool, config) = init_test_db().await;

    let mut user = User::new(
        "hpotter".into(),
        "pass123",
        "Potter".into(),
        "Harry".into(),
        "h.potter@hogwart.edu.uk".into(),
        None,
    );
    user.save(&pool).await.unwrap();

    let mut wallet = Wallet::new_for_user(
        user.id.unwrap(),
        "0x4aF8803CBAD86BA65ED347a3fbB3fb50e96eDD3e".into(),
        "test".into(),
        5,
        String::new(),
    );
    wallet.save(&pool).await.unwrap();

    let (tx, rx) = unbounded_channel::<AppEvent>();
    let (wg_tx, _) = unbounded_channel::<GatewayEvent>();

    let webapp = build_webapp(config, tx, rx, wg_tx, pool).await;
    Client::tracked(webapp).await.unwrap()
}

#[rocket::async_test]
async fn test_logout() {
    let client = make_client().await;

    let auth = Auth::new("hpotter".into(), "pass123".into());
    let response = client.post("/api/v1/auth").json(&auth).dispatch().await;
    assert_eq!(response.status(), Status::Ok);

    let response = client.get("/api/v1/me").dispatch().await;
    assert_eq!(response.status(), Status::Ok);

    let response = client.post("/api/v1/auth/logout").dispatch().await;
    assert_eq!(response.status(), Status::Ok);

    let response = client.get("/api/v1/me").dispatch().await;
    assert_eq!(response.status(), Status::Unauthorized);
}

#[rocket::async_test]
async fn test_cannot_enable_mfa() {
    let client = make_client().await;

    let auth = Auth::new("hpotter".into(), "pass123".into());
    let response = client.post("/api/v1/auth").json(&auth).dispatch().await;
    assert_eq!(response.status(), Status::Ok);

    // enable MFA
    let response = client.put("/api/v1/auth/mfa").dispatch().await;
    assert_eq!(response.status(), Status::NotModified);
}

#[rocket::async_test]
async fn test_totp() {
    let client = make_client().await;

    fn totp_code(auth_totp: &AuthTotp) -> AuthCode {
        let auth = TOTP::from_base32(auth_totp.secret.clone()).unwrap();
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        AuthCode::new(auth.generate(TOTP_CODE_VALIDITY_PERIOD, timestamp))
    }

    // login
    let auth = Auth::new("hpotter".into(), "pass123".into());
    let response = client.post("/api/v1/auth").json(&auth).dispatch().await;
    assert_eq!(response.status(), Status::Ok);

    // new TOTP secret
    let response = client.post("/api/v1/auth/totp/init").dispatch().await;
    assert_eq!(response.status(), Status::Ok);
    let auth_totp: AuthTotp = response.into_json().await.unwrap();

    // enable TOTP
    let code = totp_code(&auth_totp);
    let response = client
        .post("/api/v1/auth/totp")
        .json(&code)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);

    // check recovery codes
    let recovery_codes: RecoveryCodes = response.into_json().await.unwrap();
    assert_eq!(recovery_codes.codes.unwrap().len(), 8); // RECOVERY_CODES_COUNT

    // enable MFA
    let response = client.put("/api/v1/auth/mfa").dispatch().await;
    assert_eq!(response.status(), Status::Ok);

    // logout
    let response = client.post("/api/v1/auth/logout").dispatch().await;
    assert_eq!(response.status(), Status::Ok);

    // login again, this time a different status code is returned
    let response = client.post("/api/v1/auth").json(&auth).dispatch().await;
    assert_eq!(response.status(), Status::Created);

    // still unauthorized
    let response = client.get("/api/v1/me").dispatch().await;
    assert_eq!(response.status(), Status::Unauthorized);

    // provide wrong TOTP code
    let code = AuthCode::new(0);
    let response = client
        .post("/api/v1/auth/totp/verify")
        .json(&code)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Unauthorized);

    // provide correct TOTP code
    let code = totp_code(&auth_totp);
    let response = client
        .post("/api/v1/auth/totp/verify")
        .json(&code)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);

    // authorized
    let response = client.get("/api/v1/me").dispatch().await;
    assert_eq!(response.status(), Status::Ok);

    // disable MFA
    let response = client.delete("/api/v1/auth/mfa").dispatch().await;
    assert_eq!(response.status(), Status::Ok);

    // login again
    let auth = Auth::new("hpotter".into(), "pass123".into());
    let response = client.post("/api/v1/auth").json(&auth).dispatch().await;
    assert_eq!(response.status(), Status::Ok);
}

#[rocket::async_test]
async fn test_webauthn() {
    let client = make_client().await;

    let mut authenticator = WebauthnAuthenticator::new(SoftPasskey::new());
    let origin = Url::parse("http://localhost:8080").unwrap();

    // login
    let auth = Auth::new("hpotter".into(), "pass123".into());
    let response = client.post("/api/v1/auth").json(&auth).dispatch().await;
    assert_eq!(response.status(), Status::Ok);

    // WebAuthn registration
    let response = client.post("/api/v1/auth/webauthn/init").dispatch().await;
    assert_eq!(response.status(), Status::Ok);
    let ccr: CreationChallengeResponse = response.into_json().await.unwrap();
    let rpkc = authenticator.do_registration(origin.clone(), ccr).unwrap();
    let response = client
        .post("/api/v1/auth/webauthn/finish")
        .json(&json!({
            "name": "My security key",
            "rpkc": &rpkc
        }))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);

    // check recovery codes
    let recovery_codes: RecoveryCodes = response.into_json().await.unwrap();
    assert_eq!(recovery_codes.codes.unwrap().len(), 8); // RECOVERY_CODES_COUNT

    // WebAuthn authentication
    let response = client.post("/api/v1/auth/webauthn/start").dispatch().await;
    assert_eq!(response.status(), Status::Ok);
    let rcr: RequestChallengeResponse = response.into_json().await.unwrap();
    let pkc = authenticator.do_authentication(origin, rcr).unwrap();
    let response = client
        .post("/api/v1/auth/webauthn")
        .json(&pkc)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);

    // get security keys
    let response = client.get("/api/v1/me").dispatch().await;
    assert_eq!(response.status(), Status::Ok);
    let user_info: UserInfo = response.into_json().await.unwrap();
    assert_eq!(user_info.security_keys.len(), 1);

    // delete security key
    let response = client
        .delete(format!(
            "/api/v1/user/hpotter/security_key/{}",
            user_info.security_keys[0].id
        ))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);

    // disable MFA
    let response = client.delete("/api/v1/auth/mfa").dispatch().await;
    assert_eq!(response.status(), Status::Ok);

    // login again
    let auth = Auth::new("hpotter".into(), "pass123".into());
    let response = client.post("/api/v1/auth").json(&auth).dispatch().await;
    assert_eq!(response.status(), Status::Ok);
}

#[rocket::async_test]
async fn test_web3() {
    let client = make_client().await;

    // login
    let auth = Auth::new("hpotter".into(), "pass123".into());
    let response = client.post("/api/v1/auth").json(&auth).dispatch().await;
    assert_eq!(response.status(), Status::Ok);

    let response = client
        .put("/api/v1/user/hpotter/wallet/0x4aF8803CBAD86BA65ED347a3fbB3fb50e96eDD3e")
        .json(&json!({
            "use_for_mfa": true
        }))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);

    // check recovery codes
    let recovery_codes: RecoveryCodes = response.into_json().await.unwrap();
    assert_eq!(recovery_codes.codes.unwrap().len(), 8); // RECOVERY_CODES_COUNT

    #[derive(Deserialize)]
    struct Challenge {
        challenge: String,
    }

    // Web3 authentication
    let response = client.post("/api/v1/auth/web3/start").dispatch().await;
    assert_eq!(response.status(), Status::Ok);
    let data: Challenge = response.into_json().await.unwrap();
    assert_eq!(
        data.challenge,
        "By signing this message you confirm that you're the owner of the wallet"
    );

    let response = client
        .post("/api/v1/auth/web3")
        .json(&json!({
            "address": "0x4aF8803CBAD86BA65ED347a3fbB3fb50e96eDD3e",
            "signature": "0xcf9a650ed3dbb594f68a0614fc385363f17a150f0ced6e0e92f6cc40923ec0d86c70aa3a74e73216a57d6ae6a1e07e5951416491a2660a88d5d78a5ec7e4a9bd1c"
        }))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);

    // disable MFA
    let response = client.delete("/api/v1/auth/mfa").dispatch().await;
    assert_eq!(response.status(), Status::Ok);

    // login again
    let auth = Auth::new("hpotter".into(), "pass123".into());
    let response = client.post("/api/v1/auth").json(&auth).dispatch().await;
    assert_eq!(response.status(), Status::Ok);
}
