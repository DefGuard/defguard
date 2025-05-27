use defguard_core::events::{BidiStreamEvent, BidiStreamEventType};
use tracing::debug;

use crate::{error::EventRouterError, EventRouter};

impl EventRouter {
    pub(crate) fn handle_bidi_event(&self, event: BidiStreamEvent) -> Result<(), EventRouterError> {
        debug!("Processing bidi gRPC stream event: {event:?}");
        let BidiStreamEvent {
            request_context,
            event,
        } = event;

        match event {
            BidiStreamEventType::Enrollment(enrollment_event) => todo!(),
            BidiStreamEventType::PasswordReset(password_reset_event) => todo!(),
            BidiStreamEventType::DesktopCLientMfa(desktop_client_mfa_event) => todo!(),
            BidiStreamEventType::ConfigPolling(config_polling_event) => todo!(),
        }
    }
}
