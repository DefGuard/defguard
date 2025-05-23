use crate::db::Id;
use chrono::{NaiveDateTime, Utc};
use ipnetwork::IpNetwork;

/// Shared context that needs to be added to every API event
///
/// Mainly meant to be stored in the audit log.
/// By design this is a duplicate of a similar struct in the `event_logger` module.
/// This is done in order to avoid circular imports once we split the project into multiple crates.
#[derive(Debug)]
pub struct ApiRequestContext {
    pub timestamp: NaiveDateTime,
    pub user_id: Id,
    pub username: String,
    pub ip: IpNetwork,
    pub device: String,
}

impl ApiRequestContext {
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

/// Events from Web API
#[derive(Debug)]
pub enum ApiEvent {
    UserLogin {
        context: ApiRequestContext,
    },
    UserLogout {
        context: ApiRequestContext,
    },
    UserDeviceAdded {
        context: ApiRequestContext,
        device_name: String,
    },
    UserDeviceRemoved {
        context: ApiRequestContext,
        device_name: String,
    },
    UserDeviceModified {
        context: ApiRequestContext,
        device_name: String,
    },
}
/// Events from gRPC server
#[derive(Debug)]
pub enum GrpcEvent {}
