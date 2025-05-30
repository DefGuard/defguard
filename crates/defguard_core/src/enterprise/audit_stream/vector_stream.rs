use std::sync::Arc;

use base64::prelude::{Engine, BASE64_STANDARD};
use bytes::Bytes;
use tokio::{sync::broadcast::Sender, task::JoinSet};
use tokio_util::sync::CancellationToken;

use tracing::{debug, error};

use crate::enterprise::db::models::audit_stream::VectorHttpAuditStream;

pub fn run_vector_http_task(
    stream_config: VectorHttpAuditStream,
    tx: Arc<Sender<Bytes>>,
    cancel_token: Arc<CancellationToken>,
    handle_set: &mut JoinSet<()>,
) -> anyhow::Result<()> {
    let mut rx = tx.subscribe();
    let config = stream_config.clone();
    let child_token = cancel_token.child_token();
    handle_set.spawn(async move {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("Content-Type", "application/x-ndjson".parse().unwrap());

        // add authorization if it exists
        if let (Some(username), Some(password)) = (&config.username, &config.password) {
            let authorization_token =
                BASE64_STANDARD.encode(format!("{0}:{1}", username, password.expose_secret()));
            let auth_header_value = format!("Basic {authorization_token}");
            headers.insert("Authorization", auth_header_value.parse().unwrap());
        }

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
    Ok(())
}
