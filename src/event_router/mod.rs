use error::EventRouterError;
use events::{AuditLogContext, MainEvent};
use tokio::sync::{
    broadcast::Sender,
    mpsc::{UnboundedReceiver, UnboundedSender},
};
use tracing::{debug, error, info};

use crate::{
    db::GatewayEvent,
    event_logger::message::{DefguardEvent, EventLoggerMessage, EventType},
    mail::Mail,
};

pub mod error;
pub mod events;

struct EventRouter {
    event_rx: UnboundedReceiver<MainEvent>,
    event_logger_tx: UnboundedSender<EventLoggerMessage>,
    wireguard_tx: Sender<GatewayEvent>,
    mail_tx: UnboundedSender<Mail>,
}

impl EventRouter {
    /// Send message to audit event logger service to persist an event in DB
    fn log_event(
        &self,
        context: AuditLogContext,
        audit_log_event: EventType,
    ) -> Result<(), EventRouterError> {
        // prepare message
        let message = EventLoggerMessage::new(context.into(), audit_log_event);
        self.event_logger_tx.send(message).map_err(|err| {
            error!("Failed to send event to logger: {err}");
            EventRouterError::EventLoggerError
        })?;

        Ok(())
    }
}

impl EventRouter {
    fn new(
        event_rx: UnboundedReceiver<MainEvent>,
        event_logger_tx: UnboundedSender<EventLoggerMessage>,
        wireguard_tx: Sender<GatewayEvent>,
        mail_tx: UnboundedSender<Mail>,
    ) -> Self {
        Self {
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
                    // send event to audit log
                    self.log_event(context, EventType::Defguard(DefguardEvent::UserLogin))?;
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
    event_rx: UnboundedReceiver<MainEvent>,
    event_logger_tx: UnboundedSender<EventLoggerMessage>,
    wireguard_tx: Sender<GatewayEvent>,
    mail_tx: UnboundedSender<Mail>,
) -> Result<(), EventRouterError> {
    info!("Starting main event router service");
    let mut event_router = EventRouter::new(event_rx, event_logger_tx, wireguard_tx, mail_tx);

    event_router.run().await
}
