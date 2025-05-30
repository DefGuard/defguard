use std::sync::Arc;

use bytes::Bytes;
use error::EventLoggerError;
use message::{DefguardEvent, EventContext, EventLoggerMessage, LoggerEvent};
use sqlx::PgPool;
use tokio::sync::mpsc::UnboundedReceiver;
use tracing::{debug, info};

use crate::db::{
    models::audit_log::{
        AuditEvent, AuditModule, DeviceAddedMetadata, DeviceModifiedMetadata,
        DeviceRemovedMetadata, EventType,
    },
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
    audit_messages_tx: Arc<tokio::sync::broadcast::Sender<Bytes>>,
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
        let mut serialized_audit_events = String::new();

        // Process all messages in the batch
        for message in message_buffer {
            // Unpack shared event context
            let EventContext {
                user_id,
                username,
                timestamp,
                ip,
                device,
            } = message.context;

            // Convert each message to a related audit event
            let audit_event: AuditEvent = {
                let (module, event, metadata) = match message.event {
                    LoggerEvent::Defguard(event) => {
                        let module = AuditModule::Defguard;

                        let (event_type, metadata) = match event {
                            DefguardEvent::UserLogin => (EventType::UserLogin, None),
                            DefguardEvent::UserLogout => (EventType::UserLogout, None),
                            DefguardEvent::DeviceAdded { device_name } => (
                                EventType::DeviceAdded,
                                serde_json::to_value(DeviceAddedMetadata {
                                    device_names: vec![device_name],
                                })
                                .ok(),
                            ),
                            DefguardEvent::DeviceRemoved { device_name } => (
                                EventType::DeviceRemoved,
                                serde_json::to_value(DeviceRemovedMetadata {
                                    device_names: vec![device_name],
                                })
                                .ok(),
                            ),
                            DefguardEvent::DeviceModified { device_name } => (
                                EventType::DeviceModified,
                                serde_json::to_value(DeviceModifiedMetadata {
                                    device_names: vec![device_name],
                                })
                                .ok(),
                            ),
                            DefguardEvent::AuditStreamCreated => {
                                (EventType::AuditStreamCreated, None)
                            }
                            DefguardEvent::AuditStreamRemoved => {
                                (EventType::AuditStreamRemoved, None)
                            }
                            DefguardEvent::AuditStreamModified => {
                                (EventType::AuditStreamModified, None)
                            }
                        };
                        (module, event_type, metadata)
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

                AuditEvent {
                    id: NoId,
                    timestamp,
                    user_id,
                    username,
                    ip,
                    event,
                    module,
                    device,
                    metadata,
                }
            };

            match serde_json::to_string(&audit_event) {
                Ok(serialized_audit_event) => {
                    serialized_audit_events += &(serialized_audit_event + "\n");
                }
                Err(e) => {
                    error!("Failed to serialize audit event. Reason: {e}");
                }
            }

            // Store audit event in DB
            // TODO: do batch inserts
            audit_event.save(&mut *transaction).await?;
        }

        // Send serialized events
        if !serialized_audit_events.is_empty() {
            let in_bytes = bytes::Bytes::from(serialized_audit_events);
            if let Err(send_err) = audit_messages_tx.send(in_bytes) {
                error!("Sending serialized audit events message failed. Reason: {send_err}");
            }
        }

        // Commit the transaction
        transaction.commit().await?;
    }
}
