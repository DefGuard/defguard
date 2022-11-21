use defguard::{
    build_webapp,
    db::{AppEvent, GatewayEvent, User, UserInfo},
    handlers::{AddUserData, Auth, PasswordChange, Username, WalletChallenge},
};
use rocket::{http::Status, local::asynchronous::Client, serde::json::serde_json::json};
use tokio::sync::mpsc::unbounded_channel;

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
    let (wg_tx, _) = unbounded_channel::<GatewayEvent>();

    let webapp = build_webapp(config, tx, rx, wg_tx, pool).await;
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

    let address = "0x4aF8803CBAD86BA65ED347a3fbB3fb50e96eDD3e";
    let challenge_query =
        format!("/api/v1/user/hpotter/challenge?address={address}&name=portefeuille&chain_id=5");

    // get challenge message
    let response = client.get(challenge_query.clone()).dispatch().await;
    assert_eq!(response.status(), Status::Ok);
    let challenge: WalletChallenge = response.into_json().await.unwrap();
    // see migrations for the default message
    let address = "0x4aF8803CBAD86BA65ED347a3fbB3fb50e96eDD3e";
    let message = format!(
        "Please read this carefully:

        Click to sign to prove you are in possesion of your private key to the account.
        This request will not trigger a blockchain transaction or cost any gas fees.\n\
        Wallet address:\n\
        {}\n\
        \n\
        Date and time:\n\
        {}",
        address,
        chrono::Local::now().format("%Y-%m-%d %H:%M")
    );
    assert_eq!(challenge.message, message.trim_start());

    let response = client.get("/api/v1/user/hpotter").dispatch().await;
    assert_eq!(response.status(), Status::Ok);
    let user_info: UserInfo = response.into_json().await.unwrap();
    assert!(user_info.wallets.is_empty());

    // send signature
    let response = client
        .put("/api/v1/user/hpotter/wallet")
        .json(&json!({
            "address": address,
            "signature": "0xcf9a650ed3dbb594f68a0614fc385363f17a150f0ced6e0e92f6cc40923ec0d86c70aa3a74e73216a57d6ae6a1e07e5951416491a2660a88d5d78a5ec7e4a9bd1c",
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
    assert_eq!(wallet_info.address, address);
    assert_eq!(wallet_info.name, "portefeuille");
    assert_eq!(wallet_info.chain_id, 5);

    // challenge must not be available for verified wallet addresses
    let response = client.get(challenge_query).dispatch().await;
    assert_eq!(response.status(), Status::NotFound);

    // delete wallet
    let response = client
        .delete(format!("/api/v1/user/hpotter/wallet/{address}"))
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
