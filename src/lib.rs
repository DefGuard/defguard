use crate::{
    auth::{Claims, ClaimsType},
    config::InitVpnLocationArgs,
    db::User,
    // handlers::{
    //     mail::{send_support_data, test_mail},
    //     support::{configuration, logs},
    //     user::change_self_password,
    // },
};
use secrecy::ExposeSecret;

// #[cfg(feature = "worker")]
// use crate::handlers::worker::{
//     create_job, create_worker_token, job_status, list_workers, remove_worker,
// };
// #[cfg(feature = "openid")]
// use crate::handlers::{
//     openid_clients::{
//         add_openid_client, change_openid_client, change_openid_client_state, delete_openid_client,
//         get_openid_client, list_openid_clients,
//     },
//     openid_flow::{
//         authorization, discovery_keys, openid_configuration, secure_authorization, token, userinfo,
//     },
// };
#[cfg(any(feature = "oauth", feature = "openid", feature = "worker"))]
use crate::{
    auth::failed_login::FailedLoginMap,
    db::models::oauth2client::OAuth2Client,
    grpc::{GatewayMap, WorkerState},
    // handlers::{
    //     app_info::get_app_info,
    //     wireguard::{add_user_devices, import_network},
    // },
};
use anyhow::anyhow;
use appstate::AppState;
use axum::{
    handler::HandlerWithoutStateExt,
    http::{Request, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Router, Server,
};
use config::DefGuardConfig;
use db::{init_db, AppEvent, DbPool, Device, GatewayEvent, WireguardNetwork};
// #[cfg(feature = "wireguard")]
// use handlers::auth::{
//     authenticate, logout, mfa_disable, mfa_enable, recovery_code, totp_code, totp_disable,
//     totp_enable, totp_secret, web3auth_end, web3auth_start, webauthn_end, webauthn_finish,
//     webauthn_init, webauthn_start,
// };
use mail::Mail;
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::{Arc, Mutex},
};
use tokio::sync::{
    broadcast::Sender,
    mpsc::{UnboundedReceiver, UnboundedSender},
    OnceCell,
};
use tower_http::{
    services::{ServeDir, ServeFile},
    trace::{DefaultOnResponse, TraceLayer},
};
use tracing::Level;

pub mod appstate;
pub mod auth;
pub mod config;
pub mod db;
mod error;
pub mod grpc;
pub mod handlers;
pub mod hex;
pub mod ldap;
// pub mod license;
pub mod mail;
pub(crate) mod random;
pub mod secret;
pub mod support;
pub mod templates;
pub mod wg_config;
pub mod wireguard_stats_purge;

#[macro_use]
extern crate tracing;

#[macro_use]
extern crate serde;

pub static VERSION: &str = env!("CARGO_PKG_VERSION");
// TODO: use in more contexts instead of cloning/passing config around
pub static SERVER_CONFIG: OnceCell<DefGuardConfig> = OnceCell::const_new();

/// Simple health-check.
async fn health_check() -> &'static str {
    "alive"
}

async fn handle_404() -> (StatusCode, &'static str) {
    (StatusCode::NOT_FOUND, "Not found")
}

