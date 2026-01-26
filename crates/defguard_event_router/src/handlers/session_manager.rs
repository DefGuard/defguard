use defguard_event_logger::message::{EventContext, LoggerEvent, VpnEvent};
use defguard_session_manager::events::{SessionManagerEvent, SessionManagerEventType};
use tracing::debug;

use crate::{EventRouter, error::EventRouterError};

impl EventRouter {
    pub(crate) fn handle_session_manager_event(
        &self,
        event: SessionManagerEvent,
    ) -> Result<(), EventRouterError> {
        debug!("Processing session manager event: {event:?}");

        let SessionManagerEvent { context, event } = event;

        // FIXME: consider if we actually need this as part of event since we have the context anyway
        let location = context.location.clone();
        let device = context.device.clone();

        let logger_event = match event {
            SessionManagerEventType::ClientConnected => {
                LoggerEvent::Vpn(Box::new(VpnEvent::ConnectedToLocation { location, device }))
            }
            SessionManagerEventType::ClientDisconnected => {
                LoggerEvent::Vpn(Box::new(VpnEvent::DisconnectedFromLocation {
                    location,
                    device,
                }))
            }
            SessionManagerEventType::MfaClientConnected => todo!(),
            SessionManagerEventType::MfaClientDisconnected => todo!(),
        };
        self.log_event(
            EventContext::from_session_manager_context(context),
            logger_event,
        )
    }
}
