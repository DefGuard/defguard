use defguard_core::events::{BidiStreamEvent, BidiStreamEventType, DesktopClientMfaEvent};
use defguard_event_logger::message::{LoggerEvent, VpnEvent};
use tracing::debug;

use crate::{error::EventRouterError, EventRouter};

impl EventRouter {
    pub(crate) fn handle_bidi_event(&self, event: BidiStreamEvent) -> Result<(), EventRouterError> {
        debug!("Processing bidi gRPC stream event: {event:?}");
        let BidiStreamEvent { context, event } = event;

        let logger_event = match event {
            BidiStreamEventType::Enrollment(_enrollment_event) => todo!(),
            BidiStreamEventType::PasswordReset(_password_reset_event) => todo!(),
            BidiStreamEventType::DesktopClientMfa(event) => match event {
                DesktopClientMfaEvent::Connected { method } => {
                    LoggerEvent::Vpn(VpnEvent::ConnectedToMfaLocation {
                        location: context.location.clone(),
                        device: context.device.clone(),
                        method,
                    })
                }
                DesktopClientMfaEvent::Failed { method } => LoggerEvent::Vpn(VpnEvent::MfaFailed {
                    location: context.location.clone(),
                    device: context.device.clone(),
                    method,
                }),
            },
            BidiStreamEventType::ConfigPolling(_config_polling_event) => todo!(),
        };

        self.log_event(context.into(), logger_event)
    }
}
