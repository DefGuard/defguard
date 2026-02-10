use std::{
    fs::read_to_string,
    sync::{Arc, Mutex, RwLock},
};

use bytes::Bytes;
use defguard_common::{
    VERSION,
    config::{Command, DefGuardConfig, SERVER_CONFIG},
    db::{
        init_db,
        models::{Settings, settings::initialize_current_settings},
    },
    messages::peer_stats_update::PeerStatsUpdate,
    types::proxy::ProxyControlMessage,
};
use defguard_core::{
    auth::failed_login::FailedLoginMap,
    db::AppEvent,
    enterprise::{
        activity_log_stream::activity_log_stream_manager::run_activity_log_stream_manager,
        license::{License, run_periodic_license_check, set_cached_license},
        limits::update_counts,
    },
    events::{ApiEvent, BidiStreamEvent},
    grpc::{
        WorkerState,
        gateway::{events::GatewayEvent, run_grpc_gateway_stream},
        run_grpc_server,
    },
    init_dev_env, init_vpn_location, run_web_server,
    utility_thread::run_utility_thread,
    version::IncompatibleComponents,
};
use defguard_event_logger::{message::EventLoggerMessage, run_event_logger};
use defguard_event_router::{RouterReceiverSet, run_event_router};
use defguard_proxy_manager::{ProxyManager, ProxyTxSet};
use defguard_session_manager::{events::SessionManagerEvent, run_session_manager};
use defguard_setup::setup::run_setup_web_server;
use defguard_vpn_stats_purge::run_periodic_stats_purge;
use secrecy::ExposeSecret;
use tokio::sync::{
    broadcast,
    mpsc::{channel, unbounded_channel},
};
use tracing_subscriber::util::SubscriberInitExt;

#[macro_use]
extern crate tracing;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    if dotenvy::from_filename(".env.local").is_err() {
        dotenvy::dotenv().ok();
    }
    let mut config = DefGuardConfig::new();

    let subscriber = tracing_subscriber::registry();
    defguard_version::tracing::with_version_formatters(
        &defguard_version::Version::parse(VERSION)?,
        &config.log_level,
        subscriber,
    )
    .init();

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

    // initialize default settings
    Settings::init_defaults(&pool).await?;
    // initialize global settings struct
    initialize_current_settings(&pool).await?;
    let mut settings = Settings::get_current_settings();

    if !settings.initial_setup_completed {
        if let Err(err) =
            run_setup_web_server(pool.clone(), config.http_bind_address, config.http_port).await
        {
            anyhow::bail!("Setup web server exited with error: {err}");
        }

        settings = Settings::get_current_settings();
    }

    config.initialize_post_settings();

    SERVER_CONFIG
        .set(config.clone())
        .expect("Failed to initialize server config.");

    // create event channels for services
    let (api_event_tx, api_event_rx) = unbounded_channel::<ApiEvent>();
    let (bidi_event_tx, bidi_event_rx) = unbounded_channel::<BidiStreamEvent>();
    let (session_manager_event_tx, session_manager_event_rx) =
        unbounded_channel::<SessionManagerEvent>();

    // Activity log stream setup
    let (activity_log_messages_tx, activity_log_messages_rx) = broadcast::channel::<Bytes>(100);
    let activity_log_stream_reload_notify = Arc::new(tokio::sync::Notify::new());

    // setup communication channels for services
    let (webhook_tx, webhook_rx) = unbounded_channel::<AppEvent>();
    // RX is discarded here since it can be derived from TX later on
    let (gateway_tx, _gateway_rx) = broadcast::channel::<GatewayEvent>(256);
    let (event_logger_tx, event_logger_rx) = unbounded_channel::<EventLoggerMessage>();
    let (peer_stats_tx, peer_stats_rx) = unbounded_channel::<PeerStatsUpdate>();

    let worker_state = Arc::new(Mutex::new(WorkerState::new(webhook_tx.clone())));

    let incompatible_components: Arc<RwLock<IncompatibleComponents>> = Arc::default();

    if settings.ca_cert_der.is_none() || settings.ca_key_der.is_none() {
        anyhow::bail!("CA certificate or key were not found in settings, despite completing setup.")
    }

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

    let (proxy_control_tx, proxy_control_rx) = channel::<ProxyControlMessage>(100);
    let proxy_tx = ProxyTxSet::new(gateway_tx.clone(), bidi_event_tx.clone());
    let proxy_manager = ProxyManager::new(
        pool.clone(),
        proxy_tx,
        Arc::clone(&incompatible_components),
        proxy_control_rx,
    );

    // run services
    tokio::select! {
        res = proxy_manager.run() => error!("ProxyManager returned early: {res:?}"),
        res = run_grpc_gateway_stream(
            pool.clone(),
            gateway_tx.clone(),
            peer_stats_tx,
        ) => error!("Gateway gRPC stream returned early: {res:?}"),
        res = run_grpc_server(
            Arc::clone(&worker_state),
            pool.clone(),
            grpc_cert,
            grpc_key,
            failed_logins.clone(),
        ) => error!("gRPC server returned early: {res:?}"),
        res = run_web_server(
            worker_state,
            webhook_tx,
            webhook_rx,
            gateway_tx.clone(),
            pool.clone(),
            failed_logins,
            api_event_tx,
            incompatible_components,
            proxy_control_tx
        ) => error!("Web server returned early: {res:?}"),
        res = run_periodic_stats_purge(
            pool.clone(),
            config.stats_purge_frequency.into(),
            config.stats_purge_threshold.into()
        ), if !config.disable_stats_purge =>
            error!("Periodic stats purge task returned early: {res:?}"),
        res = run_periodic_license_check(&pool) =>
            error!("Periodic license check task returned early: {res:?}"),
        res = run_utility_thread(&pool, gateway_tx.clone()) =>
            error!("Utility thread returned early: {res:?}"),
        res = run_event_router(
            RouterReceiverSet::new(
                api_event_rx,
                bidi_event_rx,
                session_manager_event_rx
            ),
            event_logger_tx,
            gateway_tx.clone(),
            activity_log_stream_reload_notify.clone()
        ) => error!("Event router returned early: {res:?}"),
        res = run_event_logger(pool.clone(), event_logger_rx, activity_log_messages_tx.clone()) =>
            error!("Activity log event logger returned early: {res:?}"),
        res = run_activity_log_stream_manager(
            pool.clone(),
            activity_log_stream_reload_notify.clone(),
            activity_log_messages_rx
        ) => error!("Activity log stream manager returned early: {res:?}"),
        res = run_session_manager(
            pool.clone(),
            peer_stats_rx,
            session_manager_event_tx,
            gateway_tx
        ) => error!("VPN client session manager returned early: {res:?}"),
    }

    Ok(())
}
