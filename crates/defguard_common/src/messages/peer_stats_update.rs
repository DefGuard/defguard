use std::net::SocketAddr;

use chrono::{NaiveDateTime, Utc};

use crate::db::Id;

/// Represents stats read from a WireGuard interface
/// sent from a gateway
#[derive(Debug)]
pub struct PeerStatsUpdate {
    pub location_id: Id,
    pub device_id: Id,
    pub collected_at: NaiveDateTime,
    pub endpoint: SocketAddr,
    // bytes sent to peer
    pub upload: u64,
    // bytes received from peer
    pub download: u64,
    pub latest_handshake: NaiveDateTime,
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
