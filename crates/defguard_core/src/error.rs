use axum::http::StatusCode;
use sqlx::error::Error as SqlxError;
use thiserror::Error;
use tokio::sync::mpsc::error::SendError;

use crate::{
    auth::failed_login::FailedLoginError,
    db::models::{
        device::DeviceError, enrollment::TokenError, error::ModelError,
        settings::SettingsValidationError, wireguard::WireguardNetworkError,
    },
    enterprise::{
        activity_log_stream::error::ActivityLogStreamError, db::models::acl::AclError,
        firewall::FirewallError, ldap::error::LdapError, license::LicenseError,
    },
    events::ApiEvent,
    grpc::gateway::map::GatewayMapError,
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
    #[error("Object already exists: {0}")]
    ObjectAlreadyExists(String),
    #[error("Serialization error: {0}")]
    Serialization(String),
    #[error("Deserialization error: {0}")]
    Deserialization(String),
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
    #[error("Failed to get client IP address")]
    ClientIpError,
    #[error("ACL error: {0}")]
    AclError(#[from] AclError),
    #[error("Firewall config error: {0}")]
    FirewallError(#[from] FirewallError),
    #[error("API event channel error: {0}")]
    ApiEventChannelError(#[from] SendError<ApiEvent>),
    #[error("Activity log stream error: {0}")]
    ActivityLogStreamError(#[from] ActivityLogStreamError),
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
        Self::Ldap(error.to_string())
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
            DeviceError::NetworkIpAssignmentError(_) => Self::ModelError(error.to_string()),
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
            GatewayMapError::ConfigError => Self::ServerConfigMissing,
            GatewayMapError::SettingsError => Self::DbError(error.to_string()),
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
            | WireguardNetworkError::DeviceNotAllowed(_)
            | WireguardNetworkError::FirewallError(_)
            | WireguardNetworkError::TokenError(_) => Self::Http(StatusCode::INTERNAL_SERVER_ERROR),
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

impl From<SettingsValidationError> for WebError {
    fn from(err: SettingsValidationError) -> Self {
        match err {
            SettingsValidationError::CannotEnableGatewayNotifications => {
                Self::BadRequest(err.to_string())
            }
        }
    }
}
