use std::sync::Arc;

use bytes::Bytes;
use defguard_common::db::NoId;
use defguard_core::{
    db::models::activity_log::{
        ActivityLogEvent, ActivityLogModule, EventType,
        metadata::{
            ActivityLogStreamMetadata, ActivityLogStreamModifiedMetadata, ApiTokenMetadata,
            ApiTokenRenamedMetadata, AuthenticationKeyMetadata, AuthenticationKeyRenamedMetadata,
            ClientConfigurationTokenMetadata, DeviceMetadata, DeviceModifiedMetadata,
            EnrollmentDeviceAddedMetadata, EnrollmentTokenMetadata, GatewayDeletedMetadata,
            GatewayModifiedMetadata, GroupAssignedMetadata, GroupMembersModifiedMetadata,
            GroupMetadata, GroupModifiedMetadata, GroupsBulkAssignedMetadata, LoginFailedMetadata,
            MfaLoginFailedMetadata, MfaLoginMetadata, MfaSecurityKeyMetadata,
            NetworkDeviceMetadata, NetworkDeviceModifiedMetadata, OpenIdAppMetadata,
            OpenIdAppModifiedMetadata, OpenIdAppStateChangedMetadata, OpenIdProviderMetadata,
            PasswordChangedByAdminMetadata, PasswordResetMetadata, ProxyDeletedMetadata,
            ProxyModifiedMetadata, SettingsUpdateMetadata, UserGroupsModifiedMetadata,
            UserMetadata, UserMfaDisabledMetadata, UserModifiedMetadata, UserSnatBindingMetadata,
            UserSnatBindingModifiedMetadata, VpnClientMetadata, VpnClientMfaFailedMetadata,
            VpnClientMfaMetadata, VpnLocationMetadata, VpnLocationModifiedMetadata,
            WebHookMetadata, WebHookModifiedMetadata, WebHookStateChangedMetadata,
        },
    },
    events::{
        ApiEvent, ApiEventType, BidiStreamEvent, BidiStreamEventType, DesktopClientMfaEvent,
        PasswordResetEvent,
    },
};
use defguard_session_manager::events::{SessionManagerEvent, SessionManagerEventType};
use description::{get_api_event_description, get_enrollment_event_description};
use error::EventLoggerError;
use message::{Event, EventContext, EventLoggerMessage};
use sqlx::PgPool;
use tokio::sync::{Notify, mpsc::UnboundedReceiver};
use tracing::{debug, error, info, trace};

pub mod description;
pub mod error;
pub mod message;

const MESSAGE_LIMIT: usize = 100;

/// Run the event logger service
pub async fn run_event_logger(
    pool: PgPool,
    api_event_rx: UnboundedReceiver<ApiEvent>,
    bidi_event_rx: UnboundedReceiver<BidiStreamEvent>,
    session_manager_event_rx: UnboundedReceiver<SessionManagerEvent>,
    activity_log_stream_reload_notify: Arc<Notify>,
    activity_log_messages_tx: tokio::sync::broadcast::Sender<Bytes>,
) -> Result<(), EventLoggerError> {
    let (event_logger_tx, mut event_logger_rx) =
        tokio::sync::mpsc::unbounded_channel::<EventLoggerMessage>();

    // Spawn a task that reads from all three source channels and forwards
    // translated messages to the internal channel.
    tokio::spawn(translate_and_forward(
        api_event_rx,
        bidi_event_rx,
        session_manager_event_rx,
        activity_log_stream_reload_notify,
        event_logger_tx,
    ));

    info!("Starting activity log event logger service");

    loop {
        let mut message_buffer = Vec::with_capacity(MESSAGE_LIMIT);
        let message_count = event_logger_rx
            .recv_many(&mut message_buffer, MESSAGE_LIMIT)
            .await;

        if message_count == 0 {
            info!("Event logger channel closed, shutting down");
            return Ok(());
        }

        debug!("Processing batch of {message_count} activity log events");

        if let Err(e) = process_batch(&pool, message_buffer, &activity_log_messages_tx).await {
            error!("Failed to process activity log event batch, batch will be discarded: {e}");
        }
    }
}

/// Reads from all three event source channels, translates each event into
/// an `EventLoggerMessage`, and forwards it to the batch processing loop.
/// When any source channel closes, the task exits and the forwarding channel
/// is dropped, causing the batch loop to shut down gracefully.
async fn translate_and_forward(
    mut api_event_rx: UnboundedReceiver<ApiEvent>,
    mut bidi_event_rx: UnboundedReceiver<BidiStreamEvent>,
    mut session_manager_event_rx: UnboundedReceiver<SessionManagerEvent>,
    reload_notify: Arc<Notify>,
    event_logger_tx: tokio::sync::mpsc::UnboundedSender<EventLoggerMessage>,
) {
    loop {
        let message = tokio::select! {
            event = api_event_rx.recv() => match event {
                Some(e) => EventLoggerMessage::from_api_event(e, &reload_notify),
                None => {
                    error!("API event channel closed");
                    break;
                }
            },
            event = bidi_event_rx.recv() => match event {
                Some(e) => EventLoggerMessage::from_bidi_event(e),
                None => {
                    error!("Bidi gRPC stream event channel closed");
                    break;
                }
            },
            event = session_manager_event_rx.recv() => match event {
                Some(e) => EventLoggerMessage::from_session_manager_event(e),
                None => {
                    error!("Session manager event channel closed");
                    break;
                }
            },
        };

        if event_logger_tx.send(message).is_err() {
            debug!("Event logger channel closed, translation task shutting down");
            break;
        }
    }
}

