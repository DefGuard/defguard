use chrono::Utc;
use defguard_common::{
    db::{
        Id,
        models::{
            Device, User, WireguardNetwork,
            device::{DeviceNetworkInfo, WireguardNetworkDevice},
            vpn_client_session::{VpnClientSession, VpnClientSessionState},
        },
    },
    gateway_event::GatewayCommand,
};
use sqlx::PgConnection;
use thiserror::Error;
use tokio::sync::{
    broadcast::Sender,
    mpsc::{UnboundedSender, error::SendError},
};

use crate::events::{
    BidiRequestContext, BidiStreamEvent, BidiStreamEventType, DesktopClientMfaEvent,
};

#[derive(Debug, Error)]
pub enum ClientMfaServerError {
    #[error("gRPC event channel error: {0}")]
    BidiEventChannelError(#[from] SendError<BidiStreamEvent>),
}

/// Error surfaced by the authorize free functions (`create_new_session` / `disconnect_session`).
#[derive(Debug, Error)]
pub enum AuthorizeError {
    #[error(transparent)]
    Db(#[from] sqlx::Error),
    #[error(transparent)]
    Event(#[from] ClientMfaServerError),
    #[error("gateway event channel error: {0}")]
    Gateway(Box<tokio::sync::broadcast::error::SendError<GatewayCommand>>),
}

pub enum SessionDisconnectReason {
    /// Closed because a new authorization is creating a replacement session.
    Superseded,
    /// Closed for any other reason (normal teardown).
    Disconnected,
}

/// The two outbound channels the MFA engine and the posture path push to. Bundling them keeps
/// the pair from travelling as two loose parameters through the authorize free functions.
pub struct EventChannels {
    pub gateway_tx: Sender<GatewayCommand>,
    pub bidi_event_tx: UnboundedSender<BidiStreamEvent>,
}

impl EventChannels {
    #[must_use]
    pub fn new(
        gateway_tx: Sender<GatewayCommand>,
        bidi_event_tx: UnboundedSender<BidiStreamEvent>,
    ) -> Self {
        Self {
            gateway_tx,
            bidi_event_tx,
        }
    }

    /// Emit a bidi-stream event to the proxy.
    pub fn emit_event(&self, event: BidiStreamEvent) -> Result<(), ClientMfaServerError> {
        Ok(self.bidi_event_tx.send(event)?)
    }
}

/// Build the gateway network info handed to the gateway when a device is authorized.
pub fn build_authorized_gateway_network_info(
    network_device: WireguardNetworkDevice,
    preshared_key: String,
) -> DeviceNetworkInfo {
    DeviceNetworkInfo::from_authorized_vpn_session(
        network_device.wireguard_network_id,
        network_device.wireguard_ips,
        preshared_key,
    )
}

/// Close all active sessions for a device and location, then create a fresh authorized session
/// carrying `preshared_key`.
pub async fn create_new_session(
    channels: &EventChannels,
    conn: &mut PgConnection,
    location: &WireguardNetwork<Id>,
    user: &User<Id>,
    device: &Device<Id>,
    is_mfa_session: bool,
    preshared_key: String,
) -> Result<VpnClientSession<Id>, AuthorizeError> {
    debug!("Creating new VPN session for device {device} of user {user} in location {location}.");

    let active_sessions = VpnClientSession::get_all_active_device_sessions_in_location(
        &mut *conn,
        location.id,
        device.id,
    )
    .await
    .map_err(|err| {
        error!(
            "Failed to fetch active VPN sessions for device {device} in location {location}: {err}"
        );
        AuthorizeError::Db(err)
    })?;
    if !active_sessions.is_empty() {
        info!(
            "Found {} active sessions for device {device} in location {location}. Disconnecting them before creating a new MFA session",
            active_sessions.len()
        );
    }

    for session in active_sessions {
        debug!(
            "Disconnecting previous active MFA VPN session {}",
            session.id
        );
        disconnect_session(
            channels,
            &mut *conn,
            session,
            location,
            user,
            device,
            SessionDisconnectReason::Superseded,
        )
        .await?;
    }

    let mut session = VpnClientSession::new(location.id, user.id, device.id, None, is_mfa_session);
    session.preshared_key = Some(preshared_key);
    session.save(conn).await.map_err(|err| {
        error!("Failed to create new VPN client session for device {device} in location {location}: {err}");
        AuthorizeError::Db(err)
    })
}

/// Mark a session disconnected, sending the gateway deauthorization and (for a connected session)
/// the disconnect audit event.
pub async fn disconnect_session(
    channels: &EventChannels,
    conn: &mut PgConnection,
    mut session: VpnClientSession<Id>,
    location: &WireguardNetwork<Id>,
    user: &User<Id>,
    device: &Device<Id>,
    reason: SessionDisconnectReason,
) -> Result<(), AuthorizeError> {
    let is_connected = session.state == VpnClientSessionState::Connected;
    let is_mfa_session = session.is_mfa_session;
    let requires_gateway_update = is_mfa_session
        || location.has_postures(&mut *conn).await.map_err(|err| {
            error!("Failed to fetch postures for location {location}: {err}");
            AuthorizeError::Db(err)
        })?;

    let disconnect_timestamp = Utc::now().naive_utc();
    session.disconnected_at = Some(disconnect_timestamp);
    session.state = VpnClientSessionState::Disconnected;
    session.save(&mut *conn).await.map_err(|err| {
        error!("Failed to update VPN session {}: {err}", session.id);
        AuthorizeError::Db(err)
    })?;

    // The gateway update is only needed to remove peers authorized at runtime (MFA and
    // posture-check sessions), for both connected and new sessions.
    if requires_gateway_update {
        let gateway_event = GatewayCommand::VpnSessionDeauthorized(location.id, device.clone());
        channels.gateway_tx.send(gateway_event).map_err(|err| {
            error!("Error sending WireGuard event: {err}");
            AuthorizeError::Gateway(Box::new(err))
        })?;
    }

    // Only emit a disconnect event if the session was actually connected.
    if is_connected {
        let context = BidiRequestContext {
            timestamp: disconnect_timestamp,
            user_id: user.id,
            username: user.username.clone(),
            ip: None,
            device_name: format!("{device}"),
        };
        let event = match reason {
            SessionDisconnectReason::Superseded => DesktopClientMfaEvent::SessionSuperseded {
                location: location.clone(),
                device: device.clone(),
                is_mfa_session,
            },
            SessionDisconnectReason::Disconnected => DesktopClientMfaEvent::Disconnected {
                location: location.clone(),
                device: device.clone(),
                is_mfa_session,
            },
        };
        channels.emit_event(BidiStreamEvent {
            context,
            event: BidiStreamEventType::DesktopClientMfa(Box::new(event)),
        })?;
    }

    Ok(())
}
