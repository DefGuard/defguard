use chrono::NaiveDateTime;
use ipnetwork::IpNetwork;

use crate::db::{Id, User};

/// Messages that can be sent to the event logger
pub enum EventLoggerMessage {
    Defguard {
        context: EventContext,
        event: DefguardEvent,
    },
    Client {
        context: EventContext,
        event: ClientEvent,
    },
    Vpn {
        context: EventContext,
        event: VpnEvent,
    },
    Enrollment {
        context: EventContext,
        event: EnrollmentEvent,
    },
}

/// Shared context that's included in all events
pub struct EventContext {
    pub timestamp: NaiveDateTime,
    pub user_id: Id,
    pub ip: IpNetwork,
    pub device: String,
}

/// Represents audit events related to Web UI
pub enum DefguardEvent {
    UserLogin,
    UserLogout,
    DeviceAdded { device_name: String },
    DeviceRemoved { device_name: String },
}

/// Represents audit events related to client applications
pub enum ClientEvent {}

/// Represents audit events related to VPN
pub enum VpnEvent {}
///
/// Represents audit events related to enrollment process
pub enum EnrollmentEvent {}
