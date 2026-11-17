use std::net::SocketAddr;

use time::PrimitiveDateTime;

use crate::{db::Id, time_utils::now_naive_utc};

/// Represents stats read from a WireGuard interface
/// sent from a gateway
#[derive(Debug)]
pub struct PeerStatsUpdate {
    pub location_id: Id,
    pub gateway_id: Id,
    pub device_pubkey: String,
    pub collected_at: PrimitiveDateTime,
    pub endpoint: SocketAddr,
    // bytes sent to peer
    pub upload: u64,
    // bytes received from peer
    pub download: u64,
    pub latest_handshake: PrimitiveDateTime,
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
        latest_handshake: PrimitiveDateTime,
    ) -> Self {
        let collected_at = now_naive_utc();
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
