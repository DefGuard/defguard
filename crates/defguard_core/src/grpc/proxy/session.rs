use chrono::Utc;
use sqlx::PgConnection;

use defguard_common::db::{
    Id,
    models::{
        Device, User, WireguardNetwork,
        vpn_client_session::{VpnClientMfaMethod, VpnClientSession, VpnClientSessionState},
    },
};
use tokio::sync::{broadcast::Sender, mpsc::UnboundedSender};
use tonic::Status;

use crate::{
    events::{BidiRequestContext, BidiStreamEvent, BidiStreamEventType, DesktopClientMfaEvent},
    grpc::{GatewayEvent, proxy::client_mfa::ClientMfaServerError},
};

/// Helper used to close all existing active sessions while creating a new MFA session
/// and send relevant gateway updates
pub(crate) async fn create_new_session(
    conn: &mut PgConnection,
    location: &WireguardNetwork<Id>,
    user: &User<Id>,
    device: &Device<Id>,
    mfa_method: Option<VpnClientMfaMethod>,
    preshared_key: String,
    wireguard_tx: Sender<GatewayEvent>,
    bidi_event_tx: UnboundedSender<BidiStreamEvent>,
) -> Result<VpnClientSession<Id>, Status> {
    debug!(
        "Creating new VPN session for device {device} of user {user} in location {location} after successful MFA authorization."
    );

    // find all active sessions for a given device and location
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

    // disconnect all active sessions
    for session in active_sessions {
        debug!("Disconnecting previous active MFA VPN session {session:?}.");
        disconnect_session(
            &mut *conn,
            session,
            location,
            user,
            device,
            wireguard_tx.clone(),
            bidi_event_tx.clone(),
        )
        .await?;
    }

    // create new MFA session
    let mut session =
        VpnClientSession::new(location.id, user.id, device.id, None, mfa_method);
    session.preshared_key = Some(preshared_key);
    session.save(conn).await.map_err(|err| {
        error!("Failed to create new VPN client session for device {device} in location {location}: {err}");
        Status::internal("unexpected error")
    })
}

/// Update session state as disconnected and send relevant gateway update
pub(crate) async fn disconnect_session(
    conn: &mut PgConnection,
    mut session: VpnClientSession<Id>,
    location: &WireguardNetwork<Id>,
    user: &User<Id>,
    device: &Device<Id>,
    wireguard_tx: Sender<GatewayEvent>,
    bidi_event_tx: UnboundedSender<BidiStreamEvent>,
) -> Result<(), Status> {
    let is_connected = session.state == VpnClientSessionState::Connected;
    let is_mfa_session = session.mfa_method.is_some();

    // update session state in DB
    let disconnect_timestamp = Utc::now().naive_utc();
    session.disconnected_at = Some(disconnect_timestamp);
    session.state = VpnClientSessionState::Disconnected;
    session.save(&mut *conn).await.map_err(|err| {
        error!("Failed to update VPN session {session:?}: {err}");
        Status::internal("unexpected error")
    })?;

    // gateway update is only needed to remove peer for MFA sessions
    // this is needed to remove peers for both Connected and New sessions
    if is_mfa_session {
        let gateway_event = GatewayEvent::MfaSessionDisconnected(location.id, device.clone());
        wireguard_tx.send(gateway_event).map_err(|err| {
            error!("Error sending WireGuard event: {err}");
            Status::internal("unexpected error")
        })?;
    }

    // only emit disconnect events if a session has actually been connected
    if is_connected {
        let context = BidiRequestContext {
            timestamp: disconnect_timestamp,
            user_id: user.id,
            username: user.username.clone(),
            ip: None,
            device_name: format!("{device}"),
        };
        bidi_event_tx
            .send(BidiStreamEvent {
                context,
                event: BidiStreamEventType::DesktopClientMfa(Box::new(
                    DesktopClientMfaEvent::Disconnected {
                        location: location.clone(),
                        device: device.clone(),
                        is_mfa_session,
                    },
                )),
            })
        .map_err(ClientMfaServerError::from).map_err(Status::from)?;
    }

    Ok(())
}
