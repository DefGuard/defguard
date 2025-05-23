use thiserror::Error;

#[derive(Debug, Error)]
pub enum EventRouterError {
    #[error("API event channel closed")]
    ApiEventChannelClosed,
    #[error("gRPC event channel closed")]
    GrpcEventChannelClosed,
    #[error("Event logger service channel closed")]
    EventLoggerError,
}
