use std::net::IpAddr;

use chrono::NaiveDateTime;
use defguard_common::db::{
    Id,
    models::{
        AuthenticationKey, Device, MFAMethod, Settings, User, WebAuthn, WireguardNetwork,
        gateway::Gateway, group::Group, oauth2client::OAuth2Client, proxy::Proxy,
    },
};
use tokio::sync::Notify;
use tracing::debug;

use defguard_core::{
    db::WebHook,
    enterprise::db::models::{
        activity_log_stream::ActivityLogStream, api_tokens::ApiToken,
        device_posture::DevicePostureSnapshot, openid_provider::OpenIdProvider,
        snat::UserSnatBinding,
    },
    events::{
        ApiEvent, ApiEventType, ApiRequestContext, BidiRequestContext, BidiStreamEvent,
        BidiStreamEventType, ClientMFAMethod, DesktopClientMfaEvent, GrpcRequestContext,
        PasswordResetEvent,
    },
};
use defguard_session_manager::events::{
    SessionManagerEvent, SessionManagerEventContext, SessionManagerEventType,
};

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
    pub ip: Option<IpAddr>,
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
    RecoveryCodeLoginFailed,
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
    DevicePostureCreated {
        snapshot: DevicePostureSnapshot,
    },
    DevicePostureUpdated {
        before: DevicePostureSnapshot,
        after: DevicePostureSnapshot,
    },
    DevicePostureDeleted {
        snapshot: DevicePostureSnapshot,
    },
    DevicePostureDuplicated {
        original: DevicePostureSnapshot,
        duplicate: DevicePostureSnapshot,
    },
    DevicePostureLocationsAssigned {
        posture_id: Id,
        location_ids: Vec<Id>,
    },
    LocationPosturesAssigned {
        location_id: Id,
        posture_ids: Vec<Id>,
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
    MfaConnectedToLocation {
        location: WireguardNetwork<Id>,
        device: Device<Id>,
    },
    MfaDisconnectedFromLocation {
        location: WireguardNetwork<Id>,
        device: Device<Id>,
    },
}

/// Represents activity log events related to user enrollment process
#[allow(clippy::large_enum_variant)]
pub enum EnrollmentEvent {
    EnrollmentStarted,
    EnrollmentDeviceAdded { device: Device<Id> },
    EnrollmentCompleted,
    PasswordResetRequested,
    PasswordResetStarted,
    PasswordResetCompleted,
    TokenAdded { user: User<Id> },
}

