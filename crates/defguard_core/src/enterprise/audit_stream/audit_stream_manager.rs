use std::sync::Arc;

use bytes::Bytes;
use sqlx::PgPool;
use tokio::{sync::broadcast::Sender, task::JoinHandle};
use tokio_util::sync::CancellationToken;

use tracing::debug;

use crate::enterprise::{
    audit_stream::vector_stream::run_vector_http_task,
    db::models::audit_stream::{AuditStreamConfig, AuditStreamModel},
};

use super::{error::AuditStreamError, AuditStreamReconfigurationNotification};

async fn get_configurations(pool: &PgPool) -> Result<Vec<AuditStreamConfig>, AuditStreamError> {
    let db_data = AuditStreamModel::all(pool).await?;
    let mut configs: Vec<AuditStreamConfig> = Vec::with_capacity(db_data.len());
    for model in db_data {
        let stream_config = AuditStreamConfig::from(&model)?;
        configs.push(stream_config);
    }
    Ok(configs)
}

pub async fn run_audit_stream_manager(
    pool: PgPool,
    notification: AuditStreamReconfigurationNotification,
    audit_messages_tx: Arc<Sender<Bytes>>,
) -> anyhow::Result<()> {
    loop {
        let mut handles: Vec<JoinHandle<()>> = vec![];
        let cancel_token = Arc::new(CancellationToken::new());
        // check if any configurations are present
        let configs = get_configurations(&pool).await?;
        let configs_empty = configs.is_empty();
        for config in configs {
            match config {
                AuditStreamConfig::VectorHttp(stream) => {
                    let task_handle = run_vector_http_task(
                        stream,
                        audit_messages_tx.clone(),
                        cancel_token.clone(),
                    )?;
                    handles.push(task_handle);
                }
            }
        }
        if !configs_empty {
            debug!("All Audit stream manager tasks running.");
        } else {
            debug!(
                "Audit stream have no configurations, manager will wait for reload notification."
            );
        }
        // wait for next configs update
        notification.notified().await;
        // kill the worker and spawn new with refreshed configs
        debug!(
            "Audit stream manager configuration refresh notification received, reloading streaming tasks."
        );
        cancel_token.cancel();
        for handle in handles {
            let _ = handle.await;
        }
        debug!("All audit streaming tasks closed.");
    }
}
