use thiserror::Error;

#[derive(Debug, Error)]
pub enum ActivityLogStreamError {
    #[error("Deserialization of {0} error: {1}")]
    ConfigDeserializeError(String, String),
}
