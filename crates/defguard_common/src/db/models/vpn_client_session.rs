use chrono::NaiveDateTime;
use model_derive::Model;

use crate::db::{Id, NoId};

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
}