pub async fn build_webapp(
    config: DefGuardConfig,
    webhook_tx: UnboundedSender<AppEvent>,
    webhook_rx: UnboundedReceiver<AppEvent>,
    wireguard_tx: Sender<GatewayEvent>,
    mail_tx: UnboundedSender<Mail>,
    worker_state: Arc<Mutex<WorkerState>>,
    gateway_state: Arc<Mutex<GatewayMap>>,
    pool: DbPool,
    failed_logins: Arc<Mutex<FailedLoginMap>>,
) -> Router {
    // let cfg = Config {
    //     address: IpAddr::V4(Ipv4Addr::UNSPECIFIED),
    //     port: config.http_port,
    //     secret_key: SecretKey::from(config.secret_key.expose_secret().as_bytes()),
    //     ..Config::default()
    // };
    // let license_decoded = License::decode(&config.license);
    // info!("Using license: {:?}", license_decoded);
    let serve_web_dir = ServeDir::new("web/dist").fallback(ServeFile::new("web/dist/index.html"));
    let serve_images =
        ServeDir::new("web/src/shared/images/svg").not_found_service(handle_404.into_service());
    let webapp = Router::new()
        .nest(
            "/api/v1",
            Router::new().route("/health", get(health_check)), // .route("/auth", post(authenticate)),
        )
        .nest_service("/svg", serve_images)
        .fallback_service(serve_web_dir)
        .with_state(AppState::new(
            config,
            pool,
            webhook_tx,
            webhook_rx,
            wireguard_tx,
            mail_tx,
            failed_logins,
        ))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(|request: &Request<_>| {
                    info_span!(
                        "http_request",
                        method = ?request.method(),
                        path = ?request.uri(),
                    )
                })
                .on_response(DefaultOnResponse::new().level(Level::INFO)),
        );

    webapp

    // let webapp = rocket::custom(cfg)
    //     .mount("/", routes![smart_index])
    //     .mount("/", FileServer::new("./web/dist", Options::Missing).rank(3))
    //     .mount(
    //         "/svg",
    //         FileServer::new("./web/src/shared/images/svg", Options::Index).rank(1),
    //     )
    //     .mount(
    //         "/api/v1",
    //         routes![
    //             health_check,
    //             authenticate,
    //             forward_auth,
    //             logout,
    //             username_available,
    //             list_users,
    //             get_user,
    //             add_user,
    //             start_enrollment,
    //             modify_user,
    //             delete_user,
    //             delete_security_key,
    //             change_password,
    //             wallet_challenge,
    //             set_wallet,
    //             update_wallet,
    //             delete_wallet,
    //             list_groups,
    //             get_group,
    //             me,
    //             add_group_member,
    //             remove_group_member,
    //             get_license,
    //             mfa_enable,
    //             mfa_disable,
    //             totp_secret,
    //             totp_disable,
    //             totp_enable,
    //             totp_code,
    //             webauthn_init,
    //             webauthn_finish,
    //             webauthn_start,
    //             webauthn_end,
    //             web3auth_start,
    //             web3auth_end,
    //             delete_authorized_app,
    //             recovery_code,
    //             get_app_info,
    //             change_self_password,
    //         ],
    //     )
    //     .mount(
    //         "/api/v1/settings",
    //         routes![get_settings, update_settings, set_default_branding],
    //     )
    //     .mount("/api/v1/support", routes![configuration, logs])
    //     .mount(
    //         "/api/v1/webhook",
    //         routes![
    //             add_webhook,
    //             list_webhooks,
    //             get_webhook,
    //             delete_webhook,
    //             change_webhook,
    //             change_enabled
    //         ],
    //     )
    //     .mount("/api/v1/mail", routes![test_mail, send_support_data]);

    // #[cfg(feature = "wireguard")]
    // let webapp = webapp.manage(gateway_state).mount(
    //     "/api/v1",
    //     routes![
    //         add_device,
    //         get_device,
    //         list_user_devices,
    //         modify_device,
    //         delete_device,
    //         list_devices,
    //     ],
    // );

    // // initialize webapp with network routes
    // #[cfg(feature = "wireguard")]
    // let webapp = webapp.mount(
    //     "/api/v1/network",
    //     routes![
    //         create_network,
    //         delete_network,
    //         modify_network,
    //         list_networks,
    //         network_details,
    //         gateway_status,
    //         remove_gateway,
    //         import_network,
    //         add_user_devices,
    //         create_network_token,
    //         user_stats,
    //         network_stats,
    //         download_config,
    //     ],
    // );

    // #[cfg(feature = "openid")]
    // let webapp = if license_decoded.validate(&Features::Openid) {
    //     webapp
    //         .mount(
    //             "/api/v1/oauth",
    //             routes![
    //                 discovery_keys,
    //                 add_openid_client,
    //                 list_openid_clients,
    //                 delete_openid_client,
    //                 change_openid_client,
    //                 get_openid_client,
    //                 authorization,
    //                 secure_authorization,
    //                 token,
    //                 userinfo,
    //                 change_openid_client_state,
    //             ],
    //         )
    //         .mount("/.well-known", routes![openid_configuration])
    // } else {
    //     webapp
    // };

    // #[cfg(feature = "worker")]
    // let webapp = if license_decoded.validate(&Features::Worker) {
    //     webapp.manage(worker_state).mount(
    //         "/api/v1/worker",
    //         routes![
    //             create_job,
    //             list_workers,
    //             job_status,
    //             remove_worker,
    //             create_worker_token
    //         ],
    //     )
    // } else {
    //     webapp
    // };

    // webapp.manage(AppState::new(
    //     config,
    //     pool,
    //     webhook_tx,
    //     webhook_rx,
    //     wireguard_tx,
    //     mail_tx,
    //     license_decoded,
    //     failed_logins,
    // ))
}

