use defguard_core::events::{ApiEvent, GrpcEvent};

/// Enum representing all possible events that can be generated in the system.
///
/// System components can send events to the event router through their own event channels.
/// The enum itself is organized based on event source to make splitting logic into smaller chunks easier.
#[derive(Debug)]
pub enum Event {
    Api(ApiEvent),
    Grpc(GrpcEvent),
}
