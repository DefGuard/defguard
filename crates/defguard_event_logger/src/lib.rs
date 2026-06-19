use std::sync::Arc;

use bytes::Bytes;
use defguard_common::db::NoId;
use defguard_core::{
    db::models::activity_log::{
        ActivityLogEvent, ActivityLogModule, EventType,
        metadata::{
            ActivityLogStreamMetadata, ActivityLogStreamModifiedMetadata, ApiTokenMetadata,
            ApiTokenRenamedMetadata, AuthenticationKeyMetadata, AuthenticationKeyRenamedMetadata,
            ClientConfigurationTokenMetadata, ClientDeviceMetadata, DeviceMetadata,
            DeviceModifiedMetadata, EnrollmentDeviceAddedMetadata, EnrollmentTokenMetadata,
            GatewayDeletedMetadata, GatewayModifiedMetadata, GroupAssignedMetadata,
            GroupMembersModifiedMetadata, GroupMetadata, GroupModifiedMetadata,
            GroupsBulkAssignedMetadata, LoginFailedMetadata, MfaLoginFailedMetadata,
            MfaLoginMetadata, MfaSecurityKeyMetadata, NetworkDeviceMetadata,
            NetworkDeviceModifiedMetadata, OpenIdAppMetadata, OpenIdAppModifiedMetadata,
            OpenIdAppStateChangedMetadata, OpenIdProviderMetadata, PasswordChangedByAdminMetadata,
            PasswordResetMetadata, ProxyDeletedMetadata, ProxyModifiedMetadata,
            SettingsUpdateMetadata, UserGroupsModifiedMetadata, UserMetadata,
            UserMfaDisabledMetadata, UserModifiedMetadata, UserSnatBindingMetadata,
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
            event = api_event_rx.recv() => if let Some(e) = event { EventLoggerMessage::from_api_event(e, &reload_notify) } else {
                error!("API event channel closed");
                break;
            },
            event = bidi_event_rx.recv() => if let Some(e) = event { EventLoggerMessage::from_bidi_event(e) } else {
                error!("Bidi gRPC stream event channel closed");
                break;
            },
            event = session_manager_event_rx.recv() => if let Some(e) = event { EventLoggerMessage::from_session_manager_event(e) } else {
                error!("Session manager event channel closed");
                break;
            },
        };

        if event_logger_tx.send(message).is_err() {
            debug!("Event logger channel closed, translation task shutting down");
            break;
        }
    }
}

/// Returns the activity log module for an API-derived event.
///
/// Most API events are Defguard management events and keep the historical
/// `Defguard` module. Device posture management events are categorized under
/// `Posture` so they can be filtered and exported independently.
fn api_event_module(event_type: &EventType) -> ActivityLogModule {
    match event_type {
        EventType::DevicePostureCreated
        | EventType::DevicePostureUpdated
        | EventType::DevicePostureDeleted
        | EventType::DevicePostureDuplicated
        | EventType::DevicePostureLocationsAssigned
        | EventType::LocationPosturesAssigned => ActivityLogModule::Posture,
        _ => ActivityLogModule::Defguard,
    }
}

/// Returns the activity log module for a BIDI-derived event.
///
/// Desktop client MFA events default to the `Vpn` module because they describe
/// VPN authorization/session activity. Posture check pass/fail events are routed
/// to `Posture` so posture evaluation activity is not mixed with generic VPN MFA
/// activity.
fn bidi_event_module(event_type: &EventType) -> ActivityLogModule {
    match event_type {
        EventType::DevicePostureCheckPassed | EventType::DevicePostureCheckFailed => {
            ActivityLogModule::Posture
        }
        _ => ActivityLogModule::Vpn,
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
                ApiEventType::UserEnabled { user } => (
                    EventType::UserEnabled,
                    serde_json::to_value(UserMetadata { user: user.into() }).ok(),
                ),
                ApiEventType::UserDisabled { user } => (
                    EventType::UserDisabled,
                    serde_json::to_value(UserMetadata { user: user.into() }).ok(),
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

            let module = api_event_module(&event_type);
            (module, event_type, description, metadata)
        }
        Event::Bidi(BidiStreamEventType::DesktopClientMfa(event)) => {
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
                DesktopClientMfaEvent::PostureCheckPassed { device, .. } => {
                    Some(format!("Device posture check passed for device {device}"))
                }
                DesktopClientMfaEvent::PostureCheckFailed {
                    device,
                    failed_checks,
                    ..
                } => Some(format!(
                    "Device posture check failed for device {device}: {}",
                    failed_checks.join(",")
                )),
                DesktopClientMfaEvent::SessionReplaced {
                    device, location, ..
                } => Some(format!(
                    "Existing VPN session for device {device} in location {location} was closed because a new session was authorized"
                )),
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
                DesktopClientMfaEvent::PostureCheckPassed { device, .. } => (
                    EventType::DevicePostureCheckPassed,
                    serde_json::to_value(ClientDeviceMetadata {
                        device_id: device.id,
                        device_name: device.name.clone(),
                    })
                    .ok(),
                ),
                DesktopClientMfaEvent::PostureCheckFailed { device, .. } => (
                    EventType::DevicePostureCheckFailed,
                    serde_json::to_value(ClientDeviceMetadata {
                        device_id: device.id,
                        device_name: device.name.clone(),
                    })
                    .ok(),
                ),
                DesktopClientMfaEvent::SessionReplaced {
                    location, device, ..
                } => (
                    EventType::VpnClientMfaSessionReplaced,
                    serde_json::to_value(VpnClientMetadata { location, device }).ok(),
                ),
            };
            let module = bidi_event_module(&event_type);
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
mod tests;
