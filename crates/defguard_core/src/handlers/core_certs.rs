use axum::{Extension, Json, extract::State, http::StatusCode};
use defguard_certs::{CertificateInfo, der_to_pem, parse_pem_certificate};
use defguard_common::db::models::Certificates;
use serde_json::json;
use sqlx::PgPool;

use crate::{
    appstate::AppState,
    auth::{AdminRole, SessionInfo},
    cert_settings::{
        ExternalSslType, ExternalUrlSettingsConfig, InternalUrlSettingsConfig,
        apply_external_url_settings, apply_internal_url_settings,
    },
    error::WebError,
    handlers::{ApiResponse, ApiResult},
};

fn cert_common_name(cert_pem: Option<&str>) -> Option<String> {
    let cert_der = parse_pem_certificate(cert_pem?).ok()?;
    let cert_info = CertificateInfo::from_der(cert_der.as_ref()).ok()?;
    Some(cert_info.subject_common_name)
}

/// Broadcast HTTPS certificate updates to all connected proxies.
async fn broadcast_proxy_https_certs(appstate: &AppState, cert_pem: String, key_pem: String) {
    if let Err(err) = appstate
        .proxy_control_tx
        .send(
            defguard_common::types::proxy::ProxyControlMessage::BroadcastHttpsCerts {
                cert_pem,
                key_pem,
            },
        )
        .await
    {
        error!("Failed to broadcast HttpsCerts to proxies: {err:?}");
    }
}

/// Tell all connected proxies to clear their active web HTTPS certificates and serve on HTTP.
async fn clear_proxy_https_certs(appstate: &AppState) {
    if let Err(err) = appstate
        .proxy_control_tx
        .send(defguard_common::types::proxy::ProxyControlMessage::ClearHttpsCerts)
        .await
    {
        error!("Failed to broadcast ClearHttpsCerts to proxies: {err:?}");
    }
}

