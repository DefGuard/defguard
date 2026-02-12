use std::net::{IpAddr, Ipv4Addr};

use chrono::Utc;
use defguard_common::{
    db::{
        Id,
        models::{
            Device, User, WireguardNetwork,
            device::WireguardNetworkDevice,
            vpn_client_session::{VpnClientSession, VpnClientSessionState},
        },
    },
    messages::peer_stats_update::PeerStatsUpdate,
};
use defguard_core::grpc::gateway::events::GatewayEvent;
use sqlx::{PgConnection, PgPool};
use tokio::{
    sync::{
        broadcast::Sender,
        mpsc::{UnboundedReceiver, UnboundedSender},
    },
    time::{Duration, interval},
};
use tracing::{debug, error, info, trace, warn};

use crate::{
    error::SessionManagerError,
    events::{SessionManagerEvent, SessionManagerEventContext, SessionManagerEventType},
    session_state::ActiveSessionsMap,
};

pub mod error;
pub mod events;
pub mod session_state;

const MESSAGE_LIMIT: usize = 100;
const SESSION_UPDATE_INTERVAL: u64 = 60;

pub async fn run_session_manager(
    pool: PgPool,
    mut peer_stats_rx: UnboundedReceiver<PeerStatsUpdate>,
    session_manager_event_tx: UnboundedSender<SessionManagerEvent>,
    gateway_tx: Sender<GatewayEvent>,
) -> Result<(), SessionManagerError> {
    info!("Starting VPN client session manager service");
    let mut session_update_timer = interval(Duration::from_secs(SESSION_UPDATE_INTERVAL));

    // initialize session manager
    let mut session_manager = SessionManager::new(pool, session_manager_event_tx, gateway_tx);

    loop {
        // receive next batch of peer stats messages
        // if no message is received within `SESSION_UPDATE_INTERVAL` trigger session status refresh anyway
        // to disconnect inactive sessions if necessary
        let mut message_buffer: Vec<PeerStatsUpdate> = Vec::with_capacity(MESSAGE_LIMIT);
        let _message_count = tokio::select! {
            message_count = peer_stats_rx.recv_many(&mut message_buffer, MESSAGE_LIMIT) => message_count,
            _ = session_update_timer.tick() => {
                warn!("No wireguard peer stats updates received in last {SESSION_UPDATE_INTERVAL}. Triggering session status update to disconnect inactive clients.");
                session_manager.update_inactive_session_status().await?;

                // skip to next iteration
                continue;
            }

        };

        // process received messages to update active sessions
        session_manager
            .process_message_batch(message_buffer)
            .await?;

        // update inactive/disconnected sessions
        session_manager.update_inactive_session_status().await?;
    }
}

struct SessionManager {
    pool: PgPool,
    session_manager_event_tx: UnboundedSender<SessionManagerEvent>,
    gateway_tx: Sender<GatewayEvent>,
}

impl SessionManager {
    fn new(
        pool: PgPool,
        session_manager_event_tx: UnboundedSender<SessionManagerEvent>,
        gateway_tx: Sender<GatewayEvent>,
    ) -> Self {
        Self {
            pool,
            session_manager_event_tx,
            gateway_tx,
        }
    }

    /// Helper function for processing all messages read from the channel in a single batch
    ///
    /// This should only fail if there's an issue with a DB transaction.
    /// Otherwise we just log an error and move on to the next message.
    async fn process_message_batch(
        &mut self,
        messages: Vec<PeerStatsUpdate>,
    ) -> Result<(), SessionManagerError> {
        debug!("Processing batch of {} peer stats updates", messages.len());

        // begin DB transaction
        let mut transaction = self.pool.begin().await?;

        // initialize session map
        let mut active_sessions = ActiveSessionsMap::new();

        for message in messages {
            if let Err(err) = self
                .process_single_message(&mut transaction, &mut active_sessions, message)
                .await
            {
                error!("Failed to process peer stats update: {err}");
            }
        }

        // commit DB transaction after processing all messages
        transaction.commit().await?;

        debug!("Finished processing message batch.");

        Ok(())
    }

    /// Helper function for processing a single message
    async fn process_single_message(
        &mut self,
        transaction: &mut PgConnection,
        active_sessions: &mut ActiveSessionsMap,
        message: PeerStatsUpdate,
    ) -> Result<(), SessionManagerError> {
        trace!("Processing peer stats update: {message:?}");

        // check if a session exists already for a given peer
        // and attempt to add one if necessary
        let maybe_session = if let Some(session) = active_sessions
            .try_get_peer_session(
                transaction,
                message.location_id,
                message.device_pubkey.clone(),
            )
            .await?
        {
            Some(session)
        } else {
            debug!(
                "No active session found for device with pubkey {} in location {}. Creating a new session",
                message.device_pubkey, message.location_id
            );
            active_sessions
                .try_add_new_session(
                    transaction,
                    &message,
                    &message.device_pubkey,
                    &self.session_manager_event_tx,
                )
                .await?
        };

        if let Some(session) = maybe_session {
            // update session stats
            session.update_stats(transaction, message).await?;
        }

        trace!("Finished processing peer stats update");
        Ok(())
    }

