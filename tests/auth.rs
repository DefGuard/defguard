use claims::assert_none;
use defguard::db::{DbPool, MFAInfo, MFAMethod};
use defguard::{
    auth::TOTP_CODE_VALIDITY_PERIOD,
    db::{models::wallet::keccak256, UserInfo, Wallet},
    handlers::AuthResponse,
    handlers::{Auth, AuthCode, AuthTotp},
};
use ethers::core::types::transaction::eip712::{Eip712, TypedData};
use otpauth::TOTP;
use rocket::http::Cookie;
use rocket::{http::Status, local::asynchronous::Client, serde::json::serde_json::json};
use secp256k1::{rand::rngs::OsRng, Message, Secp256k1};
use serde::Deserialize;
use sqlx::query;
use std::time::SystemTime;
use webauthn_authenticator_rs::{prelude::Url, softpasskey::SoftPasskey, WebauthnAuthenticator};
use webauthn_rs::prelude::{CreationChallengeResponse, RequestChallengeResponse};

mod common;
use crate::common::make_test_client;
use defguard::hex::to_lower_hex;

#[derive(Deserialize)]
pub struct RecoveryCodes {
    codes: Option<Vec<String>>,
}

async fn make_client() -> Client {
    let (client, client_state) = make_test_client().await;

    let mut wallet = Wallet::new_for_user(
        client_state.test_user.id.unwrap(),
        "0x4aF8803CBAD86BA65ED347a3fbB3fb50e96eDD3e".into(),
        "test".into(),
        5,
        String::new(),
    );
    wallet.save(&client_state.pool).await.unwrap();

    client
}

async fn make_client_with_db() -> (Client, DbPool) {
    let (client, client_state) = make_test_client().await;

    let mut wallet = Wallet::new_for_user(
        client_state.test_user.id.unwrap(),
        "0x4aF8803CBAD86BA65ED347a3fbB3fb50e96eDD3e".into(),
        "test".into(),
        5,
        String::new(),
    );
    wallet.save(&client_state.pool).await.unwrap();

    (client, client_state.pool)
}

async fn make_client_with_wallet(address: String) -> Client {
    let (client, client_state) = make_test_client().await;

    let mut wallet = Wallet::new_for_user(
        client_state.test_user.id.unwrap(),
        address,
        "test".into(),
        5,
        String::new(),
    );
    wallet.save(&client_state.pool).await.unwrap();

    client
}

#[rocket::async_test]
async fn test_logout() {
    let client = make_client().await;

    let auth = Auth::new("hpotter".into(), "pass123".into());
    let response = client.post("/api/v1/auth").json(&auth).dispatch().await;
    assert_eq!(response.status(), Status::Ok);

    // store auth cookie for later use
    let auth_cookie = response.cookies().get("defguard_session").unwrap().value();

    let response = client.get("/api/v1/me").dispatch().await;
    assert_eq!(response.status(), Status::Ok);

    let response = client.post("/api/v1/auth/logout").dispatch().await;
    assert_eq!(response.status(), Status::Ok);

    let response = client.get("/api/v1/me").dispatch().await;
    assert_eq!(response.status(), Status::Unauthorized);

    // try reusing auth cookie
    let response = client
        .get("/api/v1/me")
        .cookie(Cookie::new("defguard_session", auth_cookie))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Unauthorized);
}

#[rocket::async_test]
async fn test_login_bruteforce() {
    let client = make_client().await;

    let invalid_auth = Auth::new("hpotter".into(), "invalid".into());

    // fail login 5 times in a row
    for _ in 1..6 {
        let response = client
            .post("/api/v1/auth")
            .json(&invalid_auth)
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Unauthorized);
    }

    let response = client
        .post("/api/v1/auth")
        .json(&invalid_auth)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::TooManyRequests);
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

fn totp_code(auth_totp: &AuthTotp) -> AuthCode {
    let auth = TOTP::from_base32(auth_totp.secret.clone()).unwrap();
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    AuthCode::new(auth.generate(TOTP_CODE_VALIDITY_PERIOD, timestamp))
}

