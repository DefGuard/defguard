use thiserror::Error;

#[derive(Debug, Error)]
pub enum EventRouterError {
    #[error("Channel closed")]
    ChannelClosed,
    #[error("Event logger service channel closed")]
    EventLoggerError,
}
