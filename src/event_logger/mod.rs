use error::EventLoggerError;
use message::{EventContext, EventLoggerMessage};
use sqlx::PgPool;
use tokio::sync::mpsc::UnboundedReceiver;
use tracing::{debug, info};

use crate::db::{
    models::audit_log::{AuditEvent, AuditModule},
    NoId,
};

pub mod error;
pub mod message;

const MESSAGE_LIMIT: usize = 100;

/// Run the event logger service
///
/// This function runs in an infinite loop, receiving messages from the event_logger_rx channel
/// and storing them in the database as audit events.
pub async fn run_event_logger(
    pool: PgPool,
    mut event_logger_rx: UnboundedReceiver<EventLoggerMessage>,
) -> Result<(), EventLoggerError> {
    info!("Starting audit event logger service");

    // Receive messages in an infinite loop
    loop {
        // Collect multiple messages from the channel (up to MESSAGE_LIMIT at a time)
        let mut message_buffer: Vec<EventLoggerMessage> = Vec::with_capacity(MESSAGE_LIMIT);
        let message_count = event_logger_rx
            .recv_many(&mut message_buffer, MESSAGE_LIMIT)
            .await;

        debug!("Processing batch of {message_count} audit events");

        let mut transaction = pool.begin().await?;

        // Process all messages in the batch
        for message in message_buffer {
            // Convert each message to an audit event
            let audit_event = match message {
                EventLoggerMessage::Defguard { context, event } => {
                    let module = AuditModule::Defguard;
                    let EventContext {
                        user_id,
                        timestamp,
                        ip,
                        device,
                    } = context;

                    match event {
                        message::DefguardEvent::UserLogin => AuditEvent {
                            id: NoId,
                            timestamp,
                            user_id,
                            ip,
                            event: "User logged in".to_string(),
                            module,
                            device,
                            details: None,
                            metadata: None,
                        },
                        message::DefguardEvent::UserLogout => todo!(),
                        message::DefguardEvent::DeviceAdded { device_name } => todo!(),
                        message::DefguardEvent::DeviceRemoved { device_name } => todo!(),
                    }
                }
                EventLoggerMessage::Client { context, event } => todo!(),
                EventLoggerMessage::Vpn { context, event } => todo!(),
                EventLoggerMessage::Enrollment { context, event } => todo!(),
            };

            // Store audit event in DB
            audit_event.save(&mut *transaction).await?;
        }

        // Commit the transaction
        transaction.commit().await?;
    }
}
