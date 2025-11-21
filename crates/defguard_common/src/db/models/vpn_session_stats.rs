use model_derive::Model;

use crate::db::{Id, NoId};

#[derive(Model)]
#[table(vpn_session_stats)]
pub struct VpnSessionStats<I = NoId> {
    pub id: I,
    pub session_id: Id,
    // uplad since last stats update
    pub upload_diff: i64,
    // download since last stats update
    pub download_diff: i64,
}
