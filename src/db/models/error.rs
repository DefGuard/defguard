use thiserror::Error;

#[derive(Debug, Error)]
pub enum ModelError {
    #[error("Cannot modify model")]
    CannotModify,
    #[error("Cannot create model")]
    CannotCreate,
    #[error("Database error")]
    DbError(#[from] sqlx::Error),
    #[error("ID field not set")]
    IdNotSet,
    #[error("Object not found")]
    NotFound,
}
