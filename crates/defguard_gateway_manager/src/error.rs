use defguard_core::enterprise::firewall::FirewallError;
use thiserror::Error;
use tonic::{Code, Status};

#[derive(Debug, Error)]
pub(crate) enum GatewayError {
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
