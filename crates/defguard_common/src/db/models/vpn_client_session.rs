use chrono::{NaiveDateTime, Utc};
use model_derive::Model;
use sqlx::{Error as SqlxError, Type, query_as};

use crate::db::{Id, NoId, models::vpn_session_stats::VpnSessionStats};

#[derive(Default, Type)]
#[sqlx(type_name = "vpn_client_session_state", rename_all = "lowercase")]
pub enum VpnClientSessionState {
    #[default]
    New,
    Connected,
    Disconnected,
}

/// Represents a single VPN client session from creation to eventual disconnection
#[derive(Model)]
#[table(vpn_client_session)]
pub struct VpnClientSession<I = NoId> {
    pub id: I,
    pub location_id: Id,
    pub user_id: Id,
    // users can delete their device, but we want to retain sessions & stats
    pub device_id: Option<Id>,
    pub created_at: NaiveDateTime,
    pub connected_at: Option<NaiveDateTime>,
    pub disconnected_at: Option<NaiveDateTime>,
    pub mfa: bool,
    #[model(enum)]
    pub state: VpnClientSessionState,
}

impl VpnClientSession {
    pub fn new(
        location_id: Id,
        user_id: Id,
        device_id: Id,
        connected_at: Option<NaiveDateTime>,
        mfa: bool,
    ) -> Self {
        // determine session state
        let state = if connected_at.is_some() {
            VpnClientSessionState::Connected
        } else {
            VpnClientSessionState::New
        };

        Self {
            id: NoId,
            location_id,
            user_id,
            device_id: Some(device_id),
            created_at: Utc::now().naive_utc(),
            connected_at,
            disconnected_at: None,
            mfa,
            state,
        }
    }
}

impl VpnClientSession<Id> {
    /// Tries to fetch the latest active session for a given location and device
    ///
    /// A session is considered active if it's state is `New` or `Connected`
    pub async fn try_get_active_session<'e, E: sqlx::PgExecutor<'e>>(
        executor: E,
        location_id: Id,
        device_id: Id,
    ) -> Result<Option<Self>, SqlxError> {
        query_as!(
            Self,
            "SELECT id, location_id, user_id, device_id, created_at, connected_at, disconnected_at, \
	            mfa, state \"state: VpnClientSessionState\" \
			FROM vpn_client_session \
			WHERE location_id = $1 AND device_id = $2",
            location_id,
            device_id
        )
        .fetch_optional(executor)
        .await
    }

    pub async fn try_get_latest_stats<'e, E: sqlx::PgExecutor<'e>>(
        &self,
        executor: E,
    ) -> Result<Option<VpnSessionStats<Id>>, SqlxError> {
        unimplemented!()
    }
}
