use error::EventRouterError;
use events::MainEvent;
use sqlx::PgPool;
use tokio::sync::{
    broadcast::Sender,
    mpsc::{UnboundedReceiver, UnboundedSender},
};
use tracing::{debug, error, info};

use crate::{
    db::GatewayEvent,
    event_logger::message::{DefguardEvent, EventLoggerMessage},
    mail::Mail,
};

pub mod error;
pub mod events;

struct EventRouter {
    pool: PgPool,
    event_rx: UnboundedReceiver<MainEvent>,
    event_logger_tx: UnboundedSender<EventLoggerMessage>,
    wireguard_tx: Sender<GatewayEvent>,
    mail_tx: UnboundedSender<Mail>,
}

impl EventRouter {
    fn new(
        pool: PgPool,
        event_rx: UnboundedReceiver<MainEvent>,
        event_logger_tx: UnboundedSender<EventLoggerMessage>,
        wireguard_tx: Sender<GatewayEvent>,
        mail_tx: UnboundedSender<Mail>,
    ) -> Self {
        Self {
            pool,
            event_rx,
            event_logger_tx,
            wireguard_tx,
            mail_tx,
        }
    }

    /// Runs the event processing loop
    async fn run(&mut self) -> Result<(), EventRouterError> {
        loop {
            // Receive a message from the channel
            let event = match self.event_rx.recv().await {
                Some(event) => event,
                None => {
                    error!("Event channel closed");
                    return Err(EventRouterError::ChannelClosed);
                }
            };

            debug!("Received event: {event:?}");

            // Route the event to the appropriate service
            match event {
                MainEvent::UserLogin { context } => {
                    // Log event
                    let message = EventLoggerMessage::Defguard {
                        context: context.into(),
                        event: DefguardEvent::UserLogin,
                    };
                    if let Err(err) = self.event_logger_tx.send(message) {
                        error!("Failed to send event to logger: {err}");
                    }
                }
            }
        }
    }
}

/// Run the event router service
///
/// This function runs in an infinite loop, receiving messages from the event_rx channel
/// and routing them to the appropriate service channels.
pub async fn run_event_router(
    pool: PgPool,
    event_rx: UnboundedReceiver<MainEvent>,
    event_logger_tx: UnboundedSender<EventLoggerMessage>,
    wireguard_tx: Sender<GatewayEvent>,
    mail_tx: UnboundedSender<Mail>,
) -> Result<(), EventRouterError> {
    info!("Starting main event router service");
    let mut event_router = EventRouter::new(pool, event_rx, event_logger_tx, wireguard_tx, mail_tx);

    event_router.run().await
}
