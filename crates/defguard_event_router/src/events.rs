use defguard_core::events::{ApiEvent, BidiStreamEvent, GrpcEvent, InternalEvent};

/// Enum representing all possible events that can be generated in the system.
///
/// System components can send events to the event router through their own event channels.
/// The enum itself is organized based on event source to make splitting logic into smaller chunks easier.
// TODO: remove lint override below once all events are updated to pass whole objects
#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub enum Event {
    Api(ApiEvent),
    Grpc(GrpcEvent),
    Bidi(BidiStreamEvent),
    Internal(InternalEvent),
}
