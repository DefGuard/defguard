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
                device_name,
            } => {
                self.log_event(
                    event.context,
                    LoggerEvent::Defguard(DefguardEvent::UserDeviceAdded {
                        device_name,
                        device_id: todo!(),
                        user: todo!(),
                    }),
                )?;
            }
            ApiEventKind::UserDeviceRemoved {
                device_name,
            } => {
                self.log_event(
                    event.context,
                    LoggerEvent::Defguard(DefguardEvent::UserDeviceRemoved {
                        device_name,
                        device_id: 1,
                        user: "testuser".into(),
                    }),
                )?;
            }
            ApiEventKind::UserDeviceModified {
                device_name,
            } => {
                self.log_event(
                    event.context,
                    LoggerEvent::Defguard(DefguardEvent::UserDeviceModified {
                        device_name,
                        device_id: todo!(),
                        user: todo!(),
                    }),
                )?;
            }
        }

        Ok(())
    }
}
