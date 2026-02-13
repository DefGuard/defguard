use defguard_core::{enterprise::firewall::FirewallError, events::GrpcEvent};
use thiserror::Error;
use tokio::sync::mpsc::error::SendError;
use tonic::{Code, Status};

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Error)]
pub(crate) enum GatewayError {
    #[error("gRPC event channel error: {0}")]
    GrpcEventChannelError(#[from] SendError<GrpcEvent>),
    #[error("Endpoint error: {0}")]
    EndpointError(String),
    #[error("gRPC communication error: {0}")]
    GrpcCommunicationError(#[from] tonic::Status),
    #[error(transparent)]
    CertificateError(#[from] defguard_certs::CertificateError),
    #[error(transparent)]
    SqlxError(#[from] sqlx::Error),
    #[error("Not found: {0}")]
    NotFound(String),
    // mpsc channel send/receive error
    #[error("Message channel error: {0}")]
    MessageChannelError(String),
    #[error(transparent)]
    FirewallError(#[from] FirewallError),
}

impl From<GatewayError> for Status {
    fn from(value: GatewayError) -> Self {
        Self::new(Code::Internal, value.to_string())
    }
}
