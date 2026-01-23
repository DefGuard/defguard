use defguard_core::events::{ApiEvent, BidiStreamEvent, GrpcEvent, InternalEvent};
use defguard_session_manager::events::SessionManagerEvent;

/// Enum representing all possible events that can be generated in the system.
///
/// System components can send events to the event router through their own event channels.
/// The enum itself is organized based on event source to make splitting logic into smaller chunks easier.
#[derive(Debug)]
pub enum Event {
    Api(ApiEvent),
    Grpc(Box<GrpcEvent>),
    Bidi(BidiStreamEvent),
    Internal(Box<InternalEvent>),
    SessionManager(Box<SessionManagerEvent>),
}
