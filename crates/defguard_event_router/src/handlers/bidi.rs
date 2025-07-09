use defguard_core::events::{
    self, BidiStreamEvent, BidiStreamEventType, DesktopClientMfaEvent, PasswordResetEvent,
};
use defguard_event_logger::message::{EnrollmentEvent, LoggerEvent, VpnEvent};
use tracing::debug;

use crate::{EventRouter, error::EventRouterError};

impl EventRouter {
    pub(crate) fn handle_bidi_event(&self, event: BidiStreamEvent) -> Result<(), EventRouterError> {
        debug!("Processing bidi gRPC stream event: {event:?}");
        let BidiStreamEvent { context, event } = event;

        let logger_event = match event {
            BidiStreamEventType::Enrollment(event) => match *event {
                events::EnrollmentEvent::EnrollmentStarted => {
                    LoggerEvent::Enrollment(Box::new(EnrollmentEvent::EnrollmentStarted))
                }

                events::EnrollmentEvent::EnrollmentCompleted => {
                    LoggerEvent::Enrollment(Box::new(EnrollmentEvent::EnrollmentCompleted))
                }

                events::EnrollmentEvent::EnrollmentDeviceAdded { device } => {
                    LoggerEvent::Enrollment(Box::new(EnrollmentEvent::EnrollmentDeviceAdded {
                        device,
                    }))
                }
            },
            BidiStreamEventType::PasswordReset(event) => match *event {
                PasswordResetEvent::PasswordResetRequested => {
                    LoggerEvent::Enrollment(Box::new(EnrollmentEvent::PasswordResetRequested))
                }
                PasswordResetEvent::PasswordResetStarted => {
                    LoggerEvent::Enrollment(Box::new(EnrollmentEvent::PasswordResetStarted))
                }
                PasswordResetEvent::PasswordResetCompleted => {
                    LoggerEvent::Enrollment(Box::new(EnrollmentEvent::PasswordResetCompleted))
                }
            },
            BidiStreamEventType::DesktopClientMfa(event) => match *event {
                DesktopClientMfaEvent::Connected {
                    location,
                    device,
                    method,
                } => LoggerEvent::Vpn(Box::new(VpnEvent::ConnectedToMfaLocation {
                    location,
                    device,
                    method,
                })),
                DesktopClientMfaEvent::Failed {
                    location,
                    device,
                    method,
                    message,
                } => LoggerEvent::Vpn(Box::new(VpnEvent::MfaFailed {
                    location,
                    device,
                    method,
                    message,
                })),
            },
        };

        self.log_event(context.into(), logger_event)
    }
}