impl EventLoggerMessage {
    pub fn from_api_event(api_event: ApiEvent, reload_notify: &Notify) -> Self {
        debug!("Processing API event: {api_event:?}");
        let (logger_event, location) = match *api_event.event {
            ApiEventType::UserLogin => (
                LoggerEvent::Defguard(Box::new(DefguardEvent::UserLogin)),
                None,
            ),
            ApiEventType::UserLoginFailed { message } => (
                LoggerEvent::Defguard(Box::new(DefguardEvent::UserLoginFailed { message })),
                None,
            ),
            ApiEventType::UserMfaLogin { mfa_method } => (
                LoggerEvent::Defguard(Box::new(DefguardEvent::UserMfaLogin { mfa_method })),
                None,
            ),
            ApiEventType::UserMfaLoginFailed {
                mfa_method,
                message,
            } => (
                LoggerEvent::Defguard(Box::new(DefguardEvent::UserMfaLoginFailed {
                    mfa_method,
                    message,
                })),
                None,
            ),
            ApiEventType::RecoveryCodeLoginFailed => (
                LoggerEvent::Defguard(Box::new(DefguardEvent::RecoveryCodeLoginFailed)),
                None,
            ),
            ApiEventType::RecoveryCodeUsed => (
                LoggerEvent::Defguard(Box::new(DefguardEvent::RecoveryCodeUsed)),
                None,
            ),
            ApiEventType::UserLogout => (
                LoggerEvent::Defguard(Box::new(DefguardEvent::UserLogout)),
                None,
            ),
            ApiEventType::UserAdded { user } => (
                LoggerEvent::Defguard(Box::new(DefguardEvent::UserAdded { user })),
                None,
            ),
            ApiEventType::UserRemoved { user } => (
                LoggerEvent::Defguard(Box::new(DefguardEvent::UserRemoved { user })),
                None,
            ),
            ApiEventType::UserModified { before, after } => (
                LoggerEvent::Defguard(Box::new(DefguardEvent::UserModified { before, after })),
                None,
            ),
            ApiEventType::UserGroupsModified {
                user,
                before,
                after,
            } => (
                LoggerEvent::Defguard(Box::new(DefguardEvent::UserGroupsModified {
                    user,
                    before,
                    after,
                })),
                None,
            ),
            ApiEventType::MfaDisabled => (
                LoggerEvent::Defguard(Box::new(DefguardEvent::MfaDisabled)),
                None,
            ),
            ApiEventType::UserMfaDisabled { user } => (
                LoggerEvent::Defguard(Box::new(DefguardEvent::UserMfaDisabled { user })),
                None,
            ),
            ApiEventType::MfaTotpDisabled => (
                LoggerEvent::Defguard(Box::new(DefguardEvent::MfaTotpDisabled)),
                None,
            ),
            ApiEventType::MfaTotpEnabled => (
                LoggerEvent::Defguard(Box::new(DefguardEvent::MfaTotpEnabled)),
                None,
            ),
            ApiEventType::MfaEmailDisabled => (
                LoggerEvent::Defguard(Box::new(DefguardEvent::MfaEmailDisabled)),
                None,
            ),
            ApiEventType::MfaEmailEnabled => (
                LoggerEvent::Defguard(Box::new(DefguardEvent::MfaEmailEnabled)),
                None,
            ),
            ApiEventType::MfaSecurityKeyAdded { key } => (
                LoggerEvent::Defguard(Box::new(DefguardEvent::MfaSecurityKeyAdded { key })),
                None,
            ),
            ApiEventType::MfaSecurityKeyRemoved { key } => (
                LoggerEvent::Defguard(Box::new(DefguardEvent::MfaSecurityKeyRemoved { key })),
                None,
            ),
            ApiEventType::UserDeviceAdded { owner, device } => (
                LoggerEvent::Defguard(Box::new(DefguardEvent::UserDeviceAdded { device, owner })),
                None,
            ),
            ApiEventType::UserDeviceRemoved { owner, device } => (
                LoggerEvent::Defguard(Box::new(DefguardEvent::UserDeviceRemoved { device, owner })),
                None,
            ),
            ApiEventType::UserDeviceModified {
                owner,
                before,
                after,
            } => (
                LoggerEvent::Defguard(Box::new(DefguardEvent::UserDeviceModified {
                    owner,
                    before,
                    after,
                })),
                None,
            ),
            ApiEventType::NetworkDeviceAdded { device, location } => (
                LoggerEvent::Defguard(Box::new(DefguardEvent::NetworkDeviceAdded {
                    device,
                    location: location.clone(),
                })),
                Some(location),
            ),
            ApiEventType::NetworkDeviceModified {
                before,
                after,
                location,
            } => (
                LoggerEvent::Defguard(Box::new(DefguardEvent::NetworkDeviceModified {
                    before,
                    after,
                    location: location.clone(),
                })),
                Some(location),
            ),
            ApiEventType::NetworkDeviceRemoved { device, location } => (
                LoggerEvent::Defguard(Box::new(DefguardEvent::NetworkDeviceRemoved {
                    device,
                    location: location.clone(),
                })),
                Some(location),
            ),
            ApiEventType::ActivityLogStreamCreated { stream } => {
                reload_notify.notify_waiters();
                (
                    LoggerEvent::Defguard(Box::new(DefguardEvent::ActivityLogStreamCreated {
                        stream,
                    })),
                    None,
                )
            }
            ApiEventType::ActivityLogStreamModified { before, after } => {
                reload_notify.notify_waiters();
                (
                    LoggerEvent::Defguard(Box::new(DefguardEvent::ActivityLogStreamModified {
                        before,
                        after,
                    })),
                    None,
                )
            }
            ApiEventType::ActivityLogStreamRemoved { stream } => {
                reload_notify.notify_waiters();
                (
                    LoggerEvent::Defguard(Box::new(DefguardEvent::ActivityLogStreamRemoved {
                        stream,
                    })),
                    None,
                )
            }
            ApiEventType::VpnLocationAdded { location } => (
                LoggerEvent::Defguard(Box::new(DefguardEvent::VpnLocationAdded {
                    location: location.clone(),
                })),
                Some(location),
            ),
            ApiEventType::VpnLocationRemoved { location } => (
                LoggerEvent::Defguard(Box::new(DefguardEvent::VpnLocationRemoved {
                    location: location.clone(),
                })),
                Some(location),
            ),
            ApiEventType::VpnLocationModified { before, after } => (
                LoggerEvent::Defguard(Box::new(DefguardEvent::VpnLocationModified {
                    before,
                    after: after.clone(),
                })),
                Some(after),
            ),
            ApiEventType::ApiTokenAdded { owner, token } => (
                LoggerEvent::Defguard(Box::new(DefguardEvent::ApiTokenAdded { owner, token })),
                None,
            ),
            ApiEventType::ApiTokenRemoved { owner, token } => (
                LoggerEvent::Defguard(Box::new(DefguardEvent::ApiTokenRemoved { owner, token })),
                None,
            ),
            ApiEventType::ApiTokenRenamed {
                owner,
                token,
                old_name,
                new_name,
            } => (
                LoggerEvent::Defguard(Box::new(DefguardEvent::ApiTokenRenamed {
                    owner,
                    token,
                    old_name,
                    new_name,
                })),
                None,
            ),
            ApiEventType::OpenIdAppAdded { app } => (
                LoggerEvent::Defguard(Box::new(DefguardEvent::OpenIdAppAdded { app })),
                None,
            ),
            ApiEventType::OpenIdAppRemoved { app } => (
                LoggerEvent::Defguard(Box::new(DefguardEvent::OpenIdAppRemoved { app })),
                None,
            ),
            ApiEventType::OpenIdAppModified { before, after } => (
                LoggerEvent::Defguard(Box::new(DefguardEvent::OpenIdAppModified { before, after })),
                None,
            ),
            ApiEventType::OpenIdAppStateChanged { app, enabled } => (
                LoggerEvent::Defguard(Box::new(DefguardEvent::OpenIdAppStateChanged {
                    app,
                    enabled,
                })),
                None,
            ),
            ApiEventType::OpenIdProviderRemoved { provider } => (
                LoggerEvent::Defguard(Box::new(DefguardEvent::OpenIdProviderRemoved { provider })),
                None,
            ),
            ApiEventType::OpenIdProviderModified { provider } => (
                LoggerEvent::Defguard(Box::new(DefguardEvent::OpenIdProviderModified { provider })),
                None,
            ),
            ApiEventType::SettingsUpdated { before, after } => (
                LoggerEvent::Defguard(Box::new(DefguardEvent::SettingsUpdated { before, after })),
                None,
            ),
            ApiEventType::SettingsUpdatedPartial { before, after } => (
                LoggerEvent::Defguard(Box::new(DefguardEvent::SettingsUpdatedPartial {
                    before,
                    after,
                })),
                None,
            ),
            ApiEventType::SettingsDefaultBrandingRestored => (
                LoggerEvent::Defguard(Box::new(DefguardEvent::SettingsDefaultBrandingRestored)),
                None,
            ),
            ApiEventType::GroupsBulkAssigned { users, groups } => (
                LoggerEvent::Defguard(Box::new(DefguardEvent::GroupsBulkAssigned {
                    users,
                    groups,
                })),
                None,
            ),
            ApiEventType::GroupAdded { group } => (
                LoggerEvent::Defguard(Box::new(DefguardEvent::GroupAdded { group })),
                None,
            ),
            ApiEventType::GroupModified { before, after } => (
                LoggerEvent::Defguard(Box::new(DefguardEvent::GroupModified { before, after })),
                None,
            ),
            ApiEventType::GroupRemoved { group } => (
                LoggerEvent::Defguard(Box::new(DefguardEvent::GroupRemoved { group })),
                None,
            ),
            ApiEventType::GroupMemberAdded { group, user } => (
                LoggerEvent::Defguard(Box::new(DefguardEvent::GroupMemberAdded { group, user })),
                None,
            ),
            ApiEventType::GroupMemberRemoved { group, user } => (
                LoggerEvent::Defguard(Box::new(DefguardEvent::GroupMemberRemoved { group, user })),
                None,
            ),
            ApiEventType::GroupMembersModified {
                group,
                added,
                removed,
            } => (
                LoggerEvent::Defguard(Box::new(DefguardEvent::GroupMembersModified {
                    group,
                    added,
                    removed,
                })),
                None,
            ),
            ApiEventType::WebHookAdded { webhook } => (
                LoggerEvent::Defguard(Box::new(DefguardEvent::WebHookAdded { webhook })),
                None,
            ),
            ApiEventType::WebHookModified { before, after } => (
                LoggerEvent::Defguard(Box::new(DefguardEvent::WebHookModified { before, after })),
                None,
            ),
            ApiEventType::WebHookRemoved { webhook } => (
                LoggerEvent::Defguard(Box::new(DefguardEvent::WebHookRemoved { webhook })),
                None,
            ),
            ApiEventType::WebHookStateChanged { webhook, enabled } => (
                LoggerEvent::Defguard(Box::new(DefguardEvent::WebHookStateChanged {
                    webhook,
                    enabled,
                })),
                None,
            ),
            ApiEventType::AuthenticationKeyAdded { key } => (
                LoggerEvent::Defguard(Box::new(DefguardEvent::AuthenticationKeyAdded { key })),
                None,
            ),
            ApiEventType::AuthenticationKeyRemoved { key } => (
                LoggerEvent::Defguard(Box::new(DefguardEvent::AuthenticationKeyRemoved { key })),
                None,
            ),
            ApiEventType::AuthenticationKeyRenamed {
                key,
                old_name,
                new_name,
            } => (
                LoggerEvent::Defguard(Box::new(DefguardEvent::AuthenticationKeyRenamed {
                    key,
                    old_name,
                    new_name,
                })),
                None,
            ),
            ApiEventType::EnrollmentTokenAdded { user } => (
                LoggerEvent::Enrollment(Box::new(EnrollmentEvent::TokenAdded { user })),
                None,
            ),
            ApiEventType::PasswordChanged => (
                LoggerEvent::Defguard(Box::new(DefguardEvent::PasswordChanged)),
                None,
            ),
            ApiEventType::PasswordChangedByAdmin { user } => (
                LoggerEvent::Defguard(Box::new(DefguardEvent::PasswordChangedByAdmin { user })),
                None,
            ),
            ApiEventType::PasswordReset { user } => (
                LoggerEvent::Defguard(Box::new(DefguardEvent::PasswordReset { user })),
                None,
            ),
            ApiEventType::ClientConfigurationTokenAdded { user } => (
                LoggerEvent::Defguard(Box::new(DefguardEvent::ClientConfigurationTokenAdded {
                    user,
                })),
                None,
            ),
            ApiEventType::UserSnatBindingAdded {
                user,
                location,
                binding,
            } => (
                LoggerEvent::Defguard(Box::new(DefguardEvent::UserSnatBindingAdded {
                    user,
                    binding,
                })),
                Some(location),
            ),
            ApiEventType::UserSnatBindingRemoved {
                user,
                location,
                binding,
            } => (
                LoggerEvent::Defguard(Box::new(DefguardEvent::UserSnatBindingRemoved {
                    user,
                    binding,
                })),
                Some(location),
            ),
            ApiEventType::UserSnatBindingModified {
                user,
                location,
                before,
                after,
            } => (
                LoggerEvent::Defguard(Box::new(DefguardEvent::UserSnatBindingModified {
                    user,
                    before,
                    after,
                })),
                Some(location),
            ),
            ApiEventType::ProxyModified { before, after } => (
                LoggerEvent::Defguard(Box::new(DefguardEvent::ProxyModified { before, after })),
                None,
            ),
            ApiEventType::ProxyDeleted { proxy } => (
                LoggerEvent::Defguard(Box::new(DefguardEvent::ProxyDeleted { proxy })),
                None,
            ),
            ApiEventType::GatewayModified { before, after } => (
                LoggerEvent::Defguard(Box::new(DefguardEvent::GatewayModified { before, after })),
                None,
            ),
            ApiEventType::GatewayDeleted { gateway } => (
                LoggerEvent::Defguard(Box::new(DefguardEvent::GatewayDeleted { gateway })),
                None,
            ),
            ApiEventType::DevicePostureCreated { snapshot } => (
                LoggerEvent::Defguard(Box::new(DefguardEvent::DevicePostureCreated { snapshot })),
                None,
            ),
            ApiEventType::DevicePostureUpdated { before, after } => (
                LoggerEvent::Defguard(Box::new(DefguardEvent::DevicePostureUpdated {
                    before,
                    after,
                })),
                None,
            ),
            ApiEventType::DevicePostureDeleted { snapshot } => (
                LoggerEvent::Defguard(Box::new(DefguardEvent::DevicePostureDeleted { snapshot })),
                None,
            ),
            ApiEventType::DevicePostureDuplicated {
                original,
                duplicate,
            } => (
                LoggerEvent::Defguard(Box::new(DefguardEvent::DevicePostureDuplicated {
                    original,
                    duplicate,
                })),
                None,
            ),
            ApiEventType::DevicePostureLocationsAssigned {
                device_posture,
                location_ids,
            } => (
                LoggerEvent::Defguard(Box::new(DefguardEvent::DevicePostureLocationsAssigned {
                    posture_id: device_posture.id,
                    location_ids,
                })),
                None,
            ),
            ApiEventType::LocationPosturesAssigned {
                location,
                posture_ids,
            } => (
                LoggerEvent::Defguard(Box::new(DefguardEvent::LocationPosturesAssigned {
                    location_id: location.id,
                    posture_ids,
                })),
                None,
            ),
        };

        EventLoggerMessage::new(
            EventContext::from_api_context(api_event.context, location),
            logger_event,
        )
    }

