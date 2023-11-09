use defguard::{
    auth::failed_login::FailedLoginMap,
    config::{Command, DefGuardConfig},
    db::{init_db, AppEvent, GatewayEvent, Settings, User},
    grpc::{run_grpc_server, GatewayMap, WorkerState},
    init_dev_env, init_vpn_location,
    mail::{run_mail_handler, Mail},
    run_web_server,
    wireguard_stats_purge::run_periodic_stats_purge,
    SERVER_CONFIG,
};
use secrecy::ExposeSecret;
use uaparser::UserAgentParser;
use std::{
    fs::read_to_string,
    sync::{Arc, Mutex},
};
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
    // initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                format!(
                    "defguard={},tower_http=info,axum::rejection=trace",
                    config.log_level
                )
                .into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
    SERVER_CONFIG.set(config.clone())?;

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

    debug!("Starting defguard server with config: {config:?}");

    if config.openid_signing_key.is_some() {
        info!("Using RSA OpenID signing key");
    } else {
        info!("Using HMAC OpenID signing key");
    }

    let (webhook_tx, webhook_rx) = unbounded_channel::<AppEvent>();
    let (wireguard_tx, _wireguard_rx) = broadcast::channel::<GatewayEvent>(256);
    let (mail_tx, mail_rx) = unbounded_channel::<Mail>();
    let worker_state = Arc::new(Mutex::new(WorkerState::new(webhook_tx.clone())));
    let gateway_state = Arc::new(Mutex::new(GatewayMap::new()));

    let user_agent_parser = Arc::new(UserAgentParser::builder()
        .build_from_yaml("./regexes.yaml")
        .expect("Parser creation failed"));

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

    // run services
    tokio::select! {
        _ = run_grpc_server(&config, Arc::clone(&worker_state), pool.clone(), Arc::clone(&gateway_state), wireguard_tx.clone(), mail_tx.clone(), grpc_cert, grpc_key, failed_logins.clone()) => (),
        _ = run_web_server(&config, worker_state, gateway_state, webhook_tx, webhook_rx, wireguard_tx, mail_tx, pool.clone(), user_agent_parser, failed_logins) => (),
        _ = run_mail_handler(mail_rx, pool.clone()) => (),
        _ = run_periodic_stats_purge(pool, config.stats_purge_frequency.into(), config.stats_purge_threshold.into()), if !config.disable_stats_purge => (),
    }
    Ok(())
}