#[rocket::async_test]
async fn test_totp() {
    let client = make_client().await;

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
    assert_eq!(recovery_codes.codes.as_ref().unwrap().len(), 8); // RECOVERY_CODES_COUNT

    // enable MFA
    let response = client.put("/api/v1/auth/mfa").dispatch().await;
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

    // provide recovery code
    let code = recovery_codes.codes.unwrap().first().unwrap().to_string();
    let response = client
        .post("/api/v1/auth/recovery")
        .json(&json!({ "code": code }))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);

    assert_eq!(
        response
            .into_json::<AuthResponse>()
            .await
            .unwrap()
            .user
            .username,
        "hpotter"
    );

    // authorized
    let response = client.get("/api/v1/me").dispatch().await;
    assert_eq!(response.status(), Status::Ok);

    // logout
    let response = client.post("/api/v1/auth/logout").dispatch().await;
    assert_eq!(response.status(), Status::Ok);

    // login
    let response = client.post("/api/v1/auth").json(&auth).dispatch().await;
    assert_eq!(response.status(), Status::Created);

    // reuse the same recovery code - shouldn't work
    let response = client
        .post("/api/v1/auth/recovery")
        .json(&json!({ "code": code }))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Unauthorized);

    // logout
    let response = client.post("/api/v1/auth/logout").dispatch().await;
    assert_eq!(response.status(), Status::Ok);

    // login
    let response = client.post("/api/v1/auth").json(&auth).dispatch().await;
    assert_eq!(response.status(), Status::Created);

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
    let (client, pool) = make_client_with_db().await;

    let mut authenticator = WebauthnAuthenticator::new(SoftPasskey::new());
    let origin = Url::parse("http://localhost:8000").unwrap();

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

    // enable MFA
    let response = client.put("/api/v1/auth/mfa").dispatch().await;
    assert_eq!(response.status(), Status::Ok);

    // login again
    let auth = Auth::new("hpotter".into(), "pass123".into());
    let response = client.post("/api/v1/auth").json(&auth).dispatch().await;
    assert_eq!(response.status(), Status::Created);

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

    // login again
    let auth = Auth::new("hpotter".into(), "pass123".into());
    let response = client.post("/api/v1/auth").json(&auth).dispatch().await;
    assert_eq!(response.status(), Status::Ok);

    // check that recovery codes were cleared since last MFA method was removed
    let recovery_codes = query!(
        "SELECT recovery_codes FROM \"user\" WHERE id = $1",
        user_info.id,
    )
    .fetch_optional(&pool)
    .await
    .unwrap();
    assert_none!(recovery_codes);
}

#[rocket::async_test]
async fn test_cannot_skip_otp_by_adding_yubikey() {
    let client = make_client().await;

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

    // enable MFA
    let response = client.put("/api/v1/auth/mfa").dispatch().await;
    assert_eq!(response.status(), Status::Ok);

    // login again, this time a different status code is returned
    let response = client.post("/api/v1/auth").json(&auth).dispatch().await;
    assert_eq!(response.status(), Status::Created);

    // instead of continuing TOTP login try to add a new YubiKey
    let response = client.post("/api/v1/auth/webauthn/init").dispatch().await;
    assert_eq!(response.status(), Status::Unauthorized);
}

#[rocket::async_test]
async fn test_cannot_skip_security_key_by_adding_yubikey() {
    let client = make_client().await;

    let mut authenticator = WebauthnAuthenticator::new(SoftPasskey::new());
    let origin = Url::parse("http://localhost:8000").unwrap();

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

    // enable MFA
    let response = client.put("/api/v1/auth/mfa").dispatch().await;
    assert_eq!(response.status(), Status::Ok);

    // login again
    let auth = Auth::new("hpotter".into(), "pass123".into());
    let response = client.post("/api/v1/auth").json(&auth).dispatch().await;
    assert_eq!(response.status(), Status::Created);

    // instead of continuing TOTP login try to add a new YubiKey
    let response = client.post("/api/v1/auth/webauthn/init").dispatch().await;
    assert_eq!(response.status(), Status::Unauthorized);
}

