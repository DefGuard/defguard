use std::sync::Arc;

use bytes::Bytes;
use sqlx::PgPool;
use tokio::{sync::broadcast::Sender, task::JoinSet};
use tokio_util::sync::CancellationToken;

use tracing::debug;

use crate::enterprise::{
    audit_stream::vector_stream::run_vector_http_task,
    db::models::audit_stream::{AuditStream, AuditStreamConfig},
    license::{get_cached_license, validate_license},
    limits::get_counts,
};

use super::{error::AuditStreamError, AuditStreamReconfigurationNotification};

async fn get_configurations(pool: &PgPool) -> Result<Vec<AuditStreamConfig>, AuditStreamError> {
    let db_data = AuditStream::all(pool).await?;
    let mut configs: Vec<AuditStreamConfig> = Vec::with_capacity(db_data.len());
    for model in db_data {
        let stream_config = AuditStreamConfig::from(&model)?;
        configs.push(stream_config);
    }
    Ok(configs)
}

fn check_license() -> bool {
    let license_guard = get_cached_license();
    let counts = get_counts();
    validate_license(license_guard.as_ref(), &counts).is_ok()
}

pub async fn run_audit_stream_manager(
    pool: PgPool,
    notification: AuditStreamReconfigurationNotification,
    audit_messages_tx: Arc<Sender<Bytes>>,
) -> anyhow::Result<()> {
    loop {
        let license_valid = check_license();
        let mut handles_set = JoinSet::<()>::new();
        let cancel_token = Arc::new(CancellationToken::new());
        if license_valid {
            // check if any configurations are present
            let configs = get_configurations(&pool).await?;
            let configs_empty = configs.is_empty();
            for config in configs {
                match config {
                    AuditStreamConfig::VectorHttp(stream) => {
                        if let Err(e) = run_vector_http_task(
                            stream,
                            audit_messages_tx.clone(),
                            cancel_token.clone(),
                            &mut handles_set,
                        ) {
                            error!("Failed to start vector audit stream task. Reason: {e}");
                        }
                    }
                }
            }
            if !configs_empty {
                debug!("All Audit stream manager tasks running.");
                info!("Audit logs streaming started");
            } else {
                debug!(
                "Audit stream have no configurations, manager will wait for reload notification."
            );
            }
        }
        // wait for next configs update
        notification.notified().await;
        // kill the worker and spawn new with refreshed configs
        debug!(
            "Audit stream manager configuration refresh notification received, reloading streaming tasks."
        );
        cancel_token.cancel();
        while let Some(_) = handles_set.join_next().await {}
        debug!("All audit streaming tasks closed.");
    }
}
