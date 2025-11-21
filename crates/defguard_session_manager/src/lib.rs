use std::collections::HashMap;

use defguard_common::messages::peer_stats_update::PeerStatsUpdate;
use sqlx::PgPool;
use tokio::{
    sync::mpsc::UnboundedReceiver,
    time::{Duration, interval},
};
use tracing::{debug, info};

use crate::{error::SessionManagerError, session_state::LocationSessionsMap};

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

    // initialize active sessions state based on DB content
    let mut active_sessions = LocationSessionsMap::initialize_from_db(&pool).await?;

    loop {
        // receive next batch of peer stats messages
        // if no message is received within `SESSION_UPDATE_INTERVAL` trigger session status refresh anyway
        // to disconnect inactive sessions if necessary
        let mut message_buffer: Vec<PeerStatsUpdate> = Vec::with_capacity(MESSAGE_LIMIT);
        let message_count = tokio::select! {
            message_count = peer_stats_rx.recv_many(&mut message_buffer, MESSAGE_LIMIT) => message_count,
            _ = session_update_timer.tick() => {
                info!("No wireguard peer stats updates received in last {SESSION_UPDATE_INTERVAL}. Triggering session status update.");
                active_sessions.update_session_status();
                continue;
            }

        };

        debug!("Processing batch of {message_count} peer stats updates");

        // create temporary maps of DB objects to avoid repeated queries
        // let location_map = HashMap::new();
        // let user_map = HashMap::new();
        // let device_map = HashMap::new();

        // begin DB transaction
        let transaction = pool.begin().await?;

        for message in message_buffer {
            // check if a session exists already for a given peer
            match active_sessions.try_get_peer_session() {
                Some(mut session) => {
                    // session exists already, update it based on received stats
                    session.update(message);
                }
                None => {
                    debug!(
                        "No active session found for device {} in location {}. Creating a new session",
                        message.device_id, message.location_id
                    );
                    active_sessions.new_session();
                }
            }
        }

        // commit DB transaction after processing all messages
        transaction.commit().await?;
    }
}
