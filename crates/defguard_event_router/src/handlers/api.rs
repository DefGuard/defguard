use defguard_core::events::{ApiEvent, ApiEventType};
use defguard_event_logger::message::{DefguardEvent, LoggerEvent};
use tracing::debug;

use crate::{error::EventRouterError, EventRouter};

impl EventRouter {
    pub(crate) fn handle_api_event(&self, event: ApiEvent) -> Result<(), EventRouterError> {
        debug!("Processing API event: {event:?}");
        let logger_event = match event.kind {
            ApiEventType::UserLogin => LoggerEvent::Defguard(DefguardEvent::UserLogin),
            ApiEventType::UserLoginFailed => LoggerEvent::Defguard(DefguardEvent::UserLoginFailed),
            ApiEventType::UserMfaLogin { mfa_method } => {
                LoggerEvent::Defguard(DefguardEvent::UserMfaLogin { mfa_method })
            }
            ApiEventType::UserMfaLoginFailed { mfa_method } => {
                LoggerEvent::Defguard(DefguardEvent::UserMfaLoginFailed { mfa_method })
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
        };
        self.log_event(event.context.into(), logger_event)
    }
}
