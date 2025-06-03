use bytes::Bytes;
use error::EventLoggerError;
use message::{DefguardEvent, EventContext, EventLoggerMessage, LoggerEvent};
use sqlx::PgPool;
use tokio::sync::mpsc::UnboundedReceiver;
use tracing::{debug, error, info, trace};

use defguard_core::db::{
    models::audit_log::{
        metadata::{
            DeviceAddedMetadata, DeviceModifiedMetadata, DeviceRemovedMetadata,
            MfaSecurityKeyAddedMetadata, MfaSecurityKeyRemovedMetadata, NetworkDeviceAddedMetadata,
            NetworkDeviceModifiedMetadata, NetworkDeviceRemovedMetadata, UserAddedMetadata,
            UserModifiedMetadata, UserRemovedMetadata,
        },
        AuditEvent, AuditModule, EventType,
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
    audit_messages_tx: tokio::sync::broadcast::Sender<Bytes>,
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
            let audit_event = {
                let (module, event, metadata) = match message.event {
                    LoggerEvent::Defguard(event) => {
                        let module = AuditModule::Defguard;

                        let (event_type, metadata) = match event {
                            DefguardEvent::UserLogin => (EventType::UserLogin, None),
                            DefguardEvent::UserLogout => (EventType::UserLogout, None),
                            DefguardEvent::UserDeviceAdded {
                                device_id: _,
                                device_name,
                                owner: _,
                            } => (
                                EventType::DeviceAdded,
                                serde_json::to_value(DeviceAddedMetadata {
                                    device_names: vec![device_name],
                                })
                                .ok(),
                            ),
                            DefguardEvent::UserDeviceRemoved {
                                device_id: _,
                                device_name,
                                owner: _,
                            } => (
                                EventType::DeviceRemoved,
                                serde_json::to_value(DeviceRemovedMetadata {
                                    device_names: vec![device_name],
                                })
                                .ok(),
                            ),
                            DefguardEvent::UserDeviceModified {
                                device_id: _,
                                device_name,
                                owner: _,
                            } => (
                                EventType::DeviceModified,
                                serde_json::to_value(DeviceModifiedMetadata {
                                    device_names: vec![device_name],
                                })
                                .ok(),
                            ),
                            DefguardEvent::RecoveryCodeUsed => todo!(),
                            DefguardEvent::PasswordChanged => todo!(),
                            DefguardEvent::MfaFailed => todo!(),
                            DefguardEvent::MfaDisabled => todo!(),
                            DefguardEvent::MfaDefaultChanged { mfa_method } => todo!(),
                            DefguardEvent::MfaTotpEnabled => todo!(),
                            DefguardEvent::MfaTotpDisabled => todo!(),
                            DefguardEvent::MfaEmailEnabled => todo!(),
                            DefguardEvent::MfaEmailDisabled => todo!(),
                            DefguardEvent::MfaSecurityKeyAdded { key_id, key_name } => todo!(),
                            DefguardEvent::MfaSecurityKeyRemoved { key_id, key_name } => todo!(),
                            DefguardEvent::AuthenticationKeyAdded {
                                key_id,
                                key_name,
                                key_type,
                            } => todo!(),
                            DefguardEvent::AuthenticationKeyRemoved {
                                key_id,
                                key_name,
                                key_type,
                            } => todo!(),
                            DefguardEvent::AuthenticationKeyRenamed {
                                key_id,
                                key_name,
                                key_type,
                            } => todo!(),
                            DefguardEvent::ApiTokenAdded {
                                token_id,
                                token_name,
                            } => todo!(),
                            DefguardEvent::ApiTokenRemoved {
                                token_id,
                                token_name,
                            } => todo!(),
                            DefguardEvent::ApiTokenRenamed {
                                token_id,
                                token_name,
                            } => todo!(),
                            DefguardEvent::UserAdded {
                                username,
                                enrollment,
                            } => todo!(),
                            DefguardEvent::UserRemoved { username } => todo!(),
                            DefguardEvent::UserModified { username } => todo!(),
                            DefguardEvent::UserDisabled { username } => todo!(),
=======
                            DefguardEvent::MfaDisabled => (EventType::MfaDisabled, None),
                            DefguardEvent::MfaDefaultChanged { mfa_method: _ } => todo!(),
                            DefguardEvent::MfaTotpEnabled => (EventType::MfaTotpEnabled, None),
                            DefguardEvent::MfaTotpDisabled => (EventType::MfaTotpDisabled, None),
                            DefguardEvent::MfaEmailEnabled => (EventType::MfaEmailEnabled, None),
                            DefguardEvent::MfaEmailDisabled => (EventType::MfaEmailDisabled, None),
                            DefguardEvent::MfaSecurityKeyAdded { key_id, key_name } => (
                                EventType::MfaSecurityKeyAdded,
                                serde_json::to_value(MfaSecurityKeyAddedMetadata {
                                    key_id,
                                    key_name,
                                })
                                .ok(),
                            ),
                            DefguardEvent::MfaSecurityKeyRemoved { key_id, key_name } => (
                                EventType::MfaSecurityKeyRemoved,
                                serde_json::to_value(MfaSecurityKeyRemovedMetadata {
                                    key_id,
                                    key_name,
                                })
                                .ok(),
                            ),
                            DefguardEvent::AuthenticationKeyAdded {
                                key_id: _,
                                key_name: _,
                                key_type: _,
                            } => todo!(),
                            DefguardEvent::AuthenticationKeyRemoved {
                                key_id: _,
                                key_name: _,
                                key_type: _,
                            } => todo!(),
                            DefguardEvent::AuthenticationKeyRenamed {
                                key_id: _,
                                key_name: _,
                                key_type: _,
                            } => todo!(),
                            DefguardEvent::ApiTokenAdded {
                                token_id: _,
                                token_name: _,
                            } => todo!(),
                            DefguardEvent::ApiTokenRemoved {
                                token_id: _,
                                token_name: _,
                            } => todo!(),
                            DefguardEvent::ApiTokenRenamed {
                                token_id: _,
                                token_name: _,
                            } => todo!(),
                            DefguardEvent::UserAdded { username } => (
                                EventType::UserAdded,
                                serde_json::to_value(UserAddedMetadata { username }).ok(),
                            ),
                            DefguardEvent::UserRemoved { username } => (
                                EventType::UserRemoved,
                                serde_json::to_value(UserRemovedMetadata { username }).ok(),
                            ),
                            DefguardEvent::UserModified { username } => (
                                EventType::UserModified,
                                serde_json::to_value(UserModifiedMetadata { username }).ok(),
                            ),
                            DefguardEvent::UserDisabled { username: _ } => todo!(),
                            DefguardEvent::NetworkDeviceAdded {
                                device_id,
                                device_name,
                                location_id,
                                location,
                            } => (
                                EventType::NetworkDeviceAdded,
                                serde_json::to_value(NetworkDeviceAddedMetadata {
                                    device_id,
                                    device_name,
                                    location_id,
                                    location,
                                })
                                .ok(),
                            ),
                            DefguardEvent::NetworkDeviceRemoved {
                                device_id,
                                device_name,
                                location_id,
                                location,
                            } => (
                                EventType::NetworkDeviceRemoved,
                                serde_json::to_value(NetworkDeviceRemovedMetadata {
                                    device_id,
                                    device_name,
                                    location_id,
                                    location,
                                })
                                .ok(),
                            ),
                            DefguardEvent::NetworkDeviceModified {
                                device_id,
                                device_name,
                                location_id,
                                location,
                            } => (
                                EventType::NetworkDeviceModified,
                                serde_json::to_value(NetworkDeviceModifiedMetadata {
                                    device_id,
                                    device_name,
                                    location_id,
                                    location,
                                })
                                .ok(),
                            ),
                            DefguardEvent::VpnLocationAdded {
                                location_id: _,
                                location_name: _,
                            } => todo!(),
                            DefguardEvent::VpnLocationRemoved {
                                location_id: _,
                                location_name: _,
                            } => todo!(),
                            DefguardEvent::VpnLocationModified {
                                location_id: _,
                                location_name: _,
                            } => todo!(),
                            DefguardEvent::OpenIdAppAdded {
                                app_id: _,
                                app_name: _,
                            } => todo!(),
                            DefguardEvent::OpenIdAppRemoved {
                                app_id: _,
                                app_name: _,
                            } => todo!(),
                            DefguardEvent::OpenIdAppModified {
                                app_id: _,
                                app_name: _,
                            } => todo!(),
                            DefguardEvent::OpenIdAppDisabled {
                                app_id: _,
                                app_name: _,
                            } => todo!(),
                            DefguardEvent::OpenIdProviderAdded {
                                provider_id: _,
                                provider_name: _,
                            } => todo!(),
                            DefguardEvent::OpenIdProviderRemoved {
                                provider_id: _,
                                provider_name: _,
                            } => todo!(),
                            DefguardEvent::SettingsUpdated => todo!(),
                            DefguardEvent::SettingsUpdatedPartial => todo!(),
                            DefguardEvent::SettingsDefaultBrandingRestored => todo!(),
                            DefguardEvent::AuditStreamCreated {
                                stream_id,
                                stream_name,
                            } => (
                                EventType::AuditStreamCreated,
                                serde_json::to_value(AuditStreamMetadata {
                                    id: stream_id,
                                    name: stream_name,
                                })
                                .ok(),
                            ),

                            DefguardEvent::AuditStreamRemoved {
                                stream_id,
                                stream_name,
                            } => (
                                EventType::AuditStreamRemoved,
                                serde_json::to_value(AuditStreamMetadata {
                                    id: stream_id,
                                    name: stream_name,
                                })
                                .ok(),
                            ),

                            DefguardEvent::AuditStreamModified {
                                stream_id,
                                stream_name,
                            } => (
                                EventType::AuditStreamModified,
                                serde_json::to_value(AuditStreamMetadata {
                                    id: stream_id,
                                    name: stream_name,
                                })
                                .ok(),
                            ),
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
                    ip: ip.into(),
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
                trace!("Sending serialized audit events message failed. Most likely because there is no listeners. Reason: {send_err}");
            }
        }

        // Commit the transaction
        transaction.commit().await?;
    }
}
