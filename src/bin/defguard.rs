use std::{
    fs::read_to_string,
    sync::{Arc, Mutex},
};

use defguard::{
    auth::failed_login::FailedLoginMap,
    config::{Command, DefGuardConfig},
    db::{
        init_db,
        models::{settings::Settings, user::User, webhook::AppEvent, wireguard::ChangeEvent},
    },
    enterprise::license::{run_periodic_license_check, set_cached_license, License},
    grpc::{
        run_grpc_bidi_stream, run_grpc_gateway_stream, run_grpc_server, GatewayMap, WorkerState,
    },
    headers::create_user_agent_parser,
    init_dev_env, init_vpn_location,
    mail::{run_mail_handler, Mail},
    run_web_server,
    wireguard_peer_disconnect::run_periodic_peer_disconnect,
    wireguard_stats_purge::run_periodic_stats_purge,
    SERVER_CONFIG, VERSION,
};
use secrecy::ExposeSecret;
use tokio::sync::{broadcast, mpsc::unbounded_channel};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[macro_use]
extern crate tracing;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    if dotenvy::from_filename(".env.local").is_err() {
        dotenvy::dotenv().ok();
    }
    let config = DefGuardConfig::new();
    SERVER_CONFIG.set(config.clone())?;
    // initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| config.log_level.clone().into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting ... version v{}", VERSION);
    debug!("Using config: {config:?}");

    let pool = init_db(
        &config.database_host,
        config.database_port,
        &config.database_name,
        &config.database_user,
        config.database_password.expose_secret(),
    )
    .await;

    // handle optional subcommands
    if let Some(command) = &config.cmd {
        match command {
            Command::InitDevEnv => {
                init_dev_env(&config).await;
            }
            Command::InitVpnLocation(args) => {
                let token = init_vpn_location(&pool, args).await?;
                println!("{token}");
            }
        };

        // return early
        return Ok(());
    }

    if config.openid_signing_key.is_some() {
        info!("Using RSA OpenID signing key");
    } else {
        info!("Using HMAC OpenID signing key");
    }

    let (webhook_tx, webhook_rx) = unbounded_channel::<AppEvent>();
    let (events_tx, _events_rx) = broadcast::channel::<ChangeEvent>(256);
    let (mail_tx, mail_rx) = unbounded_channel::<Mail>();
    let worker_state = Arc::new(Mutex::new(WorkerState::new(webhook_tx.clone())));
    let gateway_map = Arc::new(Mutex::new(GatewayMap::new()));
    let user_agent_parser = create_user_agent_parser();

    // initialize admin user
    User::init_admin_user(&pool, config.default_admin_password.expose_secret()).await?;

    // initialize default settings
    Settings::init_defaults(&pool).await?;

    // read grpc TLS cert and key
    let grpc_cert = config
        .grpc_cert
        .as_ref()
        .and_then(|path| read_to_string(path).ok());
    let grpc_key = config
        .grpc_key
        .as_ref()
        .and_then(|path| read_to_string(path).ok());

    // initialize failed login attempt tracker
    let failed_logins = FailedLoginMap::new();
    let failed_logins = Arc::new(Mutex::new(failed_logins));

    debug!("Checking enterprise license status");
    match License::load_or_renew(&pool).await {
        Ok(license) => {
            set_cached_license(license);
        }
        Err(err) => {
            warn!("There was an error while loading the license, error: {err}. The enterprise features will be disabled.");
            set_cached_license(None);
        }
    };

    // run services
    tokio::select! {
        res = run_grpc_gateway_stream(pool.clone(), events_tx.clone()) => error!("Gateway gRPC stream returned early: {res:#?}"),
        res = run_grpc_bidi_stream(pool.clone(), events_tx.clone(), mail_tx.clone(), user_agent_parser.clone()), if config.proxy_url.is_some() => error!("Proxy gRPC stream returned early: {res:#?}"),
        res = run_grpc_server(Arc::clone(&worker_state), pool.clone(), grpc_cert, grpc_key, failed_logins.clone()) => error!("gRPC server returned early: {res:#?}"),
        res = run_web_server(worker_state, gateway_map, webhook_tx, webhook_rx, events_tx.clone(), mail_tx, pool.clone(), user_agent_parser, failed_logins) => error!("Web server returned early: {res:#?}"),
        res = run_mail_handler(mail_rx, pool.clone()) => error!("Mail handler returned early: {res:#?}"),
        res = run_periodic_peer_disconnect(pool.clone(), events_tx) => error!("Periodic peer disconnect task returned early: {res:#?}"),
        res = run_periodic_stats_purge(pool.clone(), config.stats_purge_frequency.into(), config.stats_purge_threshold.into()), if !config.disable_stats_purge => error!("Periodic stats purge task returned early: {res:#?}"),
        res = run_periodic_license_check(pool) => error!("Periodic license check task returned early: {res:#?}"),
    }

    Ok(())
}