/// Runs core web server exposing REST API.
pub async fn run_web_server(
    config: &DefGuardConfig,
    worker_state: Arc<Mutex<WorkerState>>,
    gateway_state: Arc<Mutex<GatewayMap>>,
    webhook_tx: UnboundedSender<AppEvent>,
    webhook_rx: UnboundedReceiver<AppEvent>,
    wireguard_tx: Sender<GatewayEvent>,
    mail_tx: UnboundedSender<Mail>,
    pool: DbPool,
    failed_logins: Arc<Mutex<FailedLoginMap>>,
) -> Result<(), hyper::Error> {
    let webapp = build_webapp(
        config.clone(),
        webhook_tx,
        webhook_rx,
        wireguard_tx,
        mail_tx,
        worker_state,
        gateway_state,
        pool,
        failed_logins,
    )
    .await;
    info!("Started web services");
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), config.http_port);
    Server::bind(&addr).serve(webapp.into_make_service()).await
}

/// Automates test objects creation to easily setup development environment.
/// Test network keys:
/// Public: zGMeVGm9HV9I4wSKF9AXmYnnAIhDySyqLMuKpcfIaQo=
/// Private: MAk3d5KuB167G88HM7nGYR6ksnPMAOguAg2s5EcPp1M=
/// Test device keys:
/// Public: gQYL5eMeFDj0R+lpC7oZyIl0/sNVmQDC6ckP7husZjc=
/// Private: wGS1qdJfYbWJsOUuP1IDgaJYpR+VaKZPVZvdmLjsH2Y=
pub async fn init_dev_env(config: &DefGuardConfig) {
    info!("Initializing dev environment");
    let pool = init_db(
        &config.database_host,
        config.database_port,
        &config.database_name,
        &config.database_user,
        config.database_password.expose_secret(),
    )
    .await;

    // initialize admin user
    User::init_admin_user(&pool, config.default_admin_password.expose_secret())
        .await
        .expect("Failed to create admin user");

    let mut transaction = pool
        .begin()
        .await
        .expect("Failed to initialize transaction");

    let network = if let Some(networks) =
        WireguardNetwork::find_by_name(&mut *transaction, "TestNet")
            .await
            .expect("Failed to search for test network")
    {
        info!("Test network exists already, skipping creation...");
        networks.into_iter().next().unwrap()
    } else {
        info!("Creating test network ");
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
            .save(&mut *transaction)
            .await
            .expect("Could not save network");
        network
    };

    if Device::find_by_pubkey(
        &mut *transaction,
        "gQYL5eMeFDj0R+lpC7oZyIl0/sNVmQDC6ckP7husZjc=",
    )
    .await
    .expect("Failed to search for test device")
    .is_some()
    {
        info!("Test device exists already, skipping creation...");
    } else {
        info!("Creating test device");
        let mut device = Device::new(
            "TestDevice".to_string(),
            "gQYL5eMeFDj0R+lpC7oZyIl0/sNVmQDC6ckP7husZjc=".to_string(),
            1,
        );
        device
            .save(&mut *transaction)
            .await
            .expect("Could not save device");
        device
            .assign_network_ip(&mut transaction, &network, None)
            .await
            .expect("Could not assign IP to device");
    }

    for app_id in 1..=3 {
        let mut app = OAuth2Client::new(
            vec![format!("https://app-{app_id}.com")],
            vec!["openid".into(), "profile".into(), "email".into()],
            format!("app-{app_id}"),
        );
        app.save(&mut *transaction)
            .await
            .expect("Could not save oauth2client");
    }
    transaction
        .commit()
        .await
        .expect("Failed to commit transaction");

    info!("Dev environment initialized - TestNet, TestDevice, AuthorizedApps added");
}

/// Create a new VPN location.
/// Meant to be used to automate setting up a new defguard instance.
/// Does not handle assigning device IPs, since no device should exist at this point.
pub async fn init_vpn_location(
    pool: &DbPool,
    args: &InitVpnLocationArgs,
) -> Result<String, anyhow::Error> {
    // check if a VPN location exists already
    let networks = WireguardNetwork::all(pool).await?;
    if !networks.is_empty() {
        return Err(anyhow!(
            "Failed to initialize first VPN location. A location already exists."
        ));
    };

    // create a new network
    let mut network = WireguardNetwork::new(
        args.name.clone(),
        args.address,
        args.port,
        args.endpoint.clone(),
        args.dns.clone(),
        args.allowed_ips.clone(),
    )?;
    network.save(pool).await?;
    let network_id = network.get_id()?;

    // generate gateway token
    let token = Claims::new(
        ClaimsType::Gateway,
        format!("DEFGUARD-NETWORK-{network_id}"),
        network_id.to_string(),
        u32::MAX.into(),
    )
    .to_jwt()?;

    Ok(token)
}
