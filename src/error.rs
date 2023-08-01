use crate::auth::failed_login::FailedLoginError;
use crate::db::models::device::DeviceError;
use crate::db::models::enrollment::EnrollmentError;
use crate::db::models::wireguard::WireguardNetworkError;
use crate::grpc::GatewayMapError;
use crate::templates::TemplateError;
use crate::{db::models::error::ModelError, ldap::error::OriLDAPError};
use rocket::http::Status;
use sqlx::error::Error as SqlxError;
use thiserror::Error;

/// Represents kinds of error that occurred
#[derive(Debug, Error)]
pub enum OriWebError {
    #[error("GRPC error: {0}")]
    Grpc(String),
    #[error("LDAP error: {0}")]
    Ldap(String),
    #[error("Webauthn registration error: {0}")]
    WebauthnRegistration(String),
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
    Http(Status),
    #[error(transparent)]
    TooManyLoginAttempts(#[from] FailedLoginError),
    #[error("Bad request: {0}")]
    BadRequest(String),
    #[error(transparent)]
    TemplateError(#[from] TemplateError),
}

impl From<tonic::Status> for OriWebError {
    fn from(status: tonic::Status) -> Self {
        Self::Grpc(status.message().into())
    }
}

impl From<Status> for OriWebError {
    fn from(status: Status) -> Self {
        Self::Http(status)
    }
}

impl From<OriLDAPError> for OriWebError {
    fn from(error: OriLDAPError) -> Self {
        match error {
            OriLDAPError::ObjectNotFound(msg) => Self::ObjectNotFound(msg),
            OriLDAPError::Ldap(msg) => Self::Ldap(msg),
        }
    }
}

impl From<SqlxError> for OriWebError {
    fn from(error: SqlxError) -> Self {
        Self::DbError(error.to_string())
    }
}

impl From<ModelError> for OriWebError {
    fn from(error: ModelError) -> Self {
        Self::ModelError(error.to_string())
    }
}

impl From<DeviceError> for OriWebError {
    fn from(error: DeviceError) -> Self {
        match error {
            DeviceError::PubkeyConflict(..) => Self::PubkeyValidation(format!("{}", error)),
            DeviceError::DatabaseError(_) => Self::DbError(format!("{}", error)),
            DeviceError::ModelError(_) => Self::ModelError(format!("{}", error)),
            DeviceError::Unexpected(_) => Self::Http(Status::InternalServerError),
        }
    }
}

impl From<GatewayMapError> for OriWebError {
    fn from(error: GatewayMapError) -> Self {
        match error {
            GatewayMapError::NotFound(_, _)
            | GatewayMapError::NetworkNotFound(_)
            | GatewayMapError::UidNotFound(_) => Self::ObjectNotFound(format!("{}", error)),
            GatewayMapError::RemoveActive(_) => Self::BadRequest(format!("{}", error)),
        }
    }
}

impl From<WireguardNetworkError> for OriWebError {
    fn from(error: WireguardNetworkError) -> Self {
        match error {
            WireguardNetworkError::NetworkTooSmall
            | WireguardNetworkError::IpNetworkError(_)
            | WireguardNetworkError::InvalidDevicePubkey(_) => {
                Self::BadRequest(format!("{}", error))
            }
            WireguardNetworkError::DbError(_)
            | WireguardNetworkError::ModelError(_)
            | WireguardNetworkError::Unexpected(_)
            | WireguardNetworkError::DeviceError(_)
            | WireguardNetworkError::DeviceNotAllowed(_) => Self::Http(Status::InternalServerError),
        }
    }
}

impl From<EnrollmentError> for OriWebError {
    fn from(err: EnrollmentError) -> Self {
        error!("{}", err);
        match err {
            EnrollmentError::DbError(msg) => OriWebError::DbError(msg.to_string()),
            EnrollmentError::NotFound
            | EnrollmentError::UserNotFound
            | EnrollmentError::AdminNotFound => OriWebError::ObjectNotFound(err.to_string()),
            EnrollmentError::TokenExpired
            | EnrollmentError::SessionExpired
            | EnrollmentError::TokenUsed => OriWebError::Authorization(err.to_string()),
            EnrollmentError::AlreadyActive => OriWebError::BadRequest(err.to_string()),
            EnrollmentError::NotificationError(_) => OriWebError::Http(Status::InternalServerError),
        }
    }
}
