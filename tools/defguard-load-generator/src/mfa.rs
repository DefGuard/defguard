use std::time::{Duration, Instant, SystemTime};

use anyhow::Context;
use reqwest::{Client, StatusCode};
use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};
use sqlx::{
    FromRow,
    postgres::{PgConnectOptions, PgPoolOptions},
    query_as,
};
use tokio::task::JoinError;
use totp_lite::{Sha1, totp_custom};

use crate::{
    config::{ClientMfaArgs, DatabaseArgs},
    runner::{LoadLoopConfig, run_load_loop},
};

const START_PATH: &str = "/client-mfa/start";
const FINISH_PATH: &str = "/client-mfa/finish";
const TOTP_PERIOD: u64 = 30;
const TOTP_DIGITS: u32 = 6;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Clone, Debug, FromRow)]
struct MfaActor {
    wireguard_pubkey: String,
    totp_secret: Vec<u8>,
}

#[derive(Serialize)]
struct StartRequest<'a> {
    location_id: i64,
    pubkey: &'a str,
    method: &'static str,
}

#[derive(Deserialize)]
struct StartResponse {
    token: String,
}

#[derive(Serialize)]
struct FinishRequest<'a> {
    token: &'a str,
    code: &'a str,
}

#[derive(Deserialize)]
struct FinishResponse {
    preshared_key: String,
}

#[derive(Debug)]
struct MfaResult {
    duration: Duration,
    result: Result<(), MfaError>,
}

#[derive(Debug)]
enum MfaError {
    Http {
        phase: &'static str,
        status: StatusCode,
    },
    Transport {
        phase: &'static str,
        error: reqwest::Error,
    },
    Response {
        phase: &'static str,
        error: String,
    },
}

#[derive(Default)]
struct Metrics {
    successful: u64,
    http_errors: u64,
    transport_errors: u64,
    response_errors: u64,
    panicked: u64,
    latencies: Vec<Duration>,
}

pub async fn run(args: ClientMfaArgs) -> anyhow::Result<()> {
    let actors = load_actors(&args.database, args.location_id).await?;
    if actors.is_empty() {
        anyhow::bail!("no seeded MFA actors found");
    }

    let client = Client::builder().timeout(REQUEST_TIMEOUT).build()?;
    let base_url = args.proxy_url.trim_end_matches('/').to_owned();
    let location_id = args.location_id;
    let mut metrics = Metrics::default();
    let stats = run_load_loop(
        actors,
        LoadLoopConfig {
            requests_per_second: args.requests_per_second,
            duration: args.duration,
            max_in_flight: args.max_in_flight,
        },
        false,
        move |actor| execute(client.clone(), base_url.clone(), location_id, actor),
        |result, _| handle_result(result, &mut metrics),
    )
    .await?;

    tracing::info!(
        scheduled = stats.scheduled,
        completed = stats.completed,
        successful = metrics.successful,
        http_errors = metrics.http_errors,
        transport_errors = metrics.transport_errors,
        response_errors = metrics.response_errors,
        panicked_tasks = metrics.panicked,
        dropped = stats.dropped,
        busy_actor_skips = stats.unavailable,
        peak_in_flight = stats.peak_in_flight,
        "client MFA load test complete"
    );
    Ok(())
}

