use std::net::IpAddr;

use chrono::NaiveDateTime;
use ipnetwork::IpNetwork;

use defguard_core::{
    db::{models::authentication_key::AuthenticationKeyType, Id},
    events::ApiRequestContext,
    grpc::proto::proxy::MfaMethod,
};

/// Messages that can be sent to the event logger
pub struct EventLoggerMessage {
    pub context: EventContext,
    pub event: LoggerEvent,
}

impl EventLoggerMessage {
    pub fn new(context: EventContext, event: LoggerEvent) -> Self {
        Self { context, event }
    }
}

/// Possible audit event types split by module
pub enum LoggerEvent {
    Defguard(DefguardEvent),
    Client(ClientEvent),
    Vpn(VpnEvent),
    Enrollment(EnrollmentEvent),
}

/// Shared context that's included in all events
pub struct EventContext {
    pub timestamp: NaiveDateTime,
    pub user_id: Id,
    pub username: String,
    pub ip: IpAddr,
    pub device: String,
}

impl From<ApiRequestContext> for EventContext {
    fn from(val: ApiRequestContext) -> Self {
        EventContext {
            timestamp: val.timestamp,
            user_id: val.user_id,
            username: val.username,
            ip: val.ip,
            device: val.device,
        }
    }
}

/// Represents audit events related to actions performed in Web UI
pub enum DefguardEvent {
    // authentication
    UserLogin,
    UserLogout,
    RecoveryCodeUsed,
    PasswordChanged,
    MfaFailed,
    // user MFA management
    MfaDisabled,
    MfaDefaultChanged {
        mfa_method: MfaMethod,
    },
    MfaTotpEnabled,
    MfaTotpDisabled,
    MfaEmailEnabled,
    MfaEmailDisabled,
    MfaSecurityKeyAdded {
        key_id: Id,
        key_name: String,
    },
    MfaSecurityKeyRemoved {
        key_id: Id,
        key_name: String,
    },
    // authentication key management
    AuthenticationKeyAdded {
        key_id: Id,
        key_name: String,
        key_type: AuthenticationKeyType,
    },
    AuthenticationKeyRemoved {
        key_id: Id,
        key_name: String,
        key_type: AuthenticationKeyType,
    },
    AuthenticationKeyRenamed {
        key_id: Id,
        key_name: String,
        key_type: AuthenticationKeyType,
    },
    // API token management
    ApiTokenAdded {
        token_id: Id,
        token_name: String,
    },
    ApiTokenRemoved {
        token_id: Id,
        token_name: String,
    },
    ApiTokenRenamed {
        token_id: Id,
        token_name: String,
    },
    // user management
    UserAdded {
        username: String,
    },
    UserRemoved {
        username: String,
    },
    UserModified {
        username: String,
    },
    UserDisabled {
        username: String,
    },
    // device management
    UserDeviceAdded {
        device_id: Id,
        device_name: String,
        owner: String,
    },
    UserDeviceRemoved {
        device_id: Id,
        device_name: String,
        owner: String,
    },
    UserDeviceModified {
        device_id: Id,
        device_name: String,
        owner: String,
    },
    NetworkDeviceAdded {
        device_id: Id,
        device_name: String,
        location_id: Id,
        location: String,
    },
    NetworkDeviceRemoved {
        device_id: Id,
        device_name: String,
        location_id: Id,
        location: String,
    },
    NetworkDeviceModified {
        device_id: Id,
        device_name: String,
        location_id: Id,
        location: String,
    },
    // VPN location management
    VpnLocationAdded {
        location_id: Id,
        location_name: String,
    },
    VpnLocationRemoved {
        location_id: Id,
        location_name: String,
    },
    VpnLocationModified {
        location_id: Id,
        location_name: String,
    },
    // OpenID app management
    OpenIdAppAdded {
        app_id: Id,
        app_name: String,
    },
    OpenIdAppRemoved {
        app_id: Id,
        app_name: String,
    },
    OpenIdAppModified {
        app_id: Id,
        app_name: String,
    },
    OpenIdAppDisabled {
        app_id: Id,
        app_name: String,
    },
    // OpenID provider management
    OpenIdProviderAdded {
        provider_id: Id,
        provider_name: String,
    },
    OpenIdProviderRemoved {
        provider_id: Id,
        provider_name: String,
    },
    // settings management
    SettingsUpdated,
    SettingsUpdatedPartial,
    SettingsDefaultBrandingRestored,
}

/// Represents audit events related to client applications
pub enum ClientEvent {
    DesktopClientActivated { device_id: Id, device_name: String },
    DesktopClientUpdated { device_id: Id, device_name: String },
}

/// Represents audit events related to VPN
pub enum VpnEvent {
    ConnectedToMfaLocation {
        location_id: Id,
        location_name: String,
    },
    DisconnectedFromMfaLocation {
        location_id: Id,
        location_name: String,
    },
    MfaFailed {
        location_id: Id,
        location_name: String,
    },
}

/// Represents audit events related to user enrollment process
pub enum EnrollmentEvent {
    EnrollmentStarted,
    EnrollmentPasswordConfigured,
    EnrollmentPhoneNumberConfigured,
    EnrollmentDeviceAdded { device_id: Id, device_name: String },
    EnrollmentMfaTotpConfigured,
    EnrollmentRecoveryCodesDownloaded,
    EnrollmentCompleted,
}
