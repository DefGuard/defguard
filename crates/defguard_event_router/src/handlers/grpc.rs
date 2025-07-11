use defguard_core::events::GrpcEvent;
use defguard_event_logger::message::{EventContext, LoggerEvent, VpnEvent};
use tracing::debug;

use crate::{EventRouter, error::EventRouterError};

impl EventRouter {
    pub(crate) fn handle_grpc_event(&self, event: GrpcEvent) -> Result<(), EventRouterError> {
        debug!("Processing gRPC server event: {event:?}");

        match event {
            GrpcEvent::GatewayConnected { location: _ } => todo!(),
            GrpcEvent::GatewayDisconnected { location: _ } => todo!(),
            GrpcEvent::ClientConnected {
                context,
                location,
                device,
            } => {
                self.log_event(
                    EventContext::from_grpc_context(context, Some(location.clone())),
                    LoggerEvent::Vpn(Box::new(VpnEvent::ConnectedToLocation { location, device })),
                )?;
            }
            GrpcEvent::ClientDisconnected {
                context,
                location,
                device,
            } => {
                self.log_event(
                    EventContext::from_grpc_context(context, Some(location.clone())),
                    LoggerEvent::Vpn(Box::new(VpnEvent::DisconnectedFromLocation {
                        location,
                        device,
                    })),
                )?;
            }
        }

        Ok(())
    }
}
