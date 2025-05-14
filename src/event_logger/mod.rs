use error::EventLoggerError;
use message::{EventContext, EventLoggerMessage, LoggerEvent};
use sqlx::PgPool;
use tokio::sync::mpsc::UnboundedReceiver;
use tracing::{debug, info};

use crate::db::{
    models::audit_log::{AuditEvent, AuditModule, EventType},
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
            // Unpack event context
            let EventContext {
                user_id,
                timestamp,
                ip,
                device,
            } = message.context;

            // Convert each message to an audit event
            let audit_event = match message.event {
                LoggerEvent::Defguard(event) => {
                    let module = AuditModule::Defguard;

                    match event {
                        message::DefguardEvent::UserLogin => AuditEvent {
                            id: NoId,
                            timestamp,
                            user_id,
                            ip,
                            event: EventType::UserLogin,
                            module,
                            device,
                            metadata: None,
                        },
                        message::DefguardEvent::UserLogout => todo!(),
                        message::DefguardEvent::DeviceAdded { device_name: _ } => todo!(),
                        message::DefguardEvent::DeviceRemoved { device_name: _ } => todo!(),
                    }
                }
                LoggerEvent::Client(_event) => {
                    let _module = AuditModule::Client;
                    unimplemented!()
                }
                LoggerEvent::Vpn(_event) => {
                    let _module = AuditModule::Vpn;
                    unimplemented!()
                }
                LoggerEvent::Enrollment(_event) => {
                    let _module = AuditModule::Enrollment;
                    unimplemented!()
                }
            };

            // Store audit event in DB
            // TODO: do batch inserts
            audit_event.save(&mut *transaction).await?;
        }

        // Commit the transaction
        transaction.commit().await?;
    }
}
