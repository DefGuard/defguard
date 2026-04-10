use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::{Arc, Mutex, RwLock},
};

use anyhow::anyhow;
use axum::{
    Extension, Router,
    routing::{get, post, put},
    serve,
};
use axum_extra::extract::cookie::Key;
use defguard_common::{VERSION, db::models::Settings, types::proxy::ProxyControlMessage};
use defguard_core::{
    appstate::AppState,
    auth::failed_login::FailedLoginMap,
    db::AppEvent,
    enterprise::handlers::openid_login::{auth_callback, get_auth_info},
    events::ApiEvent,
    grpc::GatewayEvent,
    handle_404,
    handlers::{
        app_info::get_app_info, auth::{
            authenticate, email_mfa_code, email_mfa_enable, email_mfa_init, logout, mfa_disable,
            mfa_enable, recovery_code, request_email_mfa_code, totp_code, totp_enable, totp_secret,
            webauthn_end, webauthn_finish, webauthn_init, webauthn_start,
        }, component_setup::{setup_gateway_tls_stream, setup_proxy_tls_stream, stream_proxy_acme}, resource_display::get_locations_display, session_info::get_session_info, settings::{get_settings, get_settings_essentials, patch_settings}, wireguard::{count_networks, list_networks}
    },
    health_check,
    version::IncompatibleComponents,
};
use defguard_web_ui::{index, svg, web_asset};
use semver::Version;
use sqlx::PgPool;
use tokio::{
    net::TcpListener,
    sync::{broadcast, mpsc, oneshot::Sender},
};
use tracing::{info, instrument};

use crate::handlers::{
    auto_wizard::{get_external_ssl_info, get_internal_ssl_info},
    initial_wizard::{create_ca, get_ca, upload_ca},
    migration::{
        finish_setup, get_migration_state, migration_set_external_url_settings,
        migration_set_internal_url_settings, update_migration_state,
    },
};

/// FIXME: This is a workaround which enables us to reuse the same API handlers
/// Helper struct which holds all the event receivers so that channels are not closed.
pub struct MigrationWebapp {
    pub router: Router,
    _event_rx: mpsc::UnboundedReceiver<ApiEvent>,
    _wireguard_rx: broadcast::Receiver<GatewayEvent>,
    _proxy_control_rx: mpsc::Receiver<ProxyControlMessage>,
}

pub fn build_migration_webapp(
    pool: PgPool,
    version: Version,
    setup_shutdown_tx: Sender<()>,
) -> MigrationWebapp {
    let failed_logins = Arc::new(Mutex::new(FailedLoginMap::new()));
    let (webhook_tx, webhook_rx) = mpsc::unbounded_channel::<AppEvent>();
    let (event_tx, event_rx) = mpsc::unbounded_channel::<ApiEvent>();
    let (wireguard_tx, wireguard_rx) = broadcast::channel::<GatewayEvent>(64);
    let (web_reload_tx, _web_reload_rx) = broadcast::channel::<()>(8);
    let (proxy_control_tx, proxy_control_rx) = mpsc::channel(32);
    let incompatible_components = Arc::new(RwLock::new(IncompatibleComponents::default()));
    let key = Key::from(
        Settings::get_current_settings()
            .secret_key_required()
            .expect("Missing required secret key in settings")
            .as_bytes(),
    );
    let app_state = AppState::new(
        pool.clone(),
        webhook_tx,
        webhook_rx,
        wireguard_tx.clone(),
        web_reload_tx,
        key,
        failed_logins.clone(),
        event_tx,
        incompatible_components,
        proxy_control_tx.clone(),
    );

    let router = Router::new()
        .route("/", get(index))
        .route("/{*path}", get(index))
        .route("/fonts/{*path}", get(web_asset))
        .route("/assets/{*path}", get(web_asset))
        .route("/svg/{*path}", get(svg))
        .nest(
            "/api/v1",
            Router::new()
                .route("/health", get(health_check))
				.route("/info", get(get_app_info))
                .route("/session-info", get(get_session_info))
                .route("/settings_essentials", get(get_settings_essentials))
                .route("/settings", get(get_settings).patch(patch_settings))
                .route("/proxy/setup/stream", get(setup_proxy_tls_stream))
                .route("/proxy/acme/stream", get(stream_proxy_acme))
                .route("/auth", post(authenticate))
                .route("/auth/logout", post(logout))
                .route("/auth/mfa", put(mfa_enable).delete(mfa_disable))
                .route("/auth/webauthn/init", post(webauthn_init))
                .route("/auth/webauthn/finish", post(webauthn_finish))
                .route("/auth/webauthn/start", post(webauthn_start))
                .route("/auth/webauthn", post(webauthn_end))
                .route("/auth/totp/init", post(totp_secret))
                .route("/auth/totp", post(totp_enable))
                .route("/auth/totp/verify", post(totp_code))
                .route("/auth/email/init", post(email_mfa_init))
                .route(
                    "/auth/email",
                    get(request_email_mfa_code).post(email_mfa_enable),
                )
                .route("/auth/email/verify", post(email_mfa_code))
                .route("/auth/recovery", post(recovery_code))
                .route("/network", get(list_networks))
                .route("/network/count", get(count_networks))
                .route("/network/display", get(get_locations_display))
                .route(
                    "/network/{network_id}/gateways/setup",
                    get(setup_gateway_tls_stream),
                )
                .nest(
                    "/migration",
                    Router::new()
                        .route(
                            "/state",
                            get(get_migration_state).put(update_migration_state),
                        )
                        .route("/ca", post(create_ca).get(get_ca))
                        .route("/ca/upload", post(upload_ca))
                        .route("/finish", post(finish_setup))
                        .route(
                            "/internal_url_settings",
                            post(migration_set_internal_url_settings).get(get_internal_ssl_info),
                        )
                        .route(
                            "/external_url_settings",
                            post(migration_set_external_url_settings).get(get_external_ssl_info),
                        ),
                ),
        )
        .nest(
            "/api/v1/openid",
            Router::new()
                .route("/callback", post(auth_callback))
                .route("/auth_info", get(get_auth_info)),
        )
        .fallback_service(get(handle_404))
        .with_state(app_state)
        .layer(Extension(pool))
        .layer(Extension(version))
        .layer(Extension(failed_logins))
        .layer(Extension(Arc::new(Mutex::new(Some(setup_shutdown_tx)))))
        .layer(Extension(proxy_control_tx));

    MigrationWebapp {
        router,
        _event_rx: event_rx,
        _wireguard_rx: wireguard_rx,
        _proxy_control_rx: proxy_control_rx,
    }
}

#[instrument(skip_all)]
pub async fn run_migration_web_server(
    pool: PgPool,
    http_bind_address: Option<IpAddr>,
    http_port: u16,
) -> Result<(), anyhow::Error> {
    let (setup_shutdown_tx, setup_shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    let migration_webapp = build_migration_webapp(
        pool.clone(),
        defguard_version::Version::parse(VERSION)?,
        setup_shutdown_tx,
    );
    let router = migration_webapp.router;

    info!("Starting instance migration web server on port {http_port}");
    let addr = SocketAddr::new(
        http_bind_address.unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED)),
        http_port,
    );
    let listener = TcpListener::bind(&addr).await?;
    serve(
        listener,
        router.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(async move {
        setup_shutdown_rx.await.ok();
        info!("Shutting down instance migration web server");
    })
    .await
    .map_err(|err| anyhow!("Web server can't be started {err}"))
}
