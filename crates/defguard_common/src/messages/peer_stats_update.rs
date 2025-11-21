use std::net::SocketAddr;

use chrono::{NaiveDateTime, Utc};

use crate::db::Id;

/// Represents stats read from a WireGuard interface
/// sent from a gateway
pub struct PeerStatsUpdate {
    location_id: Id,
    device_id: Id,
    collected_at: NaiveDateTime,
    endpoint: SocketAddr,
    // bytes sent to peer
    upload: u64,
    // bytes received from peer
    download: u64,
    latest_handshake: NaiveDateTime,
}

impl PeerStatsUpdate {
    pub fn new(
        location_id: Id,
        device_id: Id,
        endpoint: SocketAddr,
        upload: u64,
        download: u64,
        latest_handshake: NaiveDateTime,
    ) -> Self {
        let collected_at = Utc::now().naive_utc();
        Self {
            location_id,
            device_id,
            collected_at,
            endpoint,
            upload,
            download,
            latest_handshake,
        }
    }
}
