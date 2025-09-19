use std::sync::Arc;

use base64::prelude::{BASE64_STANDARD, Engine};
use bytes::Bytes;
use defguard_common::secret::SecretStringWrapper;
use reqwest::tls;
use tokio::sync::broadcast::Receiver;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error};

use crate::enterprise::db::models::activity_log_stream::{
    LogstashHttpActivityLogStream, VectorHttpActivityLogStream,
};

/// Spawns an asynchronous task that reads activity log events from the channel and sends them as NDJSON via HTTP.
///
/// # Parameters
///
/// - `config`: Configuration for this HTTP activity log stream.
/// - `rx`: A `tokio::sync::broadcast::Receiver<Bytes>` from which activity log messages are received.
/// - `cancel_token`: Shared `CancellationToken` used to signal task shutdown.
pub(super) async fn run_http_stream_task(
    config: HttpActivityLogStreamConfig,
    mut rx: Receiver<Bytes>,
    cancel_token: Arc<CancellationToken>,
) {
    let HttpActivityLogStreamConfig {
        stream_name, url, ..
    } = &config;
    let client = match build_client(&config) {
        Ok(client) => client,
        Err(err) => {
            error!("Failed to build HTTP client for stream {stream_name}: {err}");
            return;
        }
    };
    loop {
        tokio::select! {
            () = cancel_token.cancelled() => {
                debug!("Activity log stream ({stream_name}) task received cancellation signal.");
                break;
            },
            res = rx.recv() => {
                match res {
                    Ok(msg) => {
                        match client.post(url).body(msg).send().await {
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
                                    error!("Activity log stream ({stream_name}) response code {0}. Body: {1}", status_code, body);
                                }
                            },
                            Err(e) => {
                                error!("Activity log stream {stream_name} failed to send messages. Reason: {e}");
                            }
                        }
                    },
                    Err(e) => {
                        error!("Receiving activity log stream message failed ! Reason: {}", e.to_string());
                        break;
                    }
                }
            },
        }
    }
}

/// Builds and returns a configured `reqwest::Client` for sending NDJSON activity log events.
///
/// # Returns
///
/// - `Ok(reqwest::Client)`: A fully configured `reqwest::Client` ready to send NDJSON payloads.
/// - `Err(reqwest::Error)`: If building the client fails (e.g., invalid certificate or builder error).
fn build_client(config: &HttpActivityLogStreamConfig) -> Result<reqwest::Client, reqwest::Error> {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("Content-Type", "application/x-ndjson".parse().unwrap());

    if let (Some(username), Some(password)) = (&config.username, &config.password) {
        debug!(
            "Auth config found for {} activity log stream",
            config.stream_name
        );
        let authorization_token =
            BASE64_STANDARD.encode(format!("{0}:{1}", username, password.expose_secret()));
        let auth_header_value = format!("Basic {authorization_token}");
        headers.insert("Authorization", auth_header_value.parse().unwrap());
        debug!(
            "Authorization header added to {} activity log stream",
            config.stream_name
        );
    }

    let mut client = reqwest::ClientBuilder::new().default_headers(headers);
    if let Some(cert) = &config.cert {
        if config.url.contains("https") {
            match tls::Certificate::from_pem(cert.as_bytes()) {
                Ok(parsed_cert) => {
                    client = client.add_root_certificate(parsed_cert);
                }
                Err(e) => {
                    error!(
                        "Failed to add root certificate for {} activity log stream. Reason: {e}",
                        config.stream_name
                    );
                    return Err(e);
                }
            }
        }
    }
    if cfg!(debug_assertions) {
        client = client.danger_accept_invalid_hostnames(true);
    }
    client.build()
}

#[derive(Debug, Clone)]
pub(super) struct HttpActivityLogStreamConfig {
    pub stream_name: String,
    pub url: String,
    pub username: Option<String>,
    pub password: Option<SecretStringWrapper>,
    // cert to use for tls
    pub cert: Option<String>,
}

impl HttpActivityLogStreamConfig {
    pub fn from_logstash(value: LogstashHttpActivityLogStream, stream_name: String) -> Self {
        Self {
            stream_name,
            cert: value.cert,
            password: value.password,
            url: value.url,
            username: value.username,
        }
    }

    pub fn from_vector(value: VectorHttpActivityLogStream, stream_name: String) -> Self {
        Self {
            stream_name,
            cert: value.cert,
            password: value.password,
            url: value.url,
            username: value.username,
        }
    }
}
