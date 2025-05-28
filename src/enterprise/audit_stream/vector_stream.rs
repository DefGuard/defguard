use std::sync::Arc;

use base64::prelude::{Engine, BASE64_STANDARD};
use bytes::Bytes;
use tokio::sync::broadcast::Sender;
use tokio_util::sync::CancellationToken;

use crate::enterprise::db::models::audit_stream::VectorHttpAuditStream;

pub(super) fn run_vector_http_task(
    stream_config: VectorHttpAuditStream,
    tx: Arc<Sender<Bytes>>,
    cancel_token: Arc<CancellationToken>,
) -> anyhow::Result<tokio::task::JoinHandle<()>> {
    let mut rx = tx.subscribe();
    let config = stream_config.clone();
    let child_token = cancel_token.child_token();
    let handle = tokio::spawn(async move {
        let authorization_token = BASE64_STANDARD.encode(format!(
            "{0}:{1}",
            config.username,
            config.password.expose_secret()
        ));
        let auth_header_value = format!("Basic {authorization_token}");
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("Content-Type", "application/x-ndjson".parse().unwrap());
        headers.insert("Authorization", auth_header_value.parse().unwrap());

        let client = reqwest::ClientBuilder::new()
            .default_headers(headers)
            .build()
            .unwrap();
        let url = config.url;
        loop {
            tokio::select! {
                _ = child_token.cancelled() => {
                    debug!("Audit stream task received cancellation signal.");
                    break;
                },
                res = rx.recv() => {
                    match res {
                        Ok(msg) => {
                            // Todo: Add logs ?
                            let _ = client.post(&url).body(msg).send().await;
                        },
                        Err(e) => {
                            error!("Receiving audit stream message failed ! Reason: {}", e.to_string());
                            break;
                        }
                    }
                },
            }
        }
    });
    Ok(handle)
}
