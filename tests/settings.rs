use defguard::{
    build_webapp,
    db::{models::settings::Settings, AppEvent, GatewayEvent},
    grpc::{GatewayState, WorkerState},
    handlers::Auth,
};
use rocket::{http::Status, local::asynchronous::Client};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::unbounded_channel;

mod common;
use common::init_test_db;
use defguard::db::User;

async fn make_client() -> Client {
    let (pool, config) = init_test_db().await;

    let (tx, rx) = unbounded_channel::<AppEvent>();
    let worker_state = Arc::new(Mutex::new(WorkerState::new(tx.clone())));
    let (wg_tx, wg_rx) = unbounded_channel::<GatewayEvent>();
    let gateway_state = Arc::new(Mutex::new(GatewayState::new(wg_rx)));

    User::init_admin_user(&pool, &config.default_admin_password).await.unwrap();

    let webapp = build_webapp(config, tx, rx, wg_tx, worker_state, gateway_state, pool).await;
    Client::tracked(webapp).await.unwrap()
}

#[rocket::async_test]
async fn test_settings() {
    let client = make_client().await;

    let auth = Auth::new("admin".into(), "pass123".into());
    let response = &client.post("/api/v1/auth").json(&auth).dispatch().await;
    assert_eq!(response.status(), Status::Ok);

    // get settings
    let response = client.get("/api/v1/settings").dispatch().await;
    assert_eq!(response.status(), Status::Ok);
    let mut settings: Settings = response.into_json().await.unwrap();
    assert_eq!(
        settings,
        Settings {
            id: None,
            web3_enabled: true,
            openid_enabled: true,
            oauth_enabled: true,
            ldap_enabled: true,
            wireguard_enabled: true,
            webhooks_enabled: true,
            worker_enabled: true,
            main_logo_url: "/svg/logo-defguard-white.svg".into(),
            nav_logo_url: "/svg/defguard-nav-logo.svg".into(),
            instance_name: "Defguard".into(),
            challenge_template: "
                Please read this carefully:\n\n\
                Click to sign to prove you are in possesion of your private key to the account.\n\
                This request will not trigger a blockchain transaction or cost any gas fees."
                .trim_start()
                .into(),
        }
    );

    // modify settings
    settings.wireguard_enabled = false;
    settings.challenge_template = "Modified".to_string();
    let response = client
        .put("/api/v1/settings")
        .json(&settings)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);

    // verify modified settings
    let response = client.get("/api/v1/settings").dispatch().await;
    assert_eq!(response.status(), Status::Ok);
    let new_settings: Settings = response.into_json().await.unwrap();
    assert_eq!(new_settings, settings);
}
