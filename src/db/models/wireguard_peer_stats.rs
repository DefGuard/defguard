use std::time::Duration;

use chrono::{DateTime, NaiveDateTime, Utc};
use humantime::format_duration;
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
            - chrono::Duration::from_std(stats_purge_threshold).expect("Failed to parse duration"))
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
    /// IPv6: [x::y:z]:p -> [x::y:z]
    pub(crate) fn endpoint_without_port(&self) -> Option<String> {
        self.endpoint
            .as_ref()
            .and_then(|ep| Some(ep.rsplit_once(':')?.0.to_owned()))
    }

    /// Trim `allowed_ips` returning the first one without CIDR.
    pub(crate) fn trim_allowed_ips(&self) -> Option<String> {
        self.allowed_ips
            .as_ref()
            .and_then(|ips| Some(ips.split_once('/')?.0.to_owned()))
    }
}
