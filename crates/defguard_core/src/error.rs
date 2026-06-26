use crate::mail::templates::TemplateError;
use axum::http::StatusCode;
use defguard_common::{
    db::models::{
        DeviceError, ModelError, WireguardNetworkError,
        settings::{SettingsSaveError, SettingsUrlError, SettingsValidationError},
        user::UserError,
    },
    types::UrlParseError,
};
use defguard_static_ip::error::StaticIpError;
use thiserror::Error;
use tokio::sync::mpsc::error::SendError;
use utoipa::ToSchema;

use crate::{
    auth::failed_login::FailedLoginError,
    cert_settings::CertSettingsError,
    db::models::enrollment::TokenError,
    enterprise::{
        activity_log_stream::error::ActivityLogStreamError, db::models::acl::AclError,
        firewall::FirewallError, license::LicenseError,
    },
    events::ApiEvent,
    handlers::{openid_flow::OidcFlowError, user::ValidationError},
    location_management::LocationManagementError,
    user_management::UserManagementError,
};

/// Represents kinds of error that occurred
#[derive(Debug, Error, ToSchema)]
pub enum WebError {
    #[error("GRPC error: {0}")]
    Grpc(String),
    #[error("Webauthn registration error: {0}")]
    WebauthnRegistration(String),
    #[error("Email error: {0}")]
    Email(String),
    #[error("Object not found: {0}")]
    ObjectNotFound(String),
    #[error("Object already exists: {0}")]
    ObjectAlreadyExists(String),
    #[error("Serialization error: {0}")]
    Serialization(String),
    #[error("Authorization error: {0}")]
    Authorization(String),
    #[error("User groups not synced: {0}")]
    UserGroupsNotSynced(String),
    #[error("Authentication error")]
    Authentication,
    #[error("Forbidden error: {0}")]
    Forbidden(&'static str),
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
    #[error("Network full: {0}")]
    NetworkFull(String),
    #[error(transparent)]
    #[schema(value_type=Object)]
    IpNetwork(#[from] ipnetwork::IpNetworkError),
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
            DeviceError::Unexpected(_) => Self::Http(StatusCode::INTERNAL_SERVER_ERROR),
            DeviceError::NetworkFull(_) => Self::NetworkFull(error.to_string()),
            DeviceError::NetworkIpAssignmentError(_) | DeviceError::ModelError(_) => {
                Self::ModelError(error.to_string())
            }
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
        error!("{err}");
        match err {
            TokenError::DbError(msg) => Self::DbError(msg.to_string()),
            TokenError::NotFound | TokenError::UserNotFound | TokenError::AdminNotFound => {
                Self::ObjectNotFound(err.to_string())
            }
            TokenError::TokenExpired
            | TokenError::SessionExpired
            | TokenError::TokenUsed
            | TokenError::UserDisabled => Self::Authorization(err.to_string()),
            TokenError::AlreadyActive => Self::BadRequest(err.to_string()),
            TokenError::WelcomeMsgNotConfigured
            | TokenError::WelcomeEmailNotConfigured
            | TokenError::TemplateError(_)
            | TokenError::UrlParseError(_)
            | TokenError::TemplateErrorInternal(_) => Self::Http(StatusCode::INTERNAL_SERVER_ERROR),
        }
    }
}

impl From<SettingsValidationError> for WebError {
    fn from(err: SettingsValidationError) -> Self {
        match err {
            SettingsValidationError::CannotEnableGatewayNotifications
            | SettingsValidationError::CannotEnableLdapRemoteEnrollment
            | SettingsValidationError::CannotEnableLdapRemoteEnrollmentInvite
            | SettingsValidationError::CannotEnableLdap
            | SettingsValidationError::InvalidDefguardUrl(_) => Self::BadRequest(err.to_string()),
        }
    }
}

impl From<SettingsSaveError> for WebError {
    fn from(err: SettingsSaveError) -> Self {
        match err {
            SettingsSaveError::Db(err) => Self::DbError(err.to_string()),
            SettingsSaveError::Validation(err) => err.into(),
        }
    }
}

impl From<SettingsUrlError> for WebError {
    fn from(err: SettingsUrlError) -> Self {
        Self::BadRequest(err.to_string())
    }
}

impl From<UserError> for WebError {
    fn from(err: UserError) -> Self {
        error!("{err}");
        match err {
            UserError::InvalidMfaState { username: _ } | UserError::DbError(_) => {
                Self::Http(StatusCode::INTERNAL_SERVER_ERROR)
            }
            UserError::EmailMfaError(msg) => Self::Email(msg),
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

impl From<UserManagementError> for WebError {
    fn from(err: UserManagementError) -> Self {
        match err {
            UserManagementError::Db(e) => {
                error!("Database error: {e}");
                Self::DbError(e.to_string())
            }
            UserManagementError::Model(e) => {
                error!("Model error: {e}");
                Self::ModelError(e.to_string())
            }
            UserManagementError::Network(e) => {
                error!("WireGuard network error: {e}");
                Self::from(e)
            }
            UserManagementError::Firewall(e) => {
                error!("Firewall error: {e}");
                Self::FirewallError(e)
            }
        }
    }
}

impl From<CertSettingsError> for WebError {
    fn from(err: CertSettingsError) -> Self {
        error!("{err}");
        match err {
            CertSettingsError::InvalidCert(msg) => Self::BadRequest(msg),
            CertSettingsError::Cert(e) => Self::CertificateError(e),
            CertSettingsError::Url(e) => Self::BadRequest(e),
            CertSettingsError::Settings(e) => Self::BadRequest(e.to_string()),
            CertSettingsError::Db(e) => Self::DbError(e.to_string()),
            CertSettingsError::NotFound(msg) => Self::ObjectNotFound(msg),
        }
    }
}

impl From<ValidationError> for WebError {
    fn from(err: ValidationError) -> Self {
        Self::BadRequest(err.0)
    }
}

impl From<OidcFlowError> for WebError {
    fn from(err: OidcFlowError) -> Self {
        match err {
            OidcFlowError::SigningKey(_msg) => Self::Http(StatusCode::INTERNAL_SERVER_ERROR),
            OidcFlowError::InvalidRedirectUri => Self::Http(StatusCode::BAD_REQUEST),
            OidcFlowError::Internal(_msg) => Self::Http(StatusCode::INTERNAL_SERVER_ERROR),
            OidcFlowError::Db(e) => Self::DbError(e.to_string()),
            OidcFlowError::Url(_e) => Self::Http(StatusCode::INTERNAL_SERVER_ERROR),
        }
    }
}
