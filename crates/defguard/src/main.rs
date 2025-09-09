use std::{
    fs::read_to_string,
    sync::{Arc, Mutex},
};

use bytes::Bytes;
use defguard_core::{
    SERVER_CONFIG, VERSION,
    auth::failed_login::FailedLoginMap,
    config::{Command, DefGuardConfig},
    db::{
        AppEvent, GatewayEvent, Settings, User, init_db,
        models::settings::initialize_current_settings,
    },
    enterprise::{
        activity_log_stream::activity_log_stream_manager::run_activity_log_stream_manager,
        license::{License, run_periodic_license_check, set_cached_license},
        limits::update_counts,
    },
    events::{ApiEvent, BidiStreamEvent, GrpcEvent, InternalEvent},
    grpc::{
        WorkerState,
        gateway::{client_state::ClientMap, map::GatewayMap},
        run_grpc_bidi_stream, run_grpc_server,
    },
    init_dev_env, init_vpn_location,
    mail::{Mail, run_mail_handler},
    run_web_server,
    utility_thread::run_utility_thread,
    wireguard_peer_disconnect::run_periodic_peer_disconnect,
    wireguard_stats_purge::run_periodic_stats_purge,
};
use defguard_event_logger::{message::EventLoggerMessage, run_event_logger};
use defguard_event_router::{RouterReceiverSet, run_event_router};
use defguard_version::IncompatibleComponents;
use secrecy::ExposeSecret;
use tokio::sync::{broadcast, mpsc::unbounded_channel};

#[macro_use]
extern crate tracing;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    if dotenvy::from_filename(".env.local").is_err() {
        dotenvy::dotenv().ok();
    }
    let config = DefGuardConfig::new();
    SERVER_CONFIG
        .set(config.clone())
        .expect("Failed to initialize server config.");

    // initialize tracing with version formatter
    defguard_version::tracing::init(
        defguard_version::Version::parse(VERSION)?,
        &config.log_level,
    )?;

    info!("Starting ... version v{VERSION}");
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
        }

        // return early
        return Ok(());
    }

    if config.openid_signing_key.is_some() {
        info!("Using RSA OpenID signing key");
    } else {
        info!("Using HMAC OpenID signing key");
    }

    // create event channels for services
    let (api_event_tx, api_event_rx) = unbounded_channel::<ApiEvent>();
    let (bidi_event_tx, bidi_event_rx) = unbounded_channel::<BidiStreamEvent>();
    let (internal_event_tx, internal_event_rx) = unbounded_channel::<InternalEvent>();
    let (grpc_event_tx, grpc_event_rx) = unbounded_channel::<GrpcEvent>();

    // Activity log stream setup
    let (activity_log_messages_tx, activity_log_messages_rx) = broadcast::channel::<Bytes>(100);
    let activity_log_stream_reload_notify = Arc::new(tokio::sync::Notify::new());

    // setup communication channels for services
    let (webhook_tx, webhook_rx) = unbounded_channel::<AppEvent>();
    let (wireguard_tx, _wireguard_rx) = broadcast::channel::<GatewayEvent>(256);
    let (mail_tx, mail_rx) = unbounded_channel::<Mail>();
    let (event_logger_tx, event_logger_rx) = unbounded_channel::<EventLoggerMessage>();

    let worker_state = Arc::new(Mutex::new(WorkerState::new(webhook_tx.clone())));
    let gateway_state = Arc::new(Mutex::new(GatewayMap::new()));
    let client_state = Arc::new(Mutex::new(ClientMap::new()));

    let incompatible_components: IncompatibleComponents = Default::default();

    // initialize admin user
    User::init_admin_user(&pool, config.default_admin_password.expose_secret()).await?;

    // initialize default settings
    Settings::init_defaults(&pool).await?;
    // initialize global settings struct
    initialize_current_settings(&pool).await?;

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

    update_counts(&pool).await?;

    debug!("Checking enterprise license status");
    match License::load_or_renew(&pool).await {
        Ok(license) => {
            set_cached_license(license);
        }
        Err(err) => {
            warn!(
                "There was an error while loading the license, error: {err}. The enterprise \
                features will be disabled."
            );
            set_cached_license(None);
        }
    }

    // run services
    tokio::select! {
        res = run_grpc_bidi_stream(
            pool.clone(),
            wireguard_tx.clone(),
            mail_tx.clone(),
            bidi_event_tx,
        ), if config.proxy_url.is_some() => error!("Proxy gRPC stream returned early: {res:?}"),
        res = run_grpc_server(
            Arc::clone(&worker_state),
            pool.clone(),
            Arc::clone(&gateway_state),
            client_state,
            wireguard_tx.clone(),
            mail_tx.clone(),
            grpc_cert,
            grpc_key,
            failed_logins.clone(),
            grpc_event_tx,
            Arc::clone(&incompatible_components),
        ) => error!("gRPC server returned early: {res:?}"),
        res = run_web_server(
            worker_state,
            gateway_state,
            webhook_tx,
            webhook_rx,
            wireguard_tx.clone(),
            mail_tx.clone(),
            pool.clone(),
            failed_logins,
            api_event_tx, incompatible_components
        ) => error!("Web server returned early: {res:?}"),
        res = run_mail_handler(mail_rx) => error!("Mail handler returned early: {res:?}"),
        res = run_periodic_peer_disconnect(pool.clone(), wireguard_tx.clone(), internal_event_tx.clone()) => error!("Periodic peer disconnect task returned early: {res:?}"),
        res = run_periodic_stats_purge(pool.clone(), config.stats_purge_frequency.into(), config.stats_purge_threshold.into()), if !config.disable_stats_purge => error!("Periodic stats purge task returned early: {res:?}"),
        res = run_periodic_license_check(&pool) => error!("Periodic license check task returned early: {res:?}"),
        res = run_utility_thread(&pool, wireguard_tx.clone()) => error!("Utility thread returned early: {res:?}"),
        res = run_event_router(RouterReceiverSet::new(api_event_rx, grpc_event_rx, bidi_event_rx, internal_event_rx), event_logger_tx, wireguard_tx, mail_tx, activity_log_stream_reload_notify.clone()) => error!("Event router returned early: {res:?}"),
        res = run_event_logger(pool.clone(), event_logger_rx, activity_log_messages_tx.clone()) => error!("Activity log event logger returned early: {res:?}"),
        res = run_activity_log_stream_manager(pool.clone(), activity_log_stream_reload_notify.clone(), activity_log_messages_rx) => error!("Activity log stream manager returned early: {res:?}"),
    }

    Ok(())
}
