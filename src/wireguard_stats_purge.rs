use crate::db::{DbPool, WireguardPeerStats};
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use humantime::format_duration;
use sqlx::{query, Error as SqlxError};
use std::time::Duration;
use tokio::time::sleep;

impl WireguardPeerStats {
    /// Delete stats older than a configured threshold.
    /// This is done to prevent unnecessary table growth.
    /// At least one record is retained for each device & network combination,
    /// even when older than set threshold.
    pub async fn purge_old_stats(
        pool: &DbPool,
        stats_purge_threshold: Duration,
    ) -> Result<(), SqlxError> {
        let start = Utc::now();
        info!(
            "Purging stats older than {}",
            format_duration(stats_purge_threshold)
        );

        let threshold = (Utc::now()
            - ChronoDuration::from_std(stats_purge_threshold).expect("Failed to parse duration"))
        .naive_utc();
        let result = query!(
            r#"DELETE FROM wireguard_peer_stats
            WHERE collected_at < $1
            AND (device_id, network, collected_at) NOT IN (
                SELECT device_id, network, MAX(collected_at)
                FROM wireguard_peer_stats
                GROUP BY device_id, network
            )"#,
            threshold
        )
        .execute(pool)
        .await?;

        let end = Utc::now();

        info!(
            "Removed {} old records from wireguard_peer_stats",
            result.rows_affected(),
        );

        // record successful stats purge in DB
        Self::record_stats_purge(pool, start, end).await?;

        Ok(())
    }

    // Check how much time has elapsed since last recorded stats purge
    pub async fn time_since_last_purge<'e, E>(executor: E) -> Result<Duration, SqlxError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        debug!("Checking time since last stats purge");

        unimplemented!()
    }

    async fn record_stats_purge<'e, E>(
        executor: E,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Duration, SqlxError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        debug!("Recording successful stats purge in DB");
        unimplemented!()
    }
}

pub async fn run_periodic_stats_purge(
    pool: DbPool,
    stats_purge_frequency: Duration,
    stats_purge_threshold: Duration,
) -> Result<(), SqlxError> {
    info!(
        "Starting periodic purge of stats older than {} every {}",
        format_duration(stats_purge_threshold),
        format_duration(stats_purge_frequency)
    );

    loop {
        debug!("Checking if stats purge should be executed");
        // check time elapsed since last purge
        if WireguardPeerStats::time_since_last_purge(&pool).await? >= stats_purge_frequency {
            // perform purge
            info!("Executing stats purge");
            match WireguardPeerStats::purge_old_stats(&pool, stats_purge_threshold).await {
                Ok(_) => {
                    let next_purge_timestamp = (Utc::now()
                        + ChronoDuration::from_std(stats_purge_frequency)
                            .expect("Failed to parse duration"))
                    .naive_utc();
                    info!(
                        "Stats purge successful. Next purge will be executed at {}",
                        next_purge_timestamp
                    );
                }
                Err(err) => {
                    error!("Error while purging stats: {}", err)
                }
            }
        }

        // wait till next iteration
        debug!("Sleeping until next iteration");
        sleep(Duration::from_secs(5 * 60)).await;
    }
}
