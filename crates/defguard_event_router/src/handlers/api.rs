use defguard_core::events::{ApiEvent, ApiEventKind};
use defguard_event_logger::message::{DefguardEvent, LoggerEvent};
use tracing::debug;

use crate::{error::EventRouterError, EventRouter};

impl EventRouter {
    pub(crate) fn handle_api_event(&self, event: ApiEvent) -> Result<(), EventRouterError> {
        debug!("Processing API event: {event:?}");

        match event.kind {
            ApiEventKind::UserLogin => {
                // send event to audit log
                self.log_event(event.context, LoggerEvent::Defguard(DefguardEvent::UserLogin))?;
            }
            ApiEventKind::UserLogout => {
                self.log_event(event.context, LoggerEvent::Defguard(DefguardEvent::UserLogout))?;
            }
            ApiEventKind::UserDeviceAdded {
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
            ApiEventKind::UserDeviceRemoved {
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
            ApiEventKind::UserDeviceModified {
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
