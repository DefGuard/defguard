use std::{error, fmt};

#[derive(Debug)]
pub enum ModelError {
    CannotModify,
    CannotCreate,
    NetworkTooSmall,
    SqlxError(sqlx::Error),
    IpNetworkError(ipnetwork::IpNetworkError),
}

impl error::Error for ModelError {}

impl fmt::Display for ModelError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::CannotCreate => write!(f, "Cannot create model"),
            Self::NetworkTooSmall => write!(f, "Network address will not fit existing devices"),
            Self::SqlxError(error) => {
                write!(f, "SqlxError {error}")
            }
            Self::IpNetworkError(error) => {
                write!(f, "IpNetError {error}")
            }
        }
    }
}

impl From<sqlx::Error> for ModelError {
    fn from(err: sqlx::Error) -> ModelError {
        ModelError::SqlxError(err)
    }
}

impl From<ipnetwork::IpNetworkError> for ModelError {
    fn from(err: ipnetwork::IpNetworkError) -> ModelError {
        ModelError::IpNetworkError(err)
    }
}
