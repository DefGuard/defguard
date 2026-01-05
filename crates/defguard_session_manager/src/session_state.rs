use std::collections::HashMap;

use chrono::{NaiveDateTime, TimeDelta};
use defguard_common::{
    db::{
        Id,
        models::{
            Device, User, WireguardNetwork, vpn_client_session::VpnClientSession,
            vpn_session_stats::VpnSessionStats,
        },
    },
    messages::peer_stats_update::PeerStatsUpdate,
};
use sqlx::{PgConnection, types::chrono::Utc};
use tracing::{debug, warn};

use crate::error::SessionManagerError;

struct LastStatsUpdate {
    collected_at: NaiveDateTime,
    latest_handshake: NaiveDateTime,
    total_upload: i64,
    total_download: i64,
}

impl LastStatsUpdate {
    /// Checks if the next peer stats update is valid.
    ///
    /// This includes following checks:
    /// - new update was collected after previous
    /// - transfer values are not decreased
    fn validate_update(&self, new_update: &PeerStatsUpdate) -> Result<(), SessionManagerError> {
        if new_update.collected_at < self.collected_at {
            return Err(SessionManagerError::PeerStatsUpdateOutOfOrderError);
        }

        if new_update.latest_handshake < self.latest_handshake {
            return Err(SessionManagerError::PeerStatsUpdateOutOfOrderError);
        }

        if (new_update.upload as i64) < self.total_upload
            || (new_update.download as i64) < self.total_download
        {
            return Err(SessionManagerError::PeerStatsUpdateOutOfOrderError);
        }

        Ok(())
    }
}

impl From<VpnSessionStats<Id>> for LastStatsUpdate {
    fn from(value: VpnSessionStats<Id>) -> Self {
        Self {
            collected_at: value.collected_at,
            latest_handshake: value.latest_handshake,
            total_upload: value.total_upload,
            total_download: value.total_download,
        }
    }
}

/// State of a specific VPN client session
pub(crate) struct SessionState {
    session_id: Id,
    user_id: Id,
    last_stats_update: Option<LastStatsUpdate>,
}

impl SessionState {
    fn new(session_id: Id, user: &User<Id>) -> Self {
        Self {
            session_id,
            last_stats_update: None,
            user_id: user.id,
        }
    }

    /// Updates session stats based on received peer update
    pub(crate) async fn update_stats(
        &mut self,
        transaction: &mut PgConnection,
        peer_stats_update: PeerStatsUpdate,
    ) -> Result<(), SessionManagerError> {
        // get previous stats if available and calculate transfer change
        let (upload_diff, download_diff) = match &self.last_stats_update {
            Some(last_stats_update) => {
                // validate current update against latest value
                last_stats_update.validate_update(&peer_stats_update)?;

                // calculate transfer change
                (
                    peer_stats_update.upload as i64 - last_stats_update.total_upload,
                    peer_stats_update.download as i64 - last_stats_update.total_download,
                )
            }
            None => (0, 0),
        };

        let vpn_session_stats = VpnSessionStats::new(
            self.session_id,
            peer_stats_update.collected_at,
            peer_stats_update.latest_handshake,
            peer_stats_update.endpoint.to_string(),
            peer_stats_update.upload as i64,
            peer_stats_update.download as i64,
            upload_diff,
            download_diff,
        );

        // store stats update in DB
        let stats = vpn_session_stats.save(transaction).await?;

        // update latest stats
        self.last_stats_update = Some(LastStatsUpdate::from(stats));

        Ok(())
    }
}

impl From<&VpnClientSession<Id>> for SessionState {
    fn from(value: &VpnClientSession<Id>) -> Self {
        Self {
            session_id: value.id,
            user_id: value.user_id,
            last_stats_update: None,
        }
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
}

impl ActiveSessionsMap {
    /// Checks if a session for a given peer exists already
    ///
    /// First we check current map, then try the DB.
    pub(crate) async fn try_get_peer_session(
        &mut self,
        transaction: &mut PgConnection,
        location_id: Id,
        device_id: Id,
    ) -> Result<Option<&mut SessionState>, SessionManagerError> {
        // try to get session from current map
        let session_map = self.get_or_create_location_session_map(location_id);
        if session_map.0.contains_key(&device_id) {
            return Ok(session_map.0.get_mut(&device_id));
        }

        // session not found in current map, try to fetch from DB
        let maybe_db_session =
            VpnClientSession::try_get_active_session(&mut *transaction, location_id, device_id)
                .await?;

        match maybe_db_session {
            None => Ok(None),
            Some(db_session) => {
                let mut session_state = SessionState::from(&db_session);

                // try to fetch latest available stats for a given session
                if let Some(latest_stats) = db_session.try_get_latest_stats(transaction).await? {
                    session_state.last_stats_update = Some(LastStatsUpdate::from(latest_stats));
                };

                // put session state in map
                let maybe_existing_session = session_map.insert(device_id, session_state);
                // if a session exists already there was an error in earlier logic
                assert!(maybe_existing_session.is_none());

                Ok(session_map.0.get_mut(&device_id))
            }
        }
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
