use defguard_common::db::Id;
use defguard_core::grpc::GatewayEvent;
use thiserror::Error;
use tokio::sync::{broadcast::error::SendError as BroadcastSendError, mpsc::error::SendError};

use crate::events::SessionManagerEvent;

#[derive(Debug, Error)]
pub enum SessionManagerError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
    #[error(
        "Found multiple active sessions for user {username}, device {device_name} in location {location_name}"
    )]
    MultipleActiveSessionsError {
        location_name: String,
        username: String,
        device_name: String,
    },
    #[error("User with ID {0} does not exist")]
    UserDoesNotExistError(Id),
    #[error("Device with ID {0} does not exist")]
    DeviceDoesNotExistError(Id),
    #[error("Device with pubkey {0} does not exist")]
    DevicePubkeyDoesNotExistError(String),
    #[error("Location with ID {0} does not exist")]
    LocationDoesNotExistError(Id),
    #[error("VPN client session with ID {0} does not exist")]
    SessionDoesNotExistError(Id),
    #[error("Received out of order peer stats update")]
    PeerStatsUpdateOutOfOrderError,
    #[error("Failed to send session manager event: {0}")]
    SessionManagerEventError(Box<SendError<SessionManagerEvent>>),
    #[error("Failed to send gateway manager event: {0}")]
    GatewayManagerEventError(Box<BroadcastSendError<GatewayEvent>>),
}

impl From<SendError<SessionManagerEvent>> for SessionManagerError {
    fn from(error: SendError<SessionManagerEvent>) -> Self {
        Self::SessionManagerEventError(Box::new(error))
    }
}
impl From<BroadcastSendError<GatewayEvent>> for SessionManagerError {
    fn from(error: BroadcastSendError<GatewayEvent>) -> Self {
        Self::GatewayManagerEventError(Box::new(error))
    }
}
