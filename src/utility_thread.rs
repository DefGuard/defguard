use std::time::Duration;

use sqlx::PgPool;
use tokio::time::{sleep, Instant};

use crate::enterprise::{
    directory_sync::{do_directory_sync, get_directory_sync_interval},
    limits::do_count_update,
};

const UTILITY_THREAD_MAIN_SLEEP_TIME: u64 = 5;
const COUNT_UPDATE_INTERVAL: u64 = 60 * 60;

pub async fn run_utility_thread(pool: &PgPool) -> Result<(), anyhow::Error> {
    let mut last_count_update = Instant::now();
    let mut last_directory_sync = Instant::now();

    let directory_sync_task = || async {
        if let Err(e) = do_directory_sync(pool).await {
            error!("There was an error while performing directory sync job: {e:?}",);
        }
    };

    let count_update_task = || async {
        if let Err(e) = do_count_update(pool).await {
            error!("There was an error while performing count update job: {e:?}");
        }
    };

    directory_sync_task().await;
    count_update_task().await;

    loop {
        sleep(Duration::from_secs(UTILITY_THREAD_MAIN_SLEEP_TIME)).await;

        // Count update job for updating device/user/network counts
        if last_count_update.elapsed().as_secs() >= COUNT_UPDATE_INTERVAL {
            count_update_task().await;
            last_count_update = Instant::now();
        }

        // Directory sync job for syncing with the directory service
        if last_directory_sync.elapsed().as_secs() >= get_directory_sync_interval(pool).await {
            directory_sync_task().await;
            last_directory_sync = Instant::now();
        }
    }
}
