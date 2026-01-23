use defguard_common::db::Id;
use thiserror::Error;
use tokio::sync::mpsc::error::SendError;

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
    #[error("Location with ID {0} does not exist")]
    LocationDoesNotExistError(Id),
    #[error("Received out of order peer stats update")]
    PeerStatsUpdateOutOfOrderError,
    #[error("Failed to send session manager event: {0}")]
    SessionManagerEventError(Box<SendError<SessionManagerEvent>>),
}

impl From<SendError<SessionManagerEvent>> for SessionManagerError {
    fn from(error: SendError<SessionManagerEvent>) -> Self {
        Self::SessionManagerEventError(Box::new(error))
    }
}
