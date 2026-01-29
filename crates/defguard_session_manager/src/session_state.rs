use std::collections::{HashMap, hash_map::Entry};

use chrono::{NaiveDateTime, TimeDelta};
use defguard_common::{
    db::{
        Id,
        models::{
            Device, User, WireguardNetwork,
            vpn_client_session::{VpnClientSession, VpnClientSessionState},
            vpn_session_stats::VpnSessionStats,
            wireguard::LocationMfaMode,
        },
    },
    messages::peer_stats_update::PeerStatsUpdate,
};
use sqlx::{PgConnection, types::chrono::Utc};
use tokio::sync::mpsc::UnboundedSender;
use tracing::{debug, warn};

use crate::{
    error::SessionManagerError,
    events::{SessionManagerEvent, SessionManagerEventContext, SessionManagerEventType},
};

/// Helper map to store latest stats update for each gateway in a given location
pub(crate) struct LastGatewayUpdate(HashMap<Id, LastStatsUpdate>);

impl LastGatewayUpdate {
    fn new() -> Self {
        Self(HashMap::new())
    }

    /// Store latest stats for a given gateway
    ///
    /// We assume that at this point the update has already been validated.
    fn update(&mut self, session_stats: VpnSessionStats<Id>) {
        let gateway_id = session_stats.gateway_id;
        let latest_stats = LastStatsUpdate::from(session_stats);

        debug!("Replacing latest stats update for gateway {gateway_id} with {latest_stats:?}");
        let _maybe_previous = self.0.insert(gateway_id, latest_stats);
    }
}

#[derive(Debug)]
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
    state: VpnClientSessionState,
    last_stats_update: LastGatewayUpdate,
}

impl SessionState {
    fn try_get_last_stats_update(&self, gateway_id: Id) -> Option<&LastStatsUpdate> {
        self.last_stats_update.0.get(&gateway_id)
    }

