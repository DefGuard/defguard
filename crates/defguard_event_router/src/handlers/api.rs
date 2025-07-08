use defguard_core::events::{ApiEvent, ApiEventType};
use defguard_event_logger::message::{DefguardEvent, EnrollmentEvent, LoggerEvent};
use tracing::debug;

use crate::{EventRouter, error::EventRouterError};

impl EventRouter {
    pub(crate) fn handle_api_event(&self, event: ApiEvent) -> Result<(), EventRouterError> {
        debug!("Processing API event: {event:?}");
        let logger_event = match *event.event {
            ApiEventType::UserLogin => LoggerEvent::Defguard(Box::new(DefguardEvent::UserLogin)),
            ApiEventType::UserLoginFailed { message } => {
                LoggerEvent::Defguard(Box::new(DefguardEvent::UserLoginFailed { message }))
            }
            ApiEventType::UserMfaLogin { mfa_method } => {
                LoggerEvent::Defguard(Box::new(DefguardEvent::UserMfaLogin { mfa_method }))
            }
            ApiEventType::UserMfaLoginFailed {
                mfa_method,
                message,
            } => LoggerEvent::Defguard(Box::new(DefguardEvent::UserMfaLoginFailed {
                mfa_method,
                message,
            })),
            ApiEventType::RecoveryCodeUsed => {
                LoggerEvent::Defguard(Box::new(DefguardEvent::RecoveryCodeUsed))
            }
            ApiEventType::UserLogout => LoggerEvent::Defguard(Box::new(DefguardEvent::UserLogout)),
            ApiEventType::UserAdded { user } => {
                LoggerEvent::Defguard(Box::new(DefguardEvent::UserAdded { user }))
            }
            ApiEventType::UserRemoved { user } => {
                LoggerEvent::Defguard(Box::new(DefguardEvent::UserRemoved { user }))
            }
            ApiEventType::UserModified { before, after } => {
                LoggerEvent::Defguard(Box::new(DefguardEvent::UserModified { before, after }))
            }
            ApiEventType::MfaDisabled => {
                LoggerEvent::Defguard(Box::new(DefguardEvent::MfaDisabled))
            }
            ApiEventType::UserMfaDisabled { user } => {
                LoggerEvent::Defguard(Box::new(DefguardEvent::UserMfaDisabled { user }))
            }
            ApiEventType::MfaTotpDisabled => {
                LoggerEvent::Defguard(Box::new(DefguardEvent::MfaTotpDisabled))
            }
            ApiEventType::MfaTotpEnabled => {
                LoggerEvent::Defguard(Box::new(DefguardEvent::MfaTotpEnabled))
            }
            ApiEventType::MfaEmailDisabled => {
                LoggerEvent::Defguard(Box::new(DefguardEvent::MfaEmailDisabled))
            }
            ApiEventType::MfaEmailEnabled => {
                LoggerEvent::Defguard(Box::new(DefguardEvent::MfaEmailEnabled))
            }
            ApiEventType::MfaSecurityKeyAdded { key } => {
                LoggerEvent::Defguard(Box::new(DefguardEvent::MfaSecurityKeyAdded { key }))
            }
            ApiEventType::MfaSecurityKeyRemoved { key } => {
                LoggerEvent::Defguard(Box::new(DefguardEvent::MfaSecurityKeyRemoved { key }))
            }
            ApiEventType::UserDeviceAdded { owner, device } => {
                LoggerEvent::Defguard(Box::new(DefguardEvent::UserDeviceAdded { device, owner }))
            }
            ApiEventType::UserDeviceRemoved { owner, device } => {
                LoggerEvent::Defguard(Box::new(DefguardEvent::UserDeviceRemoved { device, owner }))
            }
            ApiEventType::UserDeviceModified {
                owner,
                before,
                after,
            } => LoggerEvent::Defguard(Box::new(DefguardEvent::UserDeviceModified {
                owner,
                before,
                after,
            })),
            ApiEventType::NetworkDeviceAdded { device, location } => {
                LoggerEvent::Defguard(Box::new(DefguardEvent::NetworkDeviceAdded {
                    device,
                    location,
                }))
            }
            ApiEventType::NetworkDeviceModified {
                before,
                after,
                location,
            } => LoggerEvent::Defguard(Box::new(DefguardEvent::NetworkDeviceModified {
                before,
                after,
                location,
            })),
            ApiEventType::NetworkDeviceRemoved { device, location } => {
                LoggerEvent::Defguard(Box::new(DefguardEvent::NetworkDeviceRemoved {
                    device,
                    location,
                }))
            }
            ApiEventType::ActivityLogStreamCreated { stream } => {
                // Notify stream manager about configuration changes
                self.activity_log_stream_reload_notify.notify_waiters();
                LoggerEvent::Defguard(Box::new(DefguardEvent::ActivityLogStreamCreated { stream }))
            }
            ApiEventType::ActivityLogStreamModified { before, after } => {
                // Notify stream manager about configuration changes
                self.activity_log_stream_reload_notify.notify_waiters();
                LoggerEvent::Defguard(Box::new(DefguardEvent::ActivityLogStreamModified {
                    before,
                    after,
                }))
            }
            ApiEventType::ActivityLogStreamRemoved { stream } => {
                // Notify stream manager about configuration changes
                self.activity_log_stream_reload_notify.notify_waiters();
                LoggerEvent::Defguard(Box::new(DefguardEvent::ActivityLogStreamRemoved { stream }))
            }
            ApiEventType::VpnLocationAdded { location } => {
                LoggerEvent::Defguard(Box::new(DefguardEvent::VpnLocationAdded { location }))
            }
            ApiEventType::VpnLocationRemoved { location } => {
                LoggerEvent::Defguard(Box::new(DefguardEvent::VpnLocationRemoved { location }))
            }
            ApiEventType::VpnLocationModified { before, after } => {
                LoggerEvent::Defguard(Box::new(DefguardEvent::VpnLocationModified {
                    before,
                    after,
                }))
            }
            ApiEventType::ApiTokenAdded { owner, token } => {
                LoggerEvent::Defguard(Box::new(DefguardEvent::ApiTokenAdded { owner, token }))
            }
            ApiEventType::ApiTokenRemoved { owner, token } => {
                LoggerEvent::Defguard(Box::new(DefguardEvent::ApiTokenRemoved { owner, token }))
            }
            ApiEventType::ApiTokenRenamed {
                owner,
                token,
                old_name,
                new_name,
            } => LoggerEvent::Defguard(Box::new(DefguardEvent::ApiTokenRenamed {
                owner,
                token,
                old_name,
                new_name,
            })),
            ApiEventType::OpenIdAppAdded { app } => {
                LoggerEvent::Defguard(Box::new(DefguardEvent::OpenIdAppAdded { app }))
            }
            ApiEventType::OpenIdAppRemoved { app } => {
                LoggerEvent::Defguard(Box::new(DefguardEvent::OpenIdAppRemoved { app }))
            }
            ApiEventType::OpenIdAppModified { before, after } => {
                LoggerEvent::Defguard(Box::new(DefguardEvent::OpenIdAppModified { before, after }))
            }
            ApiEventType::OpenIdAppStateChanged { app, enabled } => {
                LoggerEvent::Defguard(Box::new(DefguardEvent::OpenIdAppStateChanged {
                    app,
                    enabled,
                }))
            }
            ApiEventType::OpenIdProviderRemoved { provider } => {
                LoggerEvent::Defguard(Box::new(DefguardEvent::OpenIdProviderRemoved { provider }))
            }
            ApiEventType::OpenIdProviderModified { provider } => {
                LoggerEvent::Defguard(Box::new(DefguardEvent::OpenIdProviderModified { provider }))
            }
            ApiEventType::SettingsUpdated { before, after } => {
                LoggerEvent::Defguard(Box::new(DefguardEvent::SettingsUpdated { before, after }))
            }
            ApiEventType::SettingsUpdatedPartial { before, after } => {
                LoggerEvent::Defguard(Box::new(DefguardEvent::SettingsUpdatedPartial {
                    before,
                    after,
                }))
            }
            ApiEventType::SettingsDefaultBrandingRestored => {
                LoggerEvent::Defguard(Box::new(DefguardEvent::SettingsDefaultBrandingRestored))
            }
            ApiEventType::GroupsBulkAssigned { users, groups } => {
                LoggerEvent::Defguard(Box::new(DefguardEvent::GroupsBulkAssigned {
                    users,
                    groups,
                }))
            }
            ApiEventType::GroupAdded { group } => {
                LoggerEvent::Defguard(Box::new(DefguardEvent::GroupAdded { group }))
            }
            ApiEventType::GroupModified { before, after } => {
                LoggerEvent::Defguard(Box::new(DefguardEvent::GroupModified { before, after }))
            }
            ApiEventType::GroupRemoved { group } => {
                LoggerEvent::Defguard(Box::new(DefguardEvent::GroupRemoved { group }))
            }
            ApiEventType::GroupMemberAdded { group, user } => {
                LoggerEvent::Defguard(Box::new(DefguardEvent::GroupMemberAdded { group, user }))
            }
            ApiEventType::GroupMemberRemoved { group, user } => {
                LoggerEvent::Defguard(Box::new(DefguardEvent::GroupMemberRemoved { group, user }))
            }
            ApiEventType::WebHookAdded { webhook } => {
                LoggerEvent::Defguard(Box::new(DefguardEvent::WebHookAdded { webhook }))
            }
            ApiEventType::WebHookModified { before, after } => {
                LoggerEvent::Defguard(Box::new(DefguardEvent::WebHookModified { before, after }))
            }
            ApiEventType::WebHookRemoved { webhook } => {
                LoggerEvent::Defguard(Box::new(DefguardEvent::WebHookRemoved { webhook }))
            }
            ApiEventType::WebHookStateChanged { webhook, enabled } => {
                LoggerEvent::Defguard(Box::new(DefguardEvent::WebHookStateChanged {
                    webhook,
                    enabled,
                }))
            }
            ApiEventType::AuthenticationKeyAdded { key } => {
                LoggerEvent::Defguard(Box::new(DefguardEvent::AuthenticationKeyAdded { key }))
            }
            ApiEventType::AuthenticationKeyRemoved { key } => {
                LoggerEvent::Defguard(Box::new(DefguardEvent::AuthenticationKeyRemoved { key }))
            }
            ApiEventType::AuthenticationKeyRenamed {
                key,
                old_name,
                new_name,
            } => LoggerEvent::Defguard(Box::new(DefguardEvent::AuthenticationKeyRenamed {
                key,
                old_name,
                new_name,
            })),
            ApiEventType::EnrollmentTokenAdded { user } => {
                LoggerEvent::Enrollment(Box::new(EnrollmentEvent::TokenAdded { user }))
            }
            ApiEventType::PasswordChanged => {
                LoggerEvent::Defguard(Box::new(DefguardEvent::PasswordChanged))
            }
            ApiEventType::PasswordChangedByAdmin { user } => {
                LoggerEvent::Defguard(Box::new(DefguardEvent::PasswordChangedByAdmin { user }))
            }
            ApiEventType::PasswordReset { user } => {
                LoggerEvent::Defguard(Box::new(DefguardEvent::PasswordReset { user }))
            }
            ApiEventType::ClientConfigurationTokenAdded { user } => {
                LoggerEvent::Defguard(Box::new(DefguardEvent::ClientConfigurationTokenAdded {
                    user,
                }))
            }
            ApiEventType::UserSnatBindingAdded { user, binding } => {
                LoggerEvent::Defguard(Box::new(DefguardEvent::UserSnatBindingAdded {
                    user,
                    binding,
                }))
            }
            ApiEventType::UserSnatBindingRemoved { user, binding } => {
                LoggerEvent::Defguard(Box::new(DefguardEvent::UserSnatBindingRemoved {
                    user,
                    binding,
                }))
            }
            ApiEventType::UserSnatBindingModified {
                user,
                before,
                after,
            } => LoggerEvent::Defguard(Box::new(DefguardEvent::UserSnatBindingModified {
                user,
                before,
                after,
            })),
        };
        self.log_event(event.context.into(), logger_event)
    }
}
