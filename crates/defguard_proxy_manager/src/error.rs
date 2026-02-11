use defguard_core::db::models::enrollment::TokenError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProxyError {
    #[error(transparent)]
    InvalidUriError(#[from] axum::http::uri::InvalidUri),
    #[error("Failed to read CA certificate: {0}")]
    CaCertReadError(std::io::Error),
    #[error(transparent)]
    TonicError(#[from] tonic::transport::Error),
    #[error(transparent)]
    SemverError(#[from] semver::Error),
    #[error(transparent)]
    SqlxError(#[from] sqlx::Error),
    #[error(transparent)]
    TokenError(#[from] TokenError),
    #[error(transparent)]
    CertificateError(#[from] defguard_certs::CertificateError),
    #[error(transparent)]
    UrlParseError(#[from] openidconnect::url::ParseError),
    #[error("Missing proxy configuration: {0}")]
    MissingConfiguration(String),
    #[error("URL error: {0}")]
    UrlError(String),
    #[error(transparent)]
    Transport(#[from] tonic::Status),
    #[error("Connection timeout: {0}")]
    ConnectionTimeout(String),
    #[error("TLS config error: {0}")]
    TlsConfigError(String),
}
