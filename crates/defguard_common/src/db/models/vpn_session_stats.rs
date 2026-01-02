use chrono::NaiveDateTime;
use model_derive::Model;

use crate::db::{Id, NoId};

#[derive(Model)]
#[table(vpn_session_stats)]
pub struct VpnSessionStats<I = NoId> {
    pub id: I,
    pub session_id: Id,
    pub collected_at: NaiveDateTime,
    // handshake must have occured for a session to be considered active
    pub latest_handshake: NaiveDateTime,
    pub endpoint: String,
    // total bytes sent to peer as read from WireGuard interface
    pub total_upload: i64,
    // total bytes received from peer as read from WireGuard interface
    pub total_download: i64,
    // uplad since last stats update
    pub upload_diff: i64,
    // download since last stats update
    pub download_diff: i64,
}
