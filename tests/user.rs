use defguard::{
    db::{models::wallet::keccak256, UserInfo},
    handlers::{AddUserData, Auth, PasswordChange, Username, WalletChallenge},
    hex::to_lower_hex,
};
use ethers::core::types::transaction::eip712::{Eip712, TypedData};
use rocket::{http::Status, local::asynchronous::Client, serde::json::serde_json::json};

use secp256k1::{rand::rngs::OsRng, Message, Secp256k1};
mod common;
use crate::common::make_test_client;
use tokio_stream::{self as stream, StreamExt};

async fn make_client() -> Client {
    let (client, _) = make_test_client().await;
    client
}

#[rocket::async_test]
async fn test_authenticate() {
    let client = make_client().await;

    let auth = Auth::new("hpotter".into(), "pass123".into());
    let response = client.post("/api/v1/auth").json(&auth).dispatch().await;
    assert_eq!(response.status(), Status::Ok);

    let auth = Auth::new("hpotter".into(), "-wrong-".into());
    let response = client.post("/api/v1/auth").json(&auth).dispatch().await;
    assert_eq!(response.status(), Status::Unauthorized);

    let auth = Auth::new("adumbledore".into(), "pass123".into());
    let response = client.post("/api/v1/auth").json(&auth).dispatch().await;
    assert_eq!(response.status(), Status::Unauthorized);
}

#[rocket::async_test]
async fn test_me() {
    let client = make_client().await;

    let auth = Auth::new("hpotter".into(), "pass123".into());
    let response = client.post("/api/v1/auth").json(&auth).dispatch().await;
    assert_eq!(response.status(), Status::Ok);

    let response = client.get("/api/v1/me").dispatch().await;
    assert_eq!(response.status(), Status::Ok);
    let user_info: UserInfo = response.into_json().await.unwrap();
    assert_eq!(user_info.first_name, "Harry");
    assert_eq!(user_info.last_name, "Potter");
}

#[rocket::async_test]
async fn test_change_password() {
    let client = make_client().await;

    let auth = Auth::new("hpotter".into(), "pass123".into());
    let response = client.post("/api/v1/auth").json(&auth).dispatch().await;
    assert_eq!(response.status(), Status::Ok);

    let new_password = "newPassword43$!";

    let password = PasswordChange {
        new_password: new_password.clone().into(),
    };
    let response = client
        .put("/api/v1/user/hpotter/password")
        .json(&password)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);

    let auth = Auth::new("hpotter".into(), "pass123".into());
    let response = client.post("/api/v1/auth").json(&auth).dispatch().await;
    assert_eq!(response.status(), Status::Unauthorized);

    let auth = Auth::new("hpotter".into(), new_password.into());
    let response = client.post("/api/v1/auth").json(&auth).dispatch().await;
    assert_eq!(response.status(), Status::Ok);
}

#[rocket::async_test]
async fn test_list_users() {
    let client = make_client().await;

    let response = client.get("/api/v1/user").dispatch().await;
    assert_eq!(response.status(), Status::Unauthorized);

    // normal user cannot list users
    let auth = Auth::new("hpotter".into(), "pass123".into());
    let response = client.post("/api/v1/auth").json(&auth).dispatch().await;
    assert_eq!(response.status(), Status::Ok);

    let response = client.get("/api/v1/user").dispatch().await;
    assert_eq!(response.status(), Status::Forbidden);

    // admin can list users
    let auth = Auth::new("admin".into(), "pass123".into());
    let response = client.post("/api/v1/auth").json(&auth).dispatch().await;
    assert_eq!(response.status(), Status::Ok);

    let response = client.get("/api/v1/user").dispatch().await;
    assert_eq!(response.status(), Status::Ok);
}

