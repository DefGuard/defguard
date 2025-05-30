use defguard_core::events::ApiEvent;
use defguard_event_logger::message::{DefguardEvent, LoggerEvent};
use tracing::debug;

use crate::{error::EventRouterError, EventRouter};

impl EventRouter {
    pub(crate) fn handle_api_event(&self, event: ApiEvent) -> Result<(), EventRouterError> {
        debug!("Processing API event: {event:?}");

        match event {
            ApiEvent::UserLogin { context } => {
                // send event to audit log
                self.log_event(context, LoggerEvent::Defguard(DefguardEvent::UserLogin))?;
            }
            ApiEvent::UserLogout { context } => {
                self.log_event(context, LoggerEvent::Defguard(DefguardEvent::UserLogout))?;
            }
            ApiEvent::UserDeviceAdded {
                context,
                device_name,
            } => {
                self.log_event(
                    context,
                    LoggerEvent::Defguard(DefguardEvent::UserDeviceAdded {
                        device_name,
                        device_id: todo!(),
                        user: todo!(),
                    }),
                )?;
            }
            ApiEvent::UserDeviceRemoved {
                context,
                device_name,
            } => {
                self.log_event(
                    context,
                    LoggerEvent::Defguard(DefguardEvent::UserDeviceRemoved {
                        device_name,
                        device_id: todo!(),
                        user: todo!(),
                    }),
                )?;
            }
            ApiEvent::UserDeviceModified {
                context,
                device_name,
            } => {
                self.log_event(
                    context,
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
