use thiserror::Error;

#[derive(Debug, Error)]
pub enum SessionManagerError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}
