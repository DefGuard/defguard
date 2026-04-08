use axum::{Extension, Json, extract::State, http::StatusCode};
use defguard_certs::{
    CertificateAuthority, CertificateInfo, Csr, DnType, der_to_pem, generate_key_pair,
    parse_pem_certificate,
};
use defguard_common::db::models::{Certificates, CoreCertSource};
use serde_json::json;
use sqlx::PgPool;
use utoipa::ToSchema;

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

/// Upload a custom PEM certificate + private key for core HTTPS.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct CoreCustomCertUpload {
    /// PEM-encoded certificate chain.
    pub cert_pem: String,
    /// PEM-encoded private key.
    pub key_pem: String,
}

#[utoipa::path(
    post,
    path = "/api/v1/core/cert/upload",
    request_body = CoreCustomCertUpload,
    responses(
        (status = 200, description = "Custom certificate uploaded.", body = ApiResponse),
        (status = 401, description = "Unauthorized.", body = ApiResponse),
        (status = 403, description = "Forbidden.", body = ApiResponse),
        (status = 500, description = "Internal server error.", body = ApiResponse)
    ),
    security(("cookie" = []), ("api_token" = []))
)]
pub(crate) async fn core_cert_upload(
    _role: AdminRole,
    session: SessionInfo,
    State(appstate): State<AppState>,
    Json(data): Json<CoreCustomCertUpload>,
) -> ApiResult {
    debug!(
        "User {} uploading custom core certificate",
        session.user.username
    );

    let mut certs = Certificates::get_or_default(&appstate.pool)
        .await
        .map_err(|err| {
            error!("Failed to load certificates: {err}");
            WebError::Http(StatusCode::INTERNAL_SERVER_ERROR)
        })?;
    certs.core_http_cert_pem = Some(data.cert_pem);
    certs.core_http_cert_key_pem = Some(data.key_pem);
    certs.core_http_cert_source = CoreCertSource::Custom;
    certs.save(&appstate.pool).await.map_err(|err| {
        error!("Failed to save custom core cert: {err}");
        WebError::Http(StatusCode::INTERNAL_SERVER_ERROR)
    })?;

    reload_core_web_server(&appstate);

    info!(
        "User {} uploaded custom core certificate",
        session.user.username
    );
    Ok(ApiResponse::default())
}

/// Provision a core HTTPS certificate signed by the built-in Core CA.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct CoreSelfSignedCertRequest {
    /// List of Subject Alternative Names (domain names or IP addresses).
    pub san: Vec<String>,
}

