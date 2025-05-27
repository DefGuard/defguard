use defguard_core::events::GrpcEvent;
use tracing::debug;

use crate::{error::EventRouterError, EventRouter};

impl EventRouter {
    pub(crate) fn handle_grpc_event(&self, event: GrpcEvent) -> Result<(), EventRouterError> {
        debug!("Processing gRPC server event: {event:?}");

        match event {
            GrpcEvent::GatewayConnected => todo!(),
            GrpcEvent::GatewayDisconnected => todo!(),
        }
    }
}