    pub fn from_bidi_event(bidi_event: BidiStreamEvent) -> Self {
        debug!("Processing bidi gRPC stream event: {bidi_event:?}");
        let BidiStreamEvent { context, event } = bidi_event;

        let (logger_event, location) = match event {
            BidiStreamEventType::Enrollment(event) => match *event {
                defguard_core::events::EnrollmentEvent::EnrollmentStarted => (
                    LoggerEvent::Enrollment(Box::new(EnrollmentEvent::EnrollmentStarted)),
                    None,
                ),
                defguard_core::events::EnrollmentEvent::EnrollmentCompleted => (
                    LoggerEvent::Enrollment(Box::new(EnrollmentEvent::EnrollmentCompleted)),
                    None,
                ),
                defguard_core::events::EnrollmentEvent::EnrollmentDeviceAdded { device } => (
                    LoggerEvent::Enrollment(Box::new(EnrollmentEvent::EnrollmentDeviceAdded {
                        device,
                    })),
                    None,
                ),
            },
            BidiStreamEventType::PasswordReset(event) => match *event {
                PasswordResetEvent::PasswordResetRequested => (
                    LoggerEvent::Enrollment(Box::new(EnrollmentEvent::PasswordResetRequested)),
                    None,
                ),
                PasswordResetEvent::PasswordResetStarted => (
                    LoggerEvent::Enrollment(Box::new(EnrollmentEvent::PasswordResetStarted)),
                    None,
                ),
                PasswordResetEvent::PasswordResetCompleted => (
                    LoggerEvent::Enrollment(Box::new(EnrollmentEvent::PasswordResetCompleted)),
                    None,
                ),
            },
            BidiStreamEventType::DesktopClientMfa(event) => match *event {
                DesktopClientMfaEvent::Success {
                    location,
                    device,
                    method,
                } => (
                    LoggerEvent::Vpn(Box::new(VpnEvent::ClientMfaSuccess {
                        location: location.clone(),
                        device,
                        method,
                    })),
                    Some(location),
                ),
                DesktopClientMfaEvent::Failed {
                    location,
                    device,
                    method,
                    message,
                } => (
                    LoggerEvent::Vpn(Box::new(VpnEvent::ClientMfaFailed {
                        location: location.clone(),
                        device,
                        method,
                        message,
                    })),
                    Some(location),
                ),
                DesktopClientMfaEvent::Disconnected {
                    location,
                    device,
                    is_mfa_session,
                } => {
                    let vpn_event = if is_mfa_session {
                        VpnEvent::MfaDisconnectedFromLocation {
                            location: location.clone(),
                            device,
                        }
                    } else {
                        VpnEvent::DisconnectedFromLocation {
                            location: location.clone(),
                            device,
                        }
                    };

                    (LoggerEvent::Vpn(Box::new(vpn_event)), Some(location))
                }
            },
        };

        EventLoggerMessage::new(
            EventContext::from_bidi_context(context, location),
            logger_event,
        )
    }

    pub fn from_session_manager_event(session_event: SessionManagerEvent) -> Self {
        debug!("Processing session manager event: {session_event:?}");

        let SessionManagerEvent { context, event } = session_event;

        let location = context.location.clone();
        let device = context.device.clone();

        let logger_event = match event {
            SessionManagerEventType::ClientConnected => {
                LoggerEvent::Vpn(Box::new(VpnEvent::ConnectedToLocation { location, device }))
            }
            SessionManagerEventType::ClientDisconnected => {
                LoggerEvent::Vpn(Box::new(VpnEvent::DisconnectedFromLocation {
                    location,
                    device,
                }))
            }
            SessionManagerEventType::MfaClientConnected => {
                LoggerEvent::Vpn(Box::new(VpnEvent::MfaConnectedToLocation {
                    location,
                    device,
                }))
            }
            SessionManagerEventType::MfaClientDisconnected => {
                LoggerEvent::Vpn(Box::new(VpnEvent::MfaDisconnectedFromLocation {
                    location,
                    device,
                }))
            }
        };

        EventLoggerMessage::new(
            EventContext::from_session_manager_context(context),
            logger_event,
        )
    }
}
