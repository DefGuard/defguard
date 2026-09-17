use chrono::{NaiveDateTime, Utc};
use model_derive::Model;
use sqlx::{FromRow, Type, query_as};

use crate::db::{
    Id, NoId,
    models::{WireguardNetwork, vpn_session_stats::VpnSessionStats},
};

#[derive(Clone, Debug, Default, PartialEq, Type)]
#[sqlx(type_name = "vpn_client_session_state", rename_all = "lowercase")]
pub enum VpnClientSessionState {
    #[default]
    New,
    Connected,
    Disconnected,
}

#[derive(Debug, Type)]
#[sqlx(type_name = "vpn_client_mfa_method", rename_all = "lowercase")]
pub enum VpnClientMfaMethod {
    Totp,
    Email,
    Oidc,
    Biometric,
    MobileApprove,
}

/// Represents a single VPN client session from creation to eventual disconnection
#[derive(Debug, Model)]
#[table(vpn_client_session)]
pub struct VpnClientSession<I = NoId> {
    pub id: I,
    pub location_id: Id,
    pub user_id: Id,
    pub device_id: Id,
    pub created_at: NaiveDateTime,
    pub connected_at: Option<NaiveDateTime>,
    pub disconnected_at: Option<NaiveDateTime>,
    #[model(option)]
    pub mfa_method: Option<VpnClientMfaMethod>,
    #[model(enum)]
    pub state: VpnClientSessionState,
    pub preshared_key: Option<String>,
}

impl VpnClientSession {
    #[must_use]
    pub fn new(
        location_id: Id,
        user_id: Id,
        device_id: Id,
        connected_at: Option<NaiveDateTime>,
        mfa_method: Option<VpnClientMfaMethod>,
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
            device_id,
            created_at: Utc::now().naive_utc(),
            connected_at,
            disconnected_at: None,
            mfa_method,
            state,
            preshared_key: None,
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
    ) -> sqlx::Result<Option<Self>> {
        query_as!(
            Self,
            "SELECT id, location_id, user_id, device_id, created_at, connected_at, disconnected_at, \
	            mfa_method \"mfa_method: VpnClientMfaMethod\", state \"state: VpnClientSessionState\", preshared_key \
			FROM vpn_client_session \
			WHERE location_id = $1 AND device_id = $2 AND state IN ('new', 'connected') \
			ORDER BY created_at DESC, id DESC \
			LIMIT 1",
            location_id,
            device_id
        )
        .fetch_optional(executor)
        .await
    }

    /// Fetches the latest active session for every device in a location.
    pub async fn get_all_active_for_location<'e, E: sqlx::PgExecutor<'e>>(
        executor: E,
        location_id: Id,
    ) -> sqlx::Result<Vec<Self>> {
        #[derive(FromRow)]
        struct ActiveSessionRow {
            id: Id,
            location_id: Id,
            user_id: Id,
            device_id: Id,
            created_at: NaiveDateTime,
            connected_at: Option<NaiveDateTime>,
            disconnected_at: Option<NaiveDateTime>,
            mfa_method: Option<VpnClientMfaMethod>,
            state: VpnClientSessionState,
            preshared_key: Option<String>,
        }

        let rows = sqlx::query_as::<_, ActiveSessionRow>(
            "SELECT DISTINCT ON (device_id) id, location_id, user_id, device_id, created_at, connected_at, disconnected_at, \
\t            mfa_method, state, preshared_key \
\t\t\tFROM vpn_client_session \
\t\t\tWHERE location_id = $1 AND state IN ('new', 'connected') \
\t\t\tORDER BY device_id, created_at DESC, id DESC",
        )
        .bind(location_id)
        .fetch_all(executor)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| Self {
                id: row.id,
                location_id: row.location_id,
                user_id: row.user_id,
                device_id: row.device_id,
                created_at: row.created_at,
                connected_at: row.connected_at,
                disconnected_at: row.disconnected_at,
                mfa_method: row.mfa_method,
                state: row.state,
                preshared_key: row.preshared_key,
            })
            .collect())
    }

    /// Returns latest stats in a given session for each gateway
    pub async fn get_latest_stats_for_all_gateways<'e, E: sqlx::PgExecutor<'e>>(
        &self,
        executor: E,
    ) -> sqlx::Result<Vec<VpnSessionStats<Id>>> {
        query_as!(
            VpnSessionStats,
            "SELECT DISTINCT ON (gateway_id) id, session_id, gateway_id, collected_at, latest_handshake, endpoint, \
            	total_upload, total_download, upload_diff, download_diff
        	FROM vpn_session_stats \
        	WHERE session_id = $1 \
        	ORDER BY gateway_id, collected_at DESC",
            self.id
        )
        .fetch_all(executor)
        .await
    }

    /// Fetch active sessions which have become inactive for a specific location
    pub async fn get_all_inactive_for_location<'e, E: sqlx::PgExecutor<'e>>(
        executor: E,
        location: &WireguardNetwork<Id>,
    ) -> sqlx::Result<Vec<Self>> {
        query_as!(
    		Self,
            "SELECT s.id, location_id, user_id, device_id, created_at, s.connected_at, disconnected_at, \
	            mfa_method \"mfa_method: VpnClientMfaMethod\", state \"state: VpnClientSessionState\", preshared_key \
			FROM vpn_client_session s \
			LEFT JOIN LATERAL ( \
				SELECT latest_handshake \
				FROM vpn_session_stats \
				WHERE session_id = s.id \
				ORDER BY latest_handshake DESC \
				LIMIT 1 \
			) ss ON true \
			WHERE location_id = $1 AND state = 'connected' \
            AND (NOW() - ss.latest_handshake) > $2 * interval '1 second'",
			location.id,
			f64::from(location.peer_disconnect_threshold)
    	).fetch_all(executor).await
    }

    /// Fetch sessions that were created but have not become `connected` within the disconnect threshold
    pub async fn get_never_connected<'e, E: sqlx::PgExecutor<'e>>(
        executor: E,
        location: &WireguardNetwork<Id>,
    ) -> sqlx::Result<Vec<Self>> {
        query_as!(
    		Self,
            "SELECT id, location_id, user_id, device_id, created_at, connected_at, disconnected_at, \
	            mfa_method \"mfa_method: VpnClientMfaMethod\", state \"state: VpnClientSessionState\", preshared_key \
			FROM vpn_client_session \
			WHERE location_id = $1 AND state = 'new' \
            AND (NOW() - created_at) > $2 * interval '1 second'",
			location.id,
			f64::from(location.peer_disconnect_threshold)
    	).fetch_all(executor).await
    }

    /// Fetch all active sessions for a given device in a given location
    pub async fn get_all_active_device_sessions_in_location<'e, E: sqlx::PgExecutor<'e>>(
        executor: E,
        location_id: Id,
        device_id: Id,
    ) -> sqlx::Result<Vec<Self>> {
        query_as!(
    		Self,
            "SELECT id, location_id, user_id, device_id, created_at, connected_at, disconnected_at, \
	            mfa_method \"mfa_method: VpnClientMfaMethod\", state \"state: VpnClientSessionState\", preshared_key \
			FROM vpn_client_session \
			WHERE location_id = $1 AND device_id = $2 AND state IN ('new', 'connected') \
			ORDER BY created_at DESC, id DESC",
			location_id,
			device_id,
    	).fetch_all(executor).await
    }
}
