use crate::db::{Id, NoId};

#[derive(Model)]
#[table(vpn_session_stats)]
pub struct VpnSessionStats<I = NoId> {
    pub id: I,
    pub session_id: Id,
}
