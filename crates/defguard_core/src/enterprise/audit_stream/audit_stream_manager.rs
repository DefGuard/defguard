use std::sync::Arc;

use bytes::Bytes;
use sqlx::PgPool;
use tokio::{sync::broadcast::Receiver, task::JoinSet, time::sleep};
use tokio_util::sync::CancellationToken;
use tracing::debug;

use super::AuditStreamReconfigurationNotification;
use crate::enterprise::{
    audit_stream::http_stream::{run_http_stream_task, HttpAuditStreamConfig},
    db::models::audit_stream::{AuditStream, AuditStreamConfig},
    is_enterprise_enabled,
};

pub async fn run_audit_stream_manager(
    pool: PgPool,
    notification: AuditStreamReconfigurationNotification,
    audit_messages_rx: Receiver<Bytes>,
) -> anyhow::Result<()> {
    loop {
        let mut handles = JoinSet::<()>::new();
        let cancel_token = Arc::new(CancellationToken::new());
        if is_enterprise_enabled() {
            let streams = AuditStream::all(&pool).await?;
            for audit_stream in streams {
                if let Ok(config) = AuditStreamConfig::from(&audit_stream) {
                    match config {
                        AuditStreamConfig::VectorHttp(stream_config) => {
                            let http_config = HttpAuditStreamConfig::from_vector(
                                stream_config,
                                audit_stream.name.clone(),
                            );
                            handles.spawn(run_http_stream_task(
                                http_config,
                                audit_messages_rx.resubscribe(),
                                cancel_token.clone(),
                            ));
                        }
                        AuditStreamConfig::LogstashHttp(stream_config) => {
                            let http_config = HttpAuditStreamConfig::from_logstash(
                                stream_config,
                                audit_stream.name.clone(),
                            );
                            handles.spawn(run_http_stream_task(
                                http_config,
                                audit_messages_rx.resubscribe(),
                                cancel_token.clone(),
                            ));
                        }
                    };
                } else {
                    error!(
                        "Failed to deserialize config for audit stream {0}",
                        &audit_stream.name
                    );
                    continue;
                }
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
        handles.join_all().await;
        debug!("All audit streaming tasks closed.");
    }
}
