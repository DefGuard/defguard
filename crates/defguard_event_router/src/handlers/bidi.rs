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
                DesktopClientMfaEvent::Success {
                    location,
                    device,
                    method,
                } => (
                    LoggerEvent::Vpn(Box::new(VpnEvent::ClientMfaSuccess {
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
                    LoggerEvent::Vpn(Box::new(VpnEvent::ClientMfaFailed {
                        location: location.clone(),
                        device,
                        method,
                        message,
                    })),
                    Some(location),
                ),
                DesktopClientMfaEvent::Disconnected {
                    location,
                    device,
                    is_mfa_session,
                } => {
                    let vpn_event = if is_mfa_session {
                        VpnEvent::MfaDisconnectedFromLocation {
                            location: location.clone(),
                            device,
                        }
                    } else {
                        VpnEvent::DisconnectedFromLocation {
                            location: location.clone(),
                            device,
                        }
                    };

                    (LoggerEvent::Vpn(Box::new(vpn_event)), Some(location))
                }
            },
        };

        self.log_event(
            EventContext::from_bidi_context(context, location),
            logger_event,
        )
    }
}

#[cfg(test)]
mod tests {
    use std::{
        net::{IpAddr, Ipv4Addr},
        sync::Arc,
    };

    use defguard_common::db::{
        Id, NoId,
        models::{
            Device, DeviceType, WireguardNetwork,
            wireguard::{LocationMfaMode, ServiceLocationMode},
        },
    };
    use defguard_core::{
        events::{BidiRequestContext, BidiStreamEventType},
        grpc::GatewayEvent,
    };
    use tokio::sync::{Notify, broadcast, mpsc::unbounded_channel};

    use super::*;
    use crate::RouterReceiverSet;

    #[test]
    fn maps_disconnect_bidi_events_from_mfa_sessions_to_mfa_disconnect_logger_events() {
        let message = route_disconnect_event(true);

        match message.event {
            LoggerEvent::Vpn(event) => match *event {
                VpnEvent::MfaDisconnectedFromLocation { location, device } => {
                    assert_eq!(location.id, sample_location().id);
                    assert_eq!(device.id, sample_device().id);
                }
                _ => panic!("expected MFA disconnect vpn event"),
            },
            _ => panic!("expected vpn logger event"),
        }
    }

    #[test]
    fn maps_disconnect_bidi_events_from_non_mfa_sessions_to_standard_disconnect_logger_events() {
        let message = route_disconnect_event(false);

        match message.event {
            LoggerEvent::Vpn(event) => match *event {
                VpnEvent::DisconnectedFromLocation { location, device } => {
                    assert_eq!(location.id, sample_location().id);
                    assert_eq!(device.id, sample_device().id);
                }
                _ => panic!("expected standard disconnect vpn event"),
            },
            _ => panic!("expected vpn logger event"),
        }
    }

    fn sample_router() -> (
        EventRouter,
        tokio::sync::mpsc::UnboundedReceiver<defguard_event_logger::message::EventLoggerMessage>,
    ) {
        let (_api_tx, api_rx) = unbounded_channel();
        let (_bidi_tx, bidi_rx) = unbounded_channel();
        let (_session_manager_tx, session_manager_rx) = unbounded_channel();
        let (event_logger_tx, event_logger_rx) = unbounded_channel();
        let (wireguard_tx, _wireguard_rx) = broadcast::channel::<GatewayEvent>(1);

        (
            EventRouter::new(
                RouterReceiverSet::new(api_rx, bidi_rx, session_manager_rx),
                event_logger_tx,
                wireguard_tx,
                Arc::new(Notify::new()),
            ),
            event_logger_rx,
        )
    }

    fn route_disconnect_event(
        is_mfa_session: bool,
    ) -> defguard_event_logger::message::EventLoggerMessage {
        let (router, mut event_logger_rx) = sample_router();

        router
            .handle_bidi_event(BidiStreamEvent {
                context: sample_context(),
                event: BidiStreamEventType::DesktopClientMfa(Box::new(
                    DesktopClientMfaEvent::Disconnected {
                        location: sample_location(),
                        device: sample_device(),
                        is_mfa_session,
                    },
                )),
            })
            .expect("bidi disconnect event should be routed");

        event_logger_rx
            .try_recv()
            .expect("router should emit an activity log message")
    }

    fn sample_context() -> BidiRequestContext {
        BidiRequestContext::new(
            1,
            "alice".to_string(),
            IpAddr::V4(Ipv4Addr::LOCALHOST),
            "desktop-app".to_string(),
        )
    }

    fn sample_device() -> Device<Id> {
        Device::new(
            "vpn-device".to_string(),
            "pubkey".to_string(),
            1,
            DeviceType::User,
            None,
            true,
        )
        .save_placeholder_id(20)
    }

    fn sample_location() -> WireguardNetwork<Id> {
        WireguardNetwork::new(
            "vpn-location".to_string(),
            vec!["10.0.0.0/24".parse().unwrap()],
            51820,
            "vpn.example.com".to_string(),
            None,
            1420,
            0,
            vec!["0.0.0.0/0".parse().unwrap()],
            true,
            25,
            300,
            false,
            false,
            LocationMfaMode::Internal,
            ServiceLocationMode::Disabled,
        )
        .save_placeholder_id(10)
    }

    trait WithPlaceholderId<T> {
        fn save_placeholder_id(self, id: Id) -> T;
    }

    impl WithPlaceholderId<Device<Id>> for Device<NoId> {
        fn save_placeholder_id(self, id: Id) -> Device<Id> {
            Device {
                id,
                name: self.name,
                wireguard_pubkey: self.wireguard_pubkey,
                user_id: self.user_id,
                created: self.created,
                device_type: self.device_type,
                description: self.description,
                configured: self.configured,
            }
        }
    }

    impl WithPlaceholderId<WireguardNetwork<Id>> for WireguardNetwork<NoId> {
        fn save_placeholder_id(self, id: Id) -> WireguardNetwork<Id> {
            WireguardNetwork {
                id,
                name: self.name,
                address: self.address,
                port: self.port,
                pubkey: self.pubkey,
                prvkey: self.prvkey,
                endpoint: self.endpoint,
                dns: self.dns,
                mtu: self.mtu,
                fwmark: self.fwmark,
                allowed_ips: self.allowed_ips,
                allow_all_groups: self.allow_all_groups,
                connected_at: self.connected_at,
                acl_enabled: self.acl_enabled,
                acl_default_allow: self.acl_default_allow,
                keepalive_interval: self.keepalive_interval,
                peer_disconnect_threshold: self.peer_disconnect_threshold,
                location_mfa_mode: self.location_mfa_mode,
                service_location_mode: self.service_location_mode,
            }
        }
    }
}