fn reload_core_web_server(appstate: &AppState) {
    if let Err(err) = appstate.web_reload_tx.send(()) {
        error!("Failed to trigger core web server reload: {err:?}");
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/core/cert/internal_url_settings",
    request_body = InternalUrlSettingsConfig,
    responses(
        (status = 201, description = "Internal URL certificate settings applied.", body = ApiResponse),
        (status = 400, description = "Invalid request.", body = ApiResponse),
        (status = 401, description = "Unauthorized.", body = ApiResponse),
        (status = 403, description = "Forbidden.", body = ApiResponse),
        (status = 500, description = "Internal server error.", body = ApiResponse)
    ),
    security(("cookie" = []), ("api_token" = []))
)]
pub(crate) async fn set_internal_url_settings(
    State(appstate): State<AppState>,
    _role: AdminRole,
    session: SessionInfo,
    Extension(pool): Extension<PgPool>,
    Json(config): Json<InternalUrlSettingsConfig>,
) -> ApiResult {
    debug!(
        "User {} applying core internal URL certificate settings",
        session.user.username
    );
    let settings = defguard_common::db::models::Settings::get_current_settings();
    let cert_info = apply_internal_url_settings(&pool, &settings.defguard_url, config).await?;
    reload_core_web_server(&appstate);
    info!(
        "User {} applied core internal URL certificate settings",
        session.user.username
    );

    Ok(ApiResponse::new(
        json!({ "cert_info": cert_info }),
        StatusCode::CREATED,
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/proxy/cert/external_url_settings",
    request_body = ExternalUrlSettingsConfig,
    responses(
        (status = 201, description = "External URL certificate settings applied.", body = ApiResponse),
        (status = 400, description = "Invalid request.", body = ApiResponse),
        (status = 401, description = "Unauthorized.", body = ApiResponse),
        (status = 403, description = "Forbidden.", body = ApiResponse),
        (status = 500, description = "Internal server error.", body = ApiResponse)
    ),
    security(("cookie" = []), ("api_token" = []))
)]
pub(crate) async fn set_external_url_settings(
    State(appstate): State<AppState>,
    _role: AdminRole,
    session: SessionInfo,
    Extension(pool): Extension<PgPool>,
    Json(config): Json<ExternalUrlSettingsConfig>,
) -> ApiResult {
    debug!(
        "User {} applying proxy external URL certificate settings",
        session.user.username
    );
    let settings = defguard_common::db::models::Settings::get_current_settings();
    let ssl_type = config.ssl_type.clone();
    let cert_info = apply_external_url_settings(&pool, &settings.public_proxy_url, config).await?;

    match ssl_type {
        ExternalSslType::DefguardCa | ExternalSslType::OwnCert => {
            let certs = Certificates::get_or_default(&pool)
                .await
                .map_err(WebError::from)?;
            if let Some((cert_pem, key_pem)) = certs.proxy_http_cert_pair() {
                broadcast_proxy_https_certs(&appstate, cert_pem.to_owned(), key_pem.to_owned())
                    .await;
            }
        }
        ExternalSslType::None => {
            clear_proxy_https_certs(&appstate).await;
        }
        ExternalSslType::LetsEncrypt => {}
    }
    info!(
        "User {} applied proxy external URL certificate settings",
        session.user.username
    );

    Ok(ApiResponse::new(
        json!({ "cert_info": cert_info }),
        StatusCode::CREATED,
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/core/cert/ca",
    responses(
        (status = 200, description = "CA cert data", body = ApiResponse),
        (status = 400, description = "Invalid request (e.g. CA not configured).", body = ApiResponse),
        (status = 401, description = "Unauthorized.", body = ApiResponse),
        (status = 403, description = "Forbidden.", body = ApiResponse),
        (status = 500, description = "Internal server error.", body = ApiResponse)
    ),
    security(("cookie" = []), ("api_token" = []))
)]
pub(crate) async fn get_ca(
    _role: AdminRole,
    session: SessionInfo,
    Extension(pool): Extension<PgPool>,
) -> ApiResult {
    debug!(
        "User {} fetching certificate authority details",
        session.user.username
    );
    let certs = Certificates::get_or_default(&pool)
        .await
        .map_err(WebError::from)?;
    if let Some(ca_cert_der) = certs.ca_cert_der {
        let ca_pem = der_to_pem(&ca_cert_der, defguard_certs::PemLabel::Certificate)?;
        let info = CertificateInfo::from_der(&ca_cert_der)?;
        let valid_for_days = (info.not_after.and_utc() - chrono::Utc::now()).num_days();

        Ok(ApiResponse::new(
            json!({
                "ca_cert_pem": ca_pem,
                "subject_common_name": info.subject_common_name,
                "not_before": info.not_before,
                "not_after": info.not_after,
                "valid_for_days": valid_for_days,
                "ca_expiry": certs.ca_expiry,
                "subject_email": info.subject_email,
            }),
            StatusCode::OK,
        ))
    } else {
        Err(WebError::ObjectNotFound(
            "CA certificate not found".to_string(),
        ))
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/core/cert/certs",
    responses(
        (status = 200, description = "Core & edge cert data", body = ApiResponse),
        (status = 400, description = "Invalid request (e.g. CA not configured).", body = ApiResponse),
        (status = 401, description = "Unauthorized.", body = ApiResponse),
        (status = 403, description = "Forbidden.", body = ApiResponse),
        (status = 500, description = "Internal server error.", body = ApiResponse)
    ),
    security(("cookie" = []), ("api_token" = []))
)]
pub(crate) async fn get_certs(
    _role: AdminRole,
    session: SessionInfo,
    Extension(pool): Extension<PgPool>,
) -> ApiResult {
    debug!(
        "User {} fetching core and edge certificate details",
        session.user.username
    );
    let certs = Certificates::get_or_default(&pool)
        .await
        .map_err(WebError::from)?;
    Ok(ApiResponse::new(
        json!({
            "core_http_cert_source": certs.core_http_cert_source,
            "core_http_cert_expiry": certs.core_http_cert_expiry,
            "core_http_cert_domain": cert_common_name(certs.core_http_cert_pem.as_deref()),
            "proxy_http_cert_source": certs.proxy_http_cert_source,
            "proxy_http_cert_expiry": certs.proxy_http_cert_expiry,
            "proxy_http_cert_domain": cert_common_name(certs.proxy_http_cert_pem.as_deref()),
        }),
        StatusCode::OK,
    ))
}
