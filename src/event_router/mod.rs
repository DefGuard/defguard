use error::EventRouterError;
use events::MainEvent;
use sqlx::PgPool;
use tokio::sync::{
    broadcast::Sender,
    mpsc::{UnboundedReceiver, UnboundedSender},
};

use crate::{db::GatewayEvent, event_logger::message::EventLoggerMessage, mail::Mail};

pub mod error;
pub mod events;

pub async fn run_event_router(
    pool: PgPool,
    event_rx: UnboundedReceiver<MainEvent>,
    event_logger_tx: UnboundedSender<EventLoggerMessage>,
    wireguard_tx: Sender<GatewayEvent>,
    mail_tx: UnboundedSender<Mail>,
) -> Result<(), EventRouterError> {
    unimplemented!()
}
