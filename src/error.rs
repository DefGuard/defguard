use axum::http::StatusCode;
use sqlx::error::Error as SqlxError;
use thiserror::Error;

use crate::{
    auth::failed_login::FailedLoginError,
    db::models::{
        device::DeviceError, enrollment::EnrollmentError, error::ModelError,
        wireguard::WireguardNetworkError,
    },
    grpc::GatewayMapError,
    ldap::error::OriLDAPError,
    templates::TemplateError,
};

/// Represents kinds of error that occurred
#[derive(Debug, Error)]
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
    #[error("HTTP error: {0}")]
    Http(StatusCode),
    #[error(transparent)]
    TooManyLoginAttempts(#[from] FailedLoginError),
    #[error("Bad request: {0}")]
    BadRequest(String),
    #[error(transparent)]
    TemplateError(#[from] TemplateError),
    #[error("Server config missing")]
    ServerConfigMissing,
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

impl From<OriLDAPError> for WebError {
    fn from(error: OriLDAPError) -> Self {
        match error {
            OriLDAPError::ObjectNotFound(msg) => Self::ObjectNotFound(msg),
            OriLDAPError::Ldap(msg) => Self::Ldap(msg),
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

impl From<GatewayMapError> for WebError {
    fn from(error: GatewayMapError) -> Self {
        match error {
            GatewayMapError::NotFound(_, _)
            | GatewayMapError::NetworkNotFound(_)
            | GatewayMapError::UidNotFound(_) => Self::ObjectNotFound(error.to_string()),
            GatewayMapError::RemoveActive(_) => Self::BadRequest(error.to_string()),
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

impl From<EnrollmentError> for WebError {
    fn from(err: EnrollmentError) -> Self {
        error!("{}", err);
        match err {
            EnrollmentError::DbError(msg) => WebError::DbError(msg.to_string()),
            EnrollmentError::NotFound
            | EnrollmentError::UserNotFound
            | EnrollmentError::AdminNotFound => WebError::ObjectNotFound(err.to_string()),
            EnrollmentError::TokenExpired
            | EnrollmentError::SessionExpired
            | EnrollmentError::TokenUsed => WebError::Authorization(err.to_string()),
            EnrollmentError::AlreadyActive => WebError::BadRequest(err.to_string()),
            EnrollmentError::NotificationError(_)
            | EnrollmentError::WelcomeMsgNotConfigured
            | EnrollmentError::WelcomeEmailNotConfigured
            | EnrollmentError::TemplateError(_)
            | EnrollmentError::TemplateErrorInternal(_) => {
                WebError::Http(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
}
