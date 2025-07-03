use defguard_core::events::InternalEvent;
use defguard_event_logger::message::{LoggerEvent, VpnEvent};
use tracing::debug;

use crate::{EventRouter, error::EventRouterError};

impl EventRouter {
    pub(crate) fn handle_internal_event(
        &self,
        event: InternalEvent,
    ) -> Result<(), EventRouterError> {
        debug!("Processing internal event: {event:?}");

        match event {
            InternalEvent::DesktopClientMfaDisconnected { context, location } => {
                let device = context.device.clone();
                self.log_event(
                    context.into(),
                    LoggerEvent::Vpn(Box::new(VpnEvent::DisconnectedFromMfaLocation {
                        device,
                        location,
                    })),
                )
            }
        }
    }
}
