use chrono::NaiveDateTime;
use ipnetwork::IpNetwork;

use crate::db::Id;

/// Messages that can be sent to the event logger
pub struct EventLoggerMessage {
    pub context: EventContext,
    pub event: LoggerEvent,
}

impl EventLoggerMessage {
    pub fn new(context: EventContext, event: LoggerEvent) -> Self {
        Self { context, event }
    }
}

/// Possible audit event types split by module
pub enum LoggerEvent {
    Defguard(DefguardEvent),
    Client(ClientEvent),
    Vpn(VpnEvent),
    Enrollment(EnrollmentEvent),
}

/// Shared context that's included in all events
pub struct EventContext {
    pub timestamp: NaiveDateTime,
    pub user_id: Id,
    pub username: String,
    pub ip: IpNetwork,
    pub device: String,
}

/// Represents audit events related to Web UI
pub enum DefguardEvent {
    UserLogin,
    UserLogout,
    DeviceAdded { device_name: String },
    DeviceRemoved { device_name: String },
    DeviceModified { device_name: String },
}

/// Represents audit events related to client applications
pub enum ClientEvent {}

/// Represents audit events related to VPN
pub enum VpnEvent {}
///
/// Represents audit events related to enrollment process
pub enum EnrollmentEvent {}
