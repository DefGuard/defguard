use bytes::Bytes;
use error::EventLoggerError;
use message::{
    DefguardEvent, EnrollmentEvent, EventContext, EventLoggerMessage, LoggerEvent, VpnEvent,
};
use sqlx::PgPool;
use tokio::sync::mpsc::UnboundedReceiver;
use tracing::{debug, error, info, trace};

use defguard_core::db::{
    models::audit_log::{
        metadata::{
            ApiTokenMetadata, ApiTokenRenamedMetadata, AuditStreamMetadata, DeviceAddedMetadata,
            DeviceModifiedMetadata, DeviceRemovedMetadata, EnrollmentDeviceAddedMetadata,
            MfaLoginMetadata, MfaSecurityKeyAddedMetadata, MfaSecurityKeyRemovedMetadata,
            NetworkDeviceAddedMetadata, NetworkDeviceModifiedMetadata,
            NetworkDeviceRemovedMetadata, UserAddedMetadata, UserModifiedMetadata,
            UserRemovedMetadata, VpnClientMetadata, VpnClientMfaMetadata, VpnLocationMetadata,
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
                            DefguardEvent::UserLoginFailed => (EventType::UserLoginFailed, None),
                            DefguardEvent::UserMfaLogin { mfa_method } => (
                                EventType::UserMfaLogin,
                                serde_json::to_value(MfaLoginMetadata { mfa_method }).ok(),
                            ),
                            DefguardEvent::UserMfaLoginFailed { mfa_method } => (
                                EventType::UserMfaLoginFailed,
                                serde_json::to_value(MfaLoginMetadata { mfa_method }).ok(),
                            ),
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
                            DefguardEvent::RecoveryCodeUsed => (EventType::RecoveryCodeUsed, None),
                            DefguardEvent::PasswordChanged => todo!(),
                            DefguardEvent::MfaDisabled => (EventType::MfaDisabled, None),
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
                            DefguardEvent::ApiTokenAdded { owner, token_name } => (
                                EventType::ApiTokenAdded,
                                serde_json::to_value(ApiTokenMetadata { owner, token_name }).ok(),
                            ),
                            DefguardEvent::ApiTokenRemoved { owner, token_name } => (
                                EventType::ApiTokenRemoved,
                                serde_json::to_value(ApiTokenMetadata { owner, token_name }).ok(),
                            ),
                            DefguardEvent::ApiTokenRenamed {
                                owner,
                                old_name,
                                new_name,
                            } => (
                                EventType::ApiTokenRenamed,
                                serde_json::to_value(ApiTokenRenamedMetadata {
                                    owner,
                                    old_name,
                                    new_name,
                                })
                                .ok(),
                            ),
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
                            DefguardEvent::VpnLocationAdded { location } => (
                                EventType::VpnLocationAdded,
                                serde_json::to_value(VpnLocationMetadata { location }).ok(),
                            ),
                            DefguardEvent::VpnLocationRemoved { location } => (
                                EventType::VpnLocationRemoved,
                                serde_json::to_value(VpnLocationMetadata { location }).ok(),
                            ),
                            DefguardEvent::VpnLocationModified { location } => (
                                EventType::VpnLocationModified,
                                serde_json::to_value(VpnLocationMetadata { location }).ok(),
                            ),
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
                    LoggerEvent::Vpn(event) => {
                        let module = AuditModule::Vpn;
                        let (event_type, metadata) = match event {
                            VpnEvent::MfaFailed {
                                location,
                                device,
                                method,
                            } => (
                                EventType::VpnClientMfaFailed,
                                serde_json::to_value(VpnClientMfaMetadata {
                                    location,
                                    device,
                                    method,
                                })
                                .ok(),
                            ),
                            VpnEvent::ConnectedToMfaLocation {
                                location,
                                device,
                                method,
                            } => (
                                EventType::VpnClientConnectedMfa,
                                serde_json::to_value(VpnClientMfaMetadata {
                                    location,
                                    device,
                                    method,
                                })
                                .ok(),
                            ),
                            VpnEvent::DisconnectedFromMfaLocation { location, device } => (
                                EventType::VpnClientDisconnectedMfa,
                                serde_json::to_value(VpnClientMetadata { location, device }).ok(),
                            ),
                            VpnEvent::ConnectedToLocation { location, device } => (
                                EventType::VpnClientConnected,
                                serde_json::to_value(VpnClientMetadata { location, device }).ok(),
                            ),
                            VpnEvent::DisconnectedFromLocation { location, device } => (
                                EventType::VpnClientDisconnected,
                                serde_json::to_value(VpnClientMetadata { location, device }).ok(),
                            ),
                        };
                        (module, event_type, metadata)
                    }
                    LoggerEvent::Enrollment(event) => {
                        let module = AuditModule::Enrollment;
                        let (event_type, metadata) = match event {
                            EnrollmentEvent::EnrollmentStarted => {
                                (EventType::EnrollmentStarted, None)
                            }
                            EnrollmentEvent::EnrollmentCompleted => {
                                (EventType::EnrollmentCompleted, None)
                            }
                            EnrollmentEvent::EnrollmentDeviceAdded { device } => (
                                EventType::EnrollmentDeviceAdded,
                                serde_json::to_value(EnrollmentDeviceAddedMetadata { device }).ok(),
                            ),
                            EnrollmentEvent::PasswordResetRequested => {
                                (EventType::PasswordResetRequested, None)
                            }
                            EnrollmentEvent::PasswordResetStarted => {
                                (EventType::PasswordResetStarted, None)
                            }
                            EnrollmentEvent::PasswordResetCompleted => {
                                (EventType::PasswordResetCompleted, None)
                            }
                        };
                        (module, event_type, metadata)
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
