use defguard_core::events::{BidiStreamEvent, BidiStreamEventType, DesktopClientMfaEvent};
use defguard_event_logger::message::{ClientEvent, EnrollmentEvent, LoggerEvent, VpnEvent};
use tracing::debug;

use crate::{error::EventRouterError, EventRouter};

impl EventRouter {
    pub(crate) fn handle_bidi_event(&self, event: BidiStreamEvent) -> Result<(), EventRouterError> {
        debug!("Processing bidi gRPC stream event: {event:?}");
        let BidiStreamEvent { context, event } = event;

        let logger_event = match event {
            BidiStreamEventType::Enrollment(_enrollment_event) => todo!(),
            BidiStreamEventType::PasswordReset(_password_reset_event) => todo!(),
            BidiStreamEventType::DesktopCLientMfa(event) => match event {
                DesktopClientMfaEvent::Connected => {
                    LoggerEvent::Vpn(VpnEvent::ConnectedToMfaLocation {
                        location_id: context.location_id,
                        location_name: context.location_name,
                    })
                }
                DesktopClientMfaEvent::Disconnected => todo!(),
            },
            BidiStreamEventType::ConfigPolling(_config_polling_event) => todo!(),
        };

        self.log_event(context.into(), logger_event)
    }
}
