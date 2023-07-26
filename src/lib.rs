#![allow(clippy::derive_partial_eq_without_eq)]
// Rocket macro
#![allow(clippy::unnecessary_lazy_evaluations)]
#![allow(clippy::too_many_arguments)]

use crate::db::User;
use crate::handlers::user::change_self_password;
#[cfg(feature = "worker")]
use crate::handlers::worker::{
    create_job, create_worker_token, job_status, list_workers, remove_worker,
};
#[cfg(feature = "openid")]
use crate::handlers::{
    openid_clients::{
        add_openid_client, change_openid_client, change_openid_client_state, delete_openid_client,
        get_openid_client, list_openid_clients,
    },
    openid_flow::{
        authorization, discovery_keys, openid_configuration, secure_authorization, token, userinfo,
    },
};
#[cfg(any(feature = "oauth", feature = "openid", feature = "worker"))]
use crate::{
    auth::failed_login::FailedLoginMap,
    db::models::oauth2client::OAuth2Client,
    grpc::GatewayMap,
    grpc::WorkerState,
    handlers::app_info::get_app_info,
    handlers::wireguard::{add_user_devices, import_network},
    license::{Features, License},
};
use appstate::AppState;
use config::DefGuardConfig;
use db::{init_db, AppEvent, DbPool, Device, GatewayEvent, WireguardNetwork};
#[cfg(feature = "wireguard")]
use handlers::{
    auth::{
        authenticate, logout, mfa_disable, mfa_enable, recovery_code, totp_code, totp_disable,
        totp_enable, totp_secret, web3auth_end, web3auth_start, webauthn_end, webauthn_finish,
        webauthn_init, webauthn_start,
    },
    group::{add_group_member, get_group, list_groups, remove_group_member},
    license::get_license,
    settings::{get_settings, set_default_branding, update_settings},
    user::{
        add_user, change_password, delete_authorized_app, delete_security_key, delete_user,
        delete_wallet, get_user, list_users, me, modify_user, set_wallet, update_wallet,
        username_available, wallet_challenge,
    },
    webhooks::{
        add_webhook, change_enabled, change_webhook, delete_webhook, get_webhook, list_webhooks,
    },
    wireguard::{
        add_device, create_network, create_network_token, delete_device, delete_network,
        download_config, gateway_status, get_device, list_devices, list_networks,
        list_user_devices, modify_device, modify_network, network_details, network_stats,
        remove_gateway, user_stats,
    },
};
use rocket::{
    config::{Config, SecretKey},
    error::Error as RocketError,
    fs::{FileServer, NamedFile, Options},
    Build, Ignite, Rocket,
};
use std::{
    net::{IpAddr, Ipv4Addr},
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tokio::sync::{
    broadcast::Sender,
    mpsc::{UnboundedReceiver, UnboundedSender},
};

pub mod appstate;
pub mod auth;
pub mod config;
pub mod db;
mod error;
pub mod grpc;
pub mod handlers;
pub mod hex;
pub mod ldap;
pub mod license;
pub(crate) mod random;
pub mod wg_config;
pub mod wireguard_stats_purge;

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
        NamedFile::open("./web/dist/index.html").await.ok()
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
    wireguard_tx: Sender<GatewayEvent>,
    worker_state: Arc<Mutex<WorkerState>>,
    gateway_state: Arc<Mutex<GatewayMap>>,
    pool: DbPool,
    failed_logins: Arc<Mutex<FailedLoginMap>>,
) -> Rocket<Build> {
    // configure Rocket webapp
    let cfg = Config {
        address: IpAddr::V4(Ipv4Addr::UNSPECIFIED),
        port: config.http_port,
        secret_key: SecretKey::from(config.secret_key.as_bytes()),
        ..Config::default()
    };
    let license_decoded = License::decode(&config.license);
    info!("Using license: {:?}", license_decoded);
    let webapp = rocket::custom(cfg)
        .mount("/", routes![smart_index])
        .mount("/", FileServer::new("./web/dist", Options::Missing).rank(3))
        .mount(
            "/svg",
            FileServer::new("./web/src/shared/images/svg", Options::Index).rank(1),
        )
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
                set_default_branding,
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
                delete_authorized_app,
                recovery_code,
                get_app_info,
                change_self_password
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
    let webapp = webapp.manage(gateway_state).mount(
        "/api/v1",
        routes![
            add_device,
            get_device,
            list_user_devices,
            modify_device,
            delete_device,
            list_devices,
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
            gateway_status,
            remove_gateway,
            import_network,
            add_user_devices,
            create_network_token,
            user_stats,
            network_stats,
            download_config,
        ],
    );

    #[cfg(feature = "openid")]
    let webapp = if license_decoded.validate(&Features::Openid) {
        webapp
            .mount(
                "/api/v1/oauth",
                routes![
                    discovery_keys,
                    add_openid_client,
                    list_openid_clients,
                    delete_openid_client,
                    change_openid_client,
                    get_openid_client,
                    authorization,
                    secure_authorization,
                    token,
                    userinfo,
                    change_openid_client_state,
                ],
            )
            .mount("/.well-known", routes![openid_configuration])
    } else {
        webapp
    };

    #[cfg(feature = "worker")]
    let webapp = if license_decoded.validate(&Features::Worker) {
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

    webapp.manage(
        AppState::new(
            config,
            pool,
            webhook_tx,
            webhook_rx,
            wireguard_tx,
            license_decoded,
            failed_logins,
        )
        .await,
    )
}

/// Runs core web server exposing REST API.
pub async fn run_web_server(
    config: DefGuardConfig,
    worker_state: Arc<Mutex<WorkerState>>,
    gateway_state: Arc<Mutex<GatewayMap>>,
    webhook_tx: UnboundedSender<AppEvent>,
    webhook_rx: UnboundedReceiver<AppEvent>,
    wireguard_tx: Sender<GatewayEvent>,
    pool: DbPool,
    failed_logins: Arc<Mutex<FailedLoginMap>>,
) -> Result<Rocket<Ignite>, RocketError> {
    let webapp = build_webapp(
        config.clone(),
        webhook_tx,
        webhook_rx,
        wireguard_tx,
        worker_state,
        gateway_state,
        pool,
        failed_logins,
    )
    .await;
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

    // initialize admin user
    User::init_admin_user(&pool, &config.default_admin_password)
        .await
        .expect("Failed to create admin user");

    let mut transaction = pool
        .begin()
        .await
        .expect("Failed to initialize transaction");

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
    network
        .save(&mut transaction)
        .await
        .expect("Could not save network");

    let mut device = Device::new(
        "TestDevice".to_string(),
        "gQYL5eMeFDj0R+lpC7oZyIl0/sNVmQDC6ckP7husZjc=".to_string(),
        1,
    );
    device
        .save(&mut transaction)
        .await
        .expect("Could not save device");
    device
        .assign_network_ip(&mut transaction, &network, None)
        .await
        .expect("Could not assign IP to device");

    for app_id in 1..=3 {
        let mut app = OAuth2Client::new(
            vec![format!("https://app-{}.com", app_id)],
            vec!["openid".into(), "profile".into(), "email".into()],
            format!("app-{}", app_id),
        );
        app.save(&mut transaction)
            .await
            .expect("Could not save oauth2client");
    }
    transaction
        .commit()
        .await
        .expect("Failed to commit transaction");

    info!("Dev environment initialized - TestNet, TestDevice, AuthorizedApps added");
}
