use std::net::IpAddr;

use chrono::NaiveDateTime;
use defguard_common::db::{
    Id,
    models::{
        AuthenticationKey, Device, MFAMethod, Settings, User, WebAuthn, WireguardNetwork,
        gateway::Gateway, group::Group, oauth2client::OAuth2Client, proxy::Proxy,
    },
};
use defguard_core::{
    db::WebHook,
    enterprise::db::models::{
        activity_log_stream::ActivityLogStream, api_tokens::ApiToken,
        openid_provider::OpenIdProvider, snat::UserSnatBinding,
    },
    events::{ApiRequestContext, BidiRequestContext, ClientMFAMethod, GrpcRequestContext},
};
use defguard_session_manager::events::SessionManagerEventContext;

/// Messages that can be sent to the event logger
pub struct EventLoggerMessage {
    pub context: EventContext,
    pub event: LoggerEvent,
}

impl EventLoggerMessage {
    #[must_use]
    pub fn new(context: EventContext, event: LoggerEvent) -> Self {
        Self { context, event }
    }
}

/// Possible activity log event types split by module
pub enum LoggerEvent {
    Defguard(Box<DefguardEvent>),
    Vpn(Box<VpnEvent>),
    Enrollment(Box<EnrollmentEvent>),
}

/// Shared context that's included in all activity log events
pub struct EventContext {
    pub timestamp: NaiveDateTime,
    pub user_id: Id,
    pub username: String,
    pub location: Option<String>,
    pub ip: IpAddr,
    pub device: String,
}

impl EventContext {
    #[must_use]
    pub fn from_api_context(
        val: ApiRequestContext,
        location: Option<WireguardNetwork<Id>>,
    ) -> Self {
        let location = location.map(|location| location.name);

        Self {
            timestamp: val.timestamp,
            user_id: val.user_id,
            username: val.username,
            location,
            ip: val.ip,
            device: val.device,
        }
    }

    #[must_use]
    pub fn from_bidi_context(
        val: BidiRequestContext,
        location: Option<WireguardNetwork<Id>>,
    ) -> Self {
        let location = location.map(|location| location.name);

        Self {
            timestamp: val.timestamp,
            user_id: val.user_id,
            username: val.username,
            location,
            ip: val.ip,
            device: val.device_name,
        }
    }

    #[must_use]
    pub fn from_session_manager_context(val: SessionManagerEventContext) -> Self {
        Self {
            timestamp: val.timestamp,
            user_id: val.user.id,
            username: val.user.username,
            location: Some(val.location.name),
            ip: val.public_ip,
            device: format!("{} (ID {})", val.device.name, val.device.id),
        }
    }
}

impl From<GrpcRequestContext> for EventContext {
    fn from(val: GrpcRequestContext) -> Self {
        Self {
            timestamp: val.timestamp,
            user_id: val.user_id,
            username: val.username,
            location: Some(val.location.name),
            ip: val.ip,
            device: format!("{} (ID {})", val.device_name, val.device_id),
        }
    }
}

/// Represents activity log events related to actions performed in Web UI
pub enum DefguardEvent {
    UserLogin,
    UserLoginFailed {
        message: String,
    },
    UserLogout,
    UserMfaLogin {
        mfa_method: MFAMethod,
    },
    UserMfaLoginFailed {
        mfa_method: MFAMethod,
        message: String,
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
    UserMfaDisabled {
        user: User<Id>,
    },
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
    UserGroupsModified {
        user: User<Id>,
        before: Vec<String>,
        after: Vec<String>,
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
    ActivityLogStreamCreated {
        stream: ActivityLogStream<Id>,
    },
    ActivityLogStreamModified {
        before: ActivityLogStream<Id>,
        after: ActivityLogStream<Id>,
    },
    ActivityLogStreamRemoved {
        stream: ActivityLogStream<Id>,
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
    SettingsUpdated {
        before: Settings,
        after: Settings,
    },
    SettingsUpdatedPartial {
        before: Settings,
        after: Settings,
    },
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
    GroupMembersModified {
        group: Group<Id>,
        added: Vec<User<Id>>,
        removed: Vec<User<Id>>,
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
    ClientConfigurationTokenAdded {
        user: User<Id>,
    },
    UserSnatBindingAdded {
        user: User<Id>,
        binding: UserSnatBinding<Id>,
    },
    UserSnatBindingRemoved {
        user: User<Id>,
        binding: UserSnatBinding<Id>,
    },
    UserSnatBindingModified {
        user: User<Id>,
        before: UserSnatBinding<Id>,
        after: UserSnatBinding<Id>,
    },
    ProxyModified {
        before: Proxy<Id>,
        after: Proxy<Id>,
    },
    ProxyDeleted {
        proxy: Proxy<Id>,
    },
    GatewayModified {
        before: Gateway<Id>,
        after: Gateway<Id>,
    },
    GatewayDeleted {
        gateway: Gateway<Id>,
    },
}

/// Represents activity log events related to client applications
pub enum ClientEvent {
    DesktopClientActivated { device_id: Id, device_name: String },
    DesktopClientUpdated { device_id: Id, device_name: String },
}

/// Represents activity log events related to VPN
pub enum VpnEvent {
    ClientMfaSuccess {
        location: WireguardNetwork<Id>,
        device: Device<Id>,
        method: ClientMFAMethod,
    },
    ClientMfaFailed {
        location: WireguardNetwork<Id>,
        device: Device<Id>,
        method: ClientMFAMethod,
        message: String,
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

/// Represents activity log events related to user enrollment process
pub enum EnrollmentEvent {
    EnrollmentStarted,
    EnrollmentDeviceAdded { device: Device<Id> },
    EnrollmentCompleted,
    PasswordResetRequested,
    PasswordResetStarted,
    PasswordResetCompleted,
    TokenAdded { user: User<Id> },
}
