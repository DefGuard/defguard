use std::sync::Arc;

use base64::prelude::{Engine, BASE64_STANDARD};
use bytes::Bytes;
use reqwest::tls;
use tokio::{sync::broadcast::Sender, task::JoinSet};
use tokio_util::sync::CancellationToken;

use tracing::{debug, error};

use crate::enterprise::db::models::audit_stream::LogstashHttpAuditStream;

use super::error::AuditStreamError;

pub fn run_logstash_http_task(
    stream_config: LogstashHttpAuditStream,
    tx: Arc<Sender<Bytes>>,
    cancel_token: Arc<CancellationToken>,
    handle_set: &mut JoinSet<()>,
) -> Result<(), AuditStreamError> {
    let mut rx = tx.subscribe();
    let config = stream_config.clone();
    let child_token = cancel_token.child_token();
    handle_set.spawn(async move {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("Content-Type", "application/x-ndjson".parse().unwrap());

        if let (Some(username), Some(password)) = (&config.username, &config.password) {
            debug!("Auth config found for Logstash audit stream");
            let authorization_token =
                BASE64_STANDARD.encode(format!("{0}:{1}", username, password.expose_secret()));
            let auth_header_value = format!("Basic {authorization_token}");
            headers.insert("Authorization", auth_header_value.parse().unwrap());
            debug!("Authorization header added to Logstash audit stream");
        }

        let mut client = reqwest::ClientBuilder::new()
            .default_headers(headers);
        if let Some(cert) = &config.cert {
            if config.url.contains("https") {
                match tls::Certificate::from_pem(cert.as_bytes()) {
                    Ok(parsed_cert) => {
                        client = client.add_root_certificate(parsed_cert);
                    }
                    Err(e) => {
                        error!("Failed to add root certificate for Logstash audit stream. Reason: {0}", e.to_string());
                        return;
                    }
                }
            }
        }
        if cfg!(debug_assertions) {
            client = client.danger_accept_invalid_hostnames(true);
        }
        let client = client.build().unwrap();
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
                            match client.post(&url).body(msg).send().await {
                                Ok(response) => {
                                    if !response.status().is_success() {
                                        let status = &response.status();
                                        let status_code = status.as_str();
                                        let body: String = {
                                            let text = &response.text().await;
                                            match text {
                                                Ok(body) => body.to_string(),
                                                Err(_) => "Body decoding failed".to_string(),
                                            }
                                        };
                                        error!("Logstash audit stream response code {0}. Body: {1}", status_code, body);
                                    }
                                },
                                Err(e) => {
                                    error!("Failed to send Logstash audit stream messages. Reason: {e}");
                                }
                            }
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
