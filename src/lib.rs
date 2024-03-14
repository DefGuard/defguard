#![allow(clippy::too_many_arguments)]
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::{Arc, Mutex},
};

use anyhow::anyhow;
use axum::{
    handler::HandlerWithoutStateExt,
    http::{Request, StatusCode},
    routing::{delete, get, patch, post, put},
    serve, Extension, Router,
};

use handlers::ssh_authorized_keys::{
    add_authentication_key, delete_authentication_key, fetch_authentication_keys,
};
use handlers::{
    group::{bulk_assign_to_groups, list_groups_info},
    ssh_authorized_keys::rename_authentication_key,
    yubikey::{delete_yubikey, rename_yubikey},
};
use ipnetwork::IpNetwork;
use secrecy::ExposeSecret;
use tokio::{
    net::TcpListener,
    sync::{
        broadcast::Sender,
        mpsc::{UnboundedReceiver, UnboundedSender},
        OnceCell,
    },
};
use tower_http::{
    services::{ServeDir, ServeFile},
    trace::{DefaultOnResponse, TraceLayer},
};
use tracing::Level;
use uaparser::UserAgentParser;

use self::{
    appstate::AppState,
    auth::{Claims, ClaimsType},
    config::{DefGuardConfig, InitVpnLocationArgs},
    db::{
        init_db,
        models::wireguard::{DEFAULT_DISCONNECT_THRESHOLD, DEFAULT_KEEPALIVE_INTERVAL},
        AppEvent, DbPool, Device, GatewayEvent, User, WireguardNetwork,
    },
    handlers::{
        auth::{
            authenticate, email_mfa_code, email_mfa_disable, email_mfa_enable, email_mfa_init,
            logout, mfa_disable, mfa_enable, recovery_code, request_email_mfa_code, totp_code,
            totp_disable, totp_enable, totp_secret, web3auth_end, web3auth_start, webauthn_end,
            webauthn_finish, webauthn_init, webauthn_start,
        },
        forward_auth::forward_auth,
        group::{
            add_group_member, create_group, delete_group, get_group, list_groups, modify_group,
            remove_group_member,
        },
        mail::{send_support_data, test_mail},
        settings::{
            get_settings, get_settings_essentials, patch_settings, set_default_branding,
            test_ldap_settings, update_settings,
        },
        ssh_authorized_keys::get_authorized_keys,
        support::{configuration, logs},
        user::{
            add_user, change_password, change_self_password, delete_authorized_app,
            delete_security_key, delete_user, delete_wallet, get_user, list_users, me, modify_user,
            reset_password, set_wallet, start_enrollment, start_remote_desktop_configuration,
            update_wallet, username_available, wallet_challenge,
        },
        webhooks::{
            add_webhook, change_enabled, change_webhook, delete_webhook, get_webhook, list_webhooks,
        },
    },
    mail::Mail,
};

#[cfg(feature = "wireguard")]
use self::handlers::wireguard::{
    add_device, add_user_devices, create_network, create_network_token, delete_device,
    delete_network, download_config, gateway_status, get_device, import_network, list_devices,
    list_networks, list_user_devices, modify_device, modify_network, network_details,
    network_stats, remove_gateway, user_stats,
};
#[cfg(feature = "worker")]
use self::handlers::worker::{
    create_job, create_worker_token, job_status, list_workers, remove_worker,
};
#[cfg(feature = "openid")]
use self::handlers::{
    openid_clients::{
        add_openid_client, change_openid_client, change_openid_client_state, delete_openid_client,
        get_openid_client, list_openid_clients,
    },
    openid_flow::{
        authorization, discovery_keys, openid_configuration, secure_authorization, token, userinfo,
    },
};
#[cfg(any(feature = "openid", feature = "worker"))]
use self::{
    auth::failed_login::FailedLoginMap,
    db::models::oauth2client::OAuth2Client,
    grpc::{GatewayMap, WorkerState},
    handlers::app_info::get_app_info,
};

pub mod appstate;
pub mod auth;
pub mod config;
pub mod db;
mod error;
pub mod grpc;
pub mod handlers;
pub mod headers;
pub mod hex;
pub mod ldap;
pub mod mail;
pub(crate) mod random;
pub mod secret;
pub mod support;
pub mod templates;
pub mod wg_config;
pub mod wireguard_peer_disconnect;
pub mod wireguard_stats_purge;

#[macro_use]
extern crate tracing;

#[macro_use]
extern crate serde;

pub static VERSION: &str = env!("CARGO_PKG_VERSION");
// TODO: use in more contexts instead of cloning/passing config around
pub static SERVER_CONFIG: OnceCell<DefGuardConfig> = OnceCell::const_new();

pub(crate) fn server_config() -> &'static DefGuardConfig {
    SERVER_CONFIG
        .get()
        .expect("Server configuration not set yet")
}

