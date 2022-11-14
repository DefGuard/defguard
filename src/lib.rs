#![allow(clippy::derive_partial_eq_without_eq)]
// oxide macro
#![allow(clippy::unnecessary_lazy_evaluations)]

#[cfg(feature = "oauth")]
use crate::enterprise::handlers::oauth::{authorize, authorize_consent, refresh, token};
#[cfg(feature = "worker")]
use crate::enterprise::handlers::worker::{
    create_job, create_worker_token, job_status, list_workers, remove_worker,
};
#[cfg(feature = "openid")]
use crate::enterprise::handlers::{
    openid_clients::{
        add_openid_client, change_openid_client, change_openid_client_state, delete_openid_client,
        delete_user_app, get_openid_client, get_user_apps, list_openid_clients, update_user_app,
    },
    openid_flow::{authentication_request, check_authorized, id_token, openid_configuration},
};
#[cfg(feature = "oauth")]
use crate::enterprise::oauth_state::OAuthState;
use crate::enterprise::{db::openid::AuthorizedApp, grpc::WorkerState};
#[cfg(any(feature = "oauth", feature = "openid", feature = "worker"))]
use crate::license::Features;
use crate::license::License;
use appstate::AppState;
use chrono::Utc;
use config::DefGuardConfig;
use db::{init_db, AppEvent, DbPool, Device, GatewayEvent, WireguardNetwork};
#[cfg(feature = "wireguard")]
use handlers::wireguard::{
    add_device, create_network, create_network_token, delete_device, delete_network,
    download_config, get_device, list_devices, list_networks, list_user_devices, modify_device,
    modify_network, network_details, network_stats, user_stats,
};
use handlers::{
    auth::{
        authenticate, logout, mfa_disable, mfa_enable, recovery_code, totp_code, totp_disable,
        totp_enable, totp_secret, web3auth_end, web3auth_start, webauthn_end, webauthn_finish,
        webauthn_init, webauthn_start,
    },
    group::{add_group_member, get_group, list_groups, remove_group_member},
    license::get_license,
    settings::{get_settings, update_settings},
    user::{
        add_user, change_password, delete_security_key, delete_user, delete_wallet, get_user,
        list_users, me, modify_user, set_wallet, update_wallet, username_available,
        wallet_challenge,
    },
    webhooks::{
        add_webhook, change_enabled, change_webhook, delete_webhook, get_webhook, list_webhooks,
    },
};
use rocket::{
    config::Config,
    error::Error as RocketError,
    fs::{FileServer, NamedFile, Options},
    Build, Ignite, Rocket,
};
use std::{
    net::{IpAddr, Ipv4Addr},
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

pub mod appstate;
pub mod auth;
pub mod config;
pub mod db;
pub mod enterprise;
mod error;
pub mod grpc;
pub mod handlers;
mod hex;
pub mod license;
#[cfg(feature = "oauth")]
pub mod oxide_auth_rocket;

#[macro_use]
extern crate rocket;

#[macro_use]
extern crate serde;

/// Catch missing files and serve "index.html".
#[get("/<path..>", rank = 4)]
async fn smart_index(path: PathBuf) -> Option<NamedFile> {
    if path.starts_with("api/") {
        None
    } else {
        NamedFile::open("./web/index.html").await.ok()
    }
}

/// Simple health-check.
#[get("/health")]
fn health_check() -> &'static str {
    "alive"
}

