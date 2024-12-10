use std::time::Duration;

use sqlx::PgPool;
use tokio::time::sleep;
use tokio::time::Instant;

use crate::enterprise::directory_sync::get_directory_sync_interval;
use crate::enterprise::{directory_sync::do_directory_sync, limits::do_count_update};

const UTILITY_THREAD_MAIN_SLEEP_TIME: u64 = 5;
const COUNT_UPDATE_INTERVAL: u64 = 60 * 60;

pub async fn run_utility_thread(pool: &PgPool) -> Result<(), anyhow::Error> {
    let mut last_count_update: Option<Instant> = None;
    let mut last_directory_sync: Option<Instant> = None;

    loop {
        // Count update job for updating device/user/network counts
        if last_count_update.is_none()
            || last_count_update
                .unwrap_or(Instant::now())
                .elapsed()
                .as_secs()
                >= COUNT_UPDATE_INTERVAL
        {
            if let Err(e) = do_count_update(pool).await {
                error!(
                    "There was an error while performing count update job: {e:?}",
                    e
                );
            }
            last_count_update = Some(Instant::now());
        }

        // Directory sync job for syncing with the directory service
        if last_directory_sync.is_none()
            || last_directory_sync
                .unwrap_or(Instant::now())
                .elapsed()
                .as_secs()
                >= get_directory_sync_interval(pool).await
        {
            if let Err(e) = do_directory_sync(pool).await {
                error!(
                    "There was an error while performing directory sync job: {:?}",
                    e
                );
            }
            last_directory_sync = Some(Instant::now());
        }

        sleep(Duration::from_secs(UTILITY_THREAD_MAIN_SLEEP_TIME)).await;
    }
}
