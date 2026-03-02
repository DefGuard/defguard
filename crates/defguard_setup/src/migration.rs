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
use defguard_common::VERSION;
use defguard_core::{
    auth::failed_login::FailedLoginMap,
    handle_404,
    handlers::{
        auth::{
            authenticate, email_mfa_code, email_mfa_enable, email_mfa_init, logout, mfa_disable,
            mfa_enable, recovery_code, request_email_mfa_code, totp_code, totp_enable, totp_secret,
            webauthn_end, webauthn_finish, webauthn_init, webauthn_start,
        },
        component_setup::setup_proxy_tls_stream,
        session_info::get_session_info,
        settings::get_settings_essentials,
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

use defguard_core::{
    appstate::AppState,
    db::AppEvent,
    enterprise::handlers::openid_login::{auth_callback, get_auth_info},
    events::ApiEvent,
    grpc::GatewayEvent,
};

use crate::handlers::initial_wizard::{
    create_ca, finish_setup, get_ca, set_general_config, setup_session, upload_ca,
};

pub fn build_migration_webapp(
    pool: PgPool,
    version: Version,
    setup_shutdown_tx: Sender<()>,
) -> Router {
    let failed_logins = Arc::new(Mutex::new(FailedLoginMap::new()));
    let (webhook_tx, webhook_rx) = mpsc::unbounded_channel::<AppEvent>();
    let (event_tx, _event_rx) = mpsc::unbounded_channel::<ApiEvent>();
    let (wireguard_tx, _wireguard_rx) = broadcast::channel::<GatewayEvent>(64);
    let (proxy_control_tx, _proxy_control_rx) = mpsc::channel(32);
    let incompatible_components = Arc::new(RwLock::new(IncompatibleComponents::default()));
    let app_state = AppState::new(
        pool.clone(),
        webhook_tx,
        webhook_rx,
        wireguard_tx,
        failed_logins.clone(),
        event_tx,
        incompatible_components,
        proxy_control_tx,
    );

    Router::new()
        .route("/", get(index))
        .route("/{*path}", get(index))
        .route("/fonts/{*path}", get(web_asset))
        .route("/assets/{*path}", get(web_asset))
        .route("/svg/{*path}", get(svg))
        .nest(
            "/api/v1",
            Router::new()
                .route("/health", get(health_check))
                .route("/session-info", get(get_session_info))
                .route("/settings_essentials", get(get_settings_essentials))
                .route("/proxy/setup/stream", get(setup_proxy_tls_stream))
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
                .nest(
                    "/initial_setup",
                    Router::new()
                        .route("/ca", post(create_ca).get(get_ca))
                        .route("/ca/upload", post(upload_ca))
                        .route("/general_config", post(set_general_config))
                        // .route("/admin", post(create_admin))
                        // .route("/login", post(setup_login))
                        .route("/session", get(setup_session))
                        .route("/finish", post(finish_setup)),
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
}

#[instrument(skip_all)]
pub async fn run_migration_web_server(
    pool: PgPool,
    http_bind_address: Option<IpAddr>,
    http_port: u16,
) -> Result<(), anyhow::Error> {
    let (setup_shutdown_tx, setup_shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    let setup_webapp = build_migration_webapp(
        pool.clone(),
        defguard_version::Version::parse(VERSION)?,
        setup_shutdown_tx,
    );

    info!("Starting instance migration web server on port {http_port}");
    let addr = SocketAddr::new(
        http_bind_address.unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED)),
        http_port,
    );
    let listener = TcpListener::bind(&addr).await?;
    serve(
        listener,
        setup_webapp.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(async move {
        setup_shutdown_rx.await.ok();
        info!("Shutting down instance migration web server");
    })
    .await
    .map_err(|err| anyhow!("Web server can't be started {err}"))
}
