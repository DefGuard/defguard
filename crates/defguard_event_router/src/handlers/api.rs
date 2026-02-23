use defguard_core::events::{ApiEvent, ApiEventType};
use defguard_event_logger::message::{DefguardEvent, EnrollmentEvent, EventContext, LoggerEvent};
use tracing::debug;

use crate::{EventRouter, error::EventRouterError};

impl EventRouter {
    pub(crate) fn handle_api_event(&self, event: ApiEvent) -> Result<(), EventRouterError> {
        debug!("Processing API event: {event:?}");
        let (logger_event, location) = match *event.event {
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
                // Notify stream manager about configuration changes
                self.activity_log_stream_reload_notify.notify_waiters();
                (
                    LoggerEvent::Defguard(Box::new(DefguardEvent::ActivityLogStreamCreated {
                        stream,
                    })),
                    None,
                )
            }
            ApiEventType::ActivityLogStreamModified { before, after } => {
                // Notify stream manager about configuration changes
                self.activity_log_stream_reload_notify.notify_waiters();
                (
                    LoggerEvent::Defguard(Box::new(DefguardEvent::ActivityLogStreamModified {
                        before,
                        after,
                    })),
                    None,
                )
            }
            ApiEventType::ActivityLogStreamRemoved { stream } => {
                // Notify stream manager about configuration changes
                self.activity_log_stream_reload_notify.notify_waiters();
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
        };
        self.log_event(
            EventContext::from_api_context(event.context, location),
            logger_event,
        )
    }
}
