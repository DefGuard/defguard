use bytes::Bytes;
use defguard_common::db::NoId;
use defguard_core::db::models::activity_log::{
    ActivityLogEvent, ActivityLogModule, EventType,
    metadata::{
        ActivityLogStreamMetadata, ActivityLogStreamModifiedMetadata, ApiTokenMetadata,
        ApiTokenRenamedMetadata, AuthenticationKeyMetadata, AuthenticationKeyRenamedMetadata,
        ClientConfigurationTokenMetadata, DeviceMetadata, DeviceModifiedMetadata,
        EnrollmentDeviceAddedMetadata, EnrollmentTokenMetadata, GatewayDeletedMetadata,
        GatewayModifiedMetadata, GroupAssignedMetadata, GroupMembersModifiedMetadata,
        GroupMetadata, GroupModifiedMetadata, GroupsBulkAssignedMetadata, LoginFailedMetadata,
        MfaLoginFailedMetadata, MfaLoginMetadata, MfaSecurityKeyMetadata, NetworkDeviceMetadata,
        NetworkDeviceModifiedMetadata, OpenIdAppMetadata, OpenIdAppModifiedMetadata,
        OpenIdAppStateChangedMetadata, OpenIdProviderMetadata, PasswordChangedByAdminMetadata,
        PasswordResetMetadata, ProxyDeletedMetadata, ProxyModifiedMetadata, SettingsUpdateMetadata,
        UserGroupsModifiedMetadata, UserMetadata, UserMfaDisabledMetadata, UserModifiedMetadata,
        UserSnatBindingMetadata, UserSnatBindingModifiedMetadata, VpnClientMetadata,
        VpnClientMfaFailedMetadata, VpnClientMfaMetadata, VpnLocationMetadata,
        VpnLocationModifiedMetadata, WebHookMetadata, WebHookModifiedMetadata,
        WebHookStateChangedMetadata,
    },
};
use description::{
    get_defguard_event_description, get_enrollment_event_description, get_vpn_event_description,
};
use error::EventLoggerError;
use message::{
    DefguardEvent, EnrollmentEvent, EventContext, EventLoggerMessage, LoggerEvent, VpnEvent,
};
use sqlx::PgPool;
use tokio::sync::mpsc::UnboundedReceiver;
use tracing::{debug, error, info, trace};

