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
            ApiTokenMetadata, ApiTokenRenamedMetadata, AuditStreamMetadata,
            AuditStreamModifiedMetadata, AuthenticationKeyMetadata,
            AuthenticationKeyRenamedMetadata, ClientConfigurationTokenMetadata, DeviceMetadata,
            DeviceModifiedMetadata, EnrollmentDeviceAddedMetadata, EnrollmentTokenMetadata,
            GroupAssignedMetadata, GroupMetadata, GroupModifiedMetadata,
            GroupsBulkAssignedMetadata, MfaLoginMetadata, MfaSecurityKeyMetadata,
            NetworkDeviceMetadata, NetworkDeviceModifiedMetadata, OpenIdAppMetadata,
            OpenIdAppModifiedMetadata, OpenIdAppStateChangedMetadata, OpenIdProviderMetadata,
            PasswordChangedByAdminMetadata, PasswordResetMetadata, UserMetadata,
            UserModifiedMetadata, VpnClientMetadata, VpnClientMfaMetadata, VpnLocationMetadata,
            VpnLocationModifiedMetadata, WebHookMetadata, WebHookModifiedMetadata,
            WebHookStateChangedMetadata,
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
                            DefguardEvent::UserDeviceAdded { owner, device } => (
                                EventType::DeviceAdded,
                                serde_json::to_value(DeviceMetadata { owner, device }).ok(),
                            ),
                            DefguardEvent::UserDeviceRemoved { owner, device } => (
                                EventType::DeviceRemoved,
                                serde_json::to_value(DeviceMetadata { owner, device }).ok(),
                            ),
                            DefguardEvent::UserDeviceModified {
                                owner,
                                before,
                                after,
                            } => (
                                EventType::DeviceModified,
                                serde_json::to_value(DeviceModifiedMetadata {
                                    owner,
                                    before,
                                    after,
                                })
                                .ok(),
                            ),
                            DefguardEvent::RecoveryCodeUsed => (EventType::RecoveryCodeUsed, None),
                            DefguardEvent::PasswordChanged => (EventType::PasswordChanged, None),
                            DefguardEvent::PasswordChangedByAdmin { user } => (
                                EventType::PasswordChangedByAdmin,
                                serde_json::to_value(PasswordChangedByAdminMetadata { user }).ok(),
                            ),
                            DefguardEvent::MfaDisabled => (EventType::MfaDisabled, None),
                            DefguardEvent::MfaTotpEnabled => (EventType::MfaTotpEnabled, None),
                            DefguardEvent::MfaTotpDisabled => (EventType::MfaTotpDisabled, None),
                            DefguardEvent::MfaEmailEnabled => (EventType::MfaEmailEnabled, None),
                            DefguardEvent::MfaEmailDisabled => (EventType::MfaEmailDisabled, None),
                            DefguardEvent::MfaSecurityKeyAdded { key } => (
                                EventType::MfaSecurityKeyAdded,
                                serde_json::to_value(MfaSecurityKeyMetadata { key: key.into() })
                                    .ok(),
                            ),
                            DefguardEvent::MfaSecurityKeyRemoved { key } => (
                                EventType::MfaSecurityKeyRemoved,
                                serde_json::to_value(MfaSecurityKeyMetadata { key: key.into() })
                                    .ok(),
                            ),
                            DefguardEvent::AuthenticationKeyAdded { key } => (
                                EventType::AuthenticationKeyAdded,
                                serde_json::to_value(AuthenticationKeyMetadata { key }).ok(),
                            ),
                            DefguardEvent::AuthenticationKeyRemoved { key } => (
                                EventType::AuthenticationKeyRemoved,
                                serde_json::to_value(AuthenticationKeyMetadata { key }).ok(),
                            ),
                            DefguardEvent::AuthenticationKeyRenamed {
                                key,
                                old_name,
                                new_name,
                            } => (
                                EventType::AuthenticationKeyRenamed,
                                serde_json::to_value(AuthenticationKeyRenamedMetadata {
                                    key,
                                    old_name,
                                    new_name,
                                })
                                .ok(),
                            ),
                            DefguardEvent::ApiTokenAdded { owner, token } => (
                                EventType::ApiTokenAdded,
                                serde_json::to_value(ApiTokenMetadata { owner, token }).ok(),
                            ),
                            DefguardEvent::ApiTokenRemoved { owner, token } => (
                                EventType::ApiTokenRemoved,
                                serde_json::to_value(ApiTokenMetadata { owner, token }).ok(),
                            ),
                            DefguardEvent::ApiTokenRenamed {
                                owner,
                                token,
                                old_name,
                                new_name,
                            } => (
                                EventType::ApiTokenRenamed,
                                serde_json::to_value(ApiTokenRenamedMetadata {
                                    owner,
                                    token,
                                    old_name,
                                    new_name,
                                })
                                .ok(),
                            ),
                            DefguardEvent::UserAdded { user } => (
                                EventType::UserAdded,
                                serde_json::to_value(UserMetadata { user }).ok(),
                            ),
                            DefguardEvent::UserRemoved { user } => (
                                EventType::UserRemoved,
                                serde_json::to_value(UserMetadata { user }).ok(),
                            ),
                            DefguardEvent::UserModified { before, after } => (
                                EventType::UserModified,
                                serde_json::to_value(UserModifiedMetadata { before, after }).ok(),
                            ),
                            DefguardEvent::NetworkDeviceAdded { device, location } => (
                                EventType::NetworkDeviceAdded,
                                serde_json::to_value(NetworkDeviceMetadata { device, location })
                                    .ok(),
                            ),
                            DefguardEvent::NetworkDeviceRemoved { device, location } => (
                                EventType::NetworkDeviceRemoved,
                                serde_json::to_value(NetworkDeviceMetadata { device, location })
                                    .ok(),
                            ),
                            DefguardEvent::NetworkDeviceModified {
                                location,
                                before,
                                after,
                            } => (
                                EventType::NetworkDeviceModified,
                                serde_json::to_value(NetworkDeviceModifiedMetadata {
                                    before,
                                    after,
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
                            DefguardEvent::VpnLocationModified { before, after } => (
                                EventType::VpnLocationModified,
                                serde_json::to_value(VpnLocationModifiedMetadata { before, after })
                                    .ok(),
                            ),
                            DefguardEvent::OpenIdAppAdded { app } => (
                                EventType::OpenIdAppAdded,
                                serde_json::to_value(OpenIdAppMetadata { app }).ok(),
                            ),
                            DefguardEvent::OpenIdAppRemoved { app } => (
                                EventType::OpenIdAppRemoved,
                                serde_json::to_value(OpenIdAppMetadata { app }).ok(),
                            ),
                            DefguardEvent::OpenIdAppModified { before, after } => (
                                EventType::OpenIdAppModified,
                                serde_json::to_value(OpenIdAppModifiedMetadata { before, after })
                                    .ok(),
                            ),
                            DefguardEvent::OpenIdAppStateChanged { app, enabled } => (
                                EventType::OpenIdAppStateChanged,
                                serde_json::to_value(OpenIdAppStateChangedMetadata {
                                    app,
                                    enabled,
                                })
                                .ok(),
                            ),
                            DefguardEvent::OpenIdProviderModified { provider } => (
                                EventType::OpenIdProviderModified,
                                serde_json::to_value(OpenIdProviderMetadata { provider }).ok(),
                            ),
                            DefguardEvent::OpenIdProviderRemoved { provider } => (
                                EventType::OpenIdProviderRemoved,
                                serde_json::to_value(OpenIdProviderMetadata { provider }).ok(),
                            ),
                            DefguardEvent::SettingsUpdated => (EventType::SettingsUpdated, None),
                            DefguardEvent::SettingsUpdatedPartial => {
                                (EventType::SettingsUpdatedPartial, None)
                            }
                            DefguardEvent::SettingsDefaultBrandingRestored => {
                                (EventType::SettingsDefaultBrandingRestored, None)
                            }
                            DefguardEvent::AuditStreamCreated { stream } => (
                                EventType::AuditStreamCreated,
                                serde_json::to_value(AuditStreamMetadata { stream }).ok(),
                            ),
                            DefguardEvent::AuditStreamRemoved { stream } => (
                                EventType::AuditStreamRemoved,
                                serde_json::to_value(AuditStreamMetadata { stream }).ok(),
                            ),
                            DefguardEvent::AuditStreamModified { before, after } => (
                                EventType::AuditStreamModified,
                                serde_json::to_value(AuditStreamModifiedMetadata { before, after })
                                    .ok(),
                            ),
                            DefguardEvent::GroupsBulkAssigned { users, groups } => (
                                EventType::GroupsBulkAssigned,
                                serde_json::to_value(GroupsBulkAssignedMetadata { users, groups })
                                    .ok(),
                            ),
                            DefguardEvent::GroupAdded { group } => (
                                EventType::GroupAdded,
                                serde_json::to_value(GroupMetadata { group }).ok(),
                            ),
                            DefguardEvent::GroupModified { before, after } => (
                                EventType::GroupModified,
                                serde_json::to_value(GroupModifiedMetadata { before, after }).ok(),
                            ),
                            DefguardEvent::GroupRemoved { group } => (
                                EventType::GroupRemoved,
                                serde_json::to_value(GroupMetadata { group }).ok(),
                            ),
                            DefguardEvent::GroupMemberAdded { group, user } => (
                                EventType::GroupMemberAdded,
                                serde_json::to_value(GroupAssignedMetadata { group, user }).ok(),
                            ),
                            DefguardEvent::GroupMemberRemoved { group, user } => (
                                EventType::GroupMemberRemoved,
                                serde_json::to_value(GroupAssignedMetadata { group, user }).ok(),
                            ),
                            DefguardEvent::WebHookAdded { webhook } => (
                                EventType::WebHookAdded,
                                serde_json::to_value(WebHookMetadata { webhook }).ok(),
                            ),
                            DefguardEvent::WebHookModified { before, after } => (
                                EventType::WebHookModified,
                                serde_json::to_value(WebHookModifiedMetadata { before, after })
                                    .ok(),
                            ),
                            DefguardEvent::WebHookRemoved { webhook } => (
                                EventType::WebHookRemoved,
                                serde_json::to_value(WebHookMetadata { webhook }).ok(),
                            ),
                            DefguardEvent::WebHookStateChanged { webhook, enabled } => (
                                EventType::WebHookStateChanged,
                                serde_json::to_value(WebHookStateChangedMetadata {
                                    webhook,
                                    enabled,
                                })
                                .ok(),
                            ),
                            DefguardEvent::PasswordReset { user } => (
                                EventType::PasswordReset,
                                serde_json::to_value(PasswordResetMetadata { user }).ok(),
                            ),
                            DefguardEvent::ClientConfigurationTokenAdded { user } => (
                                EventType::ClientConfigurationTokenAdded,
                                serde_json::to_value(ClientConfigurationTokenMetadata { user })
                                    .ok(),
                            ),
                            DefguardEvent::EnrollmentTokenAdded { user } => (
                                EventType::EnrollmentTokenAdded,
                                serde_json::to_value(EnrollmentTokenMetadata { user }).ok(),
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
                            EnrollmentEvent::TokenAdded { user } => (
                                EventType::EnrollmentTokenAdded,
                                serde_json::to_value(EnrollmentTokenMetadata { user }).ok(),
                            ),
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