pub async fn build_webapp(
    config: DefGuardConfig,
    webhook_tx: UnboundedSender<AppEvent>,
    webhook_rx: UnboundedReceiver<AppEvent>,
    wireguard_tx: UnboundedSender<GatewayEvent>,
    pool: DbPool,
) -> Rocket<Build> {
    // configure Rocket webapp
    let cfg = Config {
        address: IpAddr::V4(Ipv4Addr::UNSPECIFIED),
        port: config.http_port,
        ..Config::default()
    };
    let license_decoded = License::decode(&config.license);
    let webapp = rocket::custom(cfg)
        .mount("/", routes![smart_index])
        .mount("/", FileServer::new("./web", Options::Missing).rank(3))
        .mount(
            "/api/v1",
            routes![
                health_check,
                authenticate,
                logout,
                username_available,
                list_users,
                get_user,
                add_user,
                modify_user,
                delete_user,
                delete_security_key,
                change_password,
                wallet_challenge,
                set_wallet,
                update_wallet,
                delete_wallet,
                list_groups,
                get_group,
                me,
                add_group_member,
                remove_group_member,
                get_license,
                get_settings,
                update_settings,
                mfa_enable,
                mfa_disable,
                totp_secret,
                totp_disable,
                totp_enable,
                totp_code,
                webauthn_init,
                webauthn_finish,
                webauthn_start,
                webauthn_end,
                web3auth_start,
                web3auth_end,
                recovery_code
            ],
        )
        .mount(
            "/api/v1/webhook",
            routes![
                add_webhook,
                list_webhooks,
                get_webhook,
                delete_webhook,
                change_webhook,
                change_enabled
            ],
        );
    #[cfg(feature = "wireguard")]
    let webapp = webapp.mount(
        "/api/v1",
        routes![
            add_device,
            get_device,
            list_user_devices,
            modify_device,
            delete_device,
            list_devices,
            download_config,
        ],
    );
    // initialize webapp with network routes
    #[cfg(feature = "wireguard")]
    let webapp = webapp.mount(
        "/api/v1/network",
        routes![
            create_network,
            delete_network,
            modify_network,
            list_networks,
            network_details,
            create_network_token,
            user_stats,
            network_stats,
        ],
    );
    #[cfg(feature = "openid")]
    let webapp = if license_decoded.validate(&Features::Openid) {
        info!("Openid feature is enabled");
        webapp.mount(
            "/api/v1/openid",
            routes![
                add_openid_client,
                delete_openid_client,
                change_openid_client,
                list_openid_clients,
                get_openid_client,
                authentication_request,
                id_token,
                change_openid_client_state,
                openid_configuration,
                check_authorized,
                update_user_app,
                delete_user_app,
                get_user_apps
            ],
        )
    } else {
        webapp
    };

    // initialize OAuth2
    #[cfg(feature = "oauth")]
    let webapp = if config.oauth_enabled && license_decoded.validate(&Features::Oauth) {
        info!("OAuth2 feature is enabled");
        webapp.manage(OAuthState::new(pool.clone()).await).mount(
            "/api/oauth",
            routes![authorize, authorize_consent, token, refresh],
        )
    } else {
        webapp
    };

    webapp.manage(
        AppState::new(
            config,
            pool,
            webhook_tx,
            webhook_rx,
            wireguard_tx,
            license_decoded,
        )
        .await,
    )
}

/// Runs core web server exposing REST API.
pub async fn run_web_server(
    config: DefGuardConfig,
    worker_state: Arc<Mutex<WorkerState>>,
    webhook_tx: UnboundedSender<AppEvent>,
    webhook_rx: UnboundedReceiver<AppEvent>,
    wireguard_tx: UnboundedSender<GatewayEvent>,
    pool: DbPool,
) -> Result<Rocket<Ignite>, RocketError> {
    let webapp = build_webapp(config.clone(), webhook_tx, webhook_rx, wireguard_tx, pool).await;
    #[cfg(feature = "worker")]
    let webapp = if License::decode(&config.license).validate(&Features::Worker) {
        info!("Worker feature is enabled");
        webapp.manage(worker_state).mount(
            "/api/v1/worker",
            routes![
                create_job,
                list_workers,
                job_status,
                remove_worker,
                create_worker_token
            ],
        )
    } else {
        webapp
    };

    info!("Started web services");
    webapp.launch().await
}

/// Automates test objects creation to easily setup development environment.
/// Test network keys:
/// Public: zGMeVGm9HV9I4wSKF9AXmYnnAIhDySyqLMuKpcfIaQo=
/// Private: MAk3d5KuB167G88HM7nGYR6ksnPMAOguAg2s5EcPp1M=
/// Test device keys:
/// Public: gQYL5eMeFDj0R+lpC7oZyIl0/sNVmQDC6ckP7husZjc=
/// Private: wGS1qdJfYbWJsOUuP1IDgaJYpR+VaKZPVZvdmLjsH2Y=
pub async fn init_dev_env(config: &DefGuardConfig) {
    log::debug!("Initializing dev environment");
    let pool = init_db(
        &config.database_host,
        config.database_port,
        &config.database_name,
        &config.database_user,
        &config.database_password,
    )
    .await;
    let mut network = WireguardNetwork::new(
        "TestNet".to_string(),
        "10.1.1.1/24".parse().unwrap(),
        50051,
        "0.0.0.0".to_string(),
        None,
        vec!["10.1.1.0/24".parse().unwrap()],
    )
    .expect("Could not create network");
    network.pubkey = "zGMeVGm9HV9I4wSKF9AXmYnnAIhDySyqLMuKpcfIaQo=".to_string();
    network.prvkey = "MAk3d5KuB167G88HM7nGYR6ksnPMAOguAg2s5EcPp1M=".to_string();
    network.save(&pool).await.expect("Could not save network");

    let mut device = Device::new(
        "TestDevice".to_string(),
        "10.1.1.10".to_string(),
        "gQYL5eMeFDj0R+lpC7oZyIl0/sNVmQDC6ckP7husZjc=".to_string(),
        1,
    );
    device.save(&pool).await.expect("Could not save device");

    for app_id in &[1, 2, 3] {
        let mut app = AuthorizedApp::new(
            1,
            format!("client-id-{}", app_id),
            format!("https://app-{}.com", app_id),
            Utc::now().naive_utc().to_string(),
            format!("app-{}", app_id),
        );
        app.save(&pool)
            .await
            .expect("Could not save authorized app");
    }
    log::info!("Dev environment initialized - TestNet, TestDevice, AuthorizedApps added");
}