// WireGuard key length in bytes.
pub(crate) const KEY_LENGTH: usize = 32;

/// Simple health-check.
async fn health_check() -> &'static str {
    "alive"
}

async fn handle_404() -> (StatusCode, &'static str) {
    (StatusCode::NOT_FOUND, "Not found")
}

pub fn build_webapp(
    config: DefGuardConfig,
    webhook_tx: UnboundedSender<AppEvent>,
    webhook_rx: UnboundedReceiver<AppEvent>,
    wireguard_tx: Sender<GatewayEvent>,
    mail_tx: UnboundedSender<Mail>,
    worker_state: Arc<Mutex<WorkerState>>,
    gateway_state: Arc<Mutex<GatewayMap>>,
    pool: DbPool,
    user_agent_parser: Arc<UserAgentParser>,
    failed_logins: Arc<Mutex<FailedLoginMap>>,
) -> Router {
    let serve_web_dir = ServeDir::new("web/dist").fallback(ServeFile::new("web/dist/index.html"));
    let serve_images =
        ServeDir::new("web/src/shared/images/svg").not_found_service(handle_404.into_service());
    let webapp = Router::new().nest(
        "/api/v1",
        Router::new()
            .route("/health", get(health_check))
            .route("/info", get(get_app_info))
            .route("/ssh_authorized_keys", get(get_authorized_keys))
            // /auth
            .route("/auth", post(authenticate))
            .route("/auth/logout", post(logout))
            .route("/auth/mfa", put(mfa_enable))
            .route("/auth/mfa", delete(mfa_disable))
            .route("/auth/webauthn/init", post(webauthn_init))
            .route("/auth/webauthn/finish", post(webauthn_finish))
            .route("/auth/webauthn/start", post(webauthn_start))
            .route("/auth/webauthn", post(webauthn_end))
            .route("/auth/totp/init", post(totp_secret))
            .route("/auth/totp", post(totp_enable))
            .route("/auth/totp", delete(totp_disable))
            .route("/auth/totp/verify", post(totp_code))
            .route("/auth/email/init", post(email_mfa_init))
            .route("/auth/email", get(request_email_mfa_code))
            .route("/auth/email", post(email_mfa_enable))
            .route("/auth/email", delete(email_mfa_disable))
            .route("/auth/email/verify", post(email_mfa_code))
            .route("/auth/web3/start", post(web3auth_start))
            .route("/auth/web3", post(web3auth_end))
            .route("/auth/recovery", post(recovery_code))
            // /user
            .route("/user", get(list_users))
            .route("/user/:username", get(get_user))
            .route("/user", post(add_user))
            .route("/user/:username/start_enrollment", post(start_enrollment))
            .route(
                "/user/:username/start_desktop",
                post(start_remote_desktop_configuration),
            )
            .route("/user/available", post(username_available))
            .route("/user/:username", put(modify_user))
            .route("/user/:username", delete(delete_user))
            // FIXME: username `change_password` is invalid
            .route("/user/change_password", put(change_self_password))
            .route("/user/:username/password", put(change_password))
            .route("/user/:username/reset_password", post(reset_password))
            .route("/user/:username/challenge", get(wallet_challenge))
            // auth keys
            .route("/user/:username/auth_key", get(fetch_authentication_keys))
            .route("/user/:username/auth_key", post(add_authentication_key))
            .route(
                "/user/:username/auth_key/:key_id",
                delete(delete_authentication_key),
            )
            .route(
                "/user/:username/auth_key/:key_id/rename",
                post(rename_authentication_key),
            )
            // yubi keys
            .route("/user/:username/yubikey/:key_id", delete(delete_yubikey))
            .route(
                "/user/:username/yubikey/:key_id/rename",
                post(rename_yubikey),
            )
            .route("/user/:username/wallet", put(set_wallet))
            .route("/user/:username/wallet/:address", put(update_wallet))
            .route("/user/:username/wallet/:address", delete(delete_wallet))
            .route(
                "/user/:username/security_key/:id",
                delete(delete_security_key),
            )
            .route("/me", get(me))
            .route(
                "/user/:username/oauth_app/:oauth2client_id",
                delete(delete_authorized_app),
            )
            // forward_auth
            .route("/forward_auth", get(forward_auth))
            // group
            .route("/group", get(list_groups))
            .route("/group", post(create_group))
            .route("/group/:name", get(get_group))
            .route("/group/:name", put(modify_group))
            .route("/group/:name", delete(delete_group))
            .route("/group/:name", post(add_group_member))
            .route("/group/:name/user/:username", delete(remove_group_member))
            .route("/group-info", get(list_groups_info))
            .route("/groups-assign", post(bulk_assign_to_groups))
            // mail
            .route("/mail/test", post(test_mail))
            .route("/mail/support", post(send_support_data))
            // settings
            .route("/settings", get(get_settings))
            .route("/settings", put(update_settings))
            .route("/settings", patch(patch_settings))
            .route("/settings/:id", put(set_default_branding))
            // settings for frontend
            .route("/settings_essentials", get(get_settings_essentials))
            // support
            .route("/support/configuration", get(configuration))
            .route("/support/logs", get(logs))
            // webhooks
            .route("/webhook", post(add_webhook))
            .route("/webhook", get(list_webhooks))
            .route("/webhook/:id", get(get_webhook))
            .route("/webhook/:id", put(change_webhook))
            .route("/webhook/:id", delete(delete_webhook))
            .route("/webhook/:id", post(change_enabled))
            // ldap
            .route("/ldap/test", get(test_ldap_settings)),
    );

    #[cfg(feature = "openid")]
    let webapp = webapp
        .nest(
            "/api/v1/oauth",
            Router::new()
                .route("/discovery/keys", get(discovery_keys))
                .route("/", post(add_openid_client))
                .route("/", get(list_openid_clients))
                .route("/:client_id", get(get_openid_client))
                .route("/:client_id", put(change_openid_client))
                .route("/:client_id", post(change_openid_client_state))
                .route("/:client_id", delete(delete_openid_client))
                .route("/authorize", get(authorization))
                .route("/authorize", post(secure_authorization))
                .route("/token", post(token))
                .route("/userinfo", get(userinfo)),
        )
        .route(
            "/.well-known/openid-configuration",
            get(openid_configuration),
        );

    #[cfg(feature = "wireguard")]
    let webapp = webapp.nest(
        "/api/v1",
        Router::new()
            .route("/device/:device_id", post(add_device))
            .route("/device/:device_id", put(modify_device))
            .route("/device/:device_id", get(get_device))
            .route("/device/:device_id", delete(delete_device))
            .route("/device", get(list_devices))
            .route("/device/user/:username", get(list_user_devices))
            .route("/network", post(create_network))
            .route("/network/:network_id", put(modify_network))
            .route("/network/:network_id", delete(delete_network))
            .route("/network", get(list_networks))
            .route("/network/:network_id", get(network_details))
            .route("/network/:network_id/gateways", get(gateway_status))
            .route(
                "/network/:network_id/gateways/:gateway_id",
                delete(remove_gateway),
            )
            .route("/network/import", post(import_network))
            .route("/network/:network_id/devices", post(add_user_devices))
            .route(
                "/network/:network_id/device/:device_id/config",
                get(download_config),
            )
            .route("/network/:network_id/token", get(create_network_token))
            .route("/network/:network_id/stats/users", get(user_stats))
            .route("/network/:network_id/stats", get(network_stats))
            .layer(Extension(gateway_state)),
    );

    #[cfg(feature = "worker")]
    let webapp = webapp.nest(
        "/api/v1/worker",
        Router::new()
            .route("/job", post(create_job))
            .route("/token", get(create_worker_token))
            .route("/", get(list_workers))
            .route("/:id", delete(remove_worker))
            .route("/:id", get(job_status))
            .layer(Extension(worker_state)),
    );

    webapp
        .nest_service("/svg", serve_images)
        .nest_service("/", serve_web_dir)
        .with_state(AppState::new(
            config,
            pool,
            webhook_tx,
            webhook_rx,
            wireguard_tx,
            mail_tx,
            user_agent_parser,
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
        )
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
    user_agent_parser: Arc<UserAgentParser>,
    failed_logins: Arc<Mutex<FailedLoginMap>>,
) -> Result<(), anyhow::Error> {
    let webapp = build_webapp(
        config.clone(),
        webhook_tx,
        webhook_rx,
        wireguard_tx,
        mail_tx,
        worker_state,
        gateway_state,
        pool,
        user_agent_parser,
        failed_logins,
    );
    info!("Started web services");
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), config.http_port);
    let listener = TcpListener::bind(&addr).await?;
    serve(
        listener,
        webapp.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .map_err(|err| anyhow!("Web server can't be started {err}"))
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
        info!("Creating test network");
        let mut network = WireguardNetwork::new(
            "TestNet".to_string(),
            IpNetwork::new(IpAddr::V4(Ipv4Addr::new(10, 1, 1, 1)), 24).unwrap(),
            50051,
            "0.0.0.0".to_string(),
            None,
            vec![IpNetwork::new(IpAddr::V4(Ipv4Addr::new(10, 1, 1, 0)), 24).unwrap()],
            false,
            DEFAULT_KEEPALIVE_INTERVAL,
            DEFAULT_DISCONNECT_THRESHOLD,
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

    #[cfg(feature = "openid")]
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
        false,
        DEFAULT_KEEPALIVE_INTERVAL,
        DEFAULT_DISCONNECT_THRESHOLD,
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
