use error::EventLoggerError;
use message::{DefguardEvent, EventContext, EventLoggerMessage, LoggerEvent};
use sqlx::PgPool;
use tokio::sync::mpsc::UnboundedReceiver;
use tracing::{debug, info};

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
                                device_id,
                                device_name,
                                owner,
                            } => (
                                EventType::DeviceAdded,
                                serde_json::to_value(DeviceAddedMetadata {
                                    device_names: vec![device_name],
                                })
                                .ok(),
                            ),
                            DefguardEvent::UserDeviceRemoved {
                                device_id,
                                device_name,
                                owner,
                            } => (
                                EventType::DeviceRemoved,
                                serde_json::to_value(DeviceRemovedMetadata {
                                    device_names: vec![device_name],
                                })
                                .ok(),
                            ),
                            DefguardEvent::UserDeviceModified {
                                device_id,
                                device_name,
                                owner,
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
                            DefguardEvent::MfaDisabled => (EventType::MfaDisabled, None),
                            DefguardEvent::MfaDefaultChanged { mfa_method } => todo!(),
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
                            DefguardEvent::UserDisabled { username } => todo!(),
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
                                location_id,
                                location_name,
                            } => todo!(),
                            DefguardEvent::VpnLocationRemoved {
                                location_id,
                                location_name,
                            } => todo!(),
                            DefguardEvent::VpnLocationModified {
                                location_id,
                                location_name,
                            } => todo!(),
                            DefguardEvent::OpenIdAppAdded { app_id, app_name } => todo!(),
                            DefguardEvent::OpenIdAppRemoved { app_id, app_name } => todo!(),
                            DefguardEvent::OpenIdAppModified { app_id, app_name } => todo!(),
                            DefguardEvent::OpenIdAppDisabled { app_id, app_name } => todo!(),
                            DefguardEvent::OpenIdProviderAdded {
                                provider_id,
                                provider_name,
                            } => todo!(),
                            DefguardEvent::OpenIdProviderRemoved {
                                provider_id,
                                provider_name,
                            } => todo!(),
                            DefguardEvent::SettingsUpdated => todo!(),
                            DefguardEvent::SettingsUpdatedPartial => todo!(),
                            DefguardEvent::SettingsDefaultBrandingRestored => todo!(),
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

            // Store audit event in DB
            // TODO: do batch inserts
            audit_event.save(&mut *transaction).await?;
        }

        // Commit the transaction
        transaction.commit().await?;
    }
}
