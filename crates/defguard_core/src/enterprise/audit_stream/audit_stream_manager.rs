use std::sync::Arc;

use bytes::Bytes;
use sqlx::PgPool;
use tokio::{sync::broadcast::Sender, task::JoinSet, time::sleep};
use tokio_util::sync::CancellationToken;

use tracing::debug;

use crate::enterprise::{
    audit_stream::vector_stream::run_vector_http_task,
    db::models::audit_stream::{AuditStream, AuditStreamConfig},
    is_enterprise_enabled,
};

use super::{error::AuditStreamError, AuditStreamReconfigurationNotification};

async fn get_configurations(pool: &PgPool) -> Result<Vec<AuditStreamConfig>, AuditStreamError> {
    let db_data = AuditStream::all(pool).await?;
    let configs = db_data
        .into_iter()
        .map(|model| AuditStreamConfig::from(&model))
        .collect::<Result<Vec<AuditStreamConfig>, _>>()?;
    Ok(configs)
}

pub async fn run_audit_stream_manager(
    pool: PgPool,
    notification: AuditStreamReconfigurationNotification,
    audit_messages_tx: Arc<Sender<Bytes>>,
) -> anyhow::Result<()> {
    loop {
        let mut handles_set = JoinSet::<()>::new();
        let cancel_token = Arc::new(CancellationToken::new());
        if is_enterprise_enabled() {
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
        } else {
            debug!("Audit stream manager cannot start streams, license needs enterprise features enabled.");
        }
        // wait for next configs update or if license expired
        loop {
            tokio::select! {
                _ = notification.notified() => {
                    debug!(
                        "Audit stream manager configuration refresh notification received, reloading streaming tasks."
                    );
                    break;
               }
               _ = sleep(std::time::Duration::from_secs(60)) => {
                if !is_enterprise_enabled() {
                    debug!("Audit stream manager will reload, detected license enterprise features are not enabled");
                    break;
                }
               }
            }
        }
        cancel_token.cancel();
        while (handles_set.join_next().await).is_some() {}
        debug!("All audit streaming tasks closed.");
    }
}
