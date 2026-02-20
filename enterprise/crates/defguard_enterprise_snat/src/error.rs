use thiserror::Error;

#[derive(Debug, Error)]
pub enum UserSnatBindingError {
    #[error("Binding not found")]
    BindingNotFound,
    #[error("Binding already exists")]
    BindingAlreadyExists,
    #[error("Database error")]
    DbError { source: sqlx::Error },
}

impl From<sqlx::Error> for UserSnatBindingError {
    fn from(value: sqlx::Error) -> Self {
        match value {
            sqlx::Error::RowNotFound => Self::BindingNotFound,
            sqlx::Error::Database(err) if err.constraint() == Some("user_location") => {
                Self::BindingAlreadyExists
            }
            _ => Self::DbError { source: value },
        }
    }
}