    /// Updates session stats based on received peer update
    pub(crate) async fn update_stats(
        &mut self,
        transaction: &mut PgConnection,
        peer_stats_update: PeerStatsUpdate,
    ) -> Result<(), SessionManagerError> {
        // mark new MFA session as connected if necessary
        if self.state == VpnClientSessionState::New {
            // fetch DB session
            let mut db_session = VpnClientSession::find_by_id(&mut *transaction, self.session_id)
                .await?
                .ok_or(SessionManagerError::SessionDoesNotExistError(
                    self.session_id,
                ))?;
            // update DB session
            db_session.state = VpnClientSessionState::Connected;
            db_session.connected_at = Some(peer_stats_update.latest_handshake);
            db_session.save(&mut *transaction).await?;

            // update local session state
            self.state = VpnClientSessionState::Connected;
        }

        // get previous stats for a given gateway if available and calculate transfer change
        let (upload_diff, download_diff) =
            match self.try_get_last_stats_update(peer_stats_update.gateway_id) {
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
            peer_stats_update.gateway_id,
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
        self.last_stats_update.update(stats);

        Ok(())
    }
}

impl From<&VpnClientSession<Id>> for SessionState {
    fn from(value: &VpnClientSession<Id>) -> Self {
        Self {
            session_id: value.id,
            state: value.state.clone(),
            last_stats_update: LastGatewayUpdate::new(),
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

                // fetch latest available stats for each gateway for a given session
                let latest_gateway_stats = db_session
                    .get_latest_stats_for_all_gateways(transaction)
                    .await?;
                for stats in latest_gateway_stats {
                    session_state.last_stats_update.update(stats);
                }

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
    /// This should only happen for non-MFA sessions since MFA sessions (with `new` state) should be created once the authorization is completed
    /// in the proxy handler.
    ///
    /// We assume that at this point it's been checked that a session for this client does not exist yet,
    /// but we do check if given peer can be considered active based on a given locations peer disconnect threshold.
    pub(crate) async fn try_add_new_session(
        &mut self,
        transaction: &mut PgConnection,
        stats_update: &PeerStatsUpdate,
        event_tx: &UnboundedSender<SessionManagerEvent>,
    ) -> Result<Option<&mut SessionState>, SessionManagerError> {
        // fetch location
        let location_id = stats_update.location_id;

        let location = self
            .get_location(&mut *transaction, location_id)
            .await?
            .clone();

        // check location MFA mode since MFA sessions should be created elsewhere
        // once MFA auth is successful
        if location.location_mfa_mode != LocationMfaMode::Disabled {
            warn!(
                "Received peer stats update for MFA-enabled location {location}, but VPN session does not exist yet. Skipping creating a new session..."
            );
            return Ok(None);
        }

        // check if a given peer is considered active and should be added to active sessions
        if Utc::now().naive_utc() - stats_update.latest_handshake
            > TimeDelta::seconds(location.peer_disconnect_threshold.into())
        {
            warn!(
                "Received peer stats update for an inactive peer. Skipping creating a new session..."
            );
            return Ok(None);
        };

        // fetch other related objects from DB
        // clone them because we'll need those for event context
        let device_id = stats_update.device_id;
        let device = self.get_device(&mut *transaction, device_id).await?.clone();
        let user = self
            .get_user(&mut *transaction, device.user_id)
            .await?
            .clone();

        debug!("Adding new VPN client session for location {location}");

        // create a client session object and save it to DB
        let session = VpnClientSession::new(
            location_id,
            user.id,
            device_id,
            Some(stats_update.latest_handshake),
            None,
        )
        .save(transaction)
        .await?;

        // add to session map
        let session_state = SessionState::from(&session);
        let session_map = self.get_or_create_location_session_map(location_id);
        let maybe_existing_session = session_map.insert(device.id, session_state);

        // if a session exists already there was an error in earlier logic
        assert!(maybe_existing_session.is_none());

        // emit event
        let public_ip = stats_update.endpoint.ip();
        let context = SessionManagerEventContext {
            timestamp: stats_update.latest_handshake,
            location,
            user,
            device,
            public_ip,
        };
        let event = SessionManagerEvent {
            context,
            event: SessionManagerEventType::ClientConnected,
        };
        event_tx.send(event)?;

        Ok(session_map.0.get_mut(&device_id))
    }

    fn get_or_create_location_session_map(&mut self, location_id: Id) -> &mut SessionMap {
        // check if location is already present in session map
        match self.sessions.entry(location_id) {
            Entry::Occupied(occupied_entry) => occupied_entry.into_mut(),
            Entry::Vacant(vacant_entry) => {
                debug!("Session map for location {location_id} not found. Initializing a new map.");
                let new_session_map = SessionMap::new();
                vacant_entry.insert(new_session_map)
            }
        }
    }

    // Helper method which checks if User is already cached,
    // then attempts to fetch User from DB and returns an error if None is found or an error occurs
    async fn get_user<'e, E: sqlx::PgExecutor<'e>>(
        &mut self,
        executor: E,
        user_id: Id,
    ) -> Result<&User<Id>, SessionManagerError> {
        // first try to find user in object cache
        let user_entry = match self.users.entry(user_id) {
            Entry::Occupied(occupied_entry) => occupied_entry,
            Entry::Vacant(vacant_entry) => {
                debug!("User {user_id} not found in object cache. Trying to fetch from DB.");
                let user = User::find_by_id(executor, user_id)
                    .await?
                    .ok_or(SessionManagerError::UserDoesNotExistError(user_id))?;
                // update object cache
                vacant_entry.insert_entry(user)
            }
        };

        // return reference to the map itself
        Ok(user_entry.into_mut())
    }

    // Helper method which checks if Device is already cached,
    // then attempts to fetch Device from DB and returns an error if None is found or an error occurs
    async fn get_device<'e, E: sqlx::PgExecutor<'e>>(
        &mut self,
        executor: E,
        device_id: Id,
    ) -> Result<&Device<Id>, SessionManagerError> {
        // first try to find device in object cache
        let device_entry = match self.devices.entry(device_id) {
            Entry::Occupied(occupied_entry) => occupied_entry,
            Entry::Vacant(vacant_entry) => {
                debug!("Device {device_id} not found in object cache. Trying to fetch from DB.");
                let device = Device::find_by_id(executor, device_id)
                    .await?
                    .ok_or(SessionManagerError::DeviceDoesNotExistError(device_id))?;
                // update object cache
                vacant_entry.insert_entry(device)
            }
        };

        // return reference to the map itself
        Ok(device_entry.into_mut())
    }

    // Helper method which checks if Location is already cached,
    // then attempts to fetch Location from DB and returns an error if None is found or an error occurs
    async fn get_location<'e, E: sqlx::PgExecutor<'e>>(
        &mut self,
        executor: E,
        location_id: Id,
    ) -> Result<&WireguardNetwork<Id>, SessionManagerError> {
        // first try to find location in object cache
        let location_entry = match self.locations.entry(location_id) {
            Entry::Occupied(occupied_entry) => occupied_entry,
            Entry::Vacant(vacant_entry) => {
                debug!(
                    "Location {location_id} not found in object cache. Trying to fetch from DB."
                );
                let location = WireguardNetwork::find_by_id(executor, location_id)
                    .await?
                    .ok_or(SessionManagerError::LocationDoesNotExistError(location_id))?;
                // update object cache
                vacant_entry.insert_entry(location)
            }
        };

        // return reference to the map itself
        Ok(location_entry.into_mut())
    }
}
