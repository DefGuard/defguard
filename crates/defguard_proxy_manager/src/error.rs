use defguard_core::db::models::enrollment::TokenError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProxyError {
    #[error(transparent)]
    TonicError(#[from] tonic::transport::Error),
    #[error(transparent)]
    SemverError(#[from] semver::Error),
    #[error(transparent)]
    SqlxError(#[from] sqlx::Error),
    #[error(transparent)]
    TokenError(#[from] TokenError),
    #[error(transparent)]
    UrlParseError(#[from] openidconnect::url::ParseError),
    #[error("Missing proxy configuration: {0}")]
    MissingConfiguration(String),
    #[error(transparent)]
    Transport(#[from] tonic::Status),
    #[error("TLS config error: {0}")]
    TlsConfigError(String),
}
