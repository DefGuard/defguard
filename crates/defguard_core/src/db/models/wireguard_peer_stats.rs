use std::time::Duration;

use chrono::{DateTime, NaiveDateTime, TimeDelta, Utc};
use humantime::format_duration;
use ipnetwork::IpNetwork;
use model_derive::Model;
use sqlx::{query, query_as, query_scalar, PgExecutor, PgPool};

use crate::db::{Id, NoId};

#[derive(Debug, Deserialize, Model, Serialize)]
#[table(wireguard_peer_stats)]
pub struct WireguardPeerStats<I = NoId> {
    pub id: I,
    pub device_id: Id,
    pub collected_at: NaiveDateTime,
    pub network: i64,
    pub endpoint: Option<String>,
    pub upload: i64,
    pub download: i64,
    pub latest_handshake: NaiveDateTime,
    // FIXME: can contain multiple IP addresses
    pub allowed_ips: Option<String>,
}

impl WireguardPeerStats {
    /// Delete stats older than a configured threshold.
    /// This is done to prevent unnecessary table growth.
    /// At least one record is retained for each device and network combination,
    /// even when older than set threshold.
    pub(crate) async fn purge_old_stats(
        pool: &PgPool,
        stats_purge_threshold: Duration,
    ) -> Result<(), sqlx::Error> {
        let start = Utc::now();
        info!(
            "Purging stats older than {}",
            format_duration(stats_purge_threshold)
        );

        let threshold = (Utc::now()
            - TimeDelta::from_std(stats_purge_threshold).expect("Failed to parse duration"))
        .naive_utc();
        let result = query!(
            "DELETE FROM wireguard_peer_stats \
            WHERE collected_at < $1 \
            AND (device_id, network, collected_at) NOT IN ( \
                SELECT device_id, network, MAX(collected_at) \
                FROM wireguard_peer_stats \
                GROUP BY device_id, network)",
            threshold
        )
        .execute(pool)
        .await?;

        let end = Utc::now();
        let rows_count = result.rows_affected();

        info!("Removed {rows_count} old records from wireguard_peer_stats",);

        // Store successful stats purge in database.
        Self::record_stats_purge(pool, start, end, threshold, rows_count as i64).await?;

        Ok(())
    }

    // Check how much time has elapsed since last recorded stats purge
    pub async fn time_since_last_purge<'e, E>(executor: E) -> Result<Option<Duration>, sqlx::Error>
    where
        E: PgExecutor<'e>,
    {
        debug!("Checking time since last stats purge");

        let timestamp = query_scalar!("SELECT MAX(started_at) FROM wireguard_stats_purge")
            .fetch_one(executor)
            .await?;

        match timestamp {
            Some(timestamp) => {
                let time_since = Utc::now().signed_duration_since(timestamp.and_utc());
                let time_since = time_since.to_std().expect("Failed to parse duration");
                debug!(
                    "Time since last stats purge: {}",
                    format_duration(time_since)
                );
                Ok(Some(time_since))
            }
            None => Ok(None),
        }
    }

    async fn record_stats_purge<'e, E>(
        executor: E,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        removal_threshold: NaiveDateTime,
        records_removed: i64,
    ) -> Result<(), sqlx::Error>
    where
        E: PgExecutor<'e>,
    {
        debug!("Recording successful stats purge in database");
        query!("INSERT INTO wireguard_stats_purge (started_at, finished_at, removal_threshold, records_removed) VALUES ($1, $2, $3, $4)",
        start.naive_utc(), end.naive_utc(), removal_threshold, records_removed).execute(executor).await?;

        Ok(())
    }
}

impl WireguardPeerStats<Id> {
    pub(crate) async fn fetch_latest(
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
    pub(crate) fn endpoint_without_port(&self) -> Option<String> {
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
    pub(crate) fn trim_allowed_ips(&self) -> Vec<String> {
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
