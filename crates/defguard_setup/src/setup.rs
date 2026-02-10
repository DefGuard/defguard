use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::{Arc, Mutex},
};

use anyhow::anyhow;
use axum::{
    Extension, Router,
    routing::{get, post},
    serve,
};
use defguard_common::VERSION;
use defguard_core::{
    auth::failed_login::FailedLoginMap,
    handle_404,
    handlers::{component_setup::setup_proxy_tls_stream, settings::get_settings_essentials},
    health_check,
};
use defguard_web_ui::{index, svg, web_asset};
use semver::Version;
use sqlx::PgPool;
use tokio::{net::TcpListener, sync::oneshot::Sender};
use tracing::{info, instrument};

use crate::handlers::{
    create_admin, create_ca, finish_setup, get_ca, set_general_config, setup_login, setup_session,
    upload_ca,
};

pub fn build_setup_webapp(pool: PgPool, version: Version, setup_shutdown_tx: Sender<()>) -> Router {
    let failed_logins = Arc::new(Mutex::new(FailedLoginMap::new()));
    Router::<()>::new()
        .route("/", get(index))
        .route("/{*path}", get(index))
        .route("/fonts/{*path}", get(web_asset))
        .route("/assets/{*path}", get(web_asset))
        .route("/svg/{*path}", get(svg))
        .nest(
            "/api/v1",
            Router::<()>::new()
                .route("/health", get(health_check))
                .route("/settings_essentials", get(get_settings_essentials))
                .route("/proxy/setup/stream", get(setup_proxy_tls_stream))
                .nest(
                    "/initial_setup",
                    Router::<()>::new()
                        .route("/ca", post(create_ca).get(get_ca))
                        .route("/ca/upload", post(upload_ca))
                        .route("/general_config", post(set_general_config))
                        .route("/admin", post(create_admin))
                        .route("/login", post(setup_login))
                        .route("/session", get(setup_session))
                        .route("/finish", post(finish_setup)),
                ),
        )
        .fallback_service(get(handle_404))
        .layer(Extension(pool))
        .layer(Extension(version))
        .layer(Extension(failed_logins))
        .layer(Extension(Arc::new(Mutex::new(Some(setup_shutdown_tx)))))
}

#[instrument(skip_all)]
pub async fn run_setup_web_server(
    pool: PgPool,
    http_bind_address: Option<IpAddr>,
    http_port: u16,
) -> Result<(), anyhow::Error> {
    let (setup_shutdown_tx, setup_shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    let setup_webapp = build_setup_webapp(
        pool.clone(),
        defguard_version::Version::parse(VERSION)?,
        setup_shutdown_tx,
    );

    info!("Starting initial setup web server on port {http_port}");
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
        info!("Shutting down initial setup web server");
    })
    .await
    .map_err(|err| anyhow!("Web server can't be started {err}"))
}
