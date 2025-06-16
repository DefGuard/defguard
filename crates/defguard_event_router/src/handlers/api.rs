use defguard_core::events::{ApiEvent, ApiEventType};
use defguard_event_logger::message::{DefguardEvent, LoggerEvent};
use tracing::debug;

use crate::{error::EventRouterError, EventRouter};

impl EventRouter {
    pub(crate) fn handle_api_event(&self, event: ApiEvent) -> Result<(), EventRouterError> {
        debug!("Processing API event: {event:?}");
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
            ApiEventType::UserAdded { username } => {
                LoggerEvent::Defguard(DefguardEvent::UserAdded { username })
            }
            ApiEventType::UserRemoved { username } => {
                LoggerEvent::Defguard(DefguardEvent::UserRemoved { username })
            }
            ApiEventType::UserModified { username } => {
                LoggerEvent::Defguard(DefguardEvent::UserModified { username })
            }
            ApiEventType::MfaDisabled => LoggerEvent::Defguard(DefguardEvent::MfaDisabled),
            ApiEventType::MfaTotpDisabled => LoggerEvent::Defguard(DefguardEvent::MfaTotpDisabled),
            ApiEventType::MfaTotpEnabled => LoggerEvent::Defguard(DefguardEvent::MfaTotpEnabled),
            ApiEventType::MfaEmailDisabled => {
                LoggerEvent::Defguard(DefguardEvent::MfaEmailDisabled)
            }
            ApiEventType::MfaEmailEnabled => LoggerEvent::Defguard(DefguardEvent::MfaEmailEnabled),
            ApiEventType::MfaSecurityKeyAdded { key_id, key_name } => {
                LoggerEvent::Defguard(DefguardEvent::MfaSecurityKeyAdded { key_id, key_name })
            }
            ApiEventType::MfaSecurityKeyRemoved { key_id, key_name } => {
                LoggerEvent::Defguard(DefguardEvent::MfaSecurityKeyRemoved { key_id, key_name })
            }
            ApiEventType::UserDeviceAdded {
                owner,
                device_id,
                device_name,
            } => LoggerEvent::Defguard(DefguardEvent::UserDeviceAdded {
                device_name,
                device_id,
                owner,
            }),
            ApiEventType::UserDeviceRemoved {
                owner,
                device_id,
                device_name,
            } => LoggerEvent::Defguard(DefguardEvent::UserDeviceRemoved {
                device_name,
                device_id,
                owner,
            }),
            ApiEventType::UserDeviceModified {
                owner,
                device_id,
                device_name,
            } => LoggerEvent::Defguard(DefguardEvent::UserDeviceModified {
                device_name,
                device_id,
                owner,
            }),
            ApiEventType::NetworkDeviceAdded {
                device_id,
                device_name,
                location_id,
                location,
            } => LoggerEvent::Defguard(DefguardEvent::NetworkDeviceAdded {
                device_id,
                device_name,
                location_id,
                location,
            }),
            ApiEventType::NetworkDeviceModified {
                device_id,
                device_name,
                location_id,
                location,
            } => LoggerEvent::Defguard(DefguardEvent::NetworkDeviceModified {
                device_id,
                device_name,
                location_id,
                location,
            }),
            ApiEventType::NetworkDeviceRemoved {
                device_id,
                device_name,
                location_id,
                location,
            } => LoggerEvent::Defguard(DefguardEvent::NetworkDeviceRemoved {
                device_id,
                device_name,
                location_id,
                location,
            }),
            ApiEventType::AuditStreamCreated {
                stream_id,
                stream_name,
            } => {
                // Notify stream manager about configuration changes
                self.audit_stream_reload_notify.notify_waiters();
                LoggerEvent::Defguard(DefguardEvent::AuditStreamCreated {
                    stream_id,
                    stream_name,
                })
            }
            ApiEventType::AuditStreamModified {
                stream_id,
                stream_name,
            } => {
                // Notify stream manager about configuration changes
                self.audit_stream_reload_notify.notify_waiters();
                LoggerEvent::Defguard(DefguardEvent::AuditStreamModified {
                    stream_id,
                    stream_name,
                })
            }
            ApiEventType::AuditStreamRemoved {
                stream_id,
                stream_name,
            } => {
                // Notify stream manager about configuration changes
                self.audit_stream_reload_notify.notify_waiters();
                LoggerEvent::Defguard(DefguardEvent::AuditStreamRemoved {
                    stream_id,
                    stream_name,
                })
            }
            ApiEventType::VpnLocationAdded { location } => {
                LoggerEvent::Defguard(DefguardEvent::VpnLocationAdded { location })
            }
            ApiEventType::VpnLocationRemoved { location } => {
                LoggerEvent::Defguard(DefguardEvent::VpnLocationRemoved { location })
            }
            ApiEventType::VpnLocationModified { location } => {
                LoggerEvent::Defguard(DefguardEvent::VpnLocationModified { location })
            }
            ApiEventType::ApiTokenAdded { owner, token_name } => {
                LoggerEvent::Defguard(DefguardEvent::ApiTokenAdded { owner, token_name })
            }
            ApiEventType::ApiTokenRemoved { owner, token_name } => {
                LoggerEvent::Defguard(DefguardEvent::ApiTokenRemoved { owner, token_name })
            }
            ApiEventType::ApiTokenRenamed {
                owner,
                old_name,
                new_name,
            } => LoggerEvent::Defguard(DefguardEvent::ApiTokenRenamed {
                owner,
                old_name,
                new_name,
            }),
            ApiEventType::OpenIdAppAdded { app_id, app_name } => {
                LoggerEvent::Defguard(DefguardEvent::OpenIdAppAdded { app_id, app_name })
            }
            ApiEventType::OpenIdAppRemoved { app_id, app_name } => {
                LoggerEvent::Defguard(DefguardEvent::OpenIdAppRemoved { app_id, app_name })
            }
            ApiEventType::OpenIdAppModified { app_id, app_name } => {
                LoggerEvent::Defguard(DefguardEvent::OpenIdAppModified { app_id, app_name })
            }
            ApiEventType::OpenIdAppStateChanged {
                app_id,
                app_name,
                enabled,
            } => LoggerEvent::Defguard(DefguardEvent::OpenIdAppStateChanged {
                app_id,
                app_name,
                enabled,
            }),
            ApiEventType::OpenIdProviderRemoved {
                provider_id,
                provider_name,
            } => LoggerEvent::Defguard(DefguardEvent::OpenIdProviderRemoved {
                provider_id,
                provider_name,
            }),
            ApiEventType::OpenIdProviderModified {
                provider_id,
                provider_name,
            } => LoggerEvent::Defguard(DefguardEvent::OpenIdProviderModified {
                provider_id,
                provider_name,
            }),
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
            ApiEventType::GroupModified { group } => {
                LoggerEvent::Defguard(DefguardEvent::GroupModified { group })
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
            ApiEventType::WebHookModified { webhook } => {
                LoggerEvent::Defguard(DefguardEvent::WebHookModified { webhook })
            }
            ApiEventType::WebHookRemoved { webhook } => {
                LoggerEvent::Defguard(DefguardEvent::WebHookRemoved { webhook })
            }
            ApiEventType::WebHookStateChanged { webhook, enabled } => {
                LoggerEvent::Defguard(DefguardEvent::WebHookStateChanged { webhook, enabled })
            }
        };
        self.log_event(event.context.into(), logger_event)
    }
}
