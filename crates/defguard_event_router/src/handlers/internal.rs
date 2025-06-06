use defguard_core::events::InternalEvent;
use defguard_event_logger::message::{LoggerEvent, VpnEvent};
use tracing::debug;

use crate::{error::EventRouterError, EventRouter};

impl EventRouter {
    pub(crate) fn handle_internal_event(
        &self,
        event: InternalEvent,
    ) -> Result<(), EventRouterError> {
        debug!("Processing internal event: {event:?}");

        match event {
            InternalEvent::DesktopClientMfaDisconnected {
                timestamp,
                user_id,
                username,
                ip,
                device,
                location,
            } => {
                // TODO build context in caller
                self.log_event(
                    defguard_event_logger::message::EventContext {
                        timestamp,
                        user_id,
                        username,
                        ip,
                        device: format!("{} (ID {})", device.name, device.id),
                    },
                    LoggerEvent::Vpn(VpnEvent::DisconnectedFromMfaLocation { location, device }),
                )
            }
        }
    }
}
