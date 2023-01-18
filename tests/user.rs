use defguard::{
    build_webapp,
    db::{models::wallet::keccak256, AppEvent, GatewayEvent, User, UserInfo},
    grpc::GatewayState,
    handlers::{AddUserData, Auth, PasswordChange, Username, WalletChallenge},
    hex::to_lower_hex,
};
use ethers::core::types::transaction::eip712::{Eip712, TypedData};
use rocket::{http::Status, local::asynchronous::Client, serde::json::serde_json::json};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::unbounded_channel;

use secp256k1::{rand::rngs::OsRng, Message, Secp256k1};
mod common;
use common::init_test_db;

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

    let (tx, rx) = unbounded_channel::<AppEvent>();
    let (wg_tx, wg_rx) = unbounded_channel::<GatewayEvent>();
    let gateway_state = Arc::new(Mutex::new(GatewayState::new(wg_rx)));

    let webapp = build_webapp(config, tx, rx, wg_tx, gateway_state, pool).await;
    Client::tracked(webapp).await.unwrap()
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

    let password = PasswordChange {
        new_password: "lumos".into(),
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

    let auth = Auth::new("hpotter".into(), "lumos".into());
    let response = client.post("/api/v1/auth").json(&auth).dispatch().await;
    assert_eq!(response.status(), Status::Ok);
}

#[rocket::async_test]
async fn test_list_users() {
    let client = make_client().await;

    let response = client.get("/api/v1/user").dispatch().await;
    assert_eq!(response.status(), Status::Unauthorized);

    let auth = Auth::new("hpotter".into(), "pass123".into());
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
        password: "Alohomora!".into(),
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
    // see migrations for the default message
    let nonce = to_lower_hex(&keccak256(wallet_address.as_bytes()));
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
                "nonce": "{}"
	}}}}
        "#,
        wallet_address,
        challenge_message
            .chars()
            .filter(|c| c != &'\r' && c != &'\n' && c != &'\t')
            .collect::<String>(),
        nonce,
    )
    .trim()
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
