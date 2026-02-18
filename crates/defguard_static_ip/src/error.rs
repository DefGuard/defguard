use defguard_common::db::models::wireguard::NetworkAddressError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum StaticIpError {
    #[error("Network (location) with ID {0} not found")]
    NetworkNotFound(i64),
    #[error("Device {0} is not assigned to network {1}")]
    DeviceNotInNetwork(i64, i64),
    #[error(transparent)]
    InvalidIpAssignment(#[from] NetworkAddressError),
    #[error(transparent)]
    SqlxError(#[from] sqlx::Error),
}
