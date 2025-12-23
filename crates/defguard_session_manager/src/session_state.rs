use std::collections::HashMap;

use chrono::TimeDelta;
use defguard_common::{
    db::{
        Id,
        models::{Device, User, WireguardNetwork, vpn_client_session::VpnClientSession},
    },
    messages::peer_stats_update::PeerStatsUpdate,
};
use sqlx::{PgConnection, PgPool, types::chrono::Utc};
use tracing::{debug, error, warn};

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

    /// Updates session stats based on received peer update
    pub(crate) fn update_stats(
        &mut self,
        peer_stats_update: PeerStatsUpdate,
    ) -> Result<(), SessionManagerError> {
        // get previous stats
        todo!();

        // calculate transfer change
        todo!();

        // store stats update in DB
        todo!();
    }
}

/// Represents all active sessions for a given location
pub(crate) struct SessionMap(HashMap<Id, SessionState>);

impl SessionMap {
    pub(crate) fn new() -> Self {
        Self(HashMap::new())
    }

    /// Helper to insert into inner map
    fn insert(&mut self, key: Id, session_state: SessionState) -> Option<SessionState> {
        self.0.insert(key, session_state)
    }
}

/// Helper struct to hold session maps for all locations and object cache to avoid repeated DB queries
///
/// Since we want to support HA core deployments this structure
/// is not meant to be the source of truth, but rather a cache
/// to avoid repeated DB queries when processing a single batch of messages.
/// After a batch is processed it should be discarded and a new `ActiveSessionsMap`
/// should be created for the next batch.
pub(crate) struct ActiveSessionsMap {
    sessions: HashMap<Id, SessionMap>,
    locations: HashMap<Id, WireguardNetwork<Id>>,
    users: HashMap<Id, User<Id>>,
    devices: HashMap<Id, Device<Id>>,
}

impl ActiveSessionsMap {
    pub(crate) fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            locations: HashMap::new(),
            users: HashMap::new(),
            devices: HashMap::new(),
        }
    }

    /// Helper to insert into inner map
    fn insert(&mut self, key: Id, session_map: SessionMap) -> Option<SessionMap> {
        self.sessions.insert(key, session_map)
    }
}

impl ActiveSessionsMap {
    /// Fetch current active sessions for all locations from DB
    /// and initialize session map
    // pub(crate) async fn initialize_from_db(pool: &PgPool) -> Result<Self, SessionManagerError> {
    //     debug!("Initializing active sessions map from DB");

    //     // initialize empty map
    //     let mut active_sessions = LocationSessionsMap(HashMap::new());

    //     // fetch all locations
    //     let locations = WireguardNetwork::all(pool).await?;

    //     // get active sessions for all locations
    //     for location in locations {
    //         // fetch active sessions from DB
    //         let location_sessions = location.get_active_vpn_sessions(pool).await?;

    //         // initialize empty session map for a given location
    //         let mut location_session_map = SessionMap(HashMap::new());

    //         // insert sessions into map
    //         for session in location_sessions {
    //             // we can unwrap here since active session must have a device ID
    //             let device_id = session
    //                 .device_id
    //                 .expect("Active session must have device_id");

    //             let device = Self::fetch_device(pool, device_id).await?;

    //             let user = Self::fetch_user(pool, session.user_id).await?;

    //             let session_state = SessionState::new(session.id, &user);

    //             if let Some(existing_session) =
    //                 location_session_map.insert(device_id, session_state)
    //             {
    //                 error!(
    //                     "Found duplicate active session for device {device} in location {location}"
    //                 );
    //                 return Err(SessionManagerError::MultipleActiveSessionsError {
    //                     location_name: location.name,
    //                     username: existing_session.username,
    //                     device_name: device.name,
    //                 });
    //             };
    //         }

    //         if let Some(_) = active_sessions.insert(location.id, location_session_map) {
    //             let msg = format!(
    //                 "Active sessions for location {location} have already been initialized"
    //             );
    //             error!("{msg}");
    //             return Err(SessionManagerError::SessionMapInitializationError(msg));
    //         };
    //     }

    //     Ok(active_sessions)
    // }

    /// Checks if a session for a given peer exists already
    pub(crate) fn try_get_peer_session(
        &mut self,
        location_id: Id,
        device_id: Id,
    ) -> Option<&mut SessionState> {
        self.sessions
            .get_mut(&location_id)
            .map(|session_map| session_map.0.get_mut(&device_id))?
    }

    /// Checks if any sessions need to be marked as disconnected
    pub(crate) fn update_session_status(&mut self) {
        todo!()
    }

