use std::net::IpAddr;

use crate::db::{Device, Id};
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
    pub ip: IpAddr,
    pub device: String,
}

impl ApiRequestContext {
    pub fn new(user_id: Id, username: String, ip: IpAddr, device: String) -> Self {
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

#[derive(Debug)]
pub enum ApiEventType {
    UserLogin,
    UserLogout,
    MfaDisabled,
    MfaTotpDisabled,
    MfaTotpEnabled,
    MfaEmailDisabled,
    MfaEmailEnabled,
    MfaSecurityKeyAdded {
        key_id: Id,
        key_name: String,
    },
    MfaSecurityKeyRemoved {
        key_id: Id,
        key_name: String,
    },
    UserAdded {
        username: String,
    },
    UserRemoved {
        username: String,
    },
    UserModified {
        username: String,
    },
    UserDeviceAdded {
        device_id: Id,
        owner: String,
        device_name: String,
    },
    UserDeviceRemoved {
        device_id: Id,
        owner: String,
        device_name: String,
    },
    UserDeviceModified {
        device_id: Id,
        owner: String,
        device_name: String,
    },
    NetworkDeviceAdded {
        device_id: Id,
        device_name: String,
        location_id: Id,
        location: String,
    },
    NetworkDeviceRemoved {
        device_id: Id,
        device_name: String,
        location_id: Id,
        location: String,
    },
    NetworkDeviceModified {
        device_id: Id,
        device_name: String,
        location_id: Id,
        location: String,
    },
    AuditStreamCreated {
        stream_id: Id,
        stream_name: String,
    },
    AuditStreamModified {
        stream_id: Id,
        stream_name: String,
    },
    AuditStreamRemoved {
        stream_id: Id,
        stream_name: String,
    },
}

/// Events from Web API
#[derive(Debug)]
pub struct ApiEvent {
    pub context: ApiRequestContext,
    pub kind: ApiEventType,
}

/// Events from gRPC server
#[derive(Debug)]
pub enum GrpcEvent {
    GatewayConnected,
    GatewayDisconnected,
}

/// Shared context for every event generated from a user request in the bi-directional gRPC stream.
///
/// Similarly to `ApiRequestContexts` at the moment it's mostly meant to populate the audit log.
#[derive(Debug)]
pub struct BidiRequestContext {
    pub timestamp: NaiveDateTime,
    pub user_id: Id,
    pub username: String,
    pub ip: IpAddr,
    pub device_id: Id,
    pub device_name: String,
    pub location_id: Id,
    pub location_name: String,
}

impl BidiRequestContext {
    pub fn new(
        user_id: Id,
        username: String,
        ip: IpAddr,
        device_id: Id,
        device_name: String,
        location_id: Id,
        location_name: String,
    ) -> Self {
        let timestamp = Utc::now().naive_utc();
        Self {
            timestamp,
            user_id,
            username,
            ip,
            device_id,
            device_name,
            location_id,
            location_name,
        }
    }
}

/// Events emmited from gRPC bi-directional communication stream
#[derive(Debug)]
pub struct BidiStreamEvent {
    pub context: BidiRequestContext,
    pub event: BidiStreamEventType,
}

/// Wrapper enum for different types of events emitted by the bidi stream.
///
/// Each variant represents a separate gRPC service that's part of the bi-directional communications server.
#[derive(Debug)]
pub enum BidiStreamEventType {
    Enrollment(EnrollmentEvent),
    PasswordReset(PasswordResetEvent),
    DesktopClientMfa(DesktopClientMfaEvent),
    ConfigPolling(ConfigPollingEvent),
}

#[derive(Debug)]
pub enum EnrollmentEvent {
    EnrollmentStarted,
}

#[derive(Debug)]
pub enum PasswordResetEvent {}

#[derive(Debug)]
pub enum DesktopClientMfaEvent {
    Connected,
    Disconnected,
}

#[derive(Debug)]
pub enum ConfigPollingEvent {}
