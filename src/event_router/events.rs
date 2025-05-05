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
    pub ip: IpNetwork,
    pub device: String,
}

impl AuditLogContext {
    pub fn new(user_id: Id, ip: IpNetwork, device: String) -> Self {
        let timestamp = Utc::now().naive_utc();
        Self {
            timestamp,
            user_id,
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
            ip: val.ip,
            device: val.device,
        }
    }
}

/// Main events that can be routed through the system
#[derive(Debug)]
pub enum MainEvent {
    UserLogin { context: AuditLogContext },
}
