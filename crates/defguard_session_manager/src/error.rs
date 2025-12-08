use defguard_common::db::Id;
use thiserror::Error;

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
    #[error("Session map initialization error: {0}")]
    SessionMapInitializationError(String),
}
