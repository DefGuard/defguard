use error::EventLoggerError;
use message::EventLoggerMessage;
use sqlx::PgPool;
use tokio::sync::mpsc::UnboundedReceiver;

pub mod error;
pub mod message;

pub async fn run_event_logger(
    pool: PgPool,
    event_logger_rx: UnboundedReceiver<EventLoggerMessage>,
) -> Result<(), EventLoggerError> {
    unimplemented!()
}
