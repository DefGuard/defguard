use crate::db::WireguardPeerStats;
use std::time::Duration;

impl WireguardPeerStats {
    /// Delete stats older than a configured threshold.
    /// This is done to prevent unnecessary table growth.
    /// At least one record is retained for each device & network combination,
    /// even when older than set threshold.
    pub async fn purge_old_stats() {
        unimplemented!()
    }
}

pub async fn run_periodic_stats_purge(
    stats_purge_frequency: Duration,
    stats_purge_threshold: Duration,
) {
    unimplemented!()
}
