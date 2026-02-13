use std::net::SocketAddr;

use chrono::{NaiveDateTime, Utc};

use crate::db::Id;

/// Represents stats read from a WireGuard interface
/// sent from a gateway
#[derive(Debug)]
pub struct PeerStatsUpdate {
    pub location_id: Id,
    pub gateway_id: Id,
    pub device_pubkey: String,
    pub collected_at: NaiveDateTime,
    pub endpoint: SocketAddr,
    // bytes sent to peer
    pub upload: u64,
    // bytes received from peer
    pub download: u64,
    pub latest_handshake: NaiveDateTime,
}

impl PeerStatsUpdate {
    #[must_use]
    pub fn new(
        location_id: Id,
        gateway_id: Id,
        device_pubkey: String,
        endpoint: SocketAddr,
        upload: u64,
        download: u64,
        latest_handshake: NaiveDateTime,
    ) -> Self {
        let collected_at = Utc::now().naive_utc();
        Self {
            location_id,
            gateway_id,
            device_pubkey,
            collected_at,
            endpoint,
            upload,
            download,
            latest_handshake,
        }
    }
}
