use axum::http::StatusCode;
use defguard_common::{
    db::models::{
        DeviceError, ModelError, WireguardNetworkError, settings::SettingsValidationError,
        user::UserError,
    },
    types::UrlParseError,
};
use defguard_mail::templates::TemplateError;
use defguard_static_ip::error::StaticIpError;
use thiserror::Error;
use tokio::sync::mpsc::error::SendError;
use utoipa::ToSchema;

use crate::{
    auth::failed_login::FailedLoginError,
    db::models::enrollment::TokenError,
    enterprise::{
        activity_log_stream::error::ActivityLogStreamError, db::models::acl::AclError,
        firewall::FirewallError, license::LicenseError,
    },
    events::ApiEvent,
    location_management::LocationManagementError,
};

/// Represents kinds of error that occurred
#[derive(Debug, Error, ToSchema)]
pub enum WebError {
    #[error("GRPC error: {0}")]
    Grpc(String),
    #[error("Webauthn registration error: {0}")]
    WebauthnRegistration(String),
    #[error("Email MFA error: {0}")]
    EmailMfa(String),
    #[error("Object not found: {0}")]
    ObjectNotFound(String),
    #[error("Object already exists: {0}")]
    ObjectAlreadyExists(String),
    #[error("Serialization error: {0}")]
    Serialization(String),
    #[error("Authorization error: {0}")]
    Authorization(String),
    #[error("Authentication error")]
    Authentication,
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
    #[schema(value_type=Object)]
    Http(StatusCode),
    #[error(transparent)]
    #[schema(value_type=Object)]
    TooManyLoginAttempts(#[from] FailedLoginError),
    #[error("Bad request: {0}")]
    BadRequest(String),
    #[error(transparent)]
    #[schema(value_type=Object)]
    TemplateError(#[from] TemplateError),
    #[error("License error: {0}")]
    #[schema(value_type=Object)]
    LicenseError(#[from] LicenseError),
    #[error("Failed to get client IP address")]
    ClientIpError,
    #[error("ACL error: {0}")]
    #[schema(value_type=Object)]
    AclError(#[from] AclError),
    #[error("Firewall config error: {0}")]
    #[schema(value_type=Object)]
    FirewallError(#[from] FirewallError),
    #[error("API event channel error: {0}")]
    #[schema(value_type=Object)]
    ApiEventChannelError(#[from] SendError<ApiEvent>),
    #[error("Activity log stream error: {0}")]
    #[schema(value_type=Object)]
    ActivityLogStreamError(#[from] ActivityLogStreamError),
    #[error(transparent)]
    #[schema(value_type=Object)]
    CertificateError(#[from] defguard_certs::CertificateError),
    #[error(transparent)]
    #[schema(value_type=Object)]
    UrlParseError(#[from] UrlParseError),
    #[error(transparent)]
    #[schema(value_type=Object)]
    StaticIpError(#[from] StaticIpError),
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

impl From<sqlx::Error> for WebError {
    fn from(error: sqlx::Error) -> Self {
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
            | WireguardNetworkError::TokenError(_) => Self::Http(StatusCode::INTERNAL_SERVER_ERROR),
        }
    }
}

impl From<TokenError> for WebError {
    fn from(err: TokenError) -> Self {
        error!("{err}");
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
            | TokenError::UrlParseError(_)
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

impl From<UserError> for WebError {
    fn from(err: UserError) -> Self {
        error!("{err}");
        match err {
            UserError::InvalidMfaState { username: _ } | UserError::DbError(_) => {
                WebError::Http(StatusCode::INTERNAL_SERVER_ERROR)
            }
            UserError::EmailMfaError(msg) => WebError::EmailMfa(msg),
        }
    }
}

impl From<LocationManagementError> for WebError {
    fn from(err: LocationManagementError) -> Self {
        error!("{err}");
        match err {
            LocationManagementError::FirewallError(firewall_error) => firewall_error.into(),
            LocationManagementError::DbError(error) => error.into(),
            LocationManagementError::WireguardNetworkError(wireguard_network_error) => {
                wireguard_network_error.into()
            }
            LocationManagementError::ModelError(model_error) => model_error.into(),
        }
    }
}
