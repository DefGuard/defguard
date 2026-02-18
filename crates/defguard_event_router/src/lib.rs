//! Event Router
//!
//! This module provides a centralized event routing service for the application.
//! It receives events from various parts of the application and routes them to
//! the appropriate services for processing.
//! By design it should remain a thin component and not perform any processing by itself.
//!
//! # Architecture
//!
//! The event router acts as a central hub for all application events:
//!
//! 1. Components (web API, gRPC server etc.) send events to the router via the `event_tx`
//!    MPSC channel.
//! 2. The router processes these events and forwards them to the appropriate services:
//!    - Activity log events go to the event logger service
//!    - WireGuard events go to the gateway service
//!    - Mail events go to the mail service
//!    - etc.

use std::sync::Arc;

use defguard_core::{
    events::{ApiEvent, BidiStreamEvent},
    grpc::GatewayEvent,
};
use defguard_event_logger::message::{EventContext, EventLoggerMessage, LoggerEvent};
use defguard_session_manager::events::SessionManagerEvent;
use error::EventRouterError;
use events::Event;
use tokio::sync::{
    Notify,
    broadcast::Sender,
    mpsc::{UnboundedReceiver, UnboundedSender},
};
use tracing::{debug, error, info};

mod error;
mod events;
mod handlers;

pub struct RouterReceiverSet {
    api: UnboundedReceiver<ApiEvent>,
    bidi: UnboundedReceiver<BidiStreamEvent>,
    session_manager: UnboundedReceiver<SessionManagerEvent>,
}

impl RouterReceiverSet {
    #[must_use]
    pub fn new(
        api: UnboundedReceiver<ApiEvent>,
        bidi: UnboundedReceiver<BidiStreamEvent>,
        session_manager: UnboundedReceiver<SessionManagerEvent>,
    ) -> Self {
        Self {
            api,
            bidi,
            session_manager,
        }
    }
}

#[allow(dead_code)]
struct EventRouter {
    receivers: RouterReceiverSet,
    event_logger_tx: UnboundedSender<EventLoggerMessage>,
    wireguard_tx: Sender<GatewayEvent>,
    activity_log_stream_reload_notify: Arc<Notify>,
}

impl EventRouter {
    /// Send message to activity log event logger service to persist an event in DB
    fn log_event(
        &self,
        context: EventContext,
        activity_log_event: LoggerEvent,
    ) -> Result<(), EventRouterError> {
        // prepare message
        let message = EventLoggerMessage::new(context, activity_log_event);
        self.event_logger_tx.send(message).map_err(|err| {
            error!("Failed to send event to logger: {err}");
            EventRouterError::EventLoggerError
        })?;

        Ok(())
    }
}

impl EventRouter {
    fn new(
        receivers: RouterReceiverSet,
        event_logger_tx: UnboundedSender<EventLoggerMessage>,
        wireguard_tx: Sender<GatewayEvent>,
        activity_log_stream_reload_notify: Arc<Notify>,
    ) -> Self {
        Self {
            receivers,
            event_logger_tx,
            wireguard_tx,
            activity_log_stream_reload_notify,
        }
    }

    /// Runs the event processing loop
    async fn run(&mut self) -> Result<(), EventRouterError> {
        loop {
            // Receive an event from  one of the component event channels
            let event = tokio::select! {
              event = self.receivers.api.recv() => if let Some(api_event) = event { Event::Api(api_event) } else {
                    error!("API event channel closed");
                    return Err(EventRouterError::ApiEventChannelClosed);
              },
              event = self.receivers.bidi.recv() => if let Some(bidi_event) = event { Event::Bidi(bidi_event) } else {
                    error!("Bidi gRPC stream event channel closed");
                    return Err(EventRouterError::BidiEventChannelClosed);
              },
              event = self.receivers.session_manager.recv() => if let Some(session_manager_event) = event { Event::SessionManager(Box::new(session_manager_event)) } else {
                    error!("Internal event channel closed");
                    return Err(EventRouterError::InternalEventChannelClosed);
              },
            };

            debug!("Received event: {event:?}");

            // Route the event to the appropriate handler
            match event {
                Event::Api(api_event) => self.handle_api_event(api_event)?,
                Event::Bidi(bidi_event) => self.handle_bidi_event(bidi_event)?,
                Event::SessionManager(session_manager_event) => {
                    self.handle_session_manager_event(*session_manager_event)?;
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
    receivers: RouterReceiverSet,
    event_logger_tx: UnboundedSender<EventLoggerMessage>,
    wireguard_tx: Sender<GatewayEvent>,
    activity_log_stream_reload_notify: Arc<Notify>,
) -> Result<(), EventRouterError> {
    info!("Starting main event router service");

    let mut event_router = EventRouter::new(
        receivers,
        event_logger_tx,
        wireguard_tx,
        activity_log_stream_reload_notify,
    );

    event_router.run().await
}
