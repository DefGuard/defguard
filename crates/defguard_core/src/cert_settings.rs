use axum_server::tls_rustls::RustlsConfig;
use defguard_certs::{
    CertificateAuthority, CertificateInfo, Csr, DnType, PemLabel, der_to_pem, generate_key_pair,
    parse_pem_certificate,
};
use defguard_common::db::models::{
    Certificates, CoreCertSource, ProxyCertSource, Settings, settings::update_current_settings,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use utoipa::ToSchema;

use crate::error::WebError;

/// Parses an uploaded certificate, validates its key pair, and rejects invalid validity windows.
async fn parse_cert(cert_pem: &str, key_pem: &str) -> Result<CertificateInfo, WebError> {
    let _ = rustls::crypto::ring::default_provider().install_default();

    RustlsConfig::from_pem(cert_pem.as_bytes().to_vec(), key_pem.as_bytes().to_vec())
        .await
        .map_err(|_| WebError::BadRequest("Invalid certificate or private key PEM".to_string()))?;

    let cert_der = parse_pem_certificate(cert_pem)?;
    let info = CertificateInfo::from_der(cert_der.as_ref())?;

    // Validate cert dates
    let now = chrono::Utc::now().naive_utc();

    if info.not_after <= info.not_before {
        return Err(WebError::BadRequest(
            "Certificate validity period is invalid".to_string(),
        ));
    }

    if info.not_after <= now {
        return Err(WebError::BadRequest("Certificate has expired".to_string()));
    }

    if info.not_before > now {
        return Err(WebError::BadRequest(
            "Certificate is not valid yet".to_string(),
        ));
    }

    Ok(info)
}

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

fn ensure_https(url: &str) -> String {
    if let Some(rest) = url.strip_prefix("http://") {
        format!("https://{rest}")
    } else {
        url.to_owned()
    }
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

    let mut settings = Settings::get_current_settings();
    let mut transaction = pool.begin().await?;

    // Modify url schema if necessary
    settings.defguard_url = match config.ssl_type {
        InternalSslType::None => defguard_url.to_string(),
        InternalSslType::DefguardCa | InternalSslType::OwnCert => ensure_https(defguard_url),
    };
    update_current_settings(&mut *transaction, settings).await?;

    let mut certs = Certificates::get_or_default(&mut *transaction)
        .await
        .map_err(WebError::from)?;

    let cert_info = match config.ssl_type {
        InternalSslType::None => {
            certs.core_http_cert_source = CoreCertSource::None;
            certs.core_http_cert_pem = None;
            certs.core_http_cert_key_pem = None;
            certs.core_http_cert_expiry = None;
            certs
                .save(&mut *transaction)
                .await
                .map_err(WebError::from)?;
            None
        }
        InternalSslType::DefguardCa => {
            let hostname = reqwest::Url::parse(defguard_url)
                .ok()
                .and_then(|u| u.host_str().map(ToString::to_string))
                .unwrap_or_else(|| defguard_url.to_string());

            let ca_cert_der = certs.ca_cert_der.as_ref().ok_or_else(|| {
                WebError::BadRequest(
                    "CA certificate is not present; generate a CA first".to_string(),
                )
            })?;
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
            certs
                .save(&mut *transaction)
                .await
                .map_err(WebError::from)?;

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

            let info = parse_cert(&cert_pem_str, &key_pem_str).await?;
            let valid_for_days = (info.not_after.and_utc() - chrono::Utc::now()).num_days();
            let expiry = info.not_after;

            certs.core_http_cert_source = CoreCertSource::Custom;
            certs.core_http_cert_pem = Some(cert_pem_str);
            certs.core_http_cert_key_pem = Some(key_pem_str);
            certs.core_http_cert_expiry = Some(expiry);
            certs
                .save(&mut *transaction)
                .await
                .map_err(WebError::from)?;

            Some(CertInfoResponse {
                common_name: info.subject_common_name,
                valid_for_days,
                not_before: info.not_before.to_string(),
                not_after: info.not_after.to_string(),
            })
        }
    };

    transaction.commit().await?;
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

    let mut transaction = pool.begin().await?;
    let mut certs = Certificates::get_or_default(&mut *transaction)
        .await
        .map_err(WebError::from)?;

    // Modify url schema if necessary
    let mut settings = Settings::get_current_settings();
    settings.public_proxy_url = match config.ssl_type {
        ExternalSslType::None => public_proxy_url.to_string(),
        ExternalSslType::LetsEncrypt | ExternalSslType::DefguardCa | ExternalSslType::OwnCert => {
            ensure_https(public_proxy_url)
        }
    };
    update_current_settings(&mut *transaction, settings).await?;

    let hostname = match config.ssl_type {
        ExternalSslType::None => String::new(),
        ExternalSslType::DefguardCa | ExternalSslType::LetsEncrypt | ExternalSslType::OwnCert => {
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
        }
    };

    let cert_info = match config.ssl_type {
        ExternalSslType::None => {
            certs.proxy_http_cert_source = ProxyCertSource::None;
            certs.acme_domain = None;
            certs.acme_account_credentials = None;
            certs.proxy_http_cert_pem = None;
            certs.proxy_http_cert_key_pem = None;
            certs.proxy_http_cert_expiry = None;
            certs
                .save(&mut *transaction)
                .await
                .map_err(WebError::from)?;
            None
        }
        ExternalSslType::LetsEncrypt => {
            debug!(
                "Validated Let's Encrypt configuration for domain {hostname}; \
                 deferring persistence until ACME succeeds"
            );
            None
        }
        ExternalSslType::DefguardCa => {
            let ca_cert_der = certs.ca_cert_der.as_ref().ok_or_else(|| {
                WebError::BadRequest(
                    "CA certificate is not present; generate a CA first".to_string(),
                )
            })?;
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
            certs
                .save(&mut *transaction)
                .await
                .map_err(WebError::from)?;

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

            let info = parse_cert(&cert_pem_str, &key_pem_str).await?;
            let valid_for_days = (info.not_after.and_utc() - chrono::Utc::now()).num_days();
            let expiry = info.not_after;

            certs.proxy_http_cert_source = ProxyCertSource::Custom;
            certs.acme_domain = None;
            certs.proxy_http_cert_pem = Some(cert_pem_str);
            certs.proxy_http_cert_key_pem = Some(key_pem_str);
            certs.proxy_http_cert_expiry = Some(expiry);
            certs
                .save(&mut *transaction)
                .await
                .map_err(WebError::from)?;

            Some(CertInfoResponse {
                common_name: info.subject_common_name,
                valid_for_days,
                not_before: info.not_before.to_string(),
                not_after: info.not_after.to_string(),
            })
        }
    };

    transaction.commit().await?;
    Ok(cert_info)
}
