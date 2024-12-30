use std::time::Duration;

use chrono::Utc;
use humantime::format_duration;
use sqlx::PgPool;
use tokio::time::sleep;

use crate::db::models::wireguard_peer_stats::WireguardPeerStats;

// How long to sleep between loop iterations
const PURGE_LOOP_SLEEP: Duration = Duration::from_secs(300); // 5 minutes

pub async fn run_periodic_stats_purge(
    pool: PgPool,
    stats_purge_frequency: Duration,
    stats_purge_threshold: Duration,
) -> Result<(), sqlx::Error> {
    info!(
        "Starting periodic purge of stats older than {} every {}",
        format_duration(stats_purge_threshold),
        format_duration(stats_purge_frequency)
    );

    loop {
        debug!("Checking if stats purge should be executed");
        // check time elapsed since last purge
        let time_since_last_purge = WireguardPeerStats::time_since_last_purge(&pool).await?;
        if match time_since_last_purge {
            Some(time_since) => time_since >= stats_purge_frequency,
            None => true,
        } {
            // perform purge
            info!("Executing stats purge");
            match WireguardPeerStats::purge_old_stats(&pool, stats_purge_threshold).await {
                Ok(()) => {
                    let next_purge_timestamp = (Utc::now()
                        + chrono::Duration::from_std(stats_purge_frequency)
                            .expect("Failed to parse duration"))
                    .naive_utc();
                    info!(
                        "Stats purge successful. Next purge will be executed at {next_purge_timestamp}"
                    );
                }
                Err(err) => {
                    error!("Error while purging stats: {err}");
                }
            }
        }

        // wait till next iteration
        debug!("Sleeping until next iteration");
        sleep(PURGE_LOOP_SLEEP).await;
    }
}
