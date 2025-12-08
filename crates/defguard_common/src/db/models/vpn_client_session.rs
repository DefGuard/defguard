use chrono::NaiveDateTime;
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
    pub connected_at: NaiveDateTime,
    pub disconnected_at: NaiveDateTime,
    pub mfa: bool,
    #[model(enum)]
    pub state: VpnClientSessionState,
}
