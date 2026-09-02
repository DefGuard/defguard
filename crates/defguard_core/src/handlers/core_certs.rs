use axum::{Extension, Json, extract::State, http::StatusCode};
use defguard_certs::{CertificateInfo, der_to_pem, parse_pem_certificate};
use defguard_common::{
    db::models::{Certificates, Settings},
    types::proxy::ProxyControlMessage,
};
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
    handlers::{ApiErrorResponse, ApiResponse, ApiResult, settings::broadcast_public_settings},
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
        .send(ProxyControlMessage::BroadcastHttpsCerts { cert_pem, key_pem })
        .await
    {
        error!("Failed to broadcast HttpsCerts to proxies: {err:?}");
    }
}

/// Tell all connected proxies to clear their active web HTTPS certificates and serve on HTTP.
async fn clear_proxy_https_certs(appstate: &AppState) {
    if let Err(err) = appstate
        .proxy_control_tx
        .send(ProxyControlMessage::ClearHttpsCerts)
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

/// Set up the certificate for the internal (core) URL
#[utoipa::path(
    post,
    path = "/api/v1/core/cert/internal_url_settings",
    tag = "certificates",
    request_body = InternalUrlSettingsConfig,
    responses(
        (status = 201, description = "Internal URL certificate settings applied.", body = Object, example = json!({
            "cert_info": {"common_name": "vpn.example.com", "valid_for_days": 365, "not_before": "2026-08-04T10:00:00", "not_after": "2027-08-04T10:00:00"}
        })),
        (status = 400, description = "Invalid certificate settings.", body = ApiErrorResponse, example = json!({"msg": "cert_pem is required for own_cert", "code": "cert_missing_cert_pem"})),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 500, description = "Unable to apply internal URL certificate settings.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"}))
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
    let settings = Settings::get_current_settings();
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

/// Set up the certificate for the external (edge) URL
#[utoipa::path(
    post,
    path = "/api/v1/proxy/cert/external_url_settings",
    tag = "certificates",
    request_body = ExternalUrlSettingsConfig,
    responses(
        (status = 201, description = "External URL certificate settings applied.", body = Object, example = json!({
            "cert_info": {"common_name": "vpn.example.com", "valid_for_days": 90, "not_before": "2026-08-04T10:00:00", "not_after": "2026-11-02T10:00:00"}
        })),
        (status = 400, description = "Invalid certificate settings.", body = ApiErrorResponse, example = json!({"msg": "cert_pem is required for own_cert", "code": "cert_missing_cert_pem"})),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 500, description = "Unable to apply external URL certificate settings.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"}))
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
    let before = Settings::get_current_settings();
    let ssl_type = config.ssl_type.clone();
    let cert_info = apply_external_url_settings(&pool, &before.public_proxy_url, config).await?;

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

    let after = Settings::get_current_settings();
    if before.edge_public_settings_changed(&after) {
        broadcast_public_settings(&pool, &after, &appstate.proxy_control_tx).await;
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

/// Get the certificate of the internal certificate authority
#[utoipa::path(
    get,
    path = "/api/v1/core/cert/ca",
    tag = "certificates",
    responses(
        (status = 200, description = "CA certificate in PEM format.", body = Object, example = json!({
            "ca_cert_pem": "-----BEGIN CERTIFICATE-----\n...\n-----END CERTIFICATE-----\n",
            "subject_common_name": "defguard CA",
            "not_before": "2026-08-04T10:00:00",
            "not_after": "2036-08-01T10:00:00",
            "valid_for_days": 3650,
            "ca_expiry": "2036-08-01T10:00:00",
            "subject_email": null
        })),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 404, description = "The internal CA is not configured.", body = ApiErrorResponse, example = json!({"msg": "CA certificate not found"})),
        (status = 500, description = "Unable to get CA certificate.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"}))
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
            "CA certificate not found".to_owned(),
        ))
    }
}

/// Get the certificates currently used by core and edge
#[utoipa::path(
    get,
    path = "/api/v1/core/cert/certs",
    tag = "certificates",
    responses(
        (status = 200, description = "Certificates used by core and edge.", body = Object, example = json!({
            "core_http_cert_source": "SelfSigned",
            "core_http_cert_expiry": "2027-08-04T10:00:00",
            "core_http_cert_domain": "vpn.example.com",
            "proxy_http_cert_source": "LetsEncrypt",
            "proxy_http_cert_expiry": "2026-11-02T10:00:00",
            "proxy_http_cert_domain": "vpn.example.com"
        })),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 500, description = "Unable to get certificates.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"}))
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
