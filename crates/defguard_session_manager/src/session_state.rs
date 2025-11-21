use std::collections::HashMap;

use defguard_common::{db::Id, messages::peer_stats_update::PeerStatsUpdate};
use sqlx::PgPool;
use tracing::debug;

use crate::error::SessionManagerError;

/// State of a specific VPN client session
pub(crate) struct SessionState {}

impl SessionState {
    /// Updates session state based on received peer update
    pub(crate) fn update(&mut self, peer_stats_update: PeerStatsUpdate) {
        todo!()
    }
}

/// Represents all active sessions for a given location
pub(crate) struct SessionMap(HashMap<Id, SessionState>);

/// Helper struct to hold session maps for all locations
pub(crate) struct LocationSessionsMap(HashMap<Id, SessionMap>);

impl LocationSessionsMap {
    /// Fetch current active sessions for all locations from DB
    /// and initialize session map
    pub(crate) async fn initialize_from_db(pool: &PgPool) -> Result<Self, SessionManagerError> {
        debug!("Initializing active sessions map from DB");
        todo!()
    }

    /// Checks if a session for a given peer exists already
    pub(crate) fn try_get_peer_session(&self) -> Option<SessionState> {
        todo!()
    }

    pub(crate) fn get_location_sessions(&self, location_id: Id) {
        todo!()
    }

    /// Checks if any sessions need to be marked as disconnected
    pub(crate) fn update_session_status(&mut self) {
        todo!()
    }

    /// Creates a new VPN client session, adds it to curent state and persists it in DB
    pub(crate) fn new_session(&mut self) {
        todo!()
    }
}
