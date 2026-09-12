use std::{
    num::NonZeroU64,
    time::{Duration, Instant},
};

use anyhow::bail;
use reqwest::{Client, StatusCode};
use secrecy::ExposeSecret;
use sqlx::{
    FromRow,
    postgres::{PgConnectOptions, PgPoolOptions},
    query_as,
};
use tokio::task::JoinError;

use crate::{
    config::{ConfigPollingArgs, DatabaseArgs},
    runner::LoadLoopConfig,
};

const POLLING_PATH: &str = "/api/v1/poll";
const CLIENT_VERSION: &str = "2.1.0";
const CLIENT_PLATFORM: &str = "linux";
const USER_AGENT: &str = "defguard-load-generator/0.1.0";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Clone, Debug, FromRow)]
struct PollingActor {
    polling_token: String,
}

#[derive(Debug)]
enum RequestError {
    Timeout,
    Transport(reqwest::Error),
    Http(StatusCode),
}

#[derive(Debug)]
struct RequestResult {
    duration: Duration,
    result: Result<StatusCode, RequestError>,
}

#[derive(Default)]
struct LoadTestMetrics {
    scheduled_requests: u64,
    started_requests: u64,
    completed_requests: u64,
    successful_requests: u64,
    http_errors: u64,
    timeout_errors: u64,
    transport_errors: u64,
    panicked_tasks: u64,
    dropped_requests: u64,
    peak_in_flight: usize,
    latencies: Vec<Duration>,
}

struct SharedLoadTestState {
    http_client: Client,
    polling_url: String,
    actors: Vec<PollingActor>,
    started_at: Instant,
    duration: Option<Duration>,
    max_in_flight: usize,
    requests_per_second: NonZeroU64,
}

pub struct ConfigPollingLoadTest {
    proxy_url: String,
    network_id: i64,
    database: DatabaseArgs,
    requests_per_second: NonZeroU64,
    duration: Option<Duration>,
    max_in_flight: usize,
}

impl ConfigPollingLoadTest {
    #[must_use]
    pub fn new(args: ConfigPollingArgs) -> Self {
        Self {
            proxy_url: args.proxy_url,
            network_id: args.network_id,
            database: args.database,
            requests_per_second: args.requests_per_second,
            duration: args.duration,
            max_in_flight: args.max_in_flight,
        }
    }

    pub async fn run(self) -> anyhow::Result<()> {
        tracing::info!("Starting load-testing run");
        let state = self.initialize_state().await?;
        tracing::info!(
            polling_url = %state.polling_url,
            actors = state.actors.len(),
            requests_per_second = self.requests_per_second.get(),
            max_in_flight = self.max_in_flight,
            duration = ?self.duration,
            "starting config-polling load test"
        );

        run_load_loop(state).await
    }

    async fn initialize_state(&self) -> anyhow::Result<SharedLoadTestState> {
        if self.max_in_flight == 0 {
            bail!("max-in-flight must be greater than zero");
        }

        let actors = load_actors(&self.database, self.network_id).await?;
        if actors.is_empty() {
            bail!("actors file contains no actors");
        }

        Ok(SharedLoadTestState {
            http_client: Client::builder().timeout(REQUEST_TIMEOUT).build()?,
            polling_url: format!("{}{}", self.proxy_url.trim_end_matches('/'), POLLING_PATH),
            actors,
            started_at: Instant::now(),
            duration: self.duration,
            max_in_flight: self.max_in_flight,
            requests_per_second: self.requests_per_second,
        })
    }
}

async fn execute_polling_request(
    client: Client,
    polling_url: String,
    polling_token: String,
) -> RequestResult {
    let started_at = Instant::now();
    let result = match client
        .post(polling_url)
        .header("defguard-client-version", CLIENT_VERSION)
        .header("defguard-client-platform", CLIENT_PLATFORM)
        .header("user-agent", USER_AGENT)
        .json(&serde_json::json!({ "token": polling_token }))
        .send()
        .await
    {
        Ok(response) => {
            let status = response.status();
            if !status.is_success() {
                Err(RequestError::Http(status))
            } else {
                match response.bytes().await {
                    Ok(_) => Ok(status),
                    Err(error) if error.is_timeout() => Err(RequestError::Timeout),
                    Err(error) => Err(RequestError::Transport(error)),
                }
            }
        }
        Err(error) if error.is_timeout() => Err(RequestError::Timeout),
        Err(error) => Err(RequestError::Transport(error)),
    };

    RequestResult {
        duration: started_at.elapsed(),
        result,
    }
}

