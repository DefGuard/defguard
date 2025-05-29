use defguard_core::events::{ApiEvent, ApiEventType};
use defguard_event_logger::message::{DefguardEvent, LoggerEvent};
use tracing::debug;

use crate::{error::EventRouterError, EventRouter};

impl EventRouter {
    pub(crate) fn handle_api_event(&self, event: ApiEvent) -> Result<(), EventRouterError> {
        debug!("Processing API event: {event:?}");

        match event.kind {
            ApiEventType::UserLogin => {
                // send event to audit log
                self.log_event(
                    event.context,
                    LoggerEvent::Defguard(DefguardEvent::UserLogin),
                )?;
            }
            ApiEventType::UserLogout => {
                self.log_event(
                    event.context,
                    LoggerEvent::Defguard(DefguardEvent::UserLogout),
                )?;
            }
            ApiEventType::UserAdded { username } => {
                self.log_event(
                    event.context,
                    LoggerEvent::Defguard(DefguardEvent::UserAdded { username }),
                )?;
            }
            ApiEventType::UserRemoved { username } => {
                self.log_event(
                    event.context,
                    LoggerEvent::Defguard(DefguardEvent::UserRemoved { username }),
                )?;
            }
            ApiEventType::UserModified { username } => {
                self.log_event(
                    event.context,
                    LoggerEvent::Defguard(DefguardEvent::UserModified { username }),
                )?;
            }
            ApiEventType::UserDeviceAdded {
                owner,
                device_id,
                device_name,
            } => {
                self.log_event(
                    event.context,
                    LoggerEvent::Defguard(DefguardEvent::UserDeviceAdded {
                        device_name,
                        device_id,
                        owner,
                    }),
                )?;
            }
            ApiEventType::UserDeviceRemoved {
                owner,
                device_id,
                device_name,
            } => {
                self.log_event(
                    event.context,
                    LoggerEvent::Defguard(DefguardEvent::UserDeviceRemoved {
                        device_name,
                        device_id,
                        owner,
                    }),
                )?;
            }
            ApiEventType::UserDeviceModified {
                owner,
                device_id,
                device_name,
            } => {
                self.log_event(
                    event.context,
                    LoggerEvent::Defguard(DefguardEvent::UserDeviceModified {
                        device_name,
                        device_id,
                        owner,
                    }),
                )?;
            }
        }

        Ok(())
    }
}
