use crate::{db::Id, event_logger::message::EventContext};
use chrono::{NaiveDateTime, Utc};
use ipnetwork::IpNetwork;

/// Shared context that needs to be added to every event meant to be stored in the audit log
///
/// By design this is a duplicate of a similar struct in the `event_logger` module.
/// This is done in order to avoid circular imports once we split the project into multiple crates.
#[derive(Debug)]
pub struct AuditLogContext {
    pub timestamp: NaiveDateTime,
    pub user_id: Id,
    pub username: String,
    pub ip: IpNetwork,
    pub device: String,
}

impl AuditLogContext {
    pub fn new(user_id: Id, username: String, ip: IpNetwork, device: String) -> Self {
        let timestamp = Utc::now().naive_utc();
        Self {
            timestamp,
            user_id,
            username,
            ip,
            device,
        }
    }
}

impl From<AuditLogContext> for EventContext {
    fn from(val: AuditLogContext) -> Self {
        EventContext {
            timestamp: val.timestamp,
            user_id: val.user_id,
            username: val.username,
            ip: val.ip,
            device: val.device,
        }
    }
}

/// Main events that can be routed through the system
///
/// System components can send events to the event router through the `event_tx` channel.
/// The enum itself is organized based on event source to make splitting logic into smaller chunks easier.
#[derive(Debug)]
pub enum MainEvent {
    Api(ApiEvent),
    Grpc(GrpcEvent),
}

/// Events from Web API
#[derive(Debug)]
pub enum ApiEvent {
    UserLogin {
        context: AuditLogContext,
    },
    UserLogout {
        context: AuditLogContext,
    },
    DeviceAdded {
        context: AuditLogContext,
        device_name: String,
    },
    DeviceRemoved {
        context: AuditLogContext,
        device_name: String,
    },
    DeviceModified {
        context: AuditLogContext,
        device_name: String,
    },
}
/// Events from gRPC server
#[derive(Debug)]
pub enum GrpcEvent {}