/// Convert an event logger message into an activity log database row.
fn map_to_activity_log_event(message: EventLoggerMessage) -> ActivityLogEvent<NoId> {
    let EventContext {
        user_id,
        username,
        location,
        timestamp,
        ip,
        device,
    } = message.context;

    let (module, event, description, metadata) = match message.event {
        Event::Api(event) => {
            let module = ActivityLogModule::Defguard;
            let description = get_api_event_description(&event);

            let (event_type, metadata) = match event {
                ApiEventType::UserLogin => (EventType::UserLogin, None),
                ApiEventType::UserLoginFailed { message } => (
                    EventType::UserLoginFailed,
                    serde_json::to_value(LoginFailedMetadata { message }).ok(),
                ),
                ApiEventType::UserMfaLogin { mfa_method } => (
                    EventType::UserMfaLogin,
                    serde_json::to_value(MfaLoginMetadata { mfa_method }).ok(),
                ),
                ApiEventType::UserMfaLoginFailed {
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
                ApiEventType::RecoveryCodeLoginFailed => (
                    EventType::UserMfaLoginFailed,
                    serde_json::to_value(LoginFailedMetadata {
                        message: "Recovery code verification failed".to_owned(),
                    })
                    .ok(),
                ),
                ApiEventType::UserLogout => (EventType::UserLogout, None),
                ApiEventType::UserDeviceAdded { owner, device } => (
                    EventType::DeviceAdded,
                    serde_json::to_value(DeviceMetadata {
                        owner: owner.into(),
                        device,
                    })
                    .ok(),
                ),
                ApiEventType::UserDeviceRemoved { owner, device } => (
                    EventType::DeviceRemoved,
                    serde_json::to_value(DeviceMetadata {
                        owner: owner.into(),
                        device,
                    })
                    .ok(),
                ),
                ApiEventType::UserDeviceModified {
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
                ApiEventType::UserGroupsModified {
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
                ApiEventType::RecoveryCodeUsed => (EventType::RecoveryCodeUsed, None),
                ApiEventType::PasswordChanged => (EventType::PasswordChanged, None),
                ApiEventType::PasswordChangedByAdmin { user } => (
                    EventType::PasswordChangedByAdmin,
                    serde_json::to_value(PasswordChangedByAdminMetadata { user: user.into() }).ok(),
                ),
                ApiEventType::MfaDisabled => (EventType::MfaDisabled, None),
                ApiEventType::UserMfaDisabled { user } => (
                    EventType::UserMfaDisabled,
                    serde_json::to_value(UserMfaDisabledMetadata { user: user.into() }).ok(),
                ),
                ApiEventType::MfaTotpEnabled => (EventType::MfaTotpEnabled, None),
                ApiEventType::MfaTotpDisabled => (EventType::MfaTotpDisabled, None),
                ApiEventType::MfaEmailEnabled => (EventType::MfaEmailEnabled, None),
                ApiEventType::MfaEmailDisabled => (EventType::MfaEmailDisabled, None),
                ApiEventType::MfaSecurityKeyAdded { key } => (
                    EventType::MfaSecurityKeyAdded,
                    serde_json::to_value(MfaSecurityKeyMetadata { key: key.into() }).ok(),
                ),
                ApiEventType::MfaSecurityKeyRemoved { key } => (
                    EventType::MfaSecurityKeyRemoved,
                    serde_json::to_value(MfaSecurityKeyMetadata { key: key.into() }).ok(),
                ),
                ApiEventType::AuthenticationKeyAdded { key } => (
                    EventType::AuthenticationKeyAdded,
                    serde_json::to_value(AuthenticationKeyMetadata { key: key.into() }).ok(),
                ),
                ApiEventType::AuthenticationKeyRemoved { key } => (
                    EventType::AuthenticationKeyRemoved,
                    serde_json::to_value(AuthenticationKeyMetadata { key: key.into() }).ok(),
                ),
                ApiEventType::AuthenticationKeyRenamed {
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
                ApiEventType::ApiTokenAdded { owner, token } => (
                    EventType::ApiTokenAdded,
                    serde_json::to_value(ApiTokenMetadata {
                        owner: owner.into(),
                        token: token.into(),
                    })
                    .ok(),
                ),
                ApiEventType::ApiTokenRemoved { owner, token } => (
                    EventType::ApiTokenRemoved,
                    serde_json::to_value(ApiTokenMetadata {
                        owner: owner.into(),
                        token: token.into(),
                    })
                    .ok(),
                ),
                ApiEventType::ApiTokenRenamed {
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
                ApiEventType::UserAdded { user } => (
                    EventType::UserAdded,
                    serde_json::to_value(UserMetadata { user: user.into() }).ok(),
                ),
                ApiEventType::UserRemoved { user } => (
                    EventType::UserRemoved,
                    serde_json::to_value(UserMetadata { user: user.into() }).ok(),
                ),
                ApiEventType::UserModified { before, after } => (
                    EventType::UserModified,
                    serde_json::to_value(UserModifiedMetadata {
                        before: before.into(),
                        after: after.into(),
                    })
                    .ok(),
                ),
                ApiEventType::NetworkDeviceAdded { device, location } => (
                    EventType::NetworkDeviceAdded,
                    serde_json::to_value(NetworkDeviceMetadata { device, location }).ok(),
                ),
                ApiEventType::NetworkDeviceRemoved { device, location } => (
                    EventType::NetworkDeviceRemoved,
                    serde_json::to_value(NetworkDeviceMetadata { device, location }).ok(),
                ),
                ApiEventType::NetworkDeviceModified {
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
                ApiEventType::VpnLocationAdded { location } => (
                    EventType::VpnLocationAdded,
                    serde_json::to_value(VpnLocationMetadata { location }).ok(),
                ),
                ApiEventType::VpnLocationRemoved { location } => (
                    EventType::VpnLocationRemoved,
                    serde_json::to_value(VpnLocationMetadata { location }).ok(),
                ),
                ApiEventType::VpnLocationModified { before, after } => (
                    EventType::VpnLocationModified,
                    serde_json::to_value(VpnLocationModifiedMetadata { before, after }).ok(),
                ),
                ApiEventType::OpenIdAppAdded { app } => (
                    EventType::OpenIdAppAdded,
                    serde_json::to_value(OpenIdAppMetadata { app: app.into() }).ok(),
                ),
                ApiEventType::OpenIdAppRemoved { app } => (
                    EventType::OpenIdAppRemoved,
                    serde_json::to_value(OpenIdAppMetadata { app: app.into() }).ok(),
                ),
                ApiEventType::OpenIdAppModified { before, after } => (
                    EventType::OpenIdAppModified,
                    serde_json::to_value(OpenIdAppModifiedMetadata {
                        before: before.into(),
                        after: after.into(),
                    })
                    .ok(),
                ),
                ApiEventType::OpenIdAppStateChanged { app, enabled } => (
                    EventType::OpenIdAppStateChanged,
                    serde_json::to_value(OpenIdAppStateChangedMetadata {
                        app: app.into(),
                        enabled,
                    })
                    .ok(),
                ),
                ApiEventType::OpenIdProviderModified { provider } => (
                    EventType::OpenIdProviderModified,
                    serde_json::to_value(OpenIdProviderMetadata {
                        provider: provider.into(),
                    })
                    .ok(),
                ),
                ApiEventType::OpenIdProviderRemoved { provider } => (
                    EventType::OpenIdProviderRemoved,
                    serde_json::to_value(OpenIdProviderMetadata {
                        provider: provider.into(),
                    })
                    .ok(),
                ),
                ApiEventType::SettingsUpdatedPartial { before, after } => (
                    EventType::SettingsUpdatedPartial,
                    serde_json::to_value(SettingsUpdateMetadata {
                        before: before.into(),
                        after: after.into(),
                    })
                    .ok(),
                ),
                ApiEventType::SettingsUpdated { before, after } => (
                    EventType::SettingsUpdated,
                    serde_json::to_value(SettingsUpdateMetadata {
                        before: before.into(),
                        after: after.into(),
                    })
                    .ok(),
                ),
                ApiEventType::SettingsDefaultBrandingRestored => {
                    (EventType::SettingsDefaultBrandingRestored, None)
                }
                ApiEventType::ActivityLogStreamCreated { stream } => (
                    EventType::ActivityLogStreamCreated,
                    serde_json::to_value(ActivityLogStreamMetadata {
                        stream: stream.into(),
                    })
                    .ok(),
                ),
                ApiEventType::ActivityLogStreamRemoved { stream } => (
                    EventType::ActivityLogStreamRemoved,
                    serde_json::to_value(ActivityLogStreamMetadata {
                        stream: stream.into(),
                    })
                    .ok(),
                ),
                ApiEventType::ActivityLogStreamModified { before, after } => (
                    EventType::ActivityLogStreamModified,
                    serde_json::to_value(ActivityLogStreamModifiedMetadata {
                        before: before.into(),
                        after: after.into(),
                    })
                    .ok(),
                ),
                ApiEventType::GroupsBulkAssigned { users, groups } => (
                    EventType::GroupsBulkAssigned,
                    serde_json::to_value(GroupsBulkAssignedMetadata {
                        users: users.into_iter().map(Into::into).collect(),
                        groups,
                    })
                    .ok(),
                ),
                ApiEventType::GroupAdded { group } => (
                    EventType::GroupAdded,
                    serde_json::to_value(GroupMetadata { group }).ok(),
                ),
                ApiEventType::GroupModified { before, after } => (
                    EventType::GroupModified,
                    serde_json::to_value(GroupModifiedMetadata { before, after }).ok(),
                ),
                ApiEventType::GroupRemoved { group } => (
                    EventType::GroupRemoved,
                    serde_json::to_value(GroupMetadata { group }).ok(),
                ),
                ApiEventType::GroupMemberAdded { group, user } => (
                    EventType::GroupMemberAdded,
                    serde_json::to_value(GroupAssignedMetadata {
                        group,
                        user: user.into(),
                    })
                    .ok(),
                ),
                ApiEventType::GroupMemberRemoved { group, user } => (
                    EventType::GroupMemberRemoved,
                    serde_json::to_value(GroupAssignedMetadata {
                        group,
                        user: user.into(),
                    })
                    .ok(),
                ),
                ApiEventType::GroupMembersModified {
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
                ApiEventType::WebHookAdded { webhook } => (
                    EventType::WebHookAdded,
                    serde_json::to_value(WebHookMetadata { webhook }).ok(),
                ),
                ApiEventType::WebHookModified { before, after } => (
                    EventType::WebHookModified,
                    serde_json::to_value(WebHookModifiedMetadata { before, after }).ok(),
                ),
                ApiEventType::WebHookRemoved { webhook } => (
                    EventType::WebHookRemoved,
                    serde_json::to_value(WebHookMetadata { webhook }).ok(),
                ),
                ApiEventType::WebHookStateChanged { webhook, enabled } => (
                    EventType::WebHookStateChanged,
                    serde_json::to_value(WebHookStateChangedMetadata { webhook, enabled }).ok(),
                ),
                ApiEventType::PasswordReset { user } => (
                    EventType::PasswordReset,
                    serde_json::to_value(PasswordResetMetadata { user: user.into() }).ok(),
                ),
                ApiEventType::ClientConfigurationTokenAdded { user } => (
                    EventType::ClientConfigurationTokenAdded,
                    serde_json::to_value(ClientConfigurationTokenMetadata { user: user.into() })
                        .ok(),
                ),
                ApiEventType::UserSnatBindingAdded { user, binding, .. } => (
                    EventType::UserSnatBindingAdded,
                    serde_json::to_value(UserSnatBindingMetadata {
                        user: user.into(),
                        binding,
                    })
                    .ok(),
                ),
                ApiEventType::UserSnatBindingRemoved { user, binding, .. } => (
                    EventType::UserSnatBindingRemoved,
                    serde_json::to_value(UserSnatBindingMetadata {
                        user: user.into(),
                        binding,
                    })
                    .ok(),
                ),
                ApiEventType::UserSnatBindingModified {
                    user,
                    before,
                    after,
                    ..
                } => (
                    EventType::UserSnatBindingModified,
                    serde_json::to_value(UserSnatBindingModifiedMetadata {
                        user: user.into(),
                        before,
                        after,
                    })
                    .ok(),
                ),
                ApiEventType::ProxyModified { before, after } => (
                    EventType::ProxyModified,
                    serde_json::to_value(ProxyModifiedMetadata { before, after }).ok(),
                ),
                ApiEventType::ProxyDeleted { proxy } => (
                    EventType::ProxyDeleted,
                    serde_json::to_value(ProxyDeletedMetadata { proxy }).ok(),
                ),
                ApiEventType::GatewayModified { before, after } => (
                    EventType::GatewayModified,
                    serde_json::to_value(GatewayModifiedMetadata { before, after }).ok(),
                ),
                ApiEventType::GatewayDeleted { gateway } => (
                    EventType::GatewayDeleted,
                    serde_json::to_value(GatewayDeletedMetadata { gateway }).ok(),
                ),
                ApiEventType::DevicePostureCreated { snapshot } => (
                    EventType::DevicePostureCreated,
                    serde_json::to_value(snapshot).ok(),
                ),
                ApiEventType::DevicePostureUpdated { before, after } => (
                    EventType::DevicePostureUpdated,
                    serde_json::to_value(serde_json::json!({"before": before, "after": after}))
                        .ok(),
                ),
                ApiEventType::DevicePostureDeleted { snapshot } => (
                    EventType::DevicePostureDeleted,
                    serde_json::to_value(snapshot).ok(),
                ),
                ApiEventType::DevicePostureDuplicated {
                    original,
                    duplicate,
                } => (
                    EventType::DevicePostureDuplicated,
                    serde_json::to_value(
                        serde_json::json!({"original": original, "duplicate": duplicate}),
                    )
                    .ok(),
                ),
                ApiEventType::DevicePostureLocationsAssigned {
                    device_posture,
                    location_ids,
                } => (
                    EventType::DevicePostureLocationsAssigned,
                    serde_json::to_value(
                        serde_json::json!({"posture_id": device_posture.id, "location_ids": location_ids}),
                    )
                    .ok(),
                ),
                ApiEventType::LocationPosturesAssigned {
                    location,
                    posture_ids,
                } => (
                    EventType::LocationPosturesAssigned,
                    serde_json::to_value(
                        serde_json::json!({"location_id": location.id, "posture_ids": posture_ids}),
                    )
                    .ok(),
                ),
                ApiEventType::EnrollmentTokenAdded { user } => (
                    EventType::EnrollmentTokenAdded,
                    serde_json::to_value(EnrollmentTokenMetadata { user: user.into() }).ok(),
                ),
            };
            (module, event_type, description, metadata)
        }
        Event::Bidi(BidiStreamEventType::DesktopClientMfa(event)) => {
            let module = ActivityLogModule::Vpn;
            let description = match &*event {
                DesktopClientMfaEvent::Success {
                    location,
                    device,
                    method,
                } => Some(format!(
                    "Device {device} completed MFA authorization for location {location} using {method}"
                )),
                DesktopClientMfaEvent::Failed {
                    location,
                    device,
                    method,
                    message,
                } => Some(format!(
                    "Device {device} failed to connect to MFA location {location} using {method} with: {message}"
                )),
                DesktopClientMfaEvent::Disconnected {
                    location,
                    device,
                    is_mfa_session,
                } => {
                    if *is_mfa_session {
                        Some(format!(
                            "Device {device} disconnected from MFA location {location}"
                        ))
                    } else {
                        Some(format!(
                            "Device {device} disconnected from location {location}"
                        ))
                    }
                }
            };
            let (event_type, metadata) = match *event {
                DesktopClientMfaEvent::Success {
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
                DesktopClientMfaEvent::Failed {
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
                DesktopClientMfaEvent::Disconnected {
                    location,
                    device,
                    is_mfa_session,
                } => {
                    if is_mfa_session {
                        (
                            EventType::VpnClientMfaDisconnected,
                            serde_json::to_value(VpnClientMetadata { location, device }).ok(),
                        )
                    } else {
                        (
                            EventType::VpnClientDisconnected,
                            serde_json::to_value(VpnClientMetadata { location, device }).ok(),
                        )
                    }
                }
            };
            (module, event_type, description, metadata)
        }
        Event::SessionManager {
            event,
            location,
            device,
        } => {
            let module = ActivityLogModule::Vpn;
            let description = match event {
                SessionManagerEventType::ClientConnected => {
                    Some(format!("Device {device} connected to location {location}"))
                }
                SessionManagerEventType::ClientDisconnected => Some(format!(
                    "Device {device} disconnected from location {location}"
                )),
                SessionManagerEventType::MfaClientConnected => Some(format!(
                    "Device {device} connected to MFA location {location}"
                )),
                SessionManagerEventType::MfaClientDisconnected => Some(format!(
                    "Device {device} disconnected from MFA location {location}"
                )),
            };
            let (event_type, metadata) = match event {
                SessionManagerEventType::ClientConnected => (
                    EventType::VpnClientConnected,
                    serde_json::to_value(VpnClientMetadata { location, device }).ok(),
                ),
                SessionManagerEventType::ClientDisconnected => (
                    EventType::VpnClientDisconnected,
                    serde_json::to_value(VpnClientMetadata { location, device }).ok(),
                ),
                SessionManagerEventType::MfaClientConnected => (
                    EventType::VpnClientMfaConnected,
                    serde_json::to_value(VpnClientMetadata { location, device }).ok(),
                ),
                SessionManagerEventType::MfaClientDisconnected => (
                    EventType::VpnClientMfaDisconnected,
                    serde_json::to_value(VpnClientMetadata { location, device }).ok(),
                ),
            };
            (module, event_type, description, metadata)
        }
        Event::Bidi(BidiStreamEventType::Enrollment(event)) => {
            let module = ActivityLogModule::Enrollment;
            let description = get_enrollment_event_description(&event);

            let (event_type, metadata) = match *event {
                defguard_core::events::EnrollmentEvent::EnrollmentStarted => {
                    (EventType::EnrollmentStarted, None)
                }
                defguard_core::events::EnrollmentEvent::EnrollmentCompleted => {
                    (EventType::EnrollmentCompleted, None)
                }
                defguard_core::events::EnrollmentEvent::EnrollmentDeviceAdded { device } => (
                    EventType::EnrollmentDeviceAdded,
                    serde_json::to_value(EnrollmentDeviceAddedMetadata { device }).ok(),
                ),
            };
            (module, event_type, description, metadata)
        }
        Event::Bidi(BidiStreamEventType::PasswordReset(event)) => {
            let module = ActivityLogModule::Enrollment;
            let (event_type, _) = match *event {
                PasswordResetEvent::PasswordResetRequested => {
                    (EventType::PasswordResetRequested, None::<serde_json::Value>)
                }
                PasswordResetEvent::PasswordResetStarted => (EventType::PasswordResetStarted, None),
                PasswordResetEvent::PasswordResetCompleted => {
                    (EventType::PasswordResetCompleted, None)
                }
            };
            (module, event_type, None, None)
        }
    };

    ActivityLogEvent {
        id: NoId,
        timestamp,
        user_id,
        username,
        location,
        ip: ip.map(Into::into),
        event,
        module,
        device,
        description,
        metadata,
    }
}

async fn process_batch(
    pool: &PgPool,
    message_buffer: Vec<EventLoggerMessage>,
    activity_log_messages_tx: &tokio::sync::broadcast::Sender<Bytes>,
) -> Result<(), EventLoggerError> {
    let mut transaction = pool.begin().await?;
    let mut serialized_activity_log_events = String::new();

    // Process all messages in the batch
    for message in message_buffer {
        let activity_log_event = map_to_activity_log_event(message);

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

    transaction.commit().await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr};

    use chrono::Utc;
    use defguard_common::db::{
        NoId,
        models::{
            AuthenticationKey, AuthenticationKeyType, Device, DeviceType, MFAMethod, User,
            WebAuthn, WireguardNetwork,
            gateway::Gateway,
            group::Group,
            oauth2client::OAuth2Client,
            proxy::Proxy,
            wireguard::{LocationMfaMode, ServiceLocationMode},
        },
    };
    use ipnetwork::IpNetwork;
    use serde_json::Value;

    use defguard_core::events::{
        BidiRequestContext, BidiStreamEventType, DesktopClientMfaEvent,
        EnrollmentEvent as CoreEnrollmentEvent, PasswordResetEvent,
    };
    use defguard_core::{
        db::models::webhook::WebHook,
        enterprise::db::models::{
            activity_log_stream::{ActivityLogStream, ActivityLogStreamType},
            api_tokens::ApiToken,
            device_posture::{DevicePosture, DevicePostureSnapshot},
            openid_provider::{
                DirectorySyncTarget, DirectorySyncUserBehavior, OpenIdProvider, OpenIdProviderKind,
            },
            snat::UserSnatBinding,
        },
    };
    use strum::EnumCount;

    use super::*;

    fn sample_device() -> Device<i64> {
        Device::new(
            "vpn-device".to_owned(),
            "pubkey".to_owned(),
            1,
            DeviceType::User,
            None,
            true,
        )
        .with_id(20)
    }

    fn sample_location() -> WireguardNetwork<i64> {
        WireguardNetwork::new(
            "vpn-location".to_owned(),
            51820,
            "vpn.example.com".to_owned(),
            None,
            [IpNetwork::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0).unwrap()],
            true,
            false,
            false,
            false,
            LocationMfaMode::Internal,
            ServiceLocationMode::Disabled,
        )
        .set_address([IpNetwork::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)), 24).unwrap()])
        .expect("sample location address should be valid")
        .with_id(10)
    }

    #[test]
    fn activity_log_event_serialization_supports_null_ip() {
        let event = ActivityLogEvent {
            id: NoId,
            timestamp: Utc::now().naive_utc(),
            user_id: 1,
            username: "admin".to_owned(),
            location: None,
            ip: None,
            event: EventType::UserLogin,
            module: ActivityLogModule::Defguard,
            device: "test-device".to_owned(),
            description: None,
            metadata: None,
        };

        let serialized = serde_json::to_value(event).expect("activity log event should serialize");

        assert_eq!(serialized.get("ip"), Some(&Value::Null));
    }

    fn sample_bidi_context() -> BidiRequestContext {
        BidiRequestContext::new(
            1,
            "alice".to_owned(),
            IpAddr::V4(Ipv4Addr::LOCALHOST),
            "desktop-app".to_owned(),
        )
    }

    #[test]
    fn maps_disconnect_bidi_events_from_mfa_sessions_to_mfa_disconnect_logger_events() {
        let event = BidiStreamEvent {
            context: sample_bidi_context(),
            event: BidiStreamEventType::DesktopClientMfa(Box::new(
                DesktopClientMfaEvent::Disconnected {
                    location: sample_location(),
                    device: sample_device(),
                    is_mfa_session: true,
                },
            )),
        };

        let message = EventLoggerMessage::from_bidi_event(event);

        match message.event {
            Event::Bidi(BidiStreamEventType::DesktopClientMfa(event)) => match *event {
                DesktopClientMfaEvent::Disconnected {
                    location,
                    device,
                    is_mfa_session,
                } => {
                    assert!(is_mfa_session);
                    assert_eq!(location.id, sample_location().id);
                    assert_eq!(device.id, sample_device().id);
                }
                _ => panic!("expected disconnect event"),
            },
            _ => panic!("expected bidi event"),
        }
    }

    #[test]
    fn maps_disconnect_bidi_events_from_non_mfa_sessions_to_standard_disconnect_logger_events() {
        let event = BidiStreamEvent {
            context: sample_bidi_context(),
            event: BidiStreamEventType::DesktopClientMfa(Box::new(
                DesktopClientMfaEvent::Disconnected {
                    location: sample_location(),
                    device: sample_device(),
                    is_mfa_session: false,
                },
            )),
        };

        let message = EventLoggerMessage::from_bidi_event(event);

        match message.event {
            Event::Bidi(BidiStreamEventType::DesktopClientMfa(event)) => match *event {
                DesktopClientMfaEvent::Disconnected {
                    location,
                    device,
                    is_mfa_session,
                } => {
                    assert!(!is_mfa_session);
                    assert_eq!(location.id, sample_location().id);
                    assert_eq!(device.id, sample_device().id);
                }
                _ => panic!("expected disconnect event"),
            },
            _ => panic!("expected bidi event"),
        }
    }

    // Helper struct for testing mapping of all existing events
    // to activity log entries
    struct EventTestCase {
        name: &'static str,
        message: EventLoggerMessage,
        event_type: EventType,
        module: ActivityLogModule,
        description_contains: Option<&'static str>,
    }

    fn test_context() -> EventContext {
        EventContext {
            timestamp: Utc::now().naive_utc(),
            user_id: 1,
            username: "admin".into(),
            location: None,
            ip: None,
            device: "test".into(),
        }
    }

    fn api_message(event: ApiEventType) -> EventLoggerMessage {
        EventLoggerMessage {
            context: test_context(),
            event: Event::Api(event),
        }
    }

    fn api_event_cases() -> Vec<EventTestCase> {
        let user = User::new("testuser", Some("pass"), "Last", "First", "e@e", None).with_id(1);
        let device = Device {
            id: 2,
            name: "d2".into(),
            wireguard_pubkey: "pk".into(),
            user_id: 1,
            created: Utc::now().naive_utc(),
            device_type: DeviceType::User,
            description: None,
            configured: true,
        };
        let location = sample_location();
        let timestamp = Utc::now().naive_utc();
        let proxy = Proxy {
            id: 1,
            name: "px".into(),
            address: "a".into(),
            port: 1,
            connected_at: None,
            disconnected_at: None,
            version: None,
            enabled: true,
            certificate_serial: None,
            certificate_expiry: None,
            modified_at: timestamp,
            modified_by: "admin".into(),
            core_client_cert_der: None,
            core_client_cert_key_der: None,
            core_client_cert_expiry: None,
        };
        let proxy2 = Proxy {
            id: 2,
            name: "px2".into(),
            ..proxy.clone()
        };
        let gateway = Gateway {
            id: 1,
            location_id: 10,
            name: "gw".into(),
            address: "a".into(),
            port: 1,
            connected_at: None,
            disconnected_at: None,
            certificate_serial: None,
            certificate_expiry: None,
            version: None,
            enabled: true,
            modified_at: timestamp,
            modified_by: "admin".into(),
            core_client_cert_der: None,
            core_client_cert_key_der: None,
            core_client_cert_expiry: None,
        };
        let gateway2 = Gateway {
            id: 2,
            ..gateway.clone()
        };
        let oid_provider = OpenIdProvider {
            id: 1,
            name: "oid".into(),
            base_url: "http://x".into(),
            kind: OpenIdProviderKind::Google,
            client_id: "c".into(),
            client_secret: "s".into(),
            display_name: None,
            google_service_account_key: None,
            google_service_account_email: None,
            admin_email: None,
            directory_sync_enabled: false,
            directory_sync_interval: 0,
            directory_sync_user_behavior: DirectorySyncUserBehavior::Keep,
            directory_sync_admin_behavior: DirectorySyncUserBehavior::Keep,
            directory_sync_target: DirectorySyncTarget::All,
            okta_private_jwk: None,
            okta_dirsync_client_id: None,
            directory_sync_group_match: vec![],
            jumpcloud_api_key: None,
            prefetch_users: false,
        };
        let log_stream = ActivityLogStream {
            id: 1,
            name: "stream".into(),
            stream_type: ActivityLogStreamType::VectorHttp,
            config: serde_json::Value::Null,
        };
        let log_stream2 = ActivityLogStream {
            id: 2,
            name: "stream2".into(),
            stream_type: ActivityLogStreamType::LogstashHttp,
            config: serde_json::Value::Null,
        };
        let webhook = WebHook {
            id: 1,
            url: "http://x".into(),
            description: "".into(),
            token: "t".into(),
            enabled: true,
            on_user_created: false,
            on_user_deleted: false,
            on_user_modified: false,
            on_hwkey_provision: false,
        };
        let webhook2 = WebHook {
            id: 2,
            ..webhook.clone()
        };
        let auth_key = AuthenticationKey {
            id: 1,
            yubikey_id: None,
            name: Some("mykey".into()),
            user_id: 1,
            key: "kk".into(),
            key_type: AuthenticationKeyType::Ssh,
        };
        let oauth_app = OAuth2Client {
            id: 1,
            client_id: "cid".into(),
            client_secret: "cs".into(),
            redirect_uri: vec![],
            scope: vec![],
            name: "app".into(),
            enabled: true,
        };
        let group1 = Group {
            id: 1,
            name: "g1".into(),
            is_admin: false,
        };
        let group2 = Group {
            id: 2,
            name: "g2".into(),
            is_admin: true,
        };
        let admin_group = Group {
            id: 3,
            name: "admin".into(),
            is_admin: true,
        };
        let api_token = ApiToken {
            id: 1,
            user_id: 1,
            created_at: timestamp,
            name: "tok".into(),
            token_hash: "h".into(),
        };
        let snat = UserSnatBinding {
            id: 1,
            user_id: 1,
            location_id: 10,
            public_ip: "1.2.3.4".parse().unwrap(),
        };
        let snat2 = UserSnatBinding {
            id: 2,
            user_id: 1,
            location_id: 10,
            public_ip: "5.6.7.8".parse().unwrap(),
        };
        let webauthn_key = WebAuthn {
            id: 1,
            user_id: 1,
            name: "k".into(),
            passkey: vec![],
        };
        let posture = DevicePosture {
            id: 1,
            name: "dp".into(),
            description: None,
            min_client_version: None,
            allow_prerelease_client: false,
        };
        let posture_snapshot = DevicePostureSnapshot {
            device_posture: posture.clone(),
            os_rules: vec![],
            location_ids: vec![],
        };
        let posture_snapshot2 = DevicePostureSnapshot {
            device_posture: DevicePosture {
                id: 2,
                name: "dp2".into(),
                description: Some("desc".into()),
                min_client_version: None,
                allow_prerelease_client: true,
            },
            os_rules: vec![],
            location_ids: vec![],
        };

        let cases = vec![
            EventTestCase {
                name: "UserLogin",
                message: api_message(ApiEventType::UserLogin),
                event_type: EventType::UserLogin,
                module: ActivityLogModule::Defguard,
                description_contains: None,
            },
            EventTestCase {
                name: "UserLoginFailed",
                message: api_message(ApiEventType::UserLoginFailed {
                    message: "bad".into(),
                }),
                event_type: EventType::UserLoginFailed,
                module: ActivityLogModule::Defguard,
                description_contains: Some("bad"),
            },
            EventTestCase {
                name: "UserLogout",
                message: api_message(ApiEventType::UserLogout),
                event_type: EventType::UserLogout,
                module: ActivityLogModule::Defguard,
                description_contains: None,
            },
            EventTestCase {
                name: "UserMfaLogin",
                message: api_message(ApiEventType::UserMfaLogin {
                    mfa_method: MFAMethod::OneTimePassword,
                }),
                event_type: EventType::UserMfaLogin,
                module: ActivityLogModule::Defguard,
                description_contains: Some("TOTP"),
            },
            EventTestCase {
                name: "UserMfaLoginFailed",
                message: api_message(ApiEventType::UserMfaLoginFailed {
                    mfa_method: MFAMethod::OneTimePassword,
                    message: "err".into(),
                }),
                event_type: EventType::UserMfaLoginFailed,
                module: ActivityLogModule::Defguard,
                description_contains: Some("err"),
            },
            EventTestCase {
                name: "RecoveryCodeLoginFailed",
                message: api_message(ApiEventType::RecoveryCodeLoginFailed),
                event_type: EventType::UserMfaLoginFailed,
                module: ActivityLogModule::Defguard,
                description_contains: Some("Recovery"),
            },
            EventTestCase {
                name: "RecoveryCodeUsed",
                message: api_message(ApiEventType::RecoveryCodeUsed),
                event_type: EventType::RecoveryCodeUsed,
                module: ActivityLogModule::Defguard,
                description_contains: None,
            },
            EventTestCase {
                name: "PasswordChangedByAdmin",
                message: api_message(ApiEventType::PasswordChangedByAdmin { user: user.clone() }),
                event_type: EventType::PasswordChangedByAdmin,
                module: ActivityLogModule::Defguard,
                description_contains: Some("admin"),
            },
            EventTestCase {
                name: "PasswordChanged",
                message: api_message(ApiEventType::PasswordChanged),
                event_type: EventType::PasswordChanged,
                module: ActivityLogModule::Defguard,
                description_contains: None,
            },
            EventTestCase {
                name: "PasswordReset",
                message: api_message(ApiEventType::PasswordReset { user: user.clone() }),
                event_type: EventType::PasswordReset,
                module: ActivityLogModule::Defguard,
                description_contains: Some("reset"),
            },
            EventTestCase {
                name: "MfaDisabled",
                message: api_message(ApiEventType::MfaDisabled),
                event_type: EventType::MfaDisabled,
                module: ActivityLogModule::Defguard,
                description_contains: Some("Disabled"),
            },
            EventTestCase {
                name: "UserMfaDisabled",
                message: api_message(ApiEventType::UserMfaDisabled { user: user.clone() }),
                event_type: EventType::UserMfaDisabled,
                module: ActivityLogModule::Defguard,
                description_contains: Some("Disabled"),
            },
            EventTestCase {
                name: "MfaTotpDisabled",
                message: api_message(ApiEventType::MfaTotpDisabled),
                event_type: EventType::MfaTotpDisabled,
                module: ActivityLogModule::Defguard,
                description_contains: Some("TOTP"),
            },
            EventTestCase {
                name: "MfaTotpEnabled",
                message: api_message(ApiEventType::MfaTotpEnabled),
                event_type: EventType::MfaTotpEnabled,
                module: ActivityLogModule::Defguard,
                description_contains: Some("TOTP"),
            },
            EventTestCase {
                name: "MfaEmailDisabled",
                message: api_message(ApiEventType::MfaEmailDisabled),
                event_type: EventType::MfaEmailDisabled,
                module: ActivityLogModule::Defguard,
                description_contains: Some("email"),
            },
            EventTestCase {
                name: "MfaEmailEnabled",
                message: api_message(ApiEventType::MfaEmailEnabled),
                event_type: EventType::MfaEmailEnabled,
                module: ActivityLogModule::Defguard,
                description_contains: Some("email"),
            },
            EventTestCase {
                name: "MfaSecurityKeyAdded",
                message: api_message(ApiEventType::MfaSecurityKeyAdded {
                    key: webauthn_key.clone(),
                }),
                event_type: EventType::MfaSecurityKeyAdded,
                module: ActivityLogModule::Defguard,
                description_contains: Some("Added"),
            },
            EventTestCase {
                name: "MfaSecurityKeyRemoved",
                message: api_message(ApiEventType::MfaSecurityKeyRemoved { key: webauthn_key }),
                event_type: EventType::MfaSecurityKeyRemoved,
                module: ActivityLogModule::Defguard,
                description_contains: Some("Removed"),
            },
            EventTestCase {
                name: "UserAdded",
                message: api_message(ApiEventType::UserAdded { user: user.clone() }),
                event_type: EventType::UserAdded,
                module: ActivityLogModule::Defguard,
                description_contains: Some("Added"),
            },
            EventTestCase {
                name: "UserRemoved",
                message: api_message(ApiEventType::UserRemoved { user: user.clone() }),
                event_type: EventType::UserRemoved,
                module: ActivityLogModule::Defguard,
                description_contains: Some("Removed"),
            },
            EventTestCase {
                name: "UserModified",
                message: api_message(ApiEventType::UserModified {
                    before: user.clone(),
                    after: user.clone(),
                }),
                event_type: EventType::UserModified,
                module: ActivityLogModule::Defguard,
                description_contains: Some("Modified"),
            },
            EventTestCase {
                name: "UserGroupsModified",
                message: api_message(ApiEventType::UserGroupsModified {
                    user: user.clone(),
                    before: vec![],
                    after: vec![],
                }),
                event_type: EventType::UserGroupsModified,
                module: ActivityLogModule::Defguard,
                description_contains: Some("modified"),
            },
            EventTestCase {
                name: "UserDeviceAdded",
                message: api_message(ApiEventType::UserDeviceAdded {
                    owner: user.clone(),
                    device: device.clone(),
                }),
                event_type: EventType::DeviceAdded,
                module: ActivityLogModule::Defguard,
                description_contains: Some("Added"),
            },
            EventTestCase {
                name: "UserDeviceRemoved",
                message: api_message(ApiEventType::UserDeviceRemoved {
                    owner: user.clone(),
                    device: device.clone(),
                }),
                event_type: EventType::DeviceRemoved,
                module: ActivityLogModule::Defguard,
                description_contains: Some("Removed"),
            },
            EventTestCase {
                name: "UserDeviceModified",
                message: api_message(ApiEventType::UserDeviceModified {
                    owner: user.clone(),
                    before: device.clone(),
                    after: device.clone(),
                }),
                event_type: EventType::DeviceModified,
                module: ActivityLogModule::Defguard,
                description_contains: Some("Modified"),
            },
            EventTestCase {
                name: "NetworkDeviceAdded",
                message: api_message(ApiEventType::NetworkDeviceAdded {
                    device: device.clone(),
                    location: location.clone(),
                }),
                event_type: EventType::NetworkDeviceAdded,
                module: ActivityLogModule::Defguard,
                description_contains: Some("Added"),
            },
            EventTestCase {
                name: "NetworkDeviceRemoved",
                message: api_message(ApiEventType::NetworkDeviceRemoved {
                    device: device.clone(),
                    location: location.clone(),
                }),
                event_type: EventType::NetworkDeviceRemoved,
                module: ActivityLogModule::Defguard,
                description_contains: Some("Removed"),
            },
            EventTestCase {
                name: "NetworkDeviceModified",
                message: api_message(ApiEventType::NetworkDeviceModified {
                    before: device.clone(),
                    after: device.clone(),
                    location: location.clone(),
                }),
                event_type: EventType::NetworkDeviceModified,
                module: ActivityLogModule::Defguard,
                description_contains: Some("Modified"),
            },
            EventTestCase {
                name: "ActivityLogStreamCreated",
                message: api_message(ApiEventType::ActivityLogStreamCreated {
                    stream: log_stream.clone(),
                }),
                event_type: EventType::ActivityLogStreamCreated,
                module: ActivityLogModule::Defguard,
                description_contains: Some("Created"),
            },
            EventTestCase {
                name: "ActivityLogStreamModified",
                message: api_message(ApiEventType::ActivityLogStreamModified {
                    before: log_stream.clone(),
                    after: log_stream2,
                }),
                event_type: EventType::ActivityLogStreamModified,
                module: ActivityLogModule::Defguard,
                description_contains: Some("Modified"),
            },
            EventTestCase {
                name: "ActivityLogStreamRemoved",
                message: api_message(ApiEventType::ActivityLogStreamRemoved { stream: log_stream }),
                event_type: EventType::ActivityLogStreamRemoved,
                module: ActivityLogModule::Defguard,
                description_contains: Some("Removed"),
            },
            EventTestCase {
                name: "VpnLocationAdded",
                message: api_message(ApiEventType::VpnLocationAdded {
                    location: location.clone(),
                }),
                event_type: EventType::VpnLocationAdded,
                module: ActivityLogModule::Defguard,
                description_contains: Some("Added"),
            },
            EventTestCase {
                name: "VpnLocationRemoved",
                message: api_message(ApiEventType::VpnLocationRemoved {
                    location: location.clone(),
                }),
                event_type: EventType::VpnLocationRemoved,
                module: ActivityLogModule::Defguard,
                description_contains: Some("Removed"),
            },
            EventTestCase {
                name: "VpnLocationModified",
                message: api_message(ApiEventType::VpnLocationModified {
                    before: location.clone(),
                    after: location.clone(),
                }),
                event_type: EventType::VpnLocationModified,
                module: ActivityLogModule::Defguard,
                description_contains: Some("modified"),
            },
            EventTestCase {
                name: "ApiTokenAdded",
                message: api_message(ApiEventType::ApiTokenAdded {
                    owner: user.clone(),
                    token: api_token.clone(),
                }),
                event_type: EventType::ApiTokenAdded,
                module: ActivityLogModule::Defguard,
                description_contains: Some("Added"),
            },
            EventTestCase {
                name: "ApiTokenRemoved",
                message: api_message(ApiEventType::ApiTokenRemoved {
                    owner: user.clone(),
                    token: api_token.clone(),
                }),
                event_type: EventType::ApiTokenRemoved,
                module: ActivityLogModule::Defguard,
                description_contains: Some("Removed"),
            },
            EventTestCase {
                name: "ApiTokenRenamed",
                message: api_message(ApiEventType::ApiTokenRenamed {
                    owner: user.clone(),
                    token: api_token,
                    old_name: "old".into(),
                    new_name: "new".into(),
                }),
                event_type: EventType::ApiTokenRenamed,
                module: ActivityLogModule::Defguard,
                description_contains: Some("renamed"),
            },
            EventTestCase {
                name: "OpenIdAppAdded",
                message: api_message(ApiEventType::OpenIdAppAdded {
                    app: oauth_app.clone(),
                }),
                event_type: EventType::OpenIdAppAdded,
                module: ActivityLogModule::Defguard,
                description_contains: Some("Added"),
            },
            EventTestCase {
                name: "OpenIdAppRemoved",
                message: api_message(ApiEventType::OpenIdAppRemoved {
                    app: oauth_app.clone(),
                }),
                event_type: EventType::OpenIdAppRemoved,
                module: ActivityLogModule::Defguard,
                description_contains: Some("Removed"),
            },
            EventTestCase {
                name: "OpenIdAppModified",
                message: api_message(ApiEventType::OpenIdAppModified {
                    before: oauth_app.clone(),
                    after: oauth_app.clone(),
                }),
                event_type: EventType::OpenIdAppModified,
                module: ActivityLogModule::Defguard,
                description_contains: Some("Modified"),
            },
            EventTestCase {
                name: "OpenIdAppStateChanged",
                message: api_message(ApiEventType::OpenIdAppStateChanged {
                    app: oauth_app,
                    enabled: false,
                }),
                event_type: EventType::OpenIdAppStateChanged,
                module: ActivityLogModule::Defguard,
                description_contains: Some("Disabled"),
            },
            EventTestCase {
                name: "OpenIdProviderModified",
                message: api_message(ApiEventType::OpenIdProviderModified {
                    provider: oid_provider.clone(),
                }),
                event_type: EventType::OpenIdProviderModified,
                module: ActivityLogModule::Defguard,
                description_contains: Some("Modified"),
            },
            EventTestCase {
                name: "OpenIdProviderRemoved",
                message: api_message(ApiEventType::OpenIdProviderRemoved {
                    provider: oid_provider,
                }),
                event_type: EventType::OpenIdProviderRemoved,
                module: ActivityLogModule::Defguard,
                description_contains: Some("Removed"),
            },
            EventTestCase {
                name: "SettingsUpdated",
                message: api_message(ApiEventType::SettingsUpdated {
                    before: Default::default(),
                    after: Default::default(),
                }),
                event_type: EventType::SettingsUpdated,
                module: ActivityLogModule::Defguard,
                description_contains: None,
            },
            EventTestCase {
                name: "SettingsUpdatedPartial",
                message: api_message(ApiEventType::SettingsUpdatedPartial {
                    before: Default::default(),
                    after: Default::default(),
                }),
                event_type: EventType::SettingsUpdatedPartial,
                module: ActivityLogModule::Defguard,
                description_contains: None,
            },
            EventTestCase {
                name: "SettingsDefaultBrandingRestored",
                message: api_message(ApiEventType::SettingsDefaultBrandingRestored),
                event_type: EventType::SettingsDefaultBrandingRestored,
                module: ActivityLogModule::Defguard,
                description_contains: Some("Restored"),
            },
            EventTestCase {
                name: "GroupsBulkAssigned",
                message: api_message(ApiEventType::GroupsBulkAssigned {
                    users: vec![user.clone()],
                    groups: vec![group1.clone()],
                }),
                event_type: EventType::GroupsBulkAssigned,
                module: ActivityLogModule::Defguard,
                description_contains: Some("Assigned"),
            },
            EventTestCase {
                name: "GroupAdded",
                message: api_message(ApiEventType::GroupAdded {
                    group: group1.clone(),
                }),
                event_type: EventType::GroupAdded,
                module: ActivityLogModule::Defguard,
                description_contains: Some("Added"),
            },
            EventTestCase {
                name: "GroupModified",
                message: api_message(ApiEventType::GroupModified {
                    before: group1.clone(),
                    after: group2,
                }),
                event_type: EventType::GroupModified,
                module: ActivityLogModule::Defguard,
                description_contains: Some("Modified"),
            },
            EventTestCase {
                name: "GroupRemoved",
                message: api_message(ApiEventType::GroupRemoved { group: group1 }),
                event_type: EventType::GroupRemoved,
                module: ActivityLogModule::Defguard,
                description_contains: Some("Removed"),
            },
            EventTestCase {
                name: "GroupMemberAdded",
                message: api_message(ApiEventType::GroupMemberAdded {
                    group: admin_group.clone(),
                    user: user.clone(),
                }),
                event_type: EventType::GroupMemberAdded,
                module: ActivityLogModule::Defguard,
                description_contains: Some("Added"),
            },
            EventTestCase {
                name: "GroupMemberRemoved",
                message: api_message(ApiEventType::GroupMemberRemoved {
                    group: admin_group.clone(),
                    user: user.clone(),
                }),
                event_type: EventType::GroupMemberRemoved,
                module: ActivityLogModule::Defguard,
                description_contains: Some("Removed"),
            },
            EventTestCase {
                name: "GroupMembersModified",
                message: api_message(ApiEventType::GroupMembersModified {
                    group: admin_group,
                    added: vec![user.clone()],
                    removed: vec![],
                }),
                event_type: EventType::GroupMembersModified,
                module: ActivityLogModule::Defguard,
                description_contains: Some("Added"),
            },
            EventTestCase {
                name: "WebHookAdded",
                message: api_message(ApiEventType::WebHookAdded {
                    webhook: webhook.clone(),
                }),
                event_type: EventType::WebHookAdded,
                module: ActivityLogModule::Defguard,
                description_contains: Some("Added"),
            },
            EventTestCase {
                name: "WebHookModified",
                message: api_message(ApiEventType::WebHookModified {
                    before: webhook.clone(),
                    after: webhook2,
                }),
                event_type: EventType::WebHookModified,
                module: ActivityLogModule::Defguard,
                description_contains: Some("Modified"),
            },
            EventTestCase {
                name: "WebHookRemoved",
                message: api_message(ApiEventType::WebHookRemoved {
                    webhook: webhook.clone(),
                }),
                event_type: EventType::WebHookRemoved,
                module: ActivityLogModule::Defguard,
                description_contains: Some("Removed"),
            },
            EventTestCase {
                name: "WebHookStateChanged",
                message: api_message(ApiEventType::WebHookStateChanged {
                    webhook,
                    enabled: false,
                }),
                event_type: EventType::WebHookStateChanged,
                module: ActivityLogModule::Defguard,
                description_contains: Some("Disabled"),
            },
            EventTestCase {
                name: "AuthenticationKeyAdded",
                message: api_message(ApiEventType::AuthenticationKeyAdded {
                    key: auth_key.clone(),
                }),
                event_type: EventType::AuthenticationKeyAdded,
                module: ActivityLogModule::Defguard,
                description_contains: Some("Added"),
            },
            EventTestCase {
                name: "AuthenticationKeyRemoved",
                message: api_message(ApiEventType::AuthenticationKeyRemoved {
                    key: auth_key.clone(),
                }),
                event_type: EventType::AuthenticationKeyRemoved,
                module: ActivityLogModule::Defguard,
                description_contains: Some("Removed"),
            },
            EventTestCase {
                name: "AuthenticationKeyRenamed",
                message: api_message(ApiEventType::AuthenticationKeyRenamed {
                    key: auth_key,
                    old_name: Some("old".into()),
                    new_name: Some("new".into()),
                }),
                event_type: EventType::AuthenticationKeyRenamed,
                module: ActivityLogModule::Defguard,
                description_contains: Some("Renamed"),
            },
            EventTestCase {
                name: "EnrollmentTokenAdded",
                message: api_message(ApiEventType::EnrollmentTokenAdded { user: user.clone() }),
                event_type: EventType::EnrollmentTokenAdded,
                module: ActivityLogModule::Defguard,
                description_contains: Some("Added"),
            },
            EventTestCase {
                name: "ClientConfigurationTokenAdded",
                message: api_message(ApiEventType::ClientConfigurationTokenAdded {
                    user: user.clone(),
                }),
                event_type: EventType::ClientConfigurationTokenAdded,
                module: ActivityLogModule::Defguard,
                description_contains: Some("Added"),
            },
            EventTestCase {
                name: "UserSnatBindingAdded",
                message: api_message(ApiEventType::UserSnatBindingAdded {
                    user: user.clone(),
                    location: location.clone(),
                    binding: snat.clone(),
                }),
                event_type: EventType::UserSnatBindingAdded,
                module: ActivityLogModule::Defguard,
                description_contains: Some("bound"),
            },
            EventTestCase {
                name: "UserSnatBindingRemoved",
                message: api_message(ApiEventType::UserSnatBindingRemoved {
                    user: user.clone(),
                    location: location.clone(),
                    binding: snat.clone(),
                }),
                event_type: EventType::UserSnatBindingRemoved,
                module: ActivityLogModule::Defguard,
                description_contains: Some("Removed"),
            },
            EventTestCase {
                name: "UserSnatBindingModified",
                message: api_message(ApiEventType::UserSnatBindingModified {
                    user: user.clone(),
                    location: location.clone(),
                    before: snat,
                    after: snat2,
                }),
                event_type: EventType::UserSnatBindingModified,
                module: ActivityLogModule::Defguard,
                description_contains: Some("changed"),
            },
            EventTestCase {
                name: "ProxyModified",
                message: api_message(ApiEventType::ProxyModified {
                    before: proxy.clone(),
                    after: proxy2,
                }),
                event_type: EventType::ProxyModified,
                module: ActivityLogModule::Defguard,
                description_contains: Some("Modified"),
            },
            EventTestCase {
                name: "ProxyDeleted",
                message: api_message(ApiEventType::ProxyDeleted { proxy }),
                event_type: EventType::ProxyDeleted,
                module: ActivityLogModule::Defguard,
                description_contains: Some("Deleted"),
            },
            EventTestCase {
                name: "GatewayModified",
                message: api_message(ApiEventType::GatewayModified {
                    before: gateway.clone(),
                    after: gateway2,
                }),
                event_type: EventType::GatewayModified,
                module: ActivityLogModule::Defguard,
                description_contains: Some("Modified"),
            },
            EventTestCase {
                name: "GatewayDeleted",
                message: api_message(ApiEventType::GatewayDeleted { gateway }),
                event_type: EventType::GatewayDeleted,
                module: ActivityLogModule::Defguard,
                description_contains: Some("Deleted"),
            },
            EventTestCase {
                name: "DevicePostureCreated",
                message: api_message(ApiEventType::DevicePostureCreated {
                    snapshot: posture_snapshot.clone(),
                }),
                event_type: EventType::DevicePostureCreated,
                module: ActivityLogModule::Defguard,
                description_contains: Some("Created"),
            },
            EventTestCase {
                name: "DevicePostureUpdated",
                message: api_message(ApiEventType::DevicePostureUpdated {
                    before: posture_snapshot.clone(),
                    after: posture_snapshot2.clone(),
                }),
                event_type: EventType::DevicePostureUpdated,
                module: ActivityLogModule::Defguard,
                description_contains: Some("Updated"),
            },
            EventTestCase {
                name: "DevicePostureDeleted",
                message: api_message(ApiEventType::DevicePostureDeleted {
                    snapshot: posture_snapshot.clone(),
                }),
                event_type: EventType::DevicePostureDeleted,
                module: ActivityLogModule::Defguard,
                description_contains: Some("Deleted"),
            },
            EventTestCase {
                name: "DevicePostureDuplicated",
                message: api_message(ApiEventType::DevicePostureDuplicated {
                    original: posture_snapshot,
                    duplicate: posture_snapshot2,
                }),
                event_type: EventType::DevicePostureDuplicated,
                module: ActivityLogModule::Defguard,
                description_contains: Some("Duplicated"),
            },
            EventTestCase {
                name: "DevicePostureLocationsAssigned",
                message: api_message(ApiEventType::DevicePostureLocationsAssigned {
                    device_posture: posture,
                    location_ids: vec![10],
                }),
                event_type: EventType::DevicePostureLocationsAssigned,
                module: ActivityLogModule::Defguard,
                description_contains: Some("Assigned"),
            },
            EventTestCase {
                name: "LocationPosturesAssigned",
                message: api_message(ApiEventType::LocationPosturesAssigned {
                    location: location.clone(),
                    posture_ids: vec![1],
                }),
                event_type: EventType::LocationPosturesAssigned,
                module: ActivityLogModule::Defguard,
                description_contains: Some("Assigned"),
            },
        ];

        assert_eq!(
            cases.len(),
            ApiEventType::COUNT,
            "missing test case for new ApiEventType variant"
        );
        cases
    }

    fn bidi_event_cases() -> Vec<EventTestCase> {
        let location = sample_location();
        let device = sample_device();

        fn bidi_msg(
            stream_event: BidiStreamEventType,
            loc: Option<WireguardNetwork<i64>>,
        ) -> EventLoggerMessage {
            EventLoggerMessage {
                context: EventContext::from_bidi_context(
                    BidiRequestContext::new(
                        1,
                        "alice".into(),
                        IpAddr::V4(Ipv4Addr::LOCALHOST),
                        "desktop-app".into(),
                    ),
                    loc,
                ),
                event: Event::Bidi(stream_event),
            }
        }

        let cases = vec![
            EventTestCase {
                name: "EnrollmentStarted",
                message: bidi_msg(
                    BidiStreamEventType::Enrollment(Box::new(
                        CoreEnrollmentEvent::EnrollmentStarted,
                    )),
                    None,
                ),
                event_type: EventType::EnrollmentStarted,
                module: ActivityLogModule::Enrollment,
                description_contains: Some("started"),
            },
            EventTestCase {
                name: "EnrollmentCompleted",
                message: bidi_msg(
                    BidiStreamEventType::Enrollment(Box::new(
                        CoreEnrollmentEvent::EnrollmentCompleted,
                    )),
                    None,
                ),
                event_type: EventType::EnrollmentCompleted,
                module: ActivityLogModule::Enrollment,
                description_contains: Some("completed"),
            },
            EventTestCase {
                name: "EnrollmentDeviceAdded",
                message: bidi_msg(
                    BidiStreamEventType::Enrollment(Box::new(
                        CoreEnrollmentEvent::EnrollmentDeviceAdded {
                            device: device.clone(),
                        },
                    )),
                    None,
                ),
                event_type: EventType::EnrollmentDeviceAdded,
                module: ActivityLogModule::Enrollment,
                description_contains: Some("Added"),
            },
            EventTestCase {
                name: "PasswordResetRequested",
                message: bidi_msg(
                    BidiStreamEventType::PasswordReset(Box::new(
                        PasswordResetEvent::PasswordResetRequested,
                    )),
                    None,
                ),
                event_type: EventType::PasswordResetRequested,
                module: ActivityLogModule::Enrollment,
                description_contains: None,
            },
            EventTestCase {
                name: "PasswordResetStarted",
                message: bidi_msg(
                    BidiStreamEventType::PasswordReset(Box::new(
                        PasswordResetEvent::PasswordResetStarted,
                    )),
                    None,
                ),
                event_type: EventType::PasswordResetStarted,
                module: ActivityLogModule::Enrollment,
                description_contains: None,
            },
            EventTestCase {
                name: "PasswordResetCompleted",
                message: bidi_msg(
                    BidiStreamEventType::PasswordReset(Box::new(
                        PasswordResetEvent::PasswordResetCompleted,
                    )),
                    None,
                ),
                event_type: EventType::PasswordResetCompleted,
                module: ActivityLogModule::Enrollment,
                description_contains: None,
            },
            EventTestCase {
                name: "ClientMfaSuccess",
                message: bidi_msg(
                    BidiStreamEventType::DesktopClientMfa(Box::new(
                        DesktopClientMfaEvent::Success {
                            location: location.clone(),
                            device: device.clone(),
                            method: defguard_core::events::ClientMFAMethod::Totp,
                        },
                    )),
                    Some(location.clone()),
                ),
                event_type: EventType::VpnClientMfaSuccess,
                module: ActivityLogModule::Vpn,
                description_contains: Some("completed"),
            },
            EventTestCase {
                name: "ClientMfaFailed",
                message: bidi_msg(
                    BidiStreamEventType::DesktopClientMfa(Box::new(
                        DesktopClientMfaEvent::Failed {
                            location: location.clone(),
                            device: device.clone(),
                            method: defguard_core::events::ClientMFAMethod::Totp,
                            message: "bad".into(),
                        },
                    )),
                    Some(location.clone()),
                ),
                event_type: EventType::VpnClientMfaFailed,
                module: ActivityLogModule::Vpn,
                description_contains: Some("failed"),
            },
            EventTestCase {
                name: "ClientMfaDisconnected",
                message: bidi_msg(
                    BidiStreamEventType::DesktopClientMfa(Box::new(
                        DesktopClientMfaEvent::Disconnected {
                            location: location.clone(),
                            device,
                            is_mfa_session: true,
                        },
                    )),
                    Some(location),
                ),
                event_type: EventType::VpnClientMfaDisconnected,
                module: ActivityLogModule::Vpn,
                description_contains: Some("disconnected"),
            },
        ];

        assert_eq!(
            cases.len(),
            CoreEnrollmentEvent::COUNT + PasswordResetEvent::COUNT + DesktopClientMfaEvent::COUNT,
            "missing test case for new BidiStreamEventType leaf variant"
        );
        cases
    }

    fn session_manager_cases() -> Vec<EventTestCase> {
        let location = sample_location();
        let device = sample_device();

        fn sm_msg(
            event: SessionManagerEventType,
            loc: WireguardNetwork<i64>,
            dev: Device<i64>,
        ) -> EventLoggerMessage {
            EventLoggerMessage {
                context: test_context(),
                event: Event::SessionManager {
                    event,
                    location: loc,
                    device: dev,
                },
            }
        }

        let cases = vec![
            EventTestCase {
                name: "ClientConnected",
                message: sm_msg(
                    SessionManagerEventType::ClientConnected,
                    location.clone(),
                    device.clone(),
                ),
                event_type: EventType::VpnClientConnected,
                module: ActivityLogModule::Vpn,
                description_contains: Some("connected"),
            },
            EventTestCase {
                name: "ClientDisconnected",
                message: sm_msg(
                    SessionManagerEventType::ClientDisconnected,
                    location.clone(),
                    device.clone(),
                ),
                event_type: EventType::VpnClientDisconnected,
                module: ActivityLogModule::Vpn,
                description_contains: Some("disconnected"),
            },
            EventTestCase {
                name: "MfaClientConnected",
                message: sm_msg(
                    SessionManagerEventType::MfaClientConnected,
                    location.clone(),
                    device.clone(),
                ),
                event_type: EventType::VpnClientMfaConnected,
                module: ActivityLogModule::Vpn,
                description_contains: Some("connected"),
            },
            EventTestCase {
                name: "MfaClientDisconnected",
                message: sm_msg(
                    SessionManagerEventType::MfaClientDisconnected,
                    location,
                    device,
                ),
                event_type: EventType::VpnClientMfaDisconnected,
                module: ActivityLogModule::Vpn,
                description_contains: Some("disconnected"),
            },
        ];

        assert_eq!(
            cases.len(),
            SessionManagerEventType::COUNT,
            "missing test case for new SessionManagerEventType variant"
        );
        cases
    }

    #[test]
    fn test_all_event_variants_map_to_correct_activity_log_events() {
        let mut cases = api_event_cases();
        cases.extend(bidi_event_cases());
        cases.extend(session_manager_cases());

        for case in cases {
            let result = map_to_activity_log_event(case.message);
            assert_eq!(
                result.event, case.event_type,
                "{}: wrong event type",
                case.name
            );
            assert_eq!(result.module, case.module, "{}: wrong module", case.name);
            if let Some(substr) = case.description_contains {
                let description = result
                    .description
                    .unwrap_or_else(|| panic!("{}: expected description", case.name));
                assert!(
                    description.to_lowercase().contains(&substr.to_lowercase()),
                    "{}: description '{description}' missing '{substr}'",
                    case.name
                );
            }
        }
    }
}
