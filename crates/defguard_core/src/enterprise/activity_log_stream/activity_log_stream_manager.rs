use std::{sync::Arc, time::Duration};

use bytes::Bytes;
use sqlx::PgPool;
use tokio::{sync::broadcast::Receiver, task::JoinSet, time::interval};
use tokio_util::sync::CancellationToken;

use tracing::debug;

use crate::enterprise::{
    activity_log_stream::http_stream::{run_http_stream_task, HttpActivityLogStreamConfig},
    db::models::activity_log_stream::{ActivityLogStream, ActivityLogStreamConfig},
    is_enterprise_enabled,
};

use super::ActivityLogStreamReconfigurationNotification;

// check if enterprise features are enabled every minute
const ENTERPRISE_CHECK_PERIOD_SECS: u64 = 60;

#[instrument(skip_all)]
pub async fn run_activity_log_stream_manager(
    pool: PgPool,
    notification: ActivityLogStreamReconfigurationNotification,
    activity_log_messages_rx: Receiver<Bytes>,
) -> anyhow::Result<()> {
    info!("Starting activity log stream manager");

    let mut enterprise_check_timer = interval(Duration::from_secs(ENTERPRISE_CHECK_PERIOD_SECS));

    // initialize enterprise features status
    let mut enterprise_features_enabled = is_enterprise_enabled();

    loop {
        let mut handles = JoinSet::<()>::new();
        let cancel_token = Arc::new(CancellationToken::new());

        // check if activity log streams can be started
        if enterprise_features_enabled {
            info!("Starting all configured activity log streams");
            let streams = ActivityLogStream::all(&pool).await?;
            debug!("Found {} configured activity log streams", streams.len());

            // spawn all configured streaming tasks in the background
            for activity_log_stream in streams {
                if let Ok(config) = ActivityLogStreamConfig::from(&activity_log_stream) {
                    debug!("Starting activity log stream with config: {config:?}");
                    match config {
                        ActivityLogStreamConfig::VectorHttp(stream_config) => {
                            let http_config = HttpActivityLogStreamConfig::from_vector(
                                stream_config,
                                activity_log_stream.name.clone(),
                            );
                            handles.spawn(run_http_stream_task(
                                http_config,
                                activity_log_messages_rx.resubscribe(),
                                cancel_token.clone(),
                            ));
                        }
                        ActivityLogStreamConfig::LogstashHttp(stream_config) => {
                            let http_config = HttpActivityLogStreamConfig::from_logstash(
                                stream_config,
                                activity_log_stream.name.clone(),
                            );
                            handles.spawn(run_http_stream_task(
                                http_config,
                                activity_log_messages_rx.resubscribe(),
                                cancel_token.clone(),
                            ));
                        }
                    };
                } else {
                    error!(
                        "Failed to deserialize config for activity log stream {0}",
                        &activity_log_stream.name
                    );
                    continue;
                }
            }
        } else {
            info!("Activity log stream manager cannot start streams, license needs enterprise features enabled.");
        }

        // wait for one of the following:
        // - stream config update
        // - enterprise features got disabled/enabled
        // - streaming task terminated early
        loop {
            tokio::select! {
                _ = notification.notified() => {
                    info!(
                        "Activity log stream manager configuration refresh notification received, reloading streaming tasks."
                    );
                    break;
               }
               _ = enterprise_check_timer.tick() => {
                    // check if enterprise features status has changed
                    let current_enterprise_features_enabled = is_enterprise_enabled();
                    if current_enterprise_features_enabled != enterprise_features_enabled {
                        warn!("Activity log stream manager will reload, detected license enterprise features status has changed");
                        enterprise_features_enabled = current_enterprise_features_enabled;
                        break;
                    }
               }
               task_output = handles.join_next(), if !handles.is_empty() => {
                    error!("Activity log streaming task has terminated early with result: {task_output:?}, reloading activity log stream manager");
                    break;
               }
            }
        }

        // trigger all spawned tasks to stop
        cancel_token.cancel();
        // wait for all tasks to actually stop
        handles.join_all().await;

        debug!("All activity log streaming tasks closed.");
    }
}