async fn execute(client: Client, base_url: String, location_id: i64, actor: MfaActor) -> MfaResult {
    let started = Instant::now();
    let start = client
        .post(format!("{base_url}{START_PATH}"))
        .headers(client_headers())
        .json(&StartRequest {
            location_id,
            pubkey: &actor.wireguard_pubkey,
            method: "Totp",
        })
        .send()
        .await;
    let response = match start {
        Ok(response) if response.status().is_success() => response,
        Ok(response) => {
            return failure(
                started,
                MfaError::Http {
                    phase: "start",
                    status: response.status(),
                },
            );
        }
        Err(error) if error.is_timeout() => {
            return failure(
                started,
                MfaError::Transport {
                    phase: "start",
                    error,
                },
            );
        }
        Err(error) => {
            return failure(
                started,
                MfaError::Transport {
                    phase: "start",
                    error,
                },
            );
        }
    };
    let start = match response.json::<StartResponse>().await {
        Ok(response) if !response.token.is_empty() => response,
        Ok(_) => {
            return failure(
                started,
                MfaError::Response {
                    phase: "start",
                    error: "empty MFA token".into(),
                },
            );
        }
        Err(error) => {
            return failure(
                started,
                MfaError::Response {
                    phase: "start",
                    error: error.to_string(),
                },
            );
        }
    };

    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let code = totp_custom::<Sha1>(TOTP_PERIOD, TOTP_DIGITS, &actor.totp_secret, timestamp);
    let finish = client
        .post(format!("{base_url}{FINISH_PATH}"))
        .headers(client_headers())
        .json(&FinishRequest {
            token: &start.token,
            code: &code,
        })
        .send()
        .await;
    let response = match finish {
        Ok(response) if response.status().is_success() => response,
        Ok(response) => {
            return failure(
                started,
                MfaError::Http {
                    phase: "finish",
                    status: response.status(),
                },
            );
        }
        Err(error) => {
            return failure(
                started,
                MfaError::Transport {
                    phase: "finish",
                    error,
                },
            );
        }
    };
    match response.json::<FinishResponse>().await {
        Ok(response) if !response.preshared_key.is_empty() => MfaResult {
            duration: started.elapsed(),
            result: Ok(()),
        },
        Ok(_) => failure(
            started,
            MfaError::Response {
                phase: "finish",
                error: "empty preshared key".into(),
            },
        ),
        Err(error) => failure(
            started,
            MfaError::Response {
                phase: "finish",
                error: error.to_string(),
            },
        ),
    }
}

fn failure(started: Instant, error: MfaError) -> MfaResult {
    MfaResult {
        duration: started.elapsed(),
        result: Err(error),
    }
}

fn client_headers() -> reqwest::header::HeaderMap {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("defguard-client-version", "2.1.0".parse().unwrap());
    headers.insert("defguard-client-platform", "linux".parse().unwrap());
    headers.insert(
        "user-agent",
        "defguard-load-generator/0.1.0".parse().unwrap(),
    );
    headers
}

fn handle_result(result: Result<(usize, MfaResult), JoinError>, metrics: &mut Metrics) {
    match result {
        Ok((_, result)) => {
            metrics.latencies.push(result.duration);
            match result.result {
                Ok(()) => metrics.successful += 1,
                Err(MfaError::Http { phase, status }) => {
                    metrics.http_errors += 1;
                    tracing::warn!(phase, %status, "MFA request returned HTTP error");
                }
                Err(MfaError::Transport { phase, error }) => {
                    metrics.transport_errors += 1;
                    tracing::warn!(phase, ?error, "MFA request failed at transport level");
                }
                Err(MfaError::Response { phase, error }) => {
                    metrics.response_errors += 1;
                    tracing::warn!(phase, %error, "MFA response was invalid");
                }
            }
        }
        Err(error) => {
            metrics.panicked += 1;
            tracing::error!(%error, "MFA task failed");
        }
    }
}

async fn load_actors(database: &DatabaseArgs, location_id: i64) -> anyhow::Result<Vec<MfaActor>> {
    let options = PgConnectOptions::new()
        .host(&database.database_host)
        .port(database.database_port)
        .database(&database.database_name)
        .username(&database.database_user)
        .password(database.database_password.expose_secret());
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await?;
    query_as("SELECT d.wireguard_pubkey, u.totp_secret FROM device d JOIN \"user\" u ON u.id = d.user_id JOIN wireguard_network_device wnd ON wnd.device_id = d.id WHERE wnd.wireguard_network_id = $1 AND u.username LIKE $2 AND u.totp_secret IS NOT NULL ORDER BY d.id")
        .bind(location_id).bind("load-test-user-%").fetch_all(&pool).await
        .context("failed to load MFA actors")
}