async fn load_actors(
    database: &DatabaseArgs,
    network_id: i64,
) -> anyhow::Result<Vec<PollingActor>> {
    tracing::debug!("Loading devices from DB");
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

    let result = query_as(
        "SELECT p.token AS polling_token \
         FROM pollingtoken p \
         JOIN device d ON d.id = p.device_id \
         JOIN \"user\" u ON u.id = d.user_id \
         JOIN wireguard_network_device wnd ON wnd.device_id = d.id \
         WHERE wnd.wireguard_network_id = $1 AND u.username LIKE $2 \
         ORDER BY d.id, p.created_at DESC",
    )
    .bind(network_id)
    .bind("load-test-user-%")
    .fetch_all(&pool)
    .await?;
    tracing::info!("Loaded devices from DB");
    Ok(result)
}

async fn run_load_loop(state: SharedLoadTestState) -> anyhow::Result<()> {
    let mut metrics = LoadTestMetrics::default();
    let client = state.http_client.clone();
    let polling_url = state.polling_url.clone();
    let stats = crate::runner::run_load_loop(
        state.actors.clone(),
        LoadLoopConfig {
            requests_per_second: state.requests_per_second,
            duration: state.duration,
            max_in_flight: state.max_in_flight,
        },
        true,
        move |actor| {
            execute_polling_request(client.clone(), polling_url.clone(), actor.polling_token)
        },
        |result, _| handle_completed_task(result.map(|(_, request)| request), &mut metrics),
    )
    .await?;

    metrics.scheduled_requests = stats.scheduled;
    metrics.started_requests = stats.scheduled;
    metrics.dropped_requests = stats.dropped;
    metrics.peak_in_flight = stats.peak_in_flight;
    report_final_results(&metrics, &state);
    Ok(())
}

fn handle_completed_task(result: Result<RequestResult, JoinError>, metrics: &mut LoadTestMetrics) {
    metrics.completed_requests += 1;

    match result {
        Ok(request) => {
            metrics.latencies.push(request.duration);
            match request.result {
                Ok(status) if status.is_success() => metrics.successful_requests += 1,
                Ok(status) if !status.is_success() => {
                    metrics.http_errors += 1;
                    tracing::warn!(%status, "polling request returned an HTTP error");
                }
                Ok(_) => metrics.successful_requests += 1,
                Err(RequestError::Http(status)) => {
                    metrics.http_errors += 1;
                    tracing::warn!(%status, "polling request returned an HTTP error");
                }
                Err(RequestError::Timeout) => metrics.timeout_errors += 1,
                Err(RequestError::Transport(error)) => {
                    metrics.transport_errors += 1;
                    tracing::warn!(error = ?error, "polling request failed at transport level");
                }
            }
        }
        Err(error) => {
            metrics.panicked_tasks += 1;
            tracing::error!(%error, "polling request task failed");
        }
    }
}

fn report_final_results(metrics: &LoadTestMetrics, state: &SharedLoadTestState) {
    let elapsed = state.started_at.elapsed();
    let actual_rps = metrics.started_requests as f64 / elapsed.as_secs_f64();
    tracing::info!(
        elapsed = ?elapsed,
        target_rps = state.requests_per_second.get(),
        actual_rps,
        scheduled = metrics.scheduled_requests,
        started = metrics.started_requests,
        completed = metrics.completed_requests,
        successful = metrics.successful_requests,
        http_errors = metrics.http_errors,
        timeout_errors = metrics.timeout_errors,
        transport_errors = metrics.transport_errors,
        panicked_tasks = metrics.panicked_tasks,
        dropped = metrics.dropped_requests,
        peak_in_flight = metrics.peak_in_flight,
        "config-polling load test complete"
    );

    if metrics.latencies.is_empty() {
        tracing::info!("latency percentiles: n/a");
    } else {
        let mut latencies = metrics.latencies.clone();
        latencies.sort_unstable();
        tracing::info!(
            p50 = ?percentile(&latencies, 0.50),
            p95 = ?percentile(&latencies, 0.95),
            p99 = ?percentile(&latencies, 0.99),
            "latency percentiles"
        );
    }
}

fn percentile(values: &[Duration], percentile: f64) -> Duration {
    let index = ((values.len() - 1) as f64 * percentile).round() as usize;
    values[index]
}
