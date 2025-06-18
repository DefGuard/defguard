use defguard_core::events::{ApiEvent, ApiEventType};
use defguard_event_logger::message::{DefguardEvent, EnrollmentEvent, LoggerEvent};
use tracing::debug;

use crate::{error::EventRouterError, EventRouter};

impl EventRouter {
    pub(crate) fn handle_api_event(&self, event: ApiEvent) -> Result<(), EventRouterError> {
        debug!("Processing API event");
        let logger_event = match event.event {
            ApiEventType::UserLogin => LoggerEvent::Defguard(DefguardEvent::UserLogin),
            ApiEventType::UserLoginFailed => LoggerEvent::Defguard(DefguardEvent::UserLoginFailed),
            ApiEventType::UserMfaLogin { mfa_method } => {
                LoggerEvent::Defguard(DefguardEvent::UserMfaLogin { mfa_method })
            }
            ApiEventType::UserMfaLoginFailed { mfa_method } => {
                LoggerEvent::Defguard(DefguardEvent::UserMfaLoginFailed { mfa_method })
            }
            ApiEventType::RecoveryCodeUsed => {
                LoggerEvent::Defguard(DefguardEvent::RecoveryCodeUsed)
            }
            ApiEventType::UserLogout => LoggerEvent::Defguard(DefguardEvent::UserLogout),
            ApiEventType::UserAdded { user } => {
                LoggerEvent::Defguard(DefguardEvent::UserAdded { user })
            }
            ApiEventType::UserRemoved { user } => {
                LoggerEvent::Defguard(DefguardEvent::UserRemoved { user })
            }
            ApiEventType::UserModified { before, after } => {
                LoggerEvent::Defguard(DefguardEvent::UserModified { before, after })
            }
            ApiEventType::MfaDisabled => LoggerEvent::Defguard(DefguardEvent::MfaDisabled),
            ApiEventType::MfaTotpDisabled => LoggerEvent::Defguard(DefguardEvent::MfaTotpDisabled),
            ApiEventType::MfaTotpEnabled => LoggerEvent::Defguard(DefguardEvent::MfaTotpEnabled),
            ApiEventType::MfaEmailDisabled => {
                LoggerEvent::Defguard(DefguardEvent::MfaEmailDisabled)
            }
            ApiEventType::MfaEmailEnabled => LoggerEvent::Defguard(DefguardEvent::MfaEmailEnabled),
            ApiEventType::MfaSecurityKeyAdded { key } => {
                LoggerEvent::Defguard(DefguardEvent::MfaSecurityKeyAdded { key })
            }
            ApiEventType::MfaSecurityKeyRemoved { key } => {
                LoggerEvent::Defguard(DefguardEvent::MfaSecurityKeyRemoved { key })
            }
            ApiEventType::UserDeviceAdded { owner, device } => {
                LoggerEvent::Defguard(DefguardEvent::UserDeviceAdded { device, owner })
            }
            ApiEventType::UserDeviceRemoved { owner, device } => {
                LoggerEvent::Defguard(DefguardEvent::UserDeviceRemoved { device, owner })
            }
            ApiEventType::UserDeviceModified {
                owner,
                before,
                after,
            } => LoggerEvent::Defguard(DefguardEvent::UserDeviceModified {
                owner,
                before,
                after,
            }),
            ApiEventType::NetworkDeviceAdded { device, location } => {
                LoggerEvent::Defguard(DefguardEvent::NetworkDeviceAdded { device, location })
            }
            ApiEventType::NetworkDeviceModified {
                before,
                after,
                location,
            } => LoggerEvent::Defguard(DefguardEvent::NetworkDeviceModified {
                before,
                after,
                location,
            }),
            ApiEventType::NetworkDeviceRemoved { device, location } => {
                LoggerEvent::Defguard(DefguardEvent::NetworkDeviceRemoved { device, location })
            }
            ApiEventType::AuditStreamCreated { stream } => {
                // Notify stream manager about configuration changes
                self.audit_stream_reload_notify.notify_waiters();
                LoggerEvent::Defguard(DefguardEvent::AuditStreamCreated { stream })
            }
            ApiEventType::AuditStreamModified { before, after } => {
                // Notify stream manager about configuration changes
                self.audit_stream_reload_notify.notify_waiters();
                LoggerEvent::Defguard(DefguardEvent::AuditStreamModified { before, after })
            }
            ApiEventType::AuditStreamRemoved { stream } => {
                // Notify stream manager about configuration changes
                self.audit_stream_reload_notify.notify_waiters();
                LoggerEvent::Defguard(DefguardEvent::AuditStreamRemoved { stream })
            }
            ApiEventType::VpnLocationAdded { location } => {
                LoggerEvent::Defguard(DefguardEvent::VpnLocationAdded { location })
            }
            ApiEventType::VpnLocationRemoved { location } => {
                LoggerEvent::Defguard(DefguardEvent::VpnLocationRemoved { location })
            }
            ApiEventType::VpnLocationModified { before, after } => {
                LoggerEvent::Defguard(DefguardEvent::VpnLocationModified { before, after })
            }
            ApiEventType::ApiTokenAdded { owner, token } => {
                LoggerEvent::Defguard(DefguardEvent::ApiTokenAdded { owner, token })
            }
            ApiEventType::ApiTokenRemoved { owner, token } => {
                LoggerEvent::Defguard(DefguardEvent::ApiTokenRemoved { owner, token })
            }
            ApiEventType::ApiTokenRenamed {
                owner,
                token,
                old_name,
                new_name,
            } => LoggerEvent::Defguard(DefguardEvent::ApiTokenRenamed {
                owner,
                token,
                old_name,
                new_name,
            }),
            ApiEventType::OpenIdAppAdded { app } => {
                LoggerEvent::Defguard(DefguardEvent::OpenIdAppAdded { app })
            }
            ApiEventType::OpenIdAppRemoved { app } => {
                LoggerEvent::Defguard(DefguardEvent::OpenIdAppRemoved { app })
            }
            ApiEventType::OpenIdAppModified { before, after } => {
                LoggerEvent::Defguard(DefguardEvent::OpenIdAppModified { before, after })
            }
            ApiEventType::OpenIdAppStateChanged { app, enabled } => {
                LoggerEvent::Defguard(DefguardEvent::OpenIdAppStateChanged { app, enabled })
            }
            ApiEventType::OpenIdProviderRemoved { provider } => {
                LoggerEvent::Defguard(DefguardEvent::OpenIdProviderRemoved { provider })
            }
            ApiEventType::OpenIdProviderModified { provider } => {
                LoggerEvent::Defguard(DefguardEvent::OpenIdProviderModified { provider })
            }
            ApiEventType::SettingsUpdated => LoggerEvent::Defguard(DefguardEvent::SettingsUpdated),
            ApiEventType::SettingsUpdatedPartial => {
                LoggerEvent::Defguard(DefguardEvent::SettingsUpdatedPartial)
            }
            ApiEventType::SettingsDefaultBrandingRestored => {
                LoggerEvent::Defguard(DefguardEvent::SettingsDefaultBrandingRestored)
            }
            ApiEventType::GroupsBulkAssigned { users, groups } => {
                LoggerEvent::Defguard(DefguardEvent::GroupsBulkAssigned { users, groups })
            }
            ApiEventType::GroupAdded { group } => {
                LoggerEvent::Defguard(DefguardEvent::GroupAdded { group })
            }
            ApiEventType::GroupModified { before, after } => {
                LoggerEvent::Defguard(DefguardEvent::GroupModified { before, after })
            }
            ApiEventType::GroupRemoved { group } => {
                LoggerEvent::Defguard(DefguardEvent::GroupRemoved { group })
            }
            ApiEventType::GroupMemberAdded { group, user } => {
                LoggerEvent::Defguard(DefguardEvent::GroupMemberAdded { group, user })
            }
            ApiEventType::GroupMemberRemoved { group, user } => {
                LoggerEvent::Defguard(DefguardEvent::GroupMemberRemoved { group, user })
            }
            ApiEventType::WebHookAdded { webhook } => {
                LoggerEvent::Defguard(DefguardEvent::WebHookAdded { webhook })
            }
            ApiEventType::WebHookModified { before, after } => {
                LoggerEvent::Defguard(DefguardEvent::WebHookModified { before, after })
            }
            ApiEventType::WebHookRemoved { webhook } => {
                LoggerEvent::Defguard(DefguardEvent::WebHookRemoved { webhook })
            }
            ApiEventType::WebHookStateChanged { webhook, enabled } => {
                LoggerEvent::Defguard(DefguardEvent::WebHookStateChanged { webhook, enabled })
            }
            ApiEventType::AuthenticationKeyAdded { key } => {
                LoggerEvent::Defguard(DefguardEvent::AuthenticationKeyAdded { key })
            }
            ApiEventType::AuthenticationKeyRemoved { key } => {
                LoggerEvent::Defguard(DefguardEvent::AuthenticationKeyRemoved { key })
            }
            ApiEventType::AuthenticationKeyRenamed {
                key,
                old_name,
                new_name,
            } => LoggerEvent::Defguard(DefguardEvent::AuthenticationKeyRenamed {
                key,
                old_name,
                new_name,
            }),
            ApiEventType::EnrollmentTokenAdded { user } => {
                LoggerEvent::Enrollment(EnrollmentEvent::TokenAdded { user })
            }
            ApiEventType::PasswordChanged => LoggerEvent::Defguard(DefguardEvent::PasswordChanged),
            ApiEventType::PasswordChangedByAdmin { user } => {
                LoggerEvent::Defguard(DefguardEvent::PasswordChangedByAdmin { user })
            }
            ApiEventType::PasswordReset { user } => {
                LoggerEvent::Defguard(DefguardEvent::PasswordReset { user })
            }
            ApiEventType::ClientConfigurationTokenAdded { user } => {
                LoggerEvent::Defguard(DefguardEvent::ClientConfigurationTokenAdded { user })
            }
        };
        self.log_event(event.context.into(), logger_event)
    }
}
