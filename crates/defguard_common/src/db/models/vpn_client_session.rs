use chrono::{NaiveDateTime, Utc};
use model_derive::Model;
use sqlx::Type;

use crate::db::{Id, NoId};

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
