use axum::{Extension, Json, extract::State, http::StatusCode};
use defguard_certs::{
    CertificateAuthority, CertificateInfo, Csr, DnType, PemLabel, der_to_pem,
    generate_key_pair, parse_pem_certificate,
};
use defguard_common::db::models::{
    Certificates, CoreCertSource, ProxyCertSource, settings::update_current_settings,
};
use serde_json::json;
use sqlx::PgPool;
use utoipa::ToSchema;

use crate::{
    appstate::AppState,
    auth::{AdminRole, SessionInfo},
    error::WebError,
    handlers::{ApiResponse, ApiResult},
};

/// SSL configuration type for Defguard's internal (core) web server.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum InternalSslType {
    /// No SSL - plain HTTP, user manages reverse proxy / SSL termination themselves.
    None,
    /// Generate certificates using Defguard's internal Certificate Authority.
    DefguardCa,
    /// Upload a custom certificate and private key.
    OwnCert,
}

#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct InternalUrlSettingsConfig {
    pub ssl_type: InternalSslType,
    pub cert_pem: Option<String>,
    pub key_pem: Option<String>,
}

#[derive(Serialize, Debug, ToSchema)]
pub struct CertInfoResponse {
    pub common_name: String,
    pub valid_for_days: i64,
    pub not_before: String,
    pub not_after: String,
}

/// SSL configuration type for the external (proxy) web server.
#[derive(Default, Serialize, Deserialize, Debug, Clone, PartialEq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExternalSslType {
    /// No SSL - plain HTTP, user manages reverse proxy / SSL termination themselves.
    #[default]
    None,
    /// Obtain a certificate via ACME / Let's Encrypt.
    LetsEncrypt,
    /// Generate certificates using Defguard's internal Certificate Authority.
    DefguardCa,
    /// Upload a custom certificate and private key.
    OwnCert,
}

#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct ExternalUrlSettingsConfig {
    pub ssl_type: ExternalSslType,
    pub cert_pem: Option<String>,
    pub key_pem: Option<String>,
}

