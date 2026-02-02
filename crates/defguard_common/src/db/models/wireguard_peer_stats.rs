use chrono::NaiveDateTime;
use ipnetwork::IpNetwork;
use model_derive::Model;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, query_as};

use crate::db::{Id, NoId};

#[derive(Debug, Deserialize, Model, Serialize)]
#[table(wireguard_peer_stats)]
pub struct WireguardPeerStats<I = NoId> {
    pub id: I,
    pub device_id: Id,
    pub collected_at: NaiveDateTime,
    pub network: i64,
    // optional because it's not available until a peer actually connects
    pub endpoint: Option<String>,
    // bytes sent to peer
    pub upload: i64,
    // bytes received from peer
    pub download: i64,
    pub latest_handshake: NaiveDateTime,
    // FIXME: can contain multiple IP addresses
    pub allowed_ips: Option<String>,
}

impl WireguardPeerStats<Id> {
    pub async fn fetch_latest(
        conn: &PgPool,
        device_id: Id,
        network_id: Id,
    ) -> Result<Option<Self>, sqlx::Error> {
        let stats = query_as!(
            Self,
            "SELECT id, device_id \"device_id!\", collected_at \"collected_at!\", \
            network \"network!\", endpoint, upload \"upload!\", download \"download!\", \
            latest_handshake \"latest_handshake!\", allowed_ips \
            FROM wireguard_peer_stats \
            WHERE device_id = $1 AND network = $2 \
            ORDER BY collected_at DESC LIMIT 1",
            device_id,
            network_id,
        )
        .fetch_optional(conn)
        .await?;

        Ok(stats)
    }

    /// Remove port part from `endpoint`.
    /// IPv4: a.b.c.d:p -> a.b.c.d
    /// IPv6: [x::y:z]:p -> x::y:z
    #[must_use]
    pub fn endpoint_without_port(&self) -> Option<String> {
        self.endpoint.as_ref().and_then(|endpoint| {
            let mut addr = endpoint.rsplit_once(':')?.0;
            // Strip square brackets.
            if addr.starts_with('[') && addr.ends_with(']') {
                let end = addr.len() - 1;
                addr = &addr[1..end];
            }
            Some(addr.to_owned())
        })
    }

    /// Returns a `Vec` of `allowed_ips` without a CIDR mask.
    /// Non-parsable addresses are omitted.
    #[must_use]
    pub fn trim_allowed_ips(&self) -> Vec<String> {
        let Some(allowed_ips) = &self.allowed_ips else {
            return Vec::new();
        };
        allowed_ips
            .split(',')
            .filter_map(|addr| Some(addr.trim().parse::<IpNetwork>().ok()?.ip().to_string()))
            .collect()
    }
}

#[cfg(test)]
mod test {
    use chrono::Utc;

    use super::*;

    #[test]
    fn test_trim_allowed_ips() {
        let mut stats = WireguardPeerStats {
            id: 1,
            device_id: 1,
            collected_at: Utc::now().naive_utc(),
            network: 1,
            endpoint: None,
            upload: 100,
            download: 100,
            latest_handshake: Utc::now().naive_utc(),
            allowed_ips: None,
        };
        assert!(stats.trim_allowed_ips().is_empty());

        stats.allowed_ips = Some("10.1.1.1".to_string());
        assert_eq!(stats.trim_allowed_ips(), vec!["10.1.1.1"]);

        stats.allowed_ips = Some("10.1.1.1/24".to_string());
        assert_eq!(stats.trim_allowed_ips(), vec!["10.1.1.1"]);

        stats.allowed_ips = Some("10.1.1.1/24, 10.1.1.2".to_string());
        assert_eq!(stats.trim_allowed_ips(), vec!["10.1.1.1", "10.1.1.2"]);

        stats.allowed_ips = Some("10.1.1.1/24, 10.1.1.2/24".to_string());
        assert_eq!(stats.trim_allowed_ips(), vec!["10.1.1.1", "10.1.1.2"]);

        stats.allowed_ips = Some("fc00::1".to_string());
        assert_eq!(stats.trim_allowed_ips(), vec!["fc00::1"]);

        stats.allowed_ips = Some("fc00::1/112".to_string());
        assert_eq!(stats.trim_allowed_ips(), vec!["fc00::1"]);

        stats.allowed_ips = Some("fc00::1/112,fc00::2".to_string());
        assert_eq!(stats.trim_allowed_ips(), vec!["fc00::1", "fc00::2"]);

        stats.allowed_ips = Some("fc00::1/112,fc00::2/112".to_string());
        assert_eq!(stats.trim_allowed_ips(), vec!["fc00::1", "fc00::2"]);

        stats.allowed_ips = Some("10.1.1.1, fc00::1".to_string());
        assert_eq!(stats.trim_allowed_ips(), vec!["10.1.1.1", "fc00::1"]);

        stats.allowed_ips = Some("10.1.1.1/24, fc00::1/112".to_string());
        assert_eq!(stats.trim_allowed_ips(), vec!["10.1.1.1", "fc00::1"]);

        stats.allowed_ips = Some("nonparsable, fc00::1/112".to_string());
        assert_eq!(stats.trim_allowed_ips(), vec!["fc00::1"]);
    }
}
