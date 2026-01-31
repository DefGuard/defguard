use std::time::Duration;

use chrono::{DateTime, NaiveDateTime, TimeDelta, Utc};
use humantime::format_duration;
use sqlx::{PgExecutor, PgPool, query, query_scalar};
use tokio::time::sleep;
use tracing::{debug, error, info, instrument};

// How long to sleep between loop iterations
const PURGE_LOOP_SLEEP: Duration = Duration::from_secs(300); // 5 minutes

#[instrument(skip_all)]
pub async fn run_periodic_stats_purge(
    pool: PgPool,
    stats_purge_frequency: Duration,
    stats_purge_threshold: Duration,
) -> Result<(), sqlx::Error> {
    info!(
        "Starting periodic purge of VPN sessions and related stats older than {} every {}",
        format_duration(stats_purge_threshold),
        format_duration(stats_purge_frequency)
    );

    loop {
        debug!("Checking if stats purge should be executed");
        // check time elapsed since last purge
        let time_since_last_purge = time_since_last_purge(&pool).await?;
        if match time_since_last_purge {
            Some(time_since) => time_since >= stats_purge_frequency,
            None => true,
        } {
            // perform purge
            info!("Executing VPN session stats purge");
            match purge_old_sessions(&pool, stats_purge_threshold).await {
                Ok(()) => {
                    let next_purge_timestamp = (Utc::now()
                        + TimeDelta::from_std(stats_purge_frequency)
                            .expect("Failed to parse duration"))
                    .naive_utc();
                    info!(
                        "VPN session stats purge successful. Next purge will be executed at {next_purge_timestamp}"
                    );
                }
                Err(err) => {
                    error!("Error while purging VPN session stats: {err}");
                }
            }
        }

        // wait till next iteration
        debug!("Sleeping until next iteration");
        sleep(PURGE_LOOP_SLEEP).await;
    }
}

// Check how much time has elapsed since last recorded stats purge
async fn time_since_last_purge<'e, E>(executor: E) -> Result<Option<Duration>, sqlx::Error>
where
    E: PgExecutor<'e>,
{
    debug!("Checking time since last VPN session stats purge");

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

/// Delete VPN sessions and related stats older than a configured threshold.
/// This is done to prevent unnecessary table growth.
/// At least one session is retained for each device and location combination,
/// even when older than set threshold to generate last connection info for VPN overview.
async fn purge_old_sessions(
    pool: &PgPool,
    stats_purge_threshold: Duration,
) -> Result<(), sqlx::Error> {
    let start = Utc::now();
    info!(
        "Purging VPN sessions older than {}",
        format_duration(stats_purge_threshold)
    );

    let threshold = (Utc::now()
        - TimeDelta::from_std(stats_purge_threshold).expect("Failed to parse duration"))
    .naive_utc();

    let result = query!(
        "DELETE FROM vpn_client_session \
            WHERE created_at < $1 \
            AND (device_id, location_id, created_at) NOT IN ( \
                SELECT device_id, location_id, MAX(created_at) \
                FROM vpn_client_session \
                GROUP BY device_id, location_id)",
        threshold
    )
    .execute(pool)
    .await?;

    let end = Utc::now();
    let rows_count = result.rows_affected();

    info!("Removed {rows_count} old records from wireguard_peer_stats",);

    // Store successful stats purge in database.
    record_stats_purge(pool, start, end, threshold, rows_count as i64).await?;

    Ok(())
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
    debug!("Recording successful VPN session stats purge in database");
    query!("INSERT INTO wireguard_stats_purge (started_at, finished_at, removal_threshold, records_removed) VALUES ($1, $2, $3, $4)",
        start.naive_utc(), end.naive_utc(), removal_threshold, records_removed).execute(executor).await?;

    Ok(())
}