#[rocket::async_test]
async fn test_mfa_method_is_updated_when_removing_last_webauthn_passkey() {
    let client = make_client().await;

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
    assert_eq!(recovery_codes.codes.as_ref().unwrap().len(), 8); // RECOVERY_CODES_COUNT

    // enable MFA
    let response = client.put("/api/v1/auth/mfa").dispatch().await;
    assert_eq!(response.status(), Status::Ok);

    // login again, this time a different status code is returned
    let response = client.post("/api/v1/auth").json(&auth).dispatch().await;
    assert_eq!(response.status(), Status::Created);

    // provide correct TOTP code
    let code = totp_code(&auth_totp);
    let response = client
        .post("/api/v1/auth/totp/verify")
        .json(&code)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);

    // WebAuthn registration
    let mut authenticator = WebauthnAuthenticator::new(SoftPasskey::new());
    let origin = Url::parse("http://localhost:8000").unwrap();

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

    // get user info
    let response = client.get("/api/v1/user/hpotter").dispatch().await;
    assert_eq!(response.status(), Status::Ok);
    let mut user_info: UserInfo = response.into_json().await.unwrap();

    // set default MFA method
    user_info.mfa_method = MFAMethod::Webauthn;
    let response = client
        .put("/api/v1/user/hpotter")
        .json(&user_info)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);

    // delete security key
    let response = client
        .delete(format!(
            "/api/v1/user/hpotter/security_key/{}",
            user_info.security_keys[0].id
        ))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);

    // login again
    let auth = Auth::new("hpotter".into(), "pass123".into());
    let response = client.post("/api/v1/auth").json(&auth).dispatch().await;
    assert_eq!(response.status(), Status::Created);
    let mfa_info: MFAInfo = response.into_json().await.unwrap();
    assert_eq!(mfa_info.mfa_method(), &MFAMethod::OneTimePassword);
}

#[rocket::async_test]
async fn test_web3() {
    let secp = Secp256k1::new();
    let (secret_key, public_key) = secp.generate_keypair(&mut OsRng);

    // create eth wallet address
    let public_key = public_key.serialize_uncompressed();
    let hash = keccak256(&public_key[1..]);
    let addr = &hash[hash.len() - 20..];
    let wallet_address = to_lower_hex(addr);

    // create client
    let client = make_client_with_wallet(wallet_address.clone()).await;

    // login
    let auth = Auth::new("hpotter".into(), "pass123".into());
    let response = client.post("/api/v1/auth").json(&auth).dispatch().await;
    assert_eq!(response.status(), Status::Ok);

    // set wallet for MFA
    let response = client
        .put(format!("/api/v1/user/hpotter/wallet/{wallet_address}"))
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

    let wallet_address_request = json!({
        "address": wallet_address.clone(),
    });

    // obtain challenge message
    let response = client
        .post("/api/v1/auth/web3/start")
        .json(&wallet_address_request)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);
    let data: Challenge = response.into_json().await.unwrap();

    let parsed_data: TypedData =
        rocket::serde::json::serde_json::from_str(&data.challenge).unwrap();
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
		"wallet": "{}",
		"content": "{}",
                "nonce": {}
	}}}}
        "#,
        wallet_address,
        challenge_message,
        parsed_message.get("nonce").unwrap(),
    )
    .chars()
    .filter(|c| c != &'\r' && c != &'\n' && c != &'\t')
    .collect::<String>();
    assert_eq!(data.challenge, message);

    // Sign message
    let typed_data: TypedData = rocket::serde::json::serde_json::from_str(&message).unwrap();
    let hash_msg = typed_data.encode_eip712().unwrap();
    let message = Message::from_slice(&hash_msg).unwrap();
    let sig_r = secp.sign_ecdsa_recoverable(&message, &secret_key);
    let (rec_id, sig) = sig_r.serialize_compact();

    // Create recoverable_signature array
    let mut sig_arr = [0; 65];
    sig_arr[0..64].copy_from_slice(&sig[0..64]);
    sig_arr[64] = rec_id.to_i32() as u8;

    // Check if invalid signature results into 401

    let invalid_request_response = client
        .post("/api/v1/auth/web3")
        .json(&json!({
            "address": wallet_address.clone(),
            "signature": "0x00"
        }))
        .dispatch()
        .await;

    assert_eq!(invalid_request_response.status(), Status::Unauthorized);

    // Web3 authentication
    let response = client
        .post("/api/v1/auth/web3")
        .json(&json!({
            "address": wallet_address.clone(),
            "signature": to_lower_hex(&sig_arr),
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
