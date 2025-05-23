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
//! 1. Components (web API, gRPC server etc.) send events to the router via the `event_tx` MPSC channel
//! 2. The router processes these events and forwards them to the appropriate services:
//!    - Audit events go to the event logger service
//!    - WireGuard events go to the gateway service
//!    - Mail events go to the mail service
//!    - etc.
//!
//! # Usage
//!
//! To use the event router, components should send `MainEvent` instances to the
//! event channel. The router will handle routing these events to the appropriate
//! services based on their type.
//!
//! ```
//! // Example:
//! let event = MainEvent::UserLogin { context: user_context };
//! event_tx.send(event).await.unwrap();
//! ```

use defguard_core::events::{ApiEvent, ApiRequestContext, GrpcEvent};
use error::EventRouterError;
use events::Event;
use tokio::sync::{
    broadcast::Sender,
    mpsc::{UnboundedReceiver, UnboundedSender},
};
use tracing::{debug, error, info};

use defguard_core::{db::GatewayEvent, mail::Mail};
use defguard_event_logger::message::{EventLoggerMessage, LoggerEvent};

mod error;
mod events;
mod handlers;

#[allow(dead_code)]
struct EventRouter {
    api_event_rx: UnboundedReceiver<ApiEvent>,
    grpc_event_rx: UnboundedReceiver<GrpcEvent>,
    event_logger_tx: UnboundedSender<EventLoggerMessage>,
    wireguard_tx: Sender<GatewayEvent>,
    mail_tx: UnboundedSender<Mail>,
}

impl EventRouter {
    /// Send message to audit event logger service to persist an event in DB
    fn log_event(
        &self,
        context: ApiRequestContext,
        audit_log_event: LoggerEvent,
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
        api_event_rx: UnboundedReceiver<ApiEvent>,
        grpc_event_rx: UnboundedReceiver<GrpcEvent>,
        event_logger_tx: UnboundedSender<EventLoggerMessage>,
        wireguard_tx: Sender<GatewayEvent>,
        mail_tx: UnboundedSender<Mail>,
    ) -> Self {
        Self {
            api_event_rx,
            grpc_event_rx,
            event_logger_tx,
            wireguard_tx,
            mail_tx,
        }
    }

    /// Runs the event processing loop
    async fn run(&mut self) -> Result<(), EventRouterError> {
        loop {
            // Receive an event from  one of the component event channels
            let event = tokio::select! {
              event = self.api_event_rx.recv() => match event {
                  Some(api_event) => Event::Api(api_event),
                  None => {
                        error!("API event channel closed");
                        return Err(EventRouterError::ApiEventChannelClosed);
                  }
              },
              event = self.grpc_event_rx.recv() => match event {
                  Some(grpc_event) => Event::Grpc(grpc_event),
                  None => {
                        error!("gRPC event channel closed");
                        return Err(EventRouterError::GrpcEventChannelClosed);
                  }
              },
            };

            debug!("Received event: {event:?}");

            // Route the event to the appropriate handler
            match event {
                Event::Api(api_event) => self.handle_api_event(api_event)?,
                Event::Grpc(grpc_event) => self.handle_grpc_event(grpc_event)?,
            };
        }
    }

    fn handle_grpc_event(&self, event: GrpcEvent) -> Result<(), EventRouterError> {
        debug!("Processing gRPC server event: {event:?}");

        match event {}
    }
}

/// Run the event router service
///
/// This function runs in an infinite loop, receiving messages from the event_rx channel
/// and routing them to the appropriate service channels.
pub async fn run_event_router(
    api_event_rx: UnboundedReceiver<ApiEvent>,
    grpc_event_rx: UnboundedReceiver<GrpcEvent>,
    event_logger_tx: UnboundedSender<EventLoggerMessage>,
    wireguard_tx: Sender<GatewayEvent>,
    mail_tx: UnboundedSender<Mail>,
) -> Result<(), EventRouterError> {
    info!("Starting main event router service");
    let mut event_router = EventRouter::new(
        api_event_rx,
        grpc_event_rx,
        event_logger_tx,
        wireguard_tx,
        mail_tx,
    );

    event_router.run().await
}
