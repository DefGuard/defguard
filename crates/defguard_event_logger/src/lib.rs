use bytes::Bytes;
use defguard_core::db::{
    models::activity_log::{
        metadata::{
            ActivityLogStreamMetadata, DeviceAddedMetadata, DeviceModifiedMetadata,
            DeviceRemovedMetadata, EnrollmentDeviceAddedMetadata, MfaLoginMetadata,
            MfaSecurityKeyAddedMetadata, MfaSecurityKeyRemovedMetadata, NetworkDeviceAddedMetadata,
            NetworkDeviceModifiedMetadata, NetworkDeviceRemovedMetadata, UserAddedMetadata,
            UserModifiedMetadata, UserRemovedMetadata, VpnClientMetadata, VpnClientMfaMetadata,
        },
        ActivityLogEvent, ActivityLogModule, EventType,
    },
    NoId,
};
use error::EventLoggerError;
use message::{
    DefguardEvent, EnrollmentEvent, EventContext, EventLoggerMessage, LoggerEvent, VpnEvent,
};
use sqlx::PgPool;
use tokio::sync::mpsc::UnboundedReceiver;
use tracing::{debug, error, info, trace};

pub mod error;
pub mod message;

const MESSAGE_LIMIT: usize = 100;

/// Run the event logger service
///
/// This function runs in an infinite loop, receiving messages from the event_logger_rx channel
/// and storing them in the database as activity log events.
pub async fn run_event_logger(
    pool: PgPool,
    mut event_logger_rx: UnboundedReceiver<EventLoggerMessage>,
    activity_log_messages_tx: tokio::sync::broadcast::Sender<Bytes>,
) -> Result<(), EventLoggerError> {
    info!("Starting activity log event logger service");

    // Receive messages in an infinite loop
    loop {
        // Collect multiple messages from the channel (up to MESSAGE_LIMIT at a time)
        let mut message_buffer: Vec<EventLoggerMessage> = Vec::with_capacity(MESSAGE_LIMIT);
        let message_count = event_logger_rx
            .recv_many(&mut message_buffer, MESSAGE_LIMIT)
            .await;

        debug!("Processing batch of {message_count} activity log events");

        let mut transaction = pool.begin().await?;
        let mut serialized_activity_log_events = String::new();

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

            // Convert each message to a related activity log event
            let activity_log_event = {
                let (module, event, metadata) = match message.event {
                    LoggerEvent::Defguard(event) => {
                        let module = ActivityLogModule::Defguard;

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
                            DefguardEvent::ActivityLogStreamCreated {
                                stream_id,
                                stream_name,
                            } => (
                                EventType::ActivityLogStreamCreated,
                                serde_json::to_value(ActivityLogStreamMetadata {
                                    id: stream_id,
                                    name: stream_name,
                                })
                                .ok(),
                            ),
                            DefguardEvent::ActivityLogStreamRemoved {
                                stream_id,
                                stream_name,
                            } => (
                                EventType::ActivityLogStreamRemoved,
                                serde_json::to_value(ActivityLogStreamMetadata {
                                    id: stream_id,
                                    name: stream_name,
                                })
                                .ok(),
                            ),
                            DefguardEvent::ActivityLogStreamModified {
                                stream_id,
                                stream_name,
                            } => (
                                EventType::ActivityLogStreamModified,
                                serde_json::to_value(ActivityLogStreamMetadata {
                                    id: stream_id,
                                    name: stream_name,
                                })
                                .ok(),
                            ),
                        };
                        (module, event_type, metadata)
                    }
                    LoggerEvent::Client(_event) => {
                        let _module = ActivityLogModule::Client;
                        unimplemented!()
                    }
                    LoggerEvent::Vpn(event) => {
                        let module = ActivityLogModule::Vpn;
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
                        let module = ActivityLogModule::Enrollment;
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

                ActivityLogEvent {
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

            match serde_json::to_string(&activity_log_event) {
                Ok(serialized_activity_log_event) => {
                    serialized_activity_log_events += &(serialized_activity_log_event + "\n");
                }
                Err(e) => {
                    error!("Failed to serialize activity log event. Reason: {e}");
                }
            }

            // Store activity log event in DB
            // TODO: do batch inserts
            activity_log_event.save(&mut *transaction).await?;
        }

        // Send serialized events
        if !serialized_activity_log_events.is_empty() {
            let in_bytes = bytes::Bytes::from(serialized_activity_log_events);
            if let Err(send_err) = activity_log_messages_tx.send(in_bytes) {
                trace!("Sending serialized activity log events message failed. Most likely because there is no listeners. Reason: {send_err}");
            }
        }

        // Commit the transaction
        transaction.commit().await?;
    }
}
