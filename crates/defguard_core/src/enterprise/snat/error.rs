use thiserror::Error;

use crate::error::WebError;

#[derive(Debug, Error)]
pub enum UserSnatBindingError {
    #[error("Binding not found")]
    BindingNotFound,
    #[error("Database error")]
    DbError { source: sqlx::Error },
}

impl From<sqlx::Error> for UserSnatBindingError {
    fn from(value: sqlx::Error) -> Self {
        match value {
            sqlx::Error::RowNotFound => Self::BindingNotFound,
            _ => Self::DbError { source: value },
        }
    }
}

impl From<UserSnatBindingError> for WebError {
    fn from(value: UserSnatBindingError) -> Self {
        match value {
            UserSnatBindingError::BindingNotFound => WebError::ObjectNotFound(value.to_string()),
            UserSnatBindingError::DbError { source } => WebError::DbError(source.to_string()),
        }
    }
}
