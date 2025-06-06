use std::sync::Arc;

use bytes::Bytes;
use sqlx::PgPool;
use tokio::{sync::broadcast::Receiver, task::JoinSet, time::sleep};
use tokio_util::sync::CancellationToken;

use tracing::debug;

use crate::enterprise::{
    activity_log_stream::http_stream::{run_http_stream_task, HttpActivityLogStreamConfig},
    db::models::activity_log_stream::{ActivityLogStream, ActivityLogStreamConfig},
    is_enterprise_enabled,
};

use super::ActivityLogStreamReconfigurationNotification;

pub async fn run_activity_log_stream_manager(
    pool: PgPool,
    notification: ActivityLogStreamReconfigurationNotification,
    activity_log_messages_rx: Receiver<Bytes>,
) -> anyhow::Result<()> {
    loop {
        let mut handles = JoinSet::<()>::new();
        let cancel_token = Arc::new(CancellationToken::new());
        if is_enterprise_enabled() {
            let streams = ActivityLogStream::all(&pool).await?;
            for activity_log_stream in streams {
                if let Ok(config) = ActivityLogStreamConfig::from(&activity_log_stream) {
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
            debug!("Activity log stream manager cannot start streams, license needs enterprise features enabled.");
        }
        // wait for next configs update or if license expired
        loop {
            tokio::select! {
                _ = notification.notified() => {
                    debug!(
                        "Activity log stream manager configuration refresh notification received, reloading streaming tasks."
                    );
                    break;
               }
               _ = sleep(std::time::Duration::from_secs(60)) => {
                if !is_enterprise_enabled() {
                    debug!("Activity log stream manager will reload, detected license enterprise features are not enabled");
                    break;
                }
               }
            }
        }
        cancel_token.cancel();
        handles.join_all().await;
        debug!("All activity log streaming tasks closed.");
    }
}