    /// Attempts to create a new VPN client session, add it to curent state and persists it in DB
    ///
    /// We assume that at this point it's been checked that a session for this client does not exist yet,
    /// but we do check if given peer can be considered active based on a given locations peer disconnect threshold.
    pub(crate) async fn try_add_new_session(
        &mut self,
        transaction: &mut PgConnection,
        stats_update: &PeerStatsUpdate,
    ) -> Result<Option<&mut SessionState>, SessionManagerError> {
        // fetch location
        let location_id = stats_update.location_id;
        let location = self.get_location(&mut *transaction, location_id).await?;

        // check if a given peer is considered active and should be added to active sessions
        if Utc::now().naive_utc() - stats_update.latest_handshake
            > TimeDelta::seconds(location.peer_disconnect_threshold.into())
        {
            warn!(
                "Received peer stats update for an inactive peer. Skipping creating a new session..."
            );
            return Ok(None);
        }

        // fetch other related objects from DB
        let device_id = stats_update.device_id;
        let device = self.get_device(&mut *transaction, device_id).await?;
        let user = self.get_user(&mut *transaction, device.user_id).await?;

        debug!("Adding new VPN client session for location {location}");

        // create a client session object and save it to DB
        let session = VpnClientSession::new(
            location.id,
            user.id,
            device.id,
            Some(stats_update.latest_handshake),
            location.mfa_enabled(),
        )
        .save(transaction)
        .await?;

        // add to session map
        let session_state = SessionState::new(session.id, &user);
        let session_map = self.get_or_create_location_session_map(location_id);
        let maybe_existing_session = session_map.insert(device_id, session_state);
        // if a session exists already there was an error in earlier logic
        assert!(maybe_existing_session.is_none());

        Ok(Some(
            session_map
                .0
                .get_mut(&device_id)
                .expect("Session has just been created"),
        ))
    }

    fn get_or_create_location_session_map(&mut self, location_id: Id) -> &mut SessionMap {
        // check if location is already present in session map
        if self.sessions.contains_key(&location_id) {
            self.sessions
                .get_mut(&location_id)
                .expect("Location session map must exist")
        } else {
            debug!("Session map for location {location_id} not found. Initializing a new map.");
            let new_session_map = SessionMap::new();
            let maybe_existing_map = self.sessions.insert(location_id, new_session_map);
            // if a map exists already there was an error in earlier logic
            assert!(maybe_existing_map.is_none());
            self.sessions
                .get_mut(&location_id)
                .expect("Location session map has just been created")
        }
    }

    // Helper method which checks if User is already cached,
    // then attempts to fetch User from DB and returns an error if None is found or an error occurs
    async fn get_user<'e, E: sqlx::PgExecutor<'e>>(
        &mut self,
        executor: E,
        user_id: Id,
    ) -> Result<User<Id>, SessionManagerError> {
        // first try to find user in object cache
        let user = if self.users.contains_key(&user_id) {
            self.users
                .get(&user_id)
                .expect("User must exist in object cache")
        } else {
            debug!("User {user_id} not found in object cache. Trying to fetch from DB.");
            let user = User::find_by_id(executor, user_id)
                .await?
                .ok_or(SessionManagerError::LocationDoesNotExistError(user_id))?;
            // update object cache
            self.users.insert(user_id, user);
            self.users
                .get(&user_id)
                .expect("User must exist in object cache")
        };

        // TODO: figure out a way to avoid multiple mutable borrows
        // and return a reference instead of cloning
        Ok(user.clone())
    }

    // Helper method which checks if Device is already cached,
    // then attempts to fetch Device from DB and returns an error if None is found or an error occurs
    async fn get_device<'e, E: sqlx::PgExecutor<'e>>(
        &mut self,
        executor: E,
        device_id: Id,
    ) -> Result<Device<Id>, SessionManagerError> {
        // first try to find device in object cache
        let device = if self.devices.contains_key(&device_id) {
            self.devices
                .get(&device_id)
                .expect("Device must exist in object cache")
        } else {
            debug!("Device {device_id} not found in object cache. Trying to fetch from DB.");
            let device = Device::find_by_id(executor, device_id)
                .await?
                .ok_or(SessionManagerError::DeviceDoesNotExistError(device_id))?;
            // update object cache
            self.devices.insert(device_id, device);
            self.devices
                .get(&device_id)
                .expect("Device must exist in object cache")
        };

        // TODO: figure out a way to avoid multiple mutable borrows
        // and return a reference instead of cloning
        Ok(device.clone())
    }

    // Helper method which checks if Location is already cached,
    // then attempts to fetch Location from DB and returns an error if None is found or an error occurs
    async fn get_location<'e, E: sqlx::PgExecutor<'e>>(
        &mut self,
        executor: E,
        location_id: Id,
    ) -> Result<WireguardNetwork<Id>, SessionManagerError> {
        // first try to find location in object cache
        let location = if self.locations.contains_key(&location_id) {
            self.locations
                .get(&location_id)
                .expect("Location must exist in object cache")
        } else {
            debug!("Location {location_id} not found in object cache. Trying to fetch from DB.");
            let location = WireguardNetwork::find_by_id(executor, location_id)
                .await?
                .ok_or(SessionManagerError::LocationDoesNotExistError(location_id))?;
            // update object cache
            self.locations.insert(location_id, location);
            self.locations
                .get(&location_id)
                .expect("Location must exist in object cache")
        };

        // TODO: figure out a way to avoid multiple mutable borrows
        // and return a reference instead of cloning
        Ok(location.clone())
    }
}
