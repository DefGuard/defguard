use thiserror::Error;

#[derive(Debug, Error)]
pub enum ModelError {
    #[error("Cannot create model")]
    CannotCreate,
    #[error("Database error")]
    DbError(#[from] sqlx::Error),
    #[error("Object not found")]
    NotFound,
}
