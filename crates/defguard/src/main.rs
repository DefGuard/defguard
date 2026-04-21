use std::{
    fs::read_to_string,
    sync::{Arc, Mutex, RwLock},
};

use anyhow::bail;
use bytes::Bytes;
use defguard_common::{
    CARGO_VERSION, VERSION,
    config::{Command, DefGuardConfig, SERVER_CONFIG},
    db::{
        init_db,
        models::{
            ActiveWizard, Certificates, Settings, Wizard,
            gateway::Gateway,
            proxy::Proxy,
            settings::{initialize_current_settings, update_current_settings},
        },
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
    gateway_config,
    grpc::{GatewayEvent, WorkerState, run_grpc_server},
    init_dev_env, init_vpn_location, run_web_server,
    setup_logs::CoreSetupLogLayer,
    utility_thread::run_utility_thread,
    version::IncompatibleComponents,
};
use defguard_event_logger::{message::EventLoggerMessage, run_event_logger};
use defguard_event_router::{RouterReceiverSet, run_event_router};
use defguard_gateway_manager::{GatewayManager, GatewayTxSet};
use defguard_proxy_manager::{ProxyManager, ProxyTxSet};
use defguard_session_manager::{events::SessionManagerEvent, run_session_manager};
use defguard_setup::{
    auto_adoption::attempt_auto_adoption, migration::run_migration_web_server,
    setup_server::run_setup_web_server,
};
use defguard_vpn_stats_purge::run_periodic_stats_purge;
use secrecy::ExposeSecret;
use tokio::{
    signal::ctrl_c,
    sync::{
        broadcast,
        mpsc::{channel, unbounded_channel},
    },
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[macro_use]
extern crate tracing;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    rustls::crypto::ring::default_provider()
        .install_default()
        .ok();

    if dotenvy::from_filename(".env.local").is_err() {
        dotenvy::dotenv().ok();
    }
    let config = DefGuardConfig::new();
    let log_filter = format!(
        "{},defguard_core::handlers::component_setup=debug,defguard_setup::auto_adoption=debug",
        config.log_level
    );

    let subscriber = tracing_subscriber::registry().with(CoreSetupLogLayer);
    defguard_version::tracing::with_version_formatters(
        &defguard_version::Version::parse(VERSION)?,
        &log_filter,
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

    if config.openid_signing_key.is_some() {
        info!("Using RSA OpenID signing key");
    } else {
        info!("Using HMAC OpenID signing key");
    }

    // initialize global settings struct
    initialize_current_settings(&pool).await?;

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
            Command::GatewayConfig(args) => {
                let config = gateway_config(&pool, args).await?;
                println!("{config:?}");
            }
        }

        // return early
        return Ok(());
    }

    // Both flags must be provided together
    if let Err(msg) = config.validate_adopt_flags() {
        bail!(msg);
    }

    let has_auto_adopt_flags = config.adopt_edge.is_some() && config.adopt_gateway.is_some();
    let wizard = Wizard::init(&pool, has_auto_adopt_flags, &config).await?;

    Settings::initialize_runtime_defaults(&pool).await?;
    SERVER_CONFIG.set(config.clone()).ok();

    if !wizard.completed {
        match wizard.active_wizard {
            ActiveWizard::None => {}
            ActiveWizard::Initial | ActiveWizard::AutoAdoption => {
                if wizard.active_wizard == ActiveWizard::AutoAdoption {
                    if let Err(err) = attempt_auto_adoption(&pool, &config).await {
                        warn!("Failed to store startup auto-adoption states: {err}");
                    }
                }

                if let Err(err) =
                    run_setup_web_server(pool.clone(), config.http_bind_address, config.http_port)
                        .await
                {
                    bail!("Setup web server exited with error: {err}");
                }
            }
            ActiveWizard::Migration => {
                let mut settings = Settings::get_current_settings();
                settings.update_from_config(&pool, &config).await?;

                if let Err(err) = run_migration_web_server(
                    pool.clone(),
                    config.http_bind_address,
                    config.http_port,
                )
                .await
                {
                    bail!("Migration web server exited with error: {err}");
                }
            }
        }
    }

    Wizard::update_last_version_migrated_to(&pool, CARGO_VERSION).await?;

    // Reload settings from database after setup completion to ensure any changes made during setup
    // are reflected in the in-memory settings.
    let settings = Settings::get(&pool).await?.ok_or_else(|| {
        anyhow::anyhow!(
            "Failed to retrieve settings from database after setup completion. This should not \
            happen."
        )
    })?;
    update_current_settings(&pool, settings).await?;

    let settings = Settings::get_current_settings();

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

    let certs = Certificates::get_or_default(&pool).await?;
    if certs.ca_cert_der.is_none() || certs.ca_key_der.is_none() {
        bail!("CA certificate or key were not found, despite completing setup.")
    }

    // read grpc TLS cert and key from legacy config values
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

    let (proxy_control_tx, proxy_control_rx) = channel::<ProxyControlMessage>(100);
    let (web_reload_tx, _web_reload_rx) = tokio::sync::broadcast::channel::<()>(8);
    let proxy_secret_key = settings.secret_key_required()?;
    let proxy_manager = ProxyManager::new(
        pool.clone(),
        ProxyTxSet::new(gateway_tx.clone(), bidi_event_tx.clone()),
        Arc::clone(&incompatible_components),
        proxy_control_rx,
        proxy_secret_key,
    );

    let mut gateway_manager = GatewayManager::new(
        pool.clone(),
        GatewayTxSet::new(gateway_tx.clone(), peer_stats_tx),
    );

    debug!("Resetting proxy connection state on startup");
    Proxy::mark_all_disconnected(&pool).await?;
    debug!("Proxy connection states reset");

    debug!("Resetting gateway connection state on startup");
    Gateway::mark_all_disconnected(&pool).await?;
    debug!("Gateway connection states reset");

    // run services
    tokio::select! {
        res = proxy_manager.run() => bail!("ProxyManager returned early: {res:?}"),
        res = gateway_manager.run() => bail!("GatewayManager returned early: {res:?}"),
        res = run_grpc_server(
            Arc::clone(&worker_state),
            pool.clone(),
            grpc_cert,
            grpc_key,
        ) => bail!("gRPC server returned early: {res:?}"),
        res = run_web_server(
            worker_state,
            webhook_tx,
            webhook_rx,
            gateway_tx.clone(),
            web_reload_tx,
            pool.clone(),
            failed_logins,
            api_event_tx,
            incompatible_components,
            proxy_control_tx.clone()
        ) => bail!("Web server returned early: {res:?}"),
        res = run_periodic_stats_purge(
            pool.clone(),
            settings.stats_purge_frequency(),
            settings.stats_purge_threshold()
        ), if settings.enable_stats_purge =>
            bail!("Periodic stats purge task returned early: {res:?}"),
        res = run_periodic_license_check(&pool, proxy_control_tx.clone()) =>
            bail!("Periodic license check task returned early: {res:?}"),
        res = run_utility_thread(&pool, gateway_tx.clone(), proxy_control_tx) =>
            bail!("Utility thread returned early: {res:?}"),
        res = run_event_router(
            RouterReceiverSet::new(
                api_event_rx,
                bidi_event_rx,
                session_manager_event_rx
            ),
            event_logger_tx,
            gateway_tx.clone(),
            activity_log_stream_reload_notify.clone()
        ) => bail!("Event router returned early: {res:?}"),
        res = run_event_logger(pool.clone(), event_logger_rx, activity_log_messages_tx.clone()) =>
            bail!("Activity log event logger returned early: {res:?}"),
        res = run_activity_log_stream_manager(
            pool.clone(),
            activity_log_stream_reload_notify.clone(),
            activity_log_messages_rx
        ) => bail!("Activity log stream manager returned early: {res:?}"),
        res = run_session_manager(
            pool.clone(),
            peer_stats_rx,
            session_manager_event_tx,
            gateway_tx
        ) => bail!("VPN client session manager returned early: {res:?}"),
        _ = ctrl_c() => Ok(()),
    }
}
