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
use tonic::{Code, Status};

use crate::events::{
    BidiRequestContext, BidiStreamEvent, BidiStreamEventType, DesktopClientMfaEvent,
};

#[derive(Debug, Error)]
pub enum ClientMfaServerError {
    #[error("gRPC event channel error: {0}")]
    BidiEventChannelError(#[from] SendError<BidiStreamEvent>),
}

impl From<ClientMfaServerError> for Status {
    fn from(value: ClientMfaServerError) -> Self {
        Self::new(Code::Internal, value.to_string())
    }
}

pub enum SessionDisconnectReason {
    /// Closed because a new authorization is creating a replacement session.
    Superseded,
    /// Closed for any other reason (normal teardown).
    Disconnected,
}

/// Emit a bidi-stream event to the proxy.
pub fn emit_event(
    bidi_event_tx: &UnboundedSender<BidiStreamEvent>,
    event: BidiStreamEvent,
) -> Result<(), ClientMfaServerError> {
    Ok(bidi_event_tx.send(event)?)
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
    gateway_tx: &Sender<GatewayCommand>,
    bidi_event_tx: &UnboundedSender<BidiStreamEvent>,
    conn: &mut PgConnection,
    location: &WireguardNetwork<Id>,
    user: &User<Id>,
    device: &Device<Id>,
    is_mfa_session: bool,
    preshared_key: String,
) -> Result<VpnClientSession<Id>, Status> {
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
        Status::internal("unexpected error")
    })?;
    if !active_sessions.is_empty() {
        info!(
            "Found {} active sessions for device {device} in location {location}. Disconnecting them before creating a new MFA session",
            active_sessions.len()
        );
    }

    for session in active_sessions {
        debug!("Disconnecting previous active MFA VPN session {session:?}.");
        disconnect_session(
            gateway_tx,
            bidi_event_tx,
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
        Status::internal("unexpected error")
    })
}

/// Mark a session disconnected, sending the gateway deauthorization and (for a connected session)
/// the disconnect audit event.
pub async fn disconnect_session(
    gateway_tx: &Sender<GatewayCommand>,
    bidi_event_tx: &UnboundedSender<BidiStreamEvent>,
    conn: &mut PgConnection,
    mut session: VpnClientSession<Id>,
    location: &WireguardNetwork<Id>,
    user: &User<Id>,
    device: &Device<Id>,
    reason: SessionDisconnectReason,
) -> Result<(), Status> {
    let is_connected = session.state == VpnClientSessionState::Connected;
    let is_mfa_session = session.is_mfa_session;
    let requires_gateway_update = is_mfa_session
        || location.has_postures(&mut *conn).await.map_err(|err| {
            error!("Failed to fetch postures for location {location}: {err}");
            Status::internal("unexpected error")
        })?;

    let disconnect_timestamp = Utc::now().naive_utc();
    session.disconnected_at = Some(disconnect_timestamp);
    session.state = VpnClientSessionState::Disconnected;
    session.save(&mut *conn).await.map_err(|err| {
        error!("Failed to update VPN session {session:?}: {err}");
        Status::internal("unexpected error")
    })?;

    // The gateway update is only needed to remove peers authorized at runtime (MFA and
    // posture-check sessions), for both connected and new sessions.
    if requires_gateway_update {
        let gateway_event = GatewayCommand::VpnSessionDeauthorized(location.id, device.clone());
        gateway_tx.send(gateway_event).map_err(|err| {
            error!("Error sending WireGuard event: {err}");
            Status::internal("unexpected error")
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
        emit_event(
            bidi_event_tx,
            BidiStreamEvent {
                context,
                event: BidiStreamEventType::DesktopClientMfa(Box::new(event)),
            },
        )
        .map_err(Status::from)?;
    }

    Ok(())
}