#[rocket::async_test]
async fn test_get_user() {
    let client = make_client().await;

    let response = client.get("/api/v1/user/hpotter").dispatch().await;
    assert_eq!(response.status(), Status::Unauthorized);

    let auth = Auth::new("hpotter".into(), "pass123".into());
    let response = client.post("/api/v1/auth").json(&auth).dispatch().await;
    assert_eq!(response.status(), Status::Ok);

    let response = client.get("/api/v1/user/hpotter").dispatch().await;
    assert_eq!(response.status(), Status::Ok);
    let user_info: UserInfo = response.into_json().await.unwrap();
    assert_eq!(user_info.first_name, "Harry");
    assert_eq!(user_info.last_name, "Potter");
}

#[rocket::async_test]
async fn test_username_available() {
    let client = make_client().await;

    let auth = Auth::new("hpotter".into(), "pass123".into());
    let response = client.post("/api/v1/auth").json(&auth).dispatch().await;
    assert_eq!(response.status(), Status::Ok);

    let avail = Username {
        username: "CrashTestDummy".into(),
    };
    let response = client
        .post("/api/v1/user/available")
        .json(&avail)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::BadRequest);

    let avail = Username {
        username: "crashtestdummy".into(),
    };
    let response = client
        .post("/api/v1/user/available")
        .json(&avail)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);

    let avail = Username {
        username: "hpotter".into(),
    };
    let response = client
        .post("/api/v1/user/available")
        .json(&avail)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::BadRequest);
}

#[rocket::async_test]
async fn test_crud_user() {
    let client = make_client().await;

    let auth = Auth::new("admin".into(), "pass123".into());
    let response = client.post("/api/v1/auth").json(&auth).dispatch().await;
    assert_eq!(response.status(), Status::Ok);

    // create user
    let new_user = AddUserData {
        username: "adumbledore".into(),
        last_name: "Dumbledore".into(),
        first_name: "Albus".into(),
        email: "a.dumbledore@hogwart.edu.uk".into(),
        phone: "1234".into(),
        password: "Password1234543$!".into(),
    };
    let response = client.post("/api/v1/user").json(&new_user).dispatch().await;
    assert_eq!(response.status(), Status::Created);

    // get user
    let response = client.get("/api/v1/user/adumbledore").dispatch().await;
    assert_eq!(response.status(), Status::Ok);
    let mut user_info: UserInfo = response.into_json().await.unwrap();
    assert_eq!(user_info.first_name, "Albus");

    // edit user
    user_info.phone = Some("5678".into());
    let response = client
        .put("/api/v1/user/adumbledore")
        .json(&user_info)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);

    // delete user
    let response = client.delete("/api/v1/user/adumbledore").dispatch().await;
    assert_eq!(response.status(), Status::Ok);
}

#[rocket::async_test]
async fn test_admin_group() {
    let client = make_client().await;

    let auth = Auth::new("hpotter".into(), "pass123".into());
    let response = client.post("/api/v1/auth").json(&auth).dispatch().await;
    assert_eq!(response.status(), Status::Ok);

    let response = client.get("/api/v1/group").dispatch().await;
    assert_eq!(response.status(), Status::Ok);

    let response = client.get("/api/v1/group/admin").dispatch().await;
    assert_eq!(response.status(), Status::Ok);

    // TODO: check group membership
}