    /// Disconnect all inactive sessions
    ///
    /// A session is considered inactive once more than the configured `peer_disconnect_threshold`
    /// has elapsed since the last registered handshake has ocurred.
    /// This threshold is specified per location.
    async fn update_inactive_session_status(&self) -> Result<(), SessionManagerError> {
        info!("Disconnecting inactive VPN sessions");

        // begin DB transaction
        let mut transaction = self.pool.begin().await?;

        // get all locations
        let locations = WireguardNetwork::all(&mut *transaction).await?;
        let locations_count = locations.len();

        for (index, location) in locations.into_iter().enumerate() {
            debug!(
                "[{index}/{locations_count}] Disconnecting inactive sessions in location {location}"
            );

            // get all connected sessions which have become inactive
            let inactive_sessions =
                VpnClientSession::get_all_inactive_for_location(&mut *transaction, &location)
                    .await?;

            debug!(
                "Found {} inactive VPN sessions in location {location}",
                inactive_sessions.len()
            );

            for session in inactive_sessions {
                debug!(
                    "Disconnecting inactive session for user {}, device {} in location {location}",
                    session.user_id, session.device_id
                );
                self.disconnect_session(&mut transaction, session, &location)
                    .await?;
            }

            // get all sessions which were created but have never connected
            // this is only relevant for MFA locations
            if location.mfa_enabled() {
                let unused_sessions =
                    VpnClientSession::get_never_connected(&mut *transaction, &location).await?;

                debug!(
                    "Found {} new VPN sessions which have not connected within required time in location {location}",
                    unused_sessions.len()
                );

                for session in unused_sessions {
                    debug!(
                        "Disconnecting never connected session for user {}, device {} in location {location}",
                        session.user_id, session.device_id
                    );
                    self.disconnect_session(&mut transaction, session, &location)
                        .await?;
                }
            }
        }

        // commit DB transaction after processing all inactive sessions
        transaction.commit().await?;

        debug!("Finished processing inactive VPN sessions");

        Ok(())
    }

    /// Helper user to mark session as disconnected and trigger necessary sideffects
    async fn disconnect_session(
        &self,
        transaction: &mut PgConnection,
        mut session: VpnClientSession<Id>,
        location: &WireguardNetwork<Id>,
    ) -> Result<(), SessionManagerError> {
        let disconnect_timestamp = Utc::now().naive_utc();

        // update session record in DB
        session.disconnected_at = Some(disconnect_timestamp);
        session.state = VpnClientSessionState::Disconnected;
        session.save(&mut *transaction).await?;

        // fetch related objects necessary for event context
        let user = User::find_by_id(&mut *transaction, session.user_id)
            .await?
            .ok_or(SessionManagerError::UserDoesNotExistError(session.user_id))?;
        let device = Device::find_by_id(&mut *transaction, session.device_id)
            .await?
            .ok_or(SessionManagerError::DeviceDoesNotExistError(
                session.device_id,
            ))?;

        // remove peers from GW for MFA locations
        if location.mfa_enabled() {
            // FIXME: remove once MFA-related data is no longer stored here
            // update device network config
            if let Some(mut device_network_info) =
                WireguardNetworkDevice::find(&mut *transaction, device.id, location.id).await?
            {
                device_network_info.is_authorized = false;
                device_network_info.preshared_key = None;
                device_network_info.update(&mut *transaction).await?;
            }
            self.send_peer_disconnect_message(location, &device)?;
        }

        // emit event
        let context = SessionManagerEventContext {
            timestamp: disconnect_timestamp,
            location: location.clone(),
            user,
            device,
            // FIXME: this is a workaround since we require an IP for each audit log event
            public_ip: IpAddr::V4(Ipv4Addr::UNSPECIFIED),
        };
        let event = SessionManagerEvent {
            context,
            event: SessionManagerEventType::ClientDisconnected,
        };
        self.session_manager_event_tx.send(event)?;

        Ok(())
    }

    fn send_peer_disconnect_message(
        &self,
        location: &WireguardNetwork<Id>,
        device: &Device<Id>,
    ) -> Result<(), SessionManagerError> {
        debug!(
            "Sending MFA session disconnect event for device {device} in location {location} to gateway manager"
        );
        let event = GatewayEvent::MfaSessionDisconnected(location.id, device.clone());
        self.gateway_tx.send(event)?;
        Ok(())
    }
}
