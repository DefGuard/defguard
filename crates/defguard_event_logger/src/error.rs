use thiserror::Error;

#[derive(Debug, Error)]
pub enum EventLoggerError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Channel closed")]
    ChannelClosed,
}