#[rocket::async_test]
async fn test_wallet() {
    let client = make_client().await;

    let auth = Auth::new("hpotter".into(), "pass123".into());
    let response = client.post("/api/v1/auth").json(&auth).dispatch().await;
    assert_eq!(response.status(), Status::Ok);

    let secp = Secp256k1::new();
    let (secret_key, public_key) = secp.generate_keypair(&mut OsRng);

    // create eth wallet address
    let public_key = public_key.serialize_uncompressed();
    let hash = keccak256(&public_key[1..]);
    let addr = &hash[hash.len() - 20..];
    let wallet_address = to_lower_hex(addr);

    let challenge_query = format!(
        "/api/v1/user/hpotter/challenge?address={wallet_address}&name=portefeuille&chain_id=5"
    );

    // get challenge message
    let response = client.get(challenge_query.clone()).dispatch().await;
    assert_eq!(response.status(), Status::Ok);
    let challenge: WalletChallenge = response.into_json().await.unwrap();

    let parsed_data: TypedData =
        rocket::serde::json::serde_json::from_str(&challenge.message).unwrap();
    let parsed_message = parsed_data.message;

    // see migrations for the default message
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

    assert_eq!(challenge.message, message);

    let response = client.get("/api/v1/user/hpotter").dispatch().await;
    assert_eq!(response.status(), Status::Ok);
    let user_info: UserInfo = response.into_json().await.unwrap();
    assert!(user_info.wallets.is_empty());
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
    // send signature
    let response = client
        .put("/api/v1/user/hpotter/wallet")
        .json(&json!({
            "address": wallet_address,
            "signature": to_lower_hex(&sig_arr),
        }))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);

    // get user info for wallets
    let response = client.get("/api/v1/user/hpotter").dispatch().await;
    assert_eq!(response.status(), Status::Ok);
    let user_info: UserInfo = response.into_json().await.unwrap();
    assert_eq!(user_info.wallets.len(), 1);
    let wallet_info = &user_info.wallets[0];
    assert_eq!(wallet_info.address, wallet_address);
    assert_eq!(wallet_info.name, "portefeuille");
    assert_eq!(wallet_info.chain_id, 5);

    // challenge must not be available for verified wallet addresses
    let response = client.get(challenge_query).dispatch().await;
    assert_eq!(response.status(), Status::NotFound);

    // delete wallet
    let response = client
        .delete(format!("/api/v1/user/hpotter/wallet/{wallet_address}"))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);

    let response = client.get("/api/v1/user/hpotter").dispatch().await;
    assert_eq!(response.status(), Status::Ok);
    let user_info: UserInfo = response.into_json().await.unwrap();
    assert!(user_info.wallets.is_empty());
}

#[rocket::async_test]
async fn test_check_username() {
    let client = make_client().await;

    let auth = Auth::new("admin".into(), "pass123".into());
    let response = client.post("/api/v1/auth").json(&auth).dispatch().await;
    assert_eq!(response.status(), Status::Ok);

    // create user
    let new_user = AddUserData {
        username: "ADumbledore".into(),
        last_name: "Dumbledore".into(),
        first_name: "Albus".into(),
        email: "a.dumbledore@hogwart.edu.uk".into(),
        phone: "1234".into(),
        password: "Alohomora!".into(),
    };
    let response = client.post("/api/v1/user").json(&new_user).dispatch().await;
    assert_eq!(response.status(), Status::BadRequest);
}

#[rocket::async_test]
async fn test_check_password_strength()  {
    let client = make_client().await;

    // auth session with admin
    let auth = Auth::new("admin".into(), "pass123".into());
    let response = client.post("/api/v1/auth").json(&auth).dispatch().await;
    assert_eq!(response.status(), Status::Ok);

    // test 
    let strong_password = "strongPass1234$!";
    let too_short = "1H$";
    let no_upper = "notsostrong1!";
    let no_numbers = "notSostrong!";
    let no_specials = "noSoStrong1234";
    let weak_passwords = [too_short, no_upper, no_specials, no_numbers];
    let mut stream = stream::iter(weak_passwords.iter().enumerate());
    while let Some((index, password)) = stream.next().await {
    let weak_password_user = AddUserData {
        username: format!("weakpass{}", index),
        first_name: "testpassfn".into(),
        last_name: "testpassln".into(),
        email: format!("testpass{}@test.test", index),
        phone: "123456789".into(),
        password: password.clone().into(),
    };
        let response = client.post("/api/v1/user").json(&weak_password_user).dispatch().await;
        assert_eq!(response.status(), Status::BadRequest);
    }
    let strong_password_user = AddUserData {
        username: "strongpass".into(),
        first_name: "Strong".into(),
        last_name: "Pass".into(),
        email: "strongpass@test.test".into(),
        phone: "123456789".into(),
        password: strong_password.into(),
    };
    let response = client.post("/api/v1/user").json(&strong_password_user).dispatch().await;
    assert_eq!(response.status(), Status::Created);
}