#[utoipa::path(
    post,
    path = "/api/v1/core/cert/self-signed",
    request_body = CoreSelfSignedCertRequest,
    responses(
        (status = 200, description = "Self-signed certificate provisioned.", body = ApiResponse),
        (status = 400, description = "Invalid request (e.g. CA not configured).", body = ApiResponse),
        (status = 401, description = "Unauthorized.", body = ApiResponse),
        (status = 403, description = "Forbidden.", body = ApiResponse),
        (status = 500, description = "Internal server error.", body = ApiResponse)
    ),
    security(("cookie" = []), ("api_token" = []))
)]
pub(crate) async fn core_cert_self_signed(
    _role: AdminRole,
    session: SessionInfo,
    State(appstate): State<AppState>,
    Json(data): Json<CoreSelfSignedCertRequest>,
) -> ApiResult {
    debug!(
        "User {} provisioning self-signed core certificate",
        session.user.username
    );

    let mut certs = Certificates::get_or_default(&appstate.pool)
        .await
        .map_err(|err| {
            error!("Failed to load certificates: {err}");
            WebError::Http(StatusCode::INTERNAL_SERVER_ERROR)
        })?;

    let (ca_cert_der, ca_key_der) = match (certs.ca_cert_der.clone(), certs.ca_key_der.clone()) {
        (Some(c), Some(k)) => (c, k),
        _ => {
            warn!("CA not configured; cannot issue self-signed core cert");
            return Ok(ApiResponse::json(
                serde_json::json!({"msg": "Core CA is not configured"}),
                StatusCode::BAD_REQUEST,
            ));
        }
    };

    let ca =
        CertificateAuthority::from_cert_der_key_pair(&ca_cert_der, &ca_key_der).map_err(|err| {
            error!("Failed to load Core CA: {err}");
            WebError::Http(StatusCode::INTERNAL_SERVER_ERROR)
        })?;

    let leaf_key = generate_key_pair().map_err(|err| {
        error!("Failed to generate leaf key pair: {err}");
        WebError::Http(StatusCode::INTERNAL_SERVER_ERROR)
    })?;

    let Some(common_name) = data.san.first() else {
        return Err(WebError::BadRequest(
            "At least one SAN entry is required to issue a certificate".to_string(),
        ));
    };

    let csr = Csr::new(
        &leaf_key,
        &data.san,
        vec![(DnType::CommonName, common_name.as_str())],
    )
    .map_err(|err| {
        error!("Failed to build CSR: {err}");
        WebError::Http(StatusCode::INTERNAL_SERVER_ERROR)
    })?;

    let signed = ca
        .sign_csr(&csr)
        .map_err(|err: defguard_certs::CertificateError| {
            error!("Failed to sign CSR with Core CA: {err}");
            WebError::Http(StatusCode::INTERNAL_SERVER_ERROR)
        })?;

    certs.core_http_cert_pem = Some(signed.pem());
    certs.core_http_cert_key_pem = Some(leaf_key.serialize_pem());
    certs.core_http_cert_source = CoreCertSource::SelfSigned;
    certs.save(&appstate.pool).await.map_err(|err| {
        error!("Failed to save self-signed core cert: {err}");
        WebError::Http(StatusCode::INTERNAL_SERVER_ERROR)
    })?;

    reload_core_web_server(&appstate);

    info!(
        "User {} provisioned self-signed core certificate (SAN: {:?})",
        session.user.username, data.san
    );
    Ok(ApiResponse::default())
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
    Extension(pool): Extension<PgPool>,
    Json(config): Json<InternalUrlSettingsConfig>,
) -> ApiResult {
    info!("Applying core internal URL certificate settings");
    let settings = defguard_common::db::models::Settings::get_current_settings();
    let cert_info = apply_internal_url_settings(&pool, &settings.defguard_url, config).await?;
    reload_core_web_server(&appstate);
    info!("Core internal URL certificate settings applied");

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
    Extension(pool): Extension<PgPool>,
    Json(config): Json<ExternalUrlSettingsConfig>,
) -> ApiResult {
    info!("Applying proxy external URL certificate settings");
    let settings = defguard_common::db::models::Settings::get_current_settings();
    let ssl_type = config.ssl_type.clone();
    let cert_info = apply_external_url_settings(&pool, &settings.public_proxy_url, config).await?;

    if matches!(
        ssl_type,
        ExternalSslType::DefguardCa | ExternalSslType::OwnCert
    ) {
        let certs = Certificates::get_or_default(&pool)
            .await
            .map_err(WebError::from)?;
        if let Some((cert_pem, key_pem)) = certs.proxy_http_cert_pair() {
            broadcast_proxy_https_certs(&appstate, cert_pem.to_owned(), key_pem.to_owned()).await;
        }
    } else if ssl_type == ExternalSslType::None {
        clear_proxy_https_certs(&appstate).await;
    }

    info!("Proxy external URL certificate settings applied");

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
pub(crate) async fn get_ca(_role: AdminRole, Extension(pool): Extension<PgPool>) -> ApiResult {
    debug!("Fetching certificate authority details");
    let certs = Certificates::get_or_default(&pool)
        .await
        .map_err(WebError::from)?;
    if let Some(ca_cert_der) = certs.ca_cert_der {
        let ca_pem = der_to_pem(&ca_cert_der, defguard_certs::PemLabel::Certificate)?;
        let info = CertificateInfo::from_der(&ca_cert_der)?;
        let valid_for_days = (info.not_after.and_utc() - chrono::Utc::now()).num_days();

        debug!(
            "Certificate authority details prepared: subject_common_name={}, valid_for_days={}",
            info.subject_common_name, valid_for_days
        );

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
pub(crate) async fn get_certs(_role: AdminRole, Extension(pool): Extension<PgPool>) -> ApiResult {
    debug!("Fetching certificate authority details");
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