/// Core logic for applying internal URL certificate settings using the current Defguard URL.
/// Returns cert info if a certificate was generated/uploaded, `None` for `ssl_type = None`.
pub async fn apply_internal_url_settings(
    pool: &PgPool,
    defguard_url: &str,
    config: InternalUrlSettingsConfig,
) -> Result<Option<CertInfoResponse>, WebError> {
    debug!(
        "Internal URL certificate settings received: defguard_url={}, ssl_type={:?}",
        defguard_url, config.ssl_type,
    );

    let mut settings = defguard_common::db::models::Settings::get_current_settings();
    settings.defguard_url = defguard_url.to_string();
    update_current_settings(pool, settings).await?;

    let mut certs = Certificates::get_or_default(pool)
        .await
        .map_err(WebError::from)?;

    let cert_info = match config.ssl_type {
        InternalSslType::None => {
            certs.core_http_cert_source = CoreCertSource::None;
            certs.core_http_cert_pem = None;
            certs.core_http_cert_key_pem = None;
            certs.core_http_cert_expiry = None;
            certs.save(pool).await.map_err(WebError::from)?;
            None
        }
        InternalSslType::DefguardCa => {
            let hostname = reqwest::Url::parse(defguard_url)
                .ok()
                .and_then(|u| u.host_str().map(ToString::to_string))
                .unwrap_or_else(|| defguard_url.to_string());

            if certs.ca_cert_der.is_none() {
                return Err(WebError::BadRequest(
                    "CA certificate is not present; generate a CA first".to_string(),
                ));
            }

            let ca_cert_der = certs.ca_cert_der.as_ref().expect("CA cert must be present");
            let ca_key_der = certs.ca_key_der.as_ref().ok_or_else(|| {
                WebError::BadRequest("CA private key not available for signing".to_string())
            })?;

            let ca = CertificateAuthority::from_cert_der_key_pair(ca_cert_der, ca_key_der)?;
            let key_pair = generate_key_pair()?;
            let san = vec![hostname.clone()];
            let dn = vec![(DnType::CommonName, hostname.as_str())];
            let csr = Csr::new(&key_pair, &san, dn)?;
            let server_cert = ca.sign_csr(&csr)?;

            let cert_der = server_cert.der().to_vec();
            let cert_pem = der_to_pem(&cert_der, PemLabel::Certificate)?;
            let key_pem = der_to_pem(key_pair.serialize_der().as_slice(), PemLabel::PrivateKey)?;
            let info = CertificateInfo::from_der(&cert_der)?;
            let valid_for_days = (info.not_after.and_utc() - chrono::Utc::now()).num_days();
            let expiry = info.not_after;

            certs.core_http_cert_source = CoreCertSource::SelfSigned;
            certs.core_http_cert_pem = Some(cert_pem);
            certs.core_http_cert_key_pem = Some(key_pem);
            certs.core_http_cert_expiry = Some(expiry);
            certs.save(pool).await.map_err(WebError::from)?;

            Some(CertInfoResponse {
                common_name: info.subject_common_name,
                valid_for_days,
                not_before: info.not_before.to_string(),
                not_after: info.not_after.to_string(),
            })
        }
        InternalSslType::OwnCert => {
            let cert_pem_str = config.cert_pem.ok_or_else(|| {
                WebError::BadRequest("cert_pem is required for own_cert".to_string())
            })?;
            let key_pem_str = config.key_pem.ok_or_else(|| {
                WebError::BadRequest("key_pem is required for own_cert".to_string())
            })?;

            let cert_der = parse_pem_certificate(&cert_pem_str)?;
            let info = CertificateInfo::from_der(cert_der.as_ref())?;
            let valid_for_days = (info.not_after.and_utc() - chrono::Utc::now()).num_days();
            let expiry = info.not_after;

            certs.core_http_cert_source = CoreCertSource::Custom;
            certs.core_http_cert_pem = Some(cert_pem_str);
            certs.core_http_cert_key_pem = Some(key_pem_str);
            certs.core_http_cert_expiry = Some(expiry);
            certs.save(pool).await.map_err(WebError::from)?;

            Some(CertInfoResponse {
                common_name: info.subject_common_name,
                valid_for_days,
                not_before: info.not_before.to_string(),
                not_after: info.not_after.to_string(),
            })
        }
    };

    Ok(cert_info)
}

