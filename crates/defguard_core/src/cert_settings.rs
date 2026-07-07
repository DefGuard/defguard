use axum_server::tls_rustls::RustlsConfig;
use chrono::{NaiveDateTime, Utc};
use defguard_certs::{
    CertificateError, CertificateInfo, Csr, DnType, PemLabel, der_to_pem, generate_key_pair,
    parse_pem_certificate,
};
use defguard_common::db::models::{
    Certificates, CoreCertSource, ProxyCertSource, Settings,
    settings::{SettingsSaveError, update_current_settings},
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use thiserror::Error;
use utoipa::ToSchema;

/// Errors arising from certificate settings operations.
#[derive(Debug, Error)]
pub enum CertSettingsError {
    #[error("cert_pem is required for own_cert")]
    MissingCertPem,
    #[error("key_pem is required for own_cert")]
    MissingKeyPem,
    #[error("Invalid certificate or private key PEM")]
    InvalidCertOrKey,
    #[error("Certificate validity period is invalid")]
    InvalidValidityPeriod,
    #[error("Certificate has expired")]
    CertExpired,
    #[error("Certificate is not valid yet")]
    CertNotYetValid,
    #[error("Certificate error: {0}")]
    Cert(#[from] CertificateError),
    #[error("URL parse error: {0}")]
    Url(String),
    #[error("Database error: {0}")]
    Db(#[from] sqlx::Error),
    #[error("Settings error: {0}")]
    Settings(#[from] SettingsSaveError),
    #[error("Not found: {0}")]
    NotFound(String),
}

/// Parses an uploaded certificate, validates its key pair, and rejects invalid validity windows.
async fn parse_cert(cert_pem: &str, key_pem: &str) -> Result<CertificateInfo, CertSettingsError> {
    let _ = rustls::crypto::ring::default_provider().install_default();

    RustlsConfig::from_pem(cert_pem.as_bytes().to_vec(), key_pem.as_bytes().to_vec())
        .await
        .map_err(|_| CertSettingsError::InvalidCertOrKey)?;

    let cert_der = parse_pem_certificate(cert_pem)?;
    let info = CertificateInfo::from_der(cert_der.as_ref())?;

    // Validate cert dates
    let now = Utc::now().naive_utc();

    if info.not_after <= info.not_before {
        return Err(CertSettingsError::InvalidValidityPeriod);
    }

    if info.not_after <= now {
        return Err(CertSettingsError::CertExpired);
    }

    if info.not_before > now {
        return Err(CertSettingsError::CertNotYetValid);
    }

    Ok(info)
}

/// Extract a non-empty hostname from `url`, returning a [`CertSettingsError`] on failure.
fn extract_hostname(url: &str, label: &str) -> Result<String, CertSettingsError> {
    reqwest::Url::parse(url)
        .map_err(|e| CertSettingsError::Url(format!("Invalid {label}: {e}")))?
        .host_str()
        .filter(|h| !h.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| CertSettingsError::Url(format!("{label} has no hostname")))
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

pub(crate) fn ensure_https(url: &str) -> String {
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
) -> Result<Option<CertInfoResponse>, CertSettingsError> {
    debug!(
        "Internal URL certificate settings received: defguard_url={}, ssl_type={:?}",
        defguard_url, config.ssl_type,
    );

    let mut settings = Settings::get_current_settings();
    let mut transaction = pool.begin().await?;

    // Modify url schema if necessary
    settings.defguard_url = match config.ssl_type {
        InternalSslType::None => defguard_url.to_owned(),
        InternalSslType::DefguardCa | InternalSslType::OwnCert => ensure_https(defguard_url),
    };
    update_current_settings(&mut *transaction, settings).await?;

    let mut certs = Certificates::get_or_default(&mut *transaction)
        .await
        .map_err(CertSettingsError::from)?;

    let cert_info = match config.ssl_type {
        InternalSslType::None => {
            certs.core_http_cert_source = CoreCertSource::None;
            certs.core_http_cert_pem = None;
            certs.core_http_cert_key_pem = None;
            certs.core_http_cert_expiry = None;
            certs
                .save(&mut *transaction)
                .await
                .map_err(CertSettingsError::from)?;
            None
        }
        InternalSslType::DefguardCa => {
            let hostname = extract_hostname(defguard_url, "defguard URL")?;

            let ca = certs.certificate_authority()?;
            let key_pair = generate_key_pair()?;
            let san = vec![hostname.clone()];
            let dn = vec![(DnType::CommonName, hostname.as_str())];
            let csr = Csr::new(&key_pair, &san, dn)?;
            let server_cert = ca.sign_web_server_cert(&csr)?;

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
                .map_err(CertSettingsError::from)?;

            Some(CertInfoResponse {
                common_name: info.subject_common_name,
                valid_for_days,
                not_before: info.not_before.to_string(),
                not_after: info.not_after.to_string(),
            })
        }
        InternalSslType::OwnCert => {
            let cert_pem_str = config.cert_pem.ok_or(CertSettingsError::MissingCertPem)?;
            let key_pem_str = config.key_pem.ok_or(CertSettingsError::MissingKeyPem)?;

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
                .map_err(CertSettingsError::from)?;

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
) -> Result<Option<CertInfoResponse>, CertSettingsError> {
    debug!(
        "External URL certificate settings received: public_proxy_url={}, ssl_type={:?}",
        public_proxy_url, config.ssl_type,
    );

    let mut transaction = pool.begin().await?;
    let mut certs = Certificates::get_or_default(&mut *transaction)
        .await
        .map_err(CertSettingsError::from)?;

    // Modify url schema if necessary
    let mut settings = Settings::get_current_settings();
    settings.public_proxy_url = match config.ssl_type {
        ExternalSslType::None | ExternalSslType::LetsEncrypt => public_proxy_url.to_owned(),
        ExternalSslType::DefguardCa | ExternalSslType::OwnCert => ensure_https(public_proxy_url),
    };
    update_current_settings(&mut *transaction, settings).await?;

    let hostname = match config.ssl_type {
        ExternalSslType::None | ExternalSslType::OwnCert => String::new(),
        ExternalSslType::DefguardCa | ExternalSslType::LetsEncrypt => {
            let url = public_proxy_url.trim();
            if url.is_empty() {
                return Err(CertSettingsError::Url(
                    "Public proxy URL is not configured".to_owned(),
                ));
            }

            extract_hostname(url, "public proxy URL")?
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
                .map_err(CertSettingsError::from)?;
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
            let ca = certs.certificate_authority()?;
            let key_pair = generate_key_pair()?;
            let san = vec![hostname.clone()];
            let dn = vec![(DnType::CommonName, hostname.as_str())];
            let csr = Csr::new(&key_pair, &san, dn)?;
            let server_cert = ca.sign_web_server_cert(&csr)?;

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
                .map_err(CertSettingsError::from)?;

            Some(CertInfoResponse {
                common_name: info.subject_common_name,
                valid_for_days,
                not_before: info.not_before.to_string(),
                not_after: info.not_after.to_string(),
            })
        }
        ExternalSslType::OwnCert => {
            let cert_pem_str = config.cert_pem.ok_or(CertSettingsError::MissingCertPem)?;
            let key_pem_str = config.key_pem.ok_or(CertSettingsError::MissingKeyPem)?;

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
                .map_err(CertSettingsError::from)?;

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

/// Regenerate the Core self-signed HTTPS certificate using the current `defguard_url`.
/// Returns `(cert_pem, key_pem, expiry)` on success.
pub(crate) async fn refresh_core_self_signed_cert(
    pool: &PgPool,
) -> Result<(String, String, NaiveDateTime), CertSettingsError> {
    let settings = Settings::get_current_settings();
    let hostname = extract_hostname(&settings.defguard_url, "defguard URL")?;

    let mut certs = Certificates::get_or_default(pool)
        .await
        .map_err(CertSettingsError::from)?;

    let ca = certs.certificate_authority()?;
    let key_pair = generate_key_pair()?;
    let san = vec![hostname.clone()];
    let dn = vec![(DnType::CommonName, hostname.as_str())];
    let csr = Csr::new(&key_pair, &san, dn)?;
    let server_cert = ca.sign_web_server_cert(&csr)?;

    let cert_der = server_cert.der().to_vec();
    let cert_pem = der_to_pem(&cert_der, PemLabel::Certificate)?;
    let key_pem = der_to_pem(key_pair.serialize_der().as_slice(), PemLabel::PrivateKey)?;
    let info = CertificateInfo::from_der(&cert_der)?;
    let expiry = info.not_after;

    certs.core_http_cert_source = CoreCertSource::SelfSigned;
    certs.core_http_cert_pem = Some(cert_pem.clone());
    certs.core_http_cert_key_pem = Some(key_pem.clone());
    certs.core_http_cert_expiry = Some(expiry);
    certs.save(pool).await.map_err(CertSettingsError::from)?;

    Ok((cert_pem, key_pem, expiry))
}

/// Regenerate the Proxy self-signed HTTPS certificate using the current `public_proxy_url`.
/// Returns `(cert_pem, key_pem, expiry)` on success.
pub(crate) async fn refresh_proxy_self_signed_cert(
    pool: &PgPool,
) -> Result<(String, String, NaiveDateTime), CertSettingsError> {
    let settings = Settings::get_current_settings();
    let hostname = extract_hostname(&settings.public_proxy_url, "public proxy URL")?;

    let mut certs = Certificates::get_or_default(pool)
        .await
        .map_err(CertSettingsError::from)?;

    let ca = certs.certificate_authority()?;
    let key_pair = generate_key_pair()?;
    let san = vec![hostname.clone()];
    let dn = vec![(DnType::CommonName, hostname.as_str())];
    let csr = Csr::new(&key_pair, &san, dn)?;
    let server_cert = ca.sign_web_server_cert(&csr)?;

    let cert_der = server_cert.der().to_vec();
    let cert_pem = der_to_pem(&cert_der, PemLabel::Certificate)?;
    let key_pem = der_to_pem(key_pair.serialize_der().as_slice(), PemLabel::PrivateKey)?;
    let info = CertificateInfo::from_der(&cert_der)?;
    let expiry = info.not_after;

    certs.proxy_http_cert_source = ProxyCertSource::SelfSigned;
    certs.acme_domain = None;
    certs.proxy_http_cert_pem = Some(cert_pem.clone());
    certs.proxy_http_cert_key_pem = Some(key_pem.clone());
    certs.proxy_http_cert_expiry = Some(expiry);
    certs.save(pool).await.map_err(CertSettingsError::from)?;

    Ok((cert_pem, key_pem, expiry))
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use defguard_certs::{
        CertificateAuthority, Csr, DnType, ExtendedKeyUsagePurpose, PemLabel, der_to_pem,
        generate_key_pair,
    };
    use defguard_common::db::{
        models::{
            Certificates, CoreCertSource, ProxyCertSource, Settings,
            settings::{initialize_current_settings, set_settings},
        },
        setup_pool,
    };
    use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

    use super::{
        CertSettingsError, extract_hostname, parse_cert, refresh_core_self_signed_cert,
        refresh_proxy_self_signed_cert,
    };

    fn make_ca() -> CertificateAuthority<'static> {
        CertificateAuthority::new("Test CA", "test@example.com", 365).expect("failed to create CA")
    }

    async fn seed_ca(pool: &sqlx::PgPool, ca: &CertificateAuthority<'_>) {
        Certificates {
            ca_cert_der: Some(ca.cert_der().to_vec()),
            ca_key_der: Some(ca.key_pair_der().to_vec()),
            ca_expiry: Some(ca.expiry().expect("missing CA expiry")),
            ..Default::default()
        }
        .save(pool)
        .await
        .expect("failed to save CA certs");
    }

    async fn seed_settings(defguard_url: &str, public_proxy_url: &str) {
        let mut settings = Settings::get_current_settings();
        settings.defguard_url = defguard_url.into();
        settings.public_proxy_url = public_proxy_url.into();
        set_settings(Some(settings));
    }

    #[test]
    fn extract_hostname_ok() {
        assert_eq!(
            extract_hostname("https://core.example.com", "defguard URL").unwrap(),
            "core.example.com"
        );
    }

    #[test]
    fn extract_hostname_ip_ok() {
        assert_eq!(
            extract_hostname("https://10.0.0.1:8443", "public proxy URL").unwrap(),
            "10.0.0.1"
        );
    }

    #[test]
    fn extract_hostname_invalid_url() {
        let err = extract_hostname("not-a-url", "defguard URL").unwrap_err();
        assert!(matches!(err, CertSettingsError::Url(_)));
        let msg = err.to_string();
        assert!(
            msg.contains("Invalid defguard URL"),
            "unexpected error: {msg}"
        );
    }

    #[test]
    fn extract_hostname_missing_host() {
        let err = extract_hostname("mailto:test@example.com", "public proxy URL").unwrap_err();
        assert!(matches!(err, CertSettingsError::Url(_)));
        let msg = err.to_string();
        assert!(
            msg.contains("public proxy URL has no hostname"),
            "unexpected error: {msg}"
        );
    }

    #[test]
    fn extract_hostname_empty_string() {
        let err = extract_hostname("", "defguard URL").unwrap_err();
        assert!(matches!(err, CertSettingsError::Url(_)));
        let msg = err.to_string();
        assert!(
            msg.contains("Invalid defguard URL"),
            "unexpected error: {msg}"
        );
    }

    #[tokio::test]
    async fn parse_cert_invalid_pem() {
        let err = parse_cert("not a certificate", "not a key")
            .await
            .err()
            .expect("expected an error");
        assert!(matches!(err, CertSettingsError::InvalidCertOrKey));
    }

    #[tokio::test]
    async fn parse_cert_key_mismatch() {
        let ca = make_ca();

        let key_pair = generate_key_pair().expect("failed to generate key pair");
        let san = vec!["example.com".to_owned()];
        let dn = vec![(DnType::CommonName, "example.com")];
        let csr = Csr::new(&key_pair, &san, dn).expect("failed to build CSR");
        let cert = ca
            .sign_web_server_cert(&csr)
            .expect("failed to sign certificate");
        let cert_pem =
            der_to_pem(cert.der(), PemLabel::Certificate).expect("failed to encode cert");

        let other_key_pair = generate_key_pair().expect("failed to generate key pair");
        let mismatched_key_pem = der_to_pem(
            other_key_pair.serialize_der().as_slice(),
            PemLabel::PrivateKey,
        )
        .expect("failed to encode key");

        let err = parse_cert(&cert_pem, &mismatched_key_pem)
            .await
            .err()
            .expect("expected an error");
        assert!(matches!(err, CertSettingsError::InvalidCertOrKey));
    }

    #[tokio::test]
    async fn parse_cert_expired() {
        let ca = make_ca();

        let key_pair = generate_key_pair().expect("failed to generate key pair");
        let san = vec!["example.com".to_owned()];
        let dn = vec![(DnType::CommonName, "example.com")];
        let csr = Csr::new(&key_pair, &san, dn).expect("failed to build CSR");
        let cert = ca
            .sign_csr_with_validity(&csr, 0, &[ExtendedKeyUsagePurpose::ServerAuth])
            .expect("failed to sign certificate");
        let cert_pem =
            der_to_pem(cert.der(), PemLabel::Certificate).expect("failed to encode cert");
        let key_pem = der_to_pem(key_pair.serialize_der().as_slice(), PemLabel::PrivateKey)
            .expect("failed to encode key");

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let err = parse_cert(&cert_pem, &key_pem)
            .await
            .err()
            .expect("expected an error");
        assert!(matches!(err, CertSettingsError::CertExpired));
    }

    #[sqlx::test]
    async fn refresh_core_self_signed_cert_generates_new_cert(
        _: PgPoolOptions,
        options: PgConnectOptions,
    ) {
        let pool = setup_pool(options).await;
        initialize_current_settings(&pool)
            .await
            .expect("failed to initialize settings");
        seed_settings("https://core.example.com", "https://proxy.example.com").await;
        let ca = make_ca();
        seed_ca(&pool, &ca).await;

        let (cert_pem, key_pem, expiry) = refresh_core_self_signed_cert(&pool)
            .await
            .expect("refresh should succeed");

        assert!(
            cert_pem.contains("BEGIN CERTIFICATE"),
            "cert_pem should be a valid certificate"
        );
        assert!(
            key_pem.contains("BEGIN PRIVATE KEY"),
            "key_pem should be a valid private key"
        );

        let days_valid = (expiry.and_utc() - Utc::now()).num_days();
        assert!(
            (98..=100).contains(&days_valid),
            "expected ~100 days validity, got {days_valid}"
        );

        let saved = Certificates::get(&pool)
            .await
            .expect("failed to load certs")
            .expect("certs should exist");
        assert_eq!(saved.core_http_cert_source, CoreCertSource::SelfSigned);
        assert_eq!(saved.core_http_cert_pem, Some(cert_pem));
        assert_eq!(saved.core_http_cert_key_pem, Some(key_pem));
        assert_eq!(saved.core_http_cert_expiry, Some(expiry));
    }

    #[sqlx::test]
    async fn refresh_proxy_self_signed_cert_generates_new_cert(
        _: PgPoolOptions,
        options: PgConnectOptions,
    ) {
        let pool = defguard_common::db::setup_pool(options).await;
        initialize_current_settings(&pool)
            .await
            .expect("failed to initialize settings");
        seed_settings("https://core.example.com", "https://proxy.example.com").await;
        let ca = make_ca();
        seed_ca(&pool, &ca).await;

        let (cert_pem, key_pem, expiry) = refresh_proxy_self_signed_cert(&pool)
            .await
            .expect("refresh should succeed");

        assert!(
            cert_pem.contains("BEGIN CERTIFICATE"),
            "cert_pem should be a valid certificate"
        );
        assert!(
            key_pem.contains("BEGIN PRIVATE KEY"),
            "key_pem should be a valid private key"
        );

        let days_valid = (expiry.and_utc() - Utc::now()).num_days();
        assert!(
            (98..=100).contains(&days_valid),
            "expected ~100 days validity, got {days_valid}"
        );

        let saved = Certificates::get(&pool)
            .await
            .expect("failed to load certs")
            .expect("certs should exist");
        assert_eq!(saved.proxy_http_cert_source, ProxyCertSource::SelfSigned);
        assert_eq!(saved.proxy_http_cert_pem, Some(cert_pem));
        assert_eq!(saved.proxy_http_cert_key_pem, Some(key_pem));
        assert_eq!(saved.proxy_http_cert_expiry, Some(expiry));
    }
}
