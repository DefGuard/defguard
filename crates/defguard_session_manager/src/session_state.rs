use std::collections::HashMap;

use defguard_common::{
    db::{
        Id,
        models::{Device, User, WireguardNetwork, vpn_client_session::VpnClientSession},
    },
    messages::peer_stats_update::PeerStatsUpdate,
};
use sqlx::PgPool;
use tracing::{debug, error};

use crate::error::SessionManagerError;

/// State of a specific VPN client session
pub(crate) struct SessionState {
    session_id: Id,
    user_id: Id,
    username: String,
    last_stats_update: Option<PeerStatsUpdate>,
}

impl SessionState {
    fn new(session_id: Id, user: &User<Id>) -> Self {
        Self {
            session_id,
            last_stats_update: None,
            user_id: user.id,
            username: user.username.clone(),
        }
    }
    /// Updates session state based on received peer update
    pub(crate) fn update(&mut self, peer_stats_update: PeerStatsUpdate) {
        todo!()
    }
}

/// Represents all active sessions for a given location
pub(crate) struct SessionMap(HashMap<Id, SessionState>);

impl SessionMap {
    /// Helper to insert into inner map
    fn insert(&mut self, key: Id, session_state: SessionState) -> Option<SessionState> {
        self.0.insert(key, session_state)
    }
}

/// Helper struct to hold session maps for all locations
pub(crate) struct LocationSessionsMap(HashMap<Id, SessionMap>);

impl LocationSessionsMap {
    /// Helper to insert into inner map
    fn insert(&mut self, key: Id, session_map: SessionMap) -> Option<SessionMap> {
        self.0.insert(key, session_map)
    }
}

impl LocationSessionsMap {
    /// Fetch current active sessions for all locations from DB
    /// and initialize session map
    pub(crate) async fn initialize_from_db(pool: &PgPool) -> Result<Self, SessionManagerError> {
        debug!("Initializing active sessions map from DB");

        // initialize empty map
        let mut active_sessions = LocationSessionsMap(HashMap::new());

        // fetch all locations
        let locations = WireguardNetwork::all(pool).await?;

        // get active sessions for all locations
        for location in locations {
            // fetch active sessions from DB
            let location_sessions = location.get_active_vpn_sessions(pool).await?;

            // initialize empty session map for a given location
            let mut location_session_map = SessionMap(HashMap::new());

            // insert sessions into map
            for session in location_sessions {
                // we can unwrap here since active session must have a device ID
                let device_id = session
                    .device_id
                    .expect("Active session must have device_id");

                let device = Self::fetch_device(pool, device_id).await?;

                let user = Self::fetch_user(pool, session.user_id).await?;

                let session_state = SessionState::new(session.id, &user);

                if let Some(existing_session) =
                    location_session_map.insert(device_id, session_state)
                {
                    error!(
                        "Found duplicate active session for device {device} in location {location}"
                    );
                    return Err(SessionManagerError::MultipleActiveSessionsError {
                        location_name: location.name,
                        username: existing_session.username,
                        device_name: device.name,
                    });
                };
            }

            if let Some(_) = active_sessions.insert(location.id, location_session_map) {
                let msg = format!(
                    "Active sessions for location {location} have already been initialized"
                );
                error!("{msg}");
                return Err(SessionManagerError::SessionMapInitializationError(msg));
            };
        }

        Ok(active_sessions)
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

    // Wrapper method which attempts to fetch User from DB and returns an error if None is found or an error occurs
    async fn fetch_user(pool: &PgPool, user_id: Id) -> Result<User<Id>, SessionManagerError> {
        User::find_by_id(pool, user_id)
            .await?
            .ok_or(SessionManagerError::UserDoesNotExistError(user_id))
    }
    // Wrapper method which attempts to fetch Device from DB and returns an error if None is found or an error occurs
    async fn fetch_device(pool: &PgPool, device_id: Id) -> Result<Device<Id>, SessionManagerError> {
        Device::find_by_id(pool, device_id)
            .await?
            .ok_or(SessionManagerError::DeviceDoesNotExistError(device_id))
    }
}
