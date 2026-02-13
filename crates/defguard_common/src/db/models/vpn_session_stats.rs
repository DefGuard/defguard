use chrono::NaiveDateTime;
use model_derive::Model;
use sqlx::{PgExecutor, query_as};

use crate::db::{Id, NoId};

#[derive(Model)]
#[table(vpn_session_stats)]
pub struct VpnSessionStats<I = NoId> {
    pub id: I,
    pub session_id: Id,
    pub gateway_id: Id,
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

impl VpnSessionStats {
    #![allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn new(
        session_id: Id,
        gateway_id: Id,
        collected_at: NaiveDateTime,
        latest_handshake: NaiveDateTime,
        endpoint: String,
        total_upload: i64,
        total_download: i64,
        upload_diff: i64,
        download_diff: i64,
    ) -> Self {
        Self {
            id: NoId,
            session_id,
            gateway_id,
            collected_at,
            latest_handshake,
            endpoint,
            total_upload,
            total_download,
            upload_diff,
            download_diff,
        }
    }
}

impl VpnSessionStats<Id> {
    /// Returns latest available stats for a given device in a given location if available
    pub async fn fetch_latest_for_device<'e, E: PgExecutor<'e>>(
        executor: E,
        device_id: Id,
        location_id: Id,
    ) -> Result<Option<Self>, sqlx::Error> {
        let maybe_stats = query_as!(
            Self,
            "SELECT st.id, session_id, gateway_id, collected_at, latest_handshake, endpoint, \
			 total_upload, total_download, upload_diff, download_diff \
			 FROM vpn_session_stats st \
			 JOIN vpn_client_session se ON session_id = se.id \
			 WHERE device_id = $1 AND location_id = $2 \
			 ORDER BY collected_at DESC LIMIT 1",
            device_id,
            location_id
        )
        .fetch_optional(executor)
        .await?;

        Ok(maybe_stats)
    }

    /// Remove port part from `endpoint`.
    /// IPv4: a.b.c.d:p -> a.b.c.d
    /// IPv6: [x::y:z]:p -> x::y:z
    #[must_use]
    pub fn endpoint_without_port(&self) -> Option<String> {
        // Remove port part
        let mut addr = self.endpoint.rsplit_once(':')?.0;

        // Strip square brackets from IPv6 addrs
        if addr.starts_with('[') && addr.ends_with(']') {
            let end = addr.len() - 1;
            addr = &addr[1..end];
        }

        Some(addr.to_owned())
    }
}
