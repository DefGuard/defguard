use defguard_core::events::{
    self, BidiStreamEvent, BidiStreamEventType, DesktopClientMfaEvent, PasswordResetEvent,
};
use defguard_event_logger::message::{EnrollmentEvent, EventContext, LoggerEvent, VpnEvent};
use tracing::debug;

use crate::{EventRouter, error::EventRouterError};

impl EventRouter {
    pub(crate) fn handle_bidi_event(&self, event: BidiStreamEvent) -> Result<(), EventRouterError> {
        debug!("Processing bidi gRPC stream event: {event:?}");
        let BidiStreamEvent { context, event } = event;

        let (logger_event, location) = match event {
            BidiStreamEventType::Enrollment(event) => match *event {
                events::EnrollmentEvent::EnrollmentStarted => (
                    LoggerEvent::Enrollment(Box::new(EnrollmentEvent::EnrollmentStarted)),
                    None,
                ),

                events::EnrollmentEvent::EnrollmentCompleted => (
                    LoggerEvent::Enrollment(Box::new(EnrollmentEvent::EnrollmentCompleted)),
                    None,
                ),

                events::EnrollmentEvent::EnrollmentDeviceAdded { device } => (
                    LoggerEvent::Enrollment(Box::new(EnrollmentEvent::EnrollmentDeviceAdded {
                        device,
                    })),
                    None,
                ),
            },
            BidiStreamEventType::PasswordReset(event) => match *event {
                PasswordResetEvent::PasswordResetRequested => (
                    LoggerEvent::Enrollment(Box::new(EnrollmentEvent::PasswordResetRequested)),
                    None,
                ),
                PasswordResetEvent::PasswordResetStarted => (
                    LoggerEvent::Enrollment(Box::new(EnrollmentEvent::PasswordResetStarted)),
                    None,
                ),
                PasswordResetEvent::PasswordResetCompleted => (
                    LoggerEvent::Enrollment(Box::new(EnrollmentEvent::PasswordResetCompleted)),
                    None,
                ),
            },
            BidiStreamEventType::DesktopClientMfa(event) => match *event {
                DesktopClientMfaEvent::Connected {
                    location,
                    device,
                    method,
                } => (
                    LoggerEvent::Vpn(Box::new(VpnEvent::ConnectedToMfaLocation {
                        location: location.clone(),
                        device,
                        method,
                    })),
                    Some(location),
                ),
                DesktopClientMfaEvent::Failed {
                    location,
                    device,
                    method,
                    message,
                } => (
                    LoggerEvent::Vpn(Box::new(VpnEvent::MfaFailed {
                        location: location.clone(),
                        device,
                        method,
                        message,
                    })),
                    Some(location),
                ),
            },
        };

        self.log_event(
            EventContext::from_bidi_context(context, location),
            logger_event,
        )
    }
}