pub mod description;
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
                location,
                timestamp,
                ip,
                device,
            } = message.context;

            // Convert each message to a related activity log event
            let activity_log_event = {
                let (module, event, description, metadata) = match message.event {
                    LoggerEvent::Defguard(event) => {
                        let module = ActivityLogModule::Defguard;
                        let description = get_defguard_event_description(&event);

                        let (event_type, metadata) = match *event {
                            DefguardEvent::UserLogin => (EventType::UserLogin, None),
                            DefguardEvent::UserLoginFailed { message } => (
                                EventType::UserLoginFailed,
                                serde_json::to_value(LoginFailedMetadata { message }).ok(),
                            ),
                            DefguardEvent::UserMfaLogin { mfa_method } => (
                                EventType::UserMfaLogin,
                                serde_json::to_value(MfaLoginMetadata { mfa_method }).ok(),
                            ),
                            DefguardEvent::UserMfaLoginFailed {
                                mfa_method,
                                message,
                            } => (
                                EventType::UserMfaLoginFailed,
                                serde_json::to_value(MfaLoginFailedMetadata {
                                    mfa_method,
                                    message,
                                })
                                .ok(),
                            ),
                            DefguardEvent::UserLogout => (EventType::UserLogout, None),
                            DefguardEvent::UserDeviceAdded { owner, device } => (
                                EventType::DeviceAdded,
                                serde_json::to_value(DeviceMetadata {
                                    owner: owner.into(),
                                    device,
                                })
                                .ok(),
                            ),
                            DefguardEvent::UserDeviceRemoved { owner, device } => (
                                EventType::DeviceRemoved,
                                serde_json::to_value(DeviceMetadata {
                                    owner: owner.into(),
                                    device,
                                })
                                .ok(),
                            ),
                            DefguardEvent::UserDeviceModified {
                                owner,
                                before,
                                after,
                            } => (
                                EventType::DeviceModified,
                                serde_json::to_value(DeviceModifiedMetadata {
                                    owner: owner.into(),
                                    before,
                                    after,
                                })
                                .ok(),
                            ),
                            DefguardEvent::UserGroupsModified {
                                user,
                                before,
                                after,
                            } => (
                                EventType::UserGroupsModified,
                                serde_json::to_value(UserGroupsModifiedMetadata {
                                    user: user.into(),
                                    before,
                                    after,
                                })
                                .ok(),
                            ),
                            DefguardEvent::RecoveryCodeUsed => (EventType::RecoveryCodeUsed, None),
                            DefguardEvent::PasswordChanged => (EventType::PasswordChanged, None),
                            DefguardEvent::PasswordChangedByAdmin { user } => (
                                EventType::PasswordChangedByAdmin,
                                serde_json::to_value(PasswordChangedByAdminMetadata {
                                    user: user.into(),
                                })
                                .ok(),
                            ),
                            DefguardEvent::MfaDisabled => (EventType::MfaDisabled, None),
                            DefguardEvent::UserMfaDisabled { user } => (
                                EventType::UserMfaDisabled,
                                serde_json::to_value(UserMfaDisabledMetadata { user: user.into() })
                                    .ok(),
                            ),
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
                                serde_json::to_value(AuthenticationKeyMetadata { key: key.into() })
                                    .ok(),
                            ),
                            DefguardEvent::AuthenticationKeyRemoved { key } => (
                                EventType::AuthenticationKeyRemoved,
                                serde_json::to_value(AuthenticationKeyMetadata { key: key.into() })
                                    .ok(),
                            ),
                            DefguardEvent::AuthenticationKeyRenamed {
                                key,
                                old_name,
                                new_name,
                            } => (
                                EventType::AuthenticationKeyRenamed,
                                serde_json::to_value(AuthenticationKeyRenamedMetadata {
                                    key: key.into(),
                                    old_name,
                                    new_name,
                                })
                                .ok(),
                            ),
                            DefguardEvent::ApiTokenAdded { owner, token } => (
                                EventType::ApiTokenAdded,
                                serde_json::to_value(ApiTokenMetadata {
                                    owner: owner.into(),
                                    token: token.into(),
                                })
                                .ok(),
                            ),
                            DefguardEvent::ApiTokenRemoved { owner, token } => (
                                EventType::ApiTokenRemoved,
                                serde_json::to_value(ApiTokenMetadata {
                                    owner: owner.into(),
                                    token: token.into(),
                                })
                                .ok(),
                            ),
                            DefguardEvent::ApiTokenRenamed {
                                owner,
                                token,
                                old_name,
                                new_name,
                            } => (
                                EventType::ApiTokenRenamed,
                                serde_json::to_value(ApiTokenRenamedMetadata {
                                    owner: owner.into(),
                                    token: token.into(),
                                    old_name,
                                    new_name,
                                })
                                .ok(),
                            ),
                            DefguardEvent::UserAdded { user } => (
                                EventType::UserAdded,
                                serde_json::to_value(UserMetadata { user: user.into() }).ok(),
                            ),
                            DefguardEvent::UserRemoved { user } => (
                                EventType::UserRemoved,
                                serde_json::to_value(UserMetadata { user: user.into() }).ok(),
                            ),
                            DefguardEvent::UserModified { before, after } => (
                                EventType::UserModified,
                                serde_json::to_value(UserModifiedMetadata {
                                    before: before.into(),
                                    after: after.into(),
                                })
                                .ok(),
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
                                serde_json::to_value(OpenIdAppMetadata { app: app.into() }).ok(),
                            ),
                            DefguardEvent::OpenIdAppRemoved { app } => (
                                EventType::OpenIdAppRemoved,
                                serde_json::to_value(OpenIdAppMetadata { app: app.into() }).ok(),
                            ),
                            DefguardEvent::OpenIdAppModified { before, after } => (
                                EventType::OpenIdAppModified,
                                serde_json::to_value(OpenIdAppModifiedMetadata {
                                    before: before.into(),
                                    after: after.into(),
                                })
                                .ok(),
                            ),
                            DefguardEvent::OpenIdAppStateChanged { app, enabled } => (
                                EventType::OpenIdAppStateChanged,
                                serde_json::to_value(OpenIdAppStateChangedMetadata {
                                    app: app.into(),
                                    enabled,
                                })
                                .ok(),
                            ),
                            DefguardEvent::OpenIdProviderModified { provider } => (
                                EventType::OpenIdProviderModified,
                                serde_json::to_value(OpenIdProviderMetadata {
                                    provider: provider.into(),
                                })
                                .ok(),
                            ),
                            DefguardEvent::OpenIdProviderRemoved { provider } => (
                                EventType::OpenIdProviderRemoved,
                                serde_json::to_value(OpenIdProviderMetadata {
                                    provider: provider.into(),
                                })
                                .ok(),
                            ),
                            DefguardEvent::SettingsUpdatedPartial { before, after } => (
                                EventType::SettingsUpdatedPartial,
                                serde_json::to_value(SettingsUpdateMetadata {
                                    before: before.into(),
                                    after: after.into(),
                                })
                                .ok(),
                            ),
                            DefguardEvent::SettingsUpdated { before, after } => (
                                EventType::SettingsUpdated,
                                serde_json::to_value(SettingsUpdateMetadata {
                                    before: before.into(),
                                    after: after.into(),
                                })
                                .ok(),
                            ),
                            DefguardEvent::SettingsDefaultBrandingRestored => {
                                (EventType::SettingsDefaultBrandingRestored, None)
                            }
                            DefguardEvent::ActivityLogStreamCreated { stream } => (
                                EventType::ActivityLogStreamCreated,
                                serde_json::to_value(ActivityLogStreamMetadata {
                                    stream: stream.into(),
                                })
                                .ok(),
                            ),
                            DefguardEvent::ActivityLogStreamRemoved { stream } => (
                                EventType::ActivityLogStreamRemoved,
                                serde_json::to_value(ActivityLogStreamMetadata {
                                    stream: stream.into(),
                                })
                                .ok(),
                            ),
                            DefguardEvent::ActivityLogStreamModified { before, after } => (
                                EventType::ActivityLogStreamModified,
                                serde_json::to_value(ActivityLogStreamModifiedMetadata {
                                    before: before.into(),
                                    after: after.into(),
                                })
                                .ok(),
                            ),
                            DefguardEvent::GroupsBulkAssigned { users, groups } => (
                                EventType::GroupsBulkAssigned,
                                serde_json::to_value(GroupsBulkAssignedMetadata {
                                    users: users.into_iter().map(Into::into).collect(),
                                    groups,
                                })
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
                                serde_json::to_value(GroupAssignedMetadata {
                                    group,
                                    user: user.into(),
                                })
                                .ok(),
                            ),
                            DefguardEvent::GroupMemberRemoved { group, user } => (
                                EventType::GroupMemberRemoved,
                                serde_json::to_value(GroupAssignedMetadata {
                                    group,
                                    user: user.into(),
                                })
                                .ok(),
                            ),
                            DefguardEvent::GroupMembersModified {
                                group,
                                added,
                                removed,
                            } => (
                                EventType::GroupMembersModified,
                                serde_json::to_value(GroupMembersModifiedMetadata {
                                    group,
                                    added: added.into_iter().map(Into::into).collect(),
                                    removed: removed.into_iter().map(Into::into).collect(),
                                })
                                .ok(),
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
                                serde_json::to_value(PasswordResetMetadata { user: user.into() })
                                    .ok(),
                            ),
                            DefguardEvent::ClientConfigurationTokenAdded { user } => (
                                EventType::ClientConfigurationTokenAdded,
                                serde_json::to_value(ClientConfigurationTokenMetadata {
                                    user: user.into(),
                                })
                                .ok(),
                            ),
                            DefguardEvent::UserSnatBindingAdded { user, binding } => (
                                EventType::UserSnatBindingAdded,
                                serde_json::to_value(UserSnatBindingMetadata {
                                    user: user.into(),
                                    binding,
                                })
                                .ok(),
                            ),
                            DefguardEvent::UserSnatBindingRemoved { user, binding } => (
                                EventType::UserSnatBindingRemoved,
                                serde_json::to_value(UserSnatBindingMetadata {
                                    user: user.into(),
                                    binding,
                                })
                                .ok(),
                            ),
                            DefguardEvent::UserSnatBindingModified {
                                user,
                                before,
                                after,
                            } => (
                                EventType::UserSnatBindingModified,
                                serde_json::to_value(UserSnatBindingModifiedMetadata {
                                    user: user.into(),
                                    before,
                                    after,
                                })
                                .ok(),
                            ),
                            DefguardEvent::ProxyModified { before, after } => (
                                EventType::ProxyModified,
                                serde_json::to_value(ProxyModifiedMetadata { before, after }).ok(),
                            ),
                            DefguardEvent::ProxyDeleted { proxy } => (
                                EventType::ProxyDeleted,
                                serde_json::to_value(ProxyDeletedMetadata { proxy }).ok(),
                            ),
                            DefguardEvent::GatewayModified { before, after } => (
                                EventType::GatewayModified,
                                serde_json::to_value(GatewayModifiedMetadata { before, after })
                                    .ok(),
                            ),
                            DefguardEvent::GatewayDeleted { gateway } => (
                                EventType::GatewayDeleted,
                                serde_json::to_value(GatewayDeletedMetadata { gateway }).ok(),
                            ),
                        };
                        (module, event_type, description, metadata)
                    }
                    LoggerEvent::Vpn(event) => {
                        let module = ActivityLogModule::Vpn;
                        let description = get_vpn_event_description(&event);

                        let (event_type, metadata) = match *event {
                            VpnEvent::ClientMfaFailed {
                                location,
                                device,
                                method,
                                message,
                            } => (
                                EventType::VpnClientMfaFailed,
                                serde_json::to_value(VpnClientMfaFailedMetadata {
                                    location,
                                    device,
                                    method,
                                    message,
                                })
                                .ok(),
                            ),
                            VpnEvent::ClientMfaSuccess {
                                location,
                                device,
                                method,
                            } => (
                                EventType::VpnClientMfaSuccess,
                                serde_json::to_value(VpnClientMfaMetadata {
                                    location,
                                    device,
                                    method,
                                })
                                .ok(),
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
                        (module, event_type, description, metadata)
                    }
                    LoggerEvent::Enrollment(event) => {
                        let module = ActivityLogModule::Enrollment;
                        let description = get_enrollment_event_description(&event);

                        let (event_type, metadata) = match *event {
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
                                serde_json::to_value(EnrollmentTokenMetadata { user: user.into() })
                                    .ok(),
                            ),
                        };
                        (module, event_type, description, metadata)
                    }
                };

                ActivityLogEvent {
                    id: NoId,
                    timestamp,
                    user_id,
                    username,
                    location,
                    ip: ip.into(),
                    event,
                    module,
                    device,
                    description,
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
                trace!(
                    "Sending serialized activity log events message failed. Most likely because there is no listeners. Reason: {send_err}"
                );
            }
        }

        // Commit the transaction
        transaction.commit().await?;
    }
}
