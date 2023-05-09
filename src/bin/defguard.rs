use defguard::{
    auth::failed_login::FailedLoginMap,
    config::{Command, DefGuardConfig},
    db::{init_db, AppEvent, GatewayEvent, User},
    grpc::{run_grpc_server, GatewayState, WorkerState},
    init_dev_env, run_web_server,
};
use fern::{
    colors::{Color, ColoredLevelConfig},
    Dispatch,
};
use log::{LevelFilter, SetLoggerError};
use std::{
    fs::read_to_string,
    str::FromStr,
    sync::{Arc, Mutex},
};
use tokio::sync::mpsc::unbounded_channel;

/// Configures fern logging library.
fn logger_setup(log_level: &str) -> Result<(), SetLoggerError> {
    let colors = ColoredLevelConfig::new()
        .trace(Color::BrightWhite)
        .debug(Color::BrightCyan)
        .info(Color::BrightGreen)
        .warn(Color::BrightYellow)
        .error(Color::BrightRed);
    Dispatch::new()
        .format(move |out, message, record| {
            out.finish(format_args!(
                "[{}][{}][{}] {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f"),
                colors.color(record.level()),
                record.target(),
                message
            ));
        })
        .level(LevelFilter::from_str(log_level).unwrap_or(LevelFilter::Info))
        .level_for("sqlx", LevelFilter::Warn)
        .chain(std::io::stdout())
        .apply()
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    dotenvy::dotenv().ok();
    let config = DefGuardConfig::new();
    logger_setup(&config.log_level)?;
    match config.openid_signing_key {
        Some(_) => log::info!("Using RSA OpenID signing key"),
        None => log::info!("Using HMAC OpenID signing key"),
    }

    if let Some(Command::InitDevEnv) = config.cmd {
        init_dev_env(&config).await;
        return Ok(());
    }

    let (webhook_tx, webhook_rx) = unbounded_channel::<AppEvent>();
    let (wireguard_tx, wireguard_rx) = unbounded_channel::<GatewayEvent>();
    let worker_state = Arc::new(Mutex::new(WorkerState::new(webhook_tx.clone())));
    let gateway_state = Arc::new(Mutex::new(GatewayState::new(wireguard_rx)));
    let pool = init_db(
        &config.database_host,
        config.database_port,
        &config.database_name,
        &config.database_user,
        &config.database_password,
    )
    .await;

    // initialize admin user
    User::init_admin_user(&pool, &config.default_admin_password).await?;

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
        _ = run_grpc_server(config.grpc_port, Arc::clone(&worker_state), pool.clone(), Arc::clone(&gateway_state), grpc_cert, grpc_key, failed_logins.clone()) => (),
        _ = run_web_server(config, worker_state, gateway_state, webhook_tx, webhook_rx, wireguard_tx, pool, failed_logins) => (),
    };
    Ok(())
}
