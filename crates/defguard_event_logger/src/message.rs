use std::net::IpAddr;

use chrono::NaiveDateTime;
use defguard_common::db::{
    Id,
    models::{Device, Settings, WireguardNetwork},
};
use defguard_core::events::{
    ApiEvent, ApiEventType, ApiRequestContext, BidiRequestContext, BidiStreamEvent,
    BidiStreamEventType, DesktopClientMfaEvent, GrpcRequestContext, LdapSyncEventType,
};
use defguard_session_manager::events::{
    SessionManagerEvent, SessionManagerEventContext, SessionManagerEventType,
};
use tokio::sync::Notify;

/// Thin wrapper around source event types.
///
/// The logger matches on these directly when building activity log entries.
/// No translation — source types are the single source of truth.
#[allow(clippy::large_enum_variant)]
pub enum Event {
    Api(ApiEventType),
    Bidi(BidiStreamEventType),
    SessionManager {
        event: SessionManagerEventType,
        location: WireguardNetwork<Id>,
        device: Device<Id>,
    },
    LdapSync {
        /// Whether the directory backend is Active Directory (vs. plain LDAP).
        /// Read from settings at log time to pick the activity log module.
        uses_ad: bool,
        event: LdapSyncEventType,
    },
}

/// Messages that can be sent to the event logger
pub struct EventLoggerMessage {
    pub context: EventContext,
    pub event: Event,
}

impl EventLoggerMessage {
    /// Translate an API event into a logger message.
    ///
    /// Side effect: calls `reload_notify.notify_waiters()` when the event is an
    /// activity log stream configuration change.
    pub fn from_api_event(api_event: ApiEvent, reload_notify: &Notify) -> Self {
        // Side effect: activity log stream configuration changed
        if matches!(
            *api_event.event,
            ApiEventType::ActivityLogStreamCreated { .. }
                | ApiEventType::ActivityLogStreamModified { .. }
                | ApiEventType::ActivityLogStreamRemoved { .. }
        ) {
            reload_notify.notify_waiters();
        }

        let location = extract_api_location(&api_event.event);

        Self {
            context: EventContext::from_api_context(api_event.context, location),
            event: Event::Api(*api_event.event),
        }
    }

    /// Translate a bidirectional gRPC stream event into a logger message.
    #[must_use]
    pub fn from_bidi_event(bidi_event: BidiStreamEvent) -> Self {
        let BidiStreamEvent { context, event } = bidi_event;

        let location = match &event {
            BidiStreamEventType::DesktopClientMfa(mfa) => match mfa.as_ref() {
                DesktopClientMfaEvent::Success { location, .. }
                | DesktopClientMfaEvent::Failed { location, .. }
                | DesktopClientMfaEvent::Disconnected { location, .. }
                | DesktopClientMfaEvent::PostureCheckPassed { location, .. }
                | DesktopClientMfaEvent::PostureCheckFailed { location, .. }
                | DesktopClientMfaEvent::SessionSuperseded { location, .. } => {
                    Some(location.clone())
                }
            },
            _ => None,
        };

        Self {
            context: EventContext::from_bidi_context(context, location),
            event: Event::Bidi(event),
        }
    }

    /// Translate a session manager event into a logger message.
    #[must_use]
    pub fn from_session_manager_event(session_event: SessionManagerEvent) -> Self {
        let location = session_event.context.location.clone();
        let device = session_event.context.device.clone();
        Self {
            context: EventContext::from_session_manager_context(session_event.context),
            event: Event::SessionManager {
                event: session_event.event,
                location,
                device,
            },
        }
    }

    /// Translate an LDAP sync event into a logger message.
    #[must_use]
    pub fn from_ldap_sync_event(event: LdapSyncEventType) -> Self {
        // Read the directory backend type at log time to pick the activity log
        // module (Active Directory vs. plain LDAP). The event types themselves are
        // shared between both backends.
        let uses_ad = Settings::get_current_settings().ldap_uses_ad;
        Self {
            context: EventContext::system_ldap_sync(),
            event: Event::LdapSync { uses_ad, event },
        }
    }
}

/// Extract location from an API event variant, if it carries one.
fn extract_api_location(event: &ApiEventType) -> Option<WireguardNetwork<Id>> {
    match event {
        ApiEventType::NetworkDeviceAdded { location, .. }
        | ApiEventType::NetworkDeviceModified { location, .. }
        | ApiEventType::NetworkDeviceRemoved { location, .. }
        | ApiEventType::VpnLocationAdded { location }
        | ApiEventType::VpnLocationRemoved { location }
        | ApiEventType::UserSnatBindingAdded { location, .. }
        | ApiEventType::UserSnatBindingRemoved { location, .. }
        | ApiEventType::UserSnatBindingModified { location, .. } => Some(location.clone()),
        ApiEventType::VpnLocationModified { after, .. } => Some(after.clone()),
        _ => None,
    }
}

/// Shared context that's included in all activity log events
pub struct EventContext {
    pub timestamp: NaiveDateTime,
    pub user_id: Option<Id>,
    pub username: String,
    pub location: Option<String>,
    pub ip: Option<IpAddr>,
    pub device: String,
}

impl EventContext {
    #[must_use]
    pub fn from_api_context(
        val: ApiRequestContext,
        location: Option<WireguardNetwork<Id>>,
    ) -> Self {
        let location = location.map(|location| location.name);

        Self {
            timestamp: val.timestamp,
            user_id: val.user_id,
            username: val.username,
            location,
            ip: val.ip,
            device: val.device,
        }
    }

    #[must_use]
    pub fn from_bidi_context(
        val: BidiRequestContext,
        location: Option<WireguardNetwork<Id>>,
    ) -> Self {
        let location = location.map(|location| location.name);

        Self {
            timestamp: val.timestamp,
            user_id: Some(val.user_id),
            username: val.username,
            location,
            ip: val.ip,
            device: val.device_name,
        }
    }

    #[must_use]
    pub fn from_session_manager_context(val: SessionManagerEventContext) -> Self {
        Self {
            timestamp: val.timestamp,
            user_id: Some(val.user.id),
            username: val.user.username,
            location: Some(val.location.name),
            ip: val.public_ip,
            device: format!("{} (ID {})", val.device.name, val.device.id),
        }
    }

    #[must_use]
    pub fn system_ldap_sync() -> Self {
        Self {
            timestamp: chrono::Utc::now().naive_utc(),
            user_id: None,
            username: "system:ldap-sync".to_owned(),
            location: None,
            ip: None,
            device: "system".to_owned(),
        }
    }
}

impl From<GrpcRequestContext> for EventContext {
    fn from(val: GrpcRequestContext) -> Self {
        Self {
            timestamp: val.timestamp,
            user_id: Some(val.user_id),
            username: val.username,
            location: Some(val.location.name),
            ip: val.ip,
            device: format!("{} (ID {})", val.device_name, val.device_id),
        }
    }
}
