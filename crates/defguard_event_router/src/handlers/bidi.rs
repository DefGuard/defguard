use defguard_core::events::{
    self, BidiStreamEvent, BidiStreamEventType, DesktopClientMfaEvent, PasswordResetEvent,
};
use defguard_event_logger::message::{EnrollmentEvent, LoggerEvent, VpnEvent};
use tracing::debug;

use crate::{error::EventRouterError, EventRouter};

impl EventRouter {
    pub(crate) fn handle_bidi_event(&self, event: BidiStreamEvent) -> Result<(), EventRouterError> {
        debug!("Processing bidi gRPC stream event: {event:?}");
        let BidiStreamEvent { context, event } = event;

        let logger_event = match event {
            BidiStreamEventType::Enrollment(event) => match event {
                events::EnrollmentEvent::EnrollmentStarted => {
                    LoggerEvent::Enrollment(EnrollmentEvent::EnrollmentStarted)
                }

                events::EnrollmentEvent::EnrollmentCompleted => {
                    LoggerEvent::Enrollment(EnrollmentEvent::EnrollmentCompleted)
                }

                events::EnrollmentEvent::EnrollmentDeviceAdded { device } => {
                    LoggerEvent::Enrollment(EnrollmentEvent::EnrollmentDeviceAdded { device })
                }
            },
            BidiStreamEventType::PasswordReset(event) => match event {
                PasswordResetEvent::PasswordResetRequested => {
                    LoggerEvent::Enrollment(EnrollmentEvent::PasswordResetRequested)
                }
                PasswordResetEvent::PasswordResetStarted => {
                    LoggerEvent::Enrollment(EnrollmentEvent::PasswordResetStarted)
                }
                PasswordResetEvent::PasswordResetCompleted => {
                    LoggerEvent::Enrollment(EnrollmentEvent::PasswordResetCompleted)
                }
            },
            BidiStreamEventType::DesktopClientMfa(event) => match event {
                DesktopClientMfaEvent::Connected {
                    location,
                    device,
                    method,
                } => LoggerEvent::Vpn(VpnEvent::ConnectedToMfaLocation {
                    location,
                    device,
                    method,
                }),
                DesktopClientMfaEvent::Failed {
                    location,
                    device,
                    method,
                } => LoggerEvent::Vpn(VpnEvent::MfaFailed {
                    location,
                    device,
                    method,
                }),
            },
        };

        self.log_event(context.into(), logger_event)
    }
}
