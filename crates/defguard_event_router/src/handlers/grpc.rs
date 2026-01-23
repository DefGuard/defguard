use defguard_core::events::GrpcEvent;
use defguard_event_logger::message::{LoggerEvent, VpnEvent};
use tracing::debug;

use crate::{EventRouter, error::EventRouterError};

impl EventRouter {
    pub(crate) fn handle_grpc_event(&self, event: GrpcEvent) -> Result<(), EventRouterError> {
        debug!("Processing gRPC server event: {event:?}");

        match event {
            GrpcEvent::GatewayConnected { location: _ } => todo!(),
            GrpcEvent::GatewayDisconnected { location: _ } => todo!(),
            GrpcEvent::ClientDisconnected {
                context,
                location,
                device,
            } => {
                self.log_event(
                    context.into(),
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
