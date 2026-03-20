use axum::{
    Json,
    extract::State,
    http::StatusCode,
};
use defguard_certs::{CertificateAuthority, Csr, DnType, generate_key_pair};
use defguard_common::db::models::{Certificates, CoreCertSource};
use utoipa::ToSchema;

use crate::{
    appstate::AppState,
    auth::{AdminRole, SessionInfo},
    error::WebError,
    handlers::{ApiResponse, ApiResult},
};

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

    let common_name = data
        .san
        .first()
        .cloned()
        .unwrap_or_else(|| "defguard".to_string());
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

    info!(
        "User {} provisioned self-signed core certificate (SAN: {:?})",
        session.user.username, data.san
    );
    Ok(ApiResponse::default())
}