/// Core logic for applying external URL certificate settings using the current public proxy URL.
/// Returns cert info if a certificate was generated/uploaded, `None` otherwise.
pub async fn apply_external_url_settings(
    pool: &PgPool,
    public_proxy_url: &str,
    config: ExternalUrlSettingsConfig,
) -> Result<Option<CertInfoResponse>, WebError> {
    debug!(
        "External URL certificate settings received: public_proxy_url={}, ssl_type={:?}",
        public_proxy_url, config.ssl_type,
    );

    let mut certs = Certificates::get_or_default(pool)
        .await
        .map_err(WebError::from)?;

    let hostname = if matches!(config.ssl_type, ExternalSslType::LetsEncrypt | ExternalSslType::DefguardCa)
    {
        let url = public_proxy_url.trim();
        if url.is_empty() {
            return Err(WebError::BadRequest(
                "Public proxy URL is not configured".to_string(),
            ));
        }

        reqwest::Url::parse(url)
            .ok()
            .and_then(|u| u.host_str().map(ToString::to_string))
            .filter(|host| !host.is_empty())
            .unwrap_or_else(|| url.to_string())
    } else {
        String::new()
    };

    let cert_info = match config.ssl_type {
        ExternalSslType::None => {
            certs.proxy_http_cert_source = ProxyCertSource::None;
            certs.acme_domain = None;
            certs.acme_account_credentials = None;
            certs.proxy_http_cert_pem = None;
            certs.proxy_http_cert_key_pem = None;
            certs.proxy_http_cert_expiry = None;
            certs.save(pool).await.map_err(WebError::from)?;
            None
        }
        ExternalSslType::LetsEncrypt => {
            certs.proxy_http_cert_source = ProxyCertSource::LetsEncrypt;
            certs.acme_domain = Some(hostname);
            certs.proxy_http_cert_pem = None;
            certs.proxy_http_cert_key_pem = None;
            certs.proxy_http_cert_expiry = None;
            certs.save(pool).await.map_err(WebError::from)?;
            None
        }
        ExternalSslType::DefguardCa => {
            if certs.ca_cert_der.is_none() {
                return Err(WebError::BadRequest(
                    "CA certificate is not present; generate a CA first".to_string(),
                ));
            }

            let ca_cert_der = certs.ca_cert_der.as_ref().expect("CA cert must be present");
            let ca_key_der = certs.ca_key_der.as_ref().ok_or_else(|| {
                WebError::BadRequest("CA private key not available for signing".to_string())
            })?;

            let ca = CertificateAuthority::from_cert_der_key_pair(ca_cert_der, ca_key_der)?;
            let key_pair = generate_key_pair()?;
            let san = vec![hostname.clone()];
            let dn = vec![(DnType::CommonName, hostname.as_str())];
            let csr = Csr::new(&key_pair, &san, dn)?;
            let server_cert = ca.sign_csr(&csr)?;

            let cert_der = server_cert.der().to_vec();
            let cert_pem = der_to_pem(&cert_der, PemLabel::Certificate)?;
            let key_pem = der_to_pem(key_pair.serialize_der().as_slice(), PemLabel::PrivateKey)?;
            let info = CertificateInfo::from_der(&cert_der)?;
            let valid_for_days = (info.not_after.and_utc() - chrono::Utc::now()).num_days();
            let expiry = info.not_after;

            certs.proxy_http_cert_source = ProxyCertSource::SelfSigned;
            certs.acme_domain = None;
            certs.proxy_http_cert_pem = Some(cert_pem);
            certs.proxy_http_cert_key_pem = Some(key_pem);
            certs.proxy_http_cert_expiry = Some(expiry);
            certs.save(pool).await.map_err(WebError::from)?;

            Some(CertInfoResponse {
                common_name: info.subject_common_name,
                valid_for_days,
                not_before: info.not_before.to_string(),
                not_after: info.not_after.to_string(),
            })
        }
        ExternalSslType::OwnCert => {
            let cert_pem_str = config.cert_pem.ok_or_else(|| {
                WebError::BadRequest("cert_pem is required for own_cert".to_string())
            })?;
            let key_pem_str = config.key_pem.ok_or_else(|| {
                WebError::BadRequest("key_pem is required for own_cert".to_string())
            })?;

            let cert_der = parse_pem_certificate(&cert_pem_str)?;
            let info = CertificateInfo::from_der(cert_der.as_ref())?;
            let valid_for_days = (info.not_after.and_utc() - chrono::Utc::now()).num_days();
            let expiry = info.not_after;

            certs.proxy_http_cert_source = ProxyCertSource::Custom;
            certs.acme_domain = None;
            certs.proxy_http_cert_pem = Some(cert_pem_str);
            certs.proxy_http_cert_key_pem = Some(key_pem_str);
            certs.proxy_http_cert_expiry = Some(expiry);
            certs.save(pool).await.map_err(WebError::from)?;

            Some(CertInfoResponse {
                common_name: info.subject_common_name,
                valid_for_days,
                not_before: info.not_before.to_string(),
                not_after: info.not_after.to_string(),
            })
        }
    };

    Ok(cert_info)
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
    _role: AdminRole,
    Extension(pool): Extension<PgPool>,
    Json(config): Json<InternalUrlSettingsConfig>,
) -> ApiResult {
    info!("Applying core internal URL certificate settings");
    let settings = defguard_common::db::models::Settings::get_current_settings();
    let cert_info = apply_internal_url_settings(&pool, &settings.defguard_url, config).await?;
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
    _role: AdminRole,
    Extension(pool): Extension<PgPool>,
    Json(config): Json<ExternalUrlSettingsConfig>,
) -> ApiResult {
    info!("Applying proxy external URL certificate settings");
    let settings = defguard_common::db::models::Settings::get_current_settings();
    let cert_info = apply_external_url_settings(&pool, &settings.public_proxy_url, config).await?;
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
            "proxy_http_cert_source": certs.proxy_http_cert_source,
            "proxy_http_cert_expiry": certs.proxy_http_cert_expiry,
        }),
        StatusCode::OK,
    ))
}
