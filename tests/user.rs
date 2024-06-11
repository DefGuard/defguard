mod common;

use defguard::{
    db::{
        models::{oauth2client::OAuth2Client, wallet::keccak256, NewOpenIDClient},
        AddDevice, UserInfo,
    },
    handlers::{AddUserData, Auth, PasswordChange, PasswordChangeSelf, Username, WalletChallenge},
    hex::to_lower_hex,
};
use ethers_core::types::transaction::eip712::{Eip712, TypedData};
use reqwest::{header::USER_AGENT, StatusCode};
use secp256k1::{rand::rngs::OsRng, Message, Secp256k1};
use serde_json::{json, Value};
use tokio_stream::{self as stream, StreamExt};

use self::common::{client::TestClient, fetch_user_details, make_test_client};

async fn make_client() -> TestClient {
    let (client, _) = make_test_client().await;
    client
}

#[tokio::test]
async fn test_authenticate() {
    let client = make_client().await;

    let auth = Auth::new("hpotter", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    let auth = Auth::new("hpotter", "-wrong-");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let auth = Auth::new("adumbledore", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_me() {
    let client = make_client().await;

    let auth = Auth::new("hpotter", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    let response = client.get("/api/v1/me").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let user_info: UserInfo = response.json().await;
    assert_eq!(user_info.first_name, "Harry");
    assert_eq!(user_info.last_name, "Potter");
}

#[tokio::test]
async fn test_change_self_password() {
    let client = make_client().await;

    let auth = Auth::new("hpotter", "pass123");

    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    let bad_old = "notCurrentPassword123!$";

    let new_password = "strongPassword123$!1";

    let bad_old_request = PasswordChangeSelf {
        old_password: bad_old.into(),
        new_password: new_password.into(),
    };

    let bad_new_request = PasswordChangeSelf {
        old_password: "pass123".into(),
        new_password: "badnew".into(),
    };

    let change_password = PasswordChangeSelf {
        old_password: "pass123".into(),
        new_password: new_password.into(),
    };

    let response = client
        .put("/api/v1/user/change_password")
        .json(&bad_old_request)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let response = client
        .put("/api/v1/user/change_password")
        .json(&bad_new_request)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let response = client
        .put("/api/v1/user/change_password")
        .json(&change_password)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    // old pass login
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let new_auth = Auth::new("hpotter", new_password);

    let response = client.post("/api/v1/auth").json(&new_auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_change_password() {
    let client = make_client().await;

    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;

    assert_eq!(response.status(), StatusCode::OK);

    let new_password = "newPassword43$!";

    let change_others_password = PasswordChange {
        new_password: new_password.into(),
    };

    let response = client
        .put("/api/v1/user/admin/password")
        .json(&change_others_password)
        .send()
        .await;

    // can't change own password with this endpoint
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // can change others password

    let response = client
        .put("/api/v1/user/hpotter/password")
        .json(&change_others_password)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::OK);

    let auth = Auth::new("hpotter", new_password);
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // route is only for admins
    let response = client
        .put("/api/v1/user/admin/password")
        .json(&change_others_password)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_list_users() {
    let client = make_client().await;

    let response = client.get("/api/v1/user").send().await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // normal user cannot list users
    let auth = Auth::new("hpotter", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    let response = client.get("/api/v1/user").send().await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    // admin can list users
    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    let response = client.get("/api/v1/user").send().await;
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_get_user() {
    let client = make_client().await;

    let response = client.get("/api/v1/user/hpotter").send().await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let auth = Auth::new("hpotter", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    let user_info = fetch_user_details(&client, "hpotter").await;
    assert_eq!(user_info.user.first_name, "Harry");
    assert_eq!(user_info.user.last_name, "Potter");
}

#[tokio::test]
async fn test_username_available() {
    let client = make_client().await;

    // standard user cannot check username availability
    let auth = Auth::new("hpotter", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    let avail = Username {
        username: "hpotter".into(),
    };
    let response = client
        .post("/api/v1/user/available")
        .json(&avail)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    // log in as admin
    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    let avail = Username {
        username: "_CrashTestDummy".into(),
    };
    let response = client
        .post("/api/v1/user/available")
        .json(&avail)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let avail = Username {
        username: "crashtestdummy42".into(),
    };
    let response = client
        .post("/api/v1/user/available")
        .json(&avail)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    let avail = Username {
        username: "hpotter".into(),
    };
    let response = client
        .post("/api/v1/user/available")
        .json(&avail)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_crud_user() {
    let client = make_client().await;

    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // create user
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

    // get user
    let mut user_details = fetch_user_details(&client, "adumbledore").await;
    assert_eq!(user_details.user.first_name, "Albus");

    // edit user
    user_details.user.phone = Some("5678".into());
    let response = client
        .put("/api/v1/user/adumbledore")
        .json(&user_details.user)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    // delete user
    let response = client.delete("/api/v1/user/adumbledore").send().await;
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_admin_group() {
    let client = make_client().await;

    let auth = Auth::new("hpotter", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    let response = client.get("/api/v1/group").send().await;
    assert_eq!(response.status(), StatusCode::OK);

    let response = client.get("/api/v1/group/admin").send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // TODO: check group membership
}

#[tokio::test]
async fn test_wallet() {
    let client = make_client().await;

    let auth = Auth::new("hpotter", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

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
    let response = client.get(challenge_query.clone()).send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let challenge: WalletChallenge = response.json().await;

    let parsed_data: TypedData = serde_json::from_str(&challenge.message).unwrap();
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

    let user_info = fetch_user_details(&client, "hpotter").await;
    assert!(user_info.wallets.is_empty());

    // Sign message
    let typed_data: TypedData = serde_json::from_str(&message).unwrap();
    let hash_msg = typed_data.encode_eip712().unwrap();
    let message = Message::from_digest_slice(&hash_msg).unwrap();

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
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    // get user info for wallets
    let user_info = fetch_user_details(&client, "hpotter").await;
    assert_eq!(user_info.wallets.len(), 1);
    let wallet_info = &user_info.wallets[0];
    assert_eq!(wallet_info.address, wallet_address);
    assert_eq!(wallet_info.name, "portefeuille");
    assert_eq!(wallet_info.chain_id, 5);

    // challenge must not be available for verified wallet addresses
    let response = client.get(challenge_query).send().await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    // delete wallet
    let response = client
        .delete(format!("/api/v1/user/hpotter/wallet/{wallet_address}"))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    let user_info = fetch_user_details(&client, "hpotter").await;
    assert!(user_info.wallets.is_empty());
}

#[tokio::test]
async fn test_check_username() {
    let client = make_client().await;

    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    let invalid_usernames = ["ADumble dore", ".1user"];
    let valid_usernames = ["user1", "use2r3", "not_wrong"];

    for username in invalid_usernames {
        let new_user = AddUserData {
            username: username.into(),
            last_name: "Dumbledore".into(),
            first_name: "Albus".into(),
            email: "a.dumbledore@hogwart.edu.uk".into(),
            phone: Some("1234".into()),
            password: Some("Alohomora!12".into()),
        };
        let response = client.post("/api/v1/user").json(&new_user).send().await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    for username in valid_usernames {
        let new_user = AddUserData {
            username: username.into(),
            last_name: "Dumbledore".into(),
            first_name: "Albus".into(),
            email: "a.dumbledore@hogwart.edu.uk".into(),
            phone: Some("1234".into()),
            password: Some("Alohomora!12".into()),
        };
        let response = client.post("/api/v1/user").json(&new_user).send().await;
        assert_eq!(response.status(), StatusCode::CREATED);
    }
}

#[tokio::test]
async fn test_check_password_strength() {
    let client = make_client().await;

    // auth session with admin
    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

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
            username: format!("weakpass{index}"),
            first_name: "testpassfn".into(),
            last_name: "testpassln".into(),
            email: format!("testpass{index}@test.test"),
            password: Some(password.to_owned().into()),
            phone: None,
        };
        let response = client
            .post("/api/v1/user")
            .json(&weak_password_user)
            .send()
            .await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
    let strong_password_user = AddUserData {
        username: "strongpass".into(),
        first_name: "Strong".into(),
        last_name: "Pass".into(),
        email: "strongpass@test.test".into(),
        phone: None,
        password: Some(strong_password.into()),
    };
    let response = client
        .post("/api/v1/user")
        .json(&strong_password_user)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn test_user_unregister_authorized_app() {
    let client = make_client().await;
    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let openid_client = NewOpenIDClient {
        name: "Test".into(),
        redirect_uri: vec!["http://localhost:3000/".into()],
        scope: vec!["openid".into()],
        enabled: true,
    };
    let response = client
        .post("/api/v1/oauth")
        .json(&openid_client)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let openid_client: OAuth2Client = response.json().await;
    assert_eq!(openid_client.name, "Test");
    let response = client
        .post(format!(
            "/api/v1/oauth/authorize?\
            response_type=code&\
            client_id={}&\
            redirect_uri=http%3A%2F%2Flocalhost%3A3000&\
            scope=openid&\
            state=ABCDEF&\
            allow=true&\
            nonce=blabla",
            openid_client.client_id
        ))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FOUND);
    let response = client.get("/api/v1/me").send().await;
    let mut user_info: UserInfo = response.json().await;
    assert_eq!(user_info.authorized_apps.len(), 1);
    user_info.authorized_apps = [].into();
    let response = client
        .put("/api/v1/user/admin")
        .json(&user_info)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let response = client.get("/api/v1/me").send().await;
    let user_info: UserInfo = response.json().await;
    assert_eq!(user_info.authorized_apps.len(), 0);
}

fn make_network() -> Value {
    json!({
        "name": "network",
        "address": "10.1.1.1/24",
        "port": 55555,
        "endpoint": "192.168.4.14",
        "allowed_ips": "10.1.1.0/24",
        "dns": "1.1.1.1",
        "allowed_groups": [],
        "mfa_enabled": false,
        "keepalive_interval": 25,
        "peer_disconnect_threshold": 180
    })
}

#[tokio::test]
async fn test_user_add_device() {
    let (client, state) = make_test_client().await;
    let mut mail_rx = state.mail_rx;
    let user_agent_header = "Mozilla/5.0 (iPhone; CPU iPhone OS 17_1 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.1 Mobile/15E148 Safari/604.1";

    // log in as admin
    let auth = Auth::new("admin", "pass123");
    let response = client
        .post("/api/v1/auth")
        .header(USER_AGENT, user_agent_header)
        .json(&auth)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    // first email received is regarding admin login
    let mail = mail_rx.try_recv().unwrap();
    assert_eq!(mail.to, "admin@defguard");
    assert_eq!(
        mail.subject,
        "Defguard: new device logged in to your account"
    );

    // create network
    let response = client
        .post("/api/v1/network")
        .json(&make_network())
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // add device for user
    let device_data = AddDevice {
        name: "TestDevice1".into(),
        wireguard_pubkey: "mgVXE8WcfStoD8mRatHcX5aaQ0DlcpjvPXibHEOr9y8=".into(),
    };
    let response = client
        .post("/api/v1/device/hpotter")
        .header(USER_AGENT, user_agent_header)
        .json(&device_data)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // send email regarding new device being added
    // it does not contain session info
    let mail = mail_rx.try_recv().unwrap();
    assert_eq!(mail.to, "h.potter@hogwart.edu.uk");
    assert_eq!(mail.subject, "Defguard: new device added to your account");
    assert!(!mail.content.contains("IP Address:</span>"));
    assert!(!mail.content.contains("Device type:</span>"));

    // add device for themselves
    let device_data = AddDevice {
        name: "TestDevice2".into(),
        wireguard_pubkey: "mgVXE8WcfStoD8mRatHcX5aaQ0DlcpjvPXibHEOr9y8=".into(),
    };
    let response = client
        .post("/api/v1/device/admin")
        .header(USER_AGENT, user_agent_header)
        .json(&device_data)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // send email regarding new device being added
    // it should contain session info
    let mail = mail_rx.try_recv().unwrap();
    assert_eq!(mail.to, "admin@defguard");
    assert_eq!(mail.subject, "Defguard: new device added to your account");
    assert!(mail.content.contains("IP Address:</span> 127.0.0.1"));
    assert!(mail
        .content
        .contains("Device type:</span> iPhone, OS: iOS 17.1, Mobile Safari"));

    // log in as normal user
    let auth = Auth::new("hpotter", "pass123");
    let response = client
        .post("/api/v1/auth")
        .header(USER_AGENT, user_agent_header)
        .json(&auth)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    let response = client.get("/api/v1/me").send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // send email regarding user login
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

    // normal user cannot add a device for other users
    let device_data = AddDevice {
        name: "TestDevice3".into(),
        wireguard_pubkey: "mgVXE8WcfStoD8mRatHcX5aaQ0DlcpjvPXibHEOr9y8=".into(),
    };
    let response = client
        .post("/api/v1/device/admin")
        .header(USER_AGENT, user_agent_header)
        .json(&device_data)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    // user adds a device for themselves
    let response = client
        .post("/api/v1/device/hpotter")
        .header(USER_AGENT, user_agent_header)
        .json(&device_data)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // send email regarding new device being added
    let mail = mail_rx.try_recv().unwrap();
    assert_eq!(mail.to, "h.potter@hogwart.edu.uk");
    assert_eq!(mail.subject, "Defguard: new device added to your account");
    assert!(mail.content.contains("IP Address:</span> 127.0.0.1"));
    assert!(mail
        .content
        .contains("Device type:</span> iPhone, OS: iOS 17.1, Mobile Safari"));
}

#[tokio::test]
async fn test_disable() {
    let client = make_client().await;

    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // get yourself
    let mut user_details = fetch_user_details(&client, "admin").await;
    user_details.user.is_active = false;

    // disable yourself
    let response = client
        .put("/api/v1/user/admin")
        .json(&user_details.user)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // create user
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

    // get user
    let mut user_details = fetch_user_details(&client, "adumbledore").await;
    assert_eq!(user_details.user.first_name, "Albus");

    // disable user
    user_details.user.is_active = false;
    let response = client
        .put("/api/v1/user/adumbledore")
        .json(&user_details.user)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
}
