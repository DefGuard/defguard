use chrono::NaiveDateTime;
use std::net::IpAddr;

use defguard_core::{
    db::{
        models::{authentication_key::AuthenticationKey, oauth2client::OAuth2Client},
        Device, Group, Id, MFAMethod, User, WebAuthn, WebHook, WireguardNetwork,
    },
    enterprise::db::models::{
        api_tokens::ApiToken, audit_stream::AuditStream, openid_provider::OpenIdProvider,
    },
    events::{ApiRequestContext, BidiRequestContext, GrpcRequestContext, InternalEventContext},
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
// TODO: remove lint override below once all events are updated to pass whole objects
#[allow(clippy::large_enum_variant)]
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

impl From<GrpcRequestContext> for EventContext {
    fn from(val: GrpcRequestContext) -> Self {
        EventContext {
            timestamp: val.timestamp,
            user_id: val.user_id,
            username: val.username,
            ip: val.ip,
            device: format!("{} (ID {})", val.device_name, val.device_id),
        }
    }
}

impl From<BidiRequestContext> for EventContext {
    fn from(val: BidiRequestContext) -> Self {
        EventContext {
            timestamp: val.timestamp,
            user_id: val.user_id,
            username: val.username,
            ip: val.ip,
            device: val.user_agent,
        }
    }
}

impl From<InternalEventContext> for EventContext {
    fn from(val: InternalEventContext) -> Self {
        EventContext {
            timestamp: val.timestamp,
            user_id: val.user_id,
            username: val.username,
            ip: val.ip,
            device: format!("{} (ID {})", val.device.name, val.device.id),
        }
    }
}

/// Represents audit events related to actions performed in Web UI
pub enum DefguardEvent {
    UserLogin,
    UserLogout,
    UserLoginFailed,
    UserMfaLogin {
        mfa_method: MFAMethod,
    },
    UserMfaLoginFailed {
        mfa_method: MFAMethod,
    },
    RecoveryCodeUsed,
    PasswordChangedByAdmin {
        user: User<Id>,
    },
    PasswordChanged,
    PasswordReset {
        user: User<Id>,
    },
    MfaDisabled,
    MfaTotpDisabled,
    MfaTotpEnabled,
    MfaEmailDisabled,
    MfaEmailEnabled,
    MfaSecurityKeyAdded {
        key: WebAuthn<Id>,
    },
    MfaSecurityKeyRemoved {
        key: WebAuthn<Id>,
    },
    UserAdded {
        user: User<Id>,
    },
    UserRemoved {
        user: User<Id>,
    },
    UserModified {
        before: User<Id>,
        after: User<Id>,
    },
    UserDeviceAdded {
        owner: User<Id>,
        device: Device<Id>,
    },
    UserDeviceRemoved {
        owner: User<Id>,
        device: Device<Id>,
    },
    UserDeviceModified {
        owner: User<Id>,
        before: Device<Id>,
        after: Device<Id>,
    },
    NetworkDeviceAdded {
        device: Device<Id>,
        location: WireguardNetwork<Id>,
    },
    NetworkDeviceRemoved {
        device: Device<Id>,
        location: WireguardNetwork<Id>,
    },
    NetworkDeviceModified {
        before: Device<Id>,
        after: Device<Id>,
        location: WireguardNetwork<Id>,
    },
    AuditStreamCreated {
        stream: AuditStream<Id>,
    },
    AuditStreamModified {
        before: AuditStream<Id>,
        after: AuditStream<Id>,
    },
    AuditStreamRemoved {
        stream: AuditStream<Id>,
    },
    VpnLocationAdded {
        location: WireguardNetwork<Id>,
    },
    VpnLocationRemoved {
        location: WireguardNetwork<Id>,
    },
    VpnLocationModified {
        before: WireguardNetwork<Id>,
        after: WireguardNetwork<Id>,
    },
    ApiTokenAdded {
        owner: User<Id>,
        token: ApiToken<Id>,
    },
    ApiTokenRemoved {
        owner: User<Id>,
        token: ApiToken<Id>,
    },
    ApiTokenRenamed {
        owner: User<Id>,
        token: ApiToken<Id>,
        old_name: String,
        new_name: String,
    },
    OpenIdAppAdded {
        app: OAuth2Client<Id>,
    },
    OpenIdAppRemoved {
        app: OAuth2Client<Id>,
    },
    OpenIdAppModified {
        before: OAuth2Client<Id>,
        after: OAuth2Client<Id>,
    },
    OpenIdAppStateChanged {
        app: OAuth2Client<Id>,
        enabled: bool,
    },
    OpenIdProviderModified {
        provider: OpenIdProvider<Id>,
    },
    OpenIdProviderRemoved {
        provider: OpenIdProvider<Id>,
    },
    SettingsUpdated,
    SettingsUpdatedPartial,
    SettingsDefaultBrandingRestored,
    GroupsBulkAssigned {
        users: Vec<User<Id>>,
        groups: Vec<Group<Id>>,
    },
    GroupAdded {
        group: Group<Id>,
    },
    GroupModified {
        before: Group<Id>,
        after: Group<Id>,
    },
    GroupRemoved {
        group: Group<Id>,
    },
    GroupMemberAdded {
        group: Group<Id>,
        user: User<Id>,
    },
    GroupMemberRemoved {
        group: Group<Id>,
        user: User<Id>,
    },
    WebHookAdded {
        webhook: WebHook<Id>,
    },
    WebHookModified {
        before: WebHook<Id>,
        after: WebHook<Id>,
    },
    WebHookRemoved {
        webhook: WebHook<Id>,
    },
    WebHookStateChanged {
        webhook: WebHook<Id>,
        enabled: bool,
    },
    AuthenticationKeyAdded {
        key: AuthenticationKey<Id>,
    },
    AuthenticationKeyRemoved {
        key: AuthenticationKey<Id>,
    },
    AuthenticationKeyRenamed {
        key: AuthenticationKey<Id>,
        old_name: Option<String>,
        new_name: Option<String>,
    },
    EnrollmentTokenAdded {
        user: User<Id>,
    },
    ClientConfigurationTokenAdded {
        user: User<Id>,
    },
}

/// Represents audit events related to client applications
pub enum ClientEvent {
    DesktopClientActivated { device_id: Id, device_name: String },
    DesktopClientUpdated { device_id: Id, device_name: String },
}

/// Represents audit events related to VPN
pub enum VpnEvent {
    ConnectedToMfaLocation {
        location: WireguardNetwork<Id>,
        device: Device<Id>,
        method: MFAMethod,
    },
    DisconnectedFromMfaLocation {
        location: WireguardNetwork<Id>,
        device: Device<Id>,
    },
    MfaFailed {
        location: WireguardNetwork<Id>,
        device: Device<Id>,
        method: MFAMethod,
    },
    ConnectedToLocation {
        location: WireguardNetwork<Id>,
        device: Device<Id>,
    },
    DisconnectedFromLocation {
        location: WireguardNetwork<Id>,
        device: Device<Id>,
    },
}

/// Represents audit events related to user enrollment process
pub enum EnrollmentEvent {
    EnrollmentStarted,
    EnrollmentDeviceAdded { device: Device<Id> },
    EnrollmentCompleted,
    PasswordResetRequested,
    PasswordResetStarted,
    PasswordResetCompleted,
    TokenAdded { user: User<Id> },
}
