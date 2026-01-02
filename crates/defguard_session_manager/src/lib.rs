use defguard_common::messages::peer_stats_update::PeerStatsUpdate;
use sqlx::{PgConnection, PgPool, types::chrono::NaiveDateTime};
use tokio::{
    sync::mpsc::UnboundedReceiver,
    time::{Duration, interval},
};
use tracing::{debug, error, info, trace, warn};

use crate::{error::SessionManagerError, session_state::ActiveSessionsMap};

pub mod error;
pub mod session_state;

const MESSAGE_LIMIT: usize = 100;
const SESSION_UPDATE_INTERVAL: u64 = 60;

pub async fn run_session_manager(
    pool: PgPool,
    mut peer_stats_rx: UnboundedReceiver<PeerStatsUpdate>,
) -> Result<(), SessionManagerError> {
    info!("Starting VPN client session manager service");
    let mut session_update_timer = interval(Duration::from_secs(SESSION_UPDATE_INTERVAL));

    // initialize session manager
    let mut session_manager = SessionManager::new(pool).await?;

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
    // active_sessions: LocationSessionsMap,
}

impl SessionManager {
    async fn new(pool: PgPool) -> Result<Self, SessionManagerError> {
        // initialize active sessions state based on DB content
        // let active_sessions = LocationSessionsMap::initialize_from_db(&pool).await?;

        Ok(Self {
            pool,
            // active_sessions,
        })
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
                .process_single_message(&mut *transaction, &mut active_sessions, message)
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
        let maybe_session = match active_sessions
            .try_get_peer_session(transaction, message.location_id, message.device_id)
            .await?
        {
            Some(session) => Some(session),
            None => {
                debug!(
                    "No active session found for device {} in location {}. Creating a new session",
                    message.device_id, message.location_id
                );
                active_sessions
                    .try_add_new_session(transaction, &message)
                    .await?
            }
        };

        if let Some(session) = maybe_session {
            // update session stats
            session.update_stats(transaction, message).await?;
        };

        trace!("Finished processing peer stats update");
        Ok(())
    }

    async fn update_inactive_session_status(&self) -> Result<(), SessionManagerError> {
        unimplemented!()
    }
}
