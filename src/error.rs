use axum::http::StatusCode;
use sqlx::error::Error as SqlxError;
use thiserror::Error;
use utoipa::ToSchema;

use crate::{
    auth::failed_login::FailedLoginError,
    db::models::{
        device::DeviceError, enrollment::TokenError, error::ModelError,
        wireguard::WireguardNetworkError,
    },
    enterprise::license::LicenseError,
    ldap::error::LdapError,
    templates::TemplateError,
};

/// Represents kinds of error that occurred
#[derive(Debug, Error, ToSchema)]
pub enum WebError {
    #[error("GRPC error: {0}")]
    Grpc(String),
    #[error("LDAP error: {0}")]
    Ldap(String),
    #[error("Webauthn registration error: {0}")]
    WebauthnRegistration(String),
    #[error("Email MFA error: {0}")]
    EmailMfa(String),
    #[error("Incorrect username: {0}")]
    IncorrectUsername(String),
    #[error("Object not found: {0}")]
    ObjectNotFound(String),
    #[error("Serialization error: {0}")]
    Serialization(String),
    #[error("Authorization error: {0}")]
    Authorization(String),
    #[error("Forbidden error: {0}")]
    Forbidden(String),
    #[error("Database error: {0}")]
    DbError(String),
    #[error("Model error: {0}")]
    ModelError(String),
    #[error("Public key invalid {0}")]
    PubkeyValidation(String),
    #[error("Public key already exists {0}")]
    PubkeyExists(String),
    #[error("HTTP error: {0}")]
    #[schema(value_type = u16)]
    Http(StatusCode),
    #[error(transparent)]
    TooManyLoginAttempts(#[from] FailedLoginError),
    #[error("Bad request: {0}")]
    BadRequest(String),
    #[error(transparent)]
    TemplateError(#[from] TemplateError),
    #[error("Server config missing")]
    ServerConfigMissing,
    #[error("License error: {0}")]
    LicenseError(#[from] LicenseError),
}

impl From<tonic::Status> for WebError {
    fn from(status: tonic::Status) -> Self {
        Self::Grpc(status.message().into())
    }
}

impl From<StatusCode> for WebError {
    fn from(status: StatusCode) -> Self {
        Self::Http(status)
    }
}

impl From<LdapError> for WebError {
    fn from(error: LdapError) -> Self {
        match error {
            LdapError::ObjectNotFound(msg) => Self::ObjectNotFound(msg),
            LdapError::Ldap(msg) => Self::Ldap(msg),
            LdapError::MissingSettings => Self::Ldap("LDAP settings are missing".into()),
            LdapError::Database => Self::Ldap("Database problem".into()),
        }
    }
}

impl From<SqlxError> for WebError {
    fn from(error: SqlxError) -> Self {
        Self::DbError(error.to_string())
    }
}

impl From<ModelError> for WebError {
    fn from(error: ModelError) -> Self {
        Self::ModelError(error.to_string())
    }
}

impl From<DeviceError> for WebError {
    fn from(error: DeviceError) -> Self {
        match error {
            DeviceError::PubkeyConflict(..) => Self::PubkeyValidation(error.to_string()),
            DeviceError::DatabaseError(_) => Self::DbError(error.to_string()),
            DeviceError::ModelError(_) => Self::ModelError(error.to_string()),
            DeviceError::Unexpected(_) => Self::Http(StatusCode::INTERNAL_SERVER_ERROR),
        }
    }
}

impl From<WireguardNetworkError> for WebError {
    fn from(error: WireguardNetworkError) -> Self {
        match error {
            WireguardNetworkError::NetworkTooSmall
            | WireguardNetworkError::IpNetworkError(_)
            | WireguardNetworkError::InvalidDevicePubkey(_) => Self::BadRequest(error.to_string()),
            WireguardNetworkError::DbError(_)
            | WireguardNetworkError::ModelError(_)
            | WireguardNetworkError::Unexpected(_)
            | WireguardNetworkError::DeviceError(_)
            | WireguardNetworkError::DeviceNotAllowed(_) => {
                Self::Http(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
}

impl From<TokenError> for WebError {
    fn from(err: TokenError) -> Self {
        error!("{}", err);
        match err {
            TokenError::DbError(msg) => WebError::DbError(msg.to_string()),
            TokenError::NotFound | TokenError::UserNotFound | TokenError::AdminNotFound => {
                WebError::ObjectNotFound(err.to_string())
            }
            TokenError::TokenExpired
            | TokenError::SessionExpired
            | TokenError::TokenUsed
            | TokenError::UserDisabled => WebError::Authorization(err.to_string()),
            TokenError::AlreadyActive => WebError::BadRequest(err.to_string()),
            TokenError::NotificationError(_)
            | TokenError::WelcomeMsgNotConfigured
            | TokenError::WelcomeEmailNotConfigured
            | TokenError::TemplateError(_)
            | TokenError::TemplateErrorInternal(_) => {
                WebError::Http(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
}
