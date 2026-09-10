use std::{
    collections::HashSet,
    fs::File,
    io::{BufRead, BufReader},
    num::NonZeroU64,
    path::PathBuf,
    time::{Duration, Instant},
};

use anyhow::{Context, bail};
use reqwest::{Client, StatusCode};
use serde::Deserialize;
use tokio::{
    task::{JoinError, JoinSet},
    time::{Interval, MissedTickBehavior},
};

use crate::config::ConfigPollingArgs;

const POLLING_PATH: &str = "/api/v1/poll";
const CLIENT_VERSION: &str = "2.1.0";
const CLIENT_PLATFORM: &str = "linux";
const USER_AGENT: &str = "defguard-load-generator/0.1.0";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Deserialize)]
struct PollingActor {
    #[serde(skip)]
    source_line: usize,
    user_id: i64,
    device_id: i64,
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
    request_interval: Interval,
    next_actor_index: usize,
    started_at: Instant,
    duration: Option<Duration>,
    max_in_flight: usize,
    requests_per_second: NonZeroU64,
}

pub struct ConfigPollingLoadTest {
    proxy_url: String,
    actors_file: PathBuf,
    requests_per_second: NonZeroU64,
    duration: Option<Duration>,
    max_in_flight: usize,
}

impl ConfigPollingLoadTest {
    #[must_use]
    pub fn new(args: ConfigPollingArgs) -> Self {
        Self {
            proxy_url: args.proxy_url,
            actors_file: args.devices_file,
            requests_per_second: args.requests_per_second,
            duration: args.duration,
            max_in_flight: args.max_in_flight,
        }
    }

    pub async fn run(self) -> anyhow::Result<()> {
        let state = self.initialize_state()?;
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

    fn initialize_state(&self) -> anyhow::Result<SharedLoadTestState> {
        if self.max_in_flight == 0 {
            bail!("max-in-flight must be greater than zero");
        }

        let actors = load_actors(&self.actors_file)?;
        validate_actors(&actors)?;
        if actors.is_empty() {
            bail!("actors file contains no actors");
        }

        let mut request_interval = tokio::time::interval(Duration::from_secs_f64(
            1.0 / self.requests_per_second.get() as f64,
        ));
        request_interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

        Ok(SharedLoadTestState {
            http_client: Client::builder().timeout(REQUEST_TIMEOUT).build()?,
            polling_url: format!("{}{}", self.proxy_url.trim_end_matches('/'), POLLING_PATH),
            actors,
            request_interval,
            next_actor_index: 0,
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

fn load_actors(path: &PathBuf) -> anyhow::Result<Vec<PollingActor>> {
    let file = File::open(path)
        .with_context(|| format!("failed to open actors file {}", path.display()))?;
    BufReader::new(file)
        .lines()
        .enumerate()
        .map(|(line_number, line)| {
            let line_number = line_number + 1;
            let line = line?;
            let mut actor: PollingActor = serde_json::from_str(&line).with_context(|| {
                format!("invalid actor JSON at {}:{line_number}", path.display())
            })?;
            actor.source_line = line_number;
            Ok(actor)
        })
        .collect()
}

async fn run_load_loop(mut state: SharedLoadTestState) -> anyhow::Result<()> {
    let mut request_tasks = JoinSet::new();
    let mut metrics = LoadTestMetrics::default();
    let duration = state.duration;
    let shutdown_timer = async move {
        match duration {
            Some(duration) => tokio::time::sleep(duration).await,
            None => std::future::pending().await,
        }
    };
    tokio::pin!(shutdown_timer);
    let first_ctrl_c = tokio::signal::ctrl_c();
    tokio::pin!(first_ctrl_c);

    loop {
        tokio::select! {
            _ = state.request_interval.tick() => {
                if request_tasks.len() >= state.max_in_flight {
                    metrics.dropped_requests += 1;
                    tracing::warn!(
                        max_in_flight = state.max_in_flight,
                        "maximum number of in-flight requests reached; skipping request"
                    );
                    continue;
                }

                let actor = &state.actors[state.next_actor_index];
                state.next_actor_index = (state.next_actor_index + 1) % state.actors.len();
                metrics.scheduled_requests += 1;
                metrics.started_requests += 1;

                let client = state.http_client.clone();
                let polling_url = state.polling_url.clone();
                let polling_token = actor.polling_token.clone();
                request_tasks.spawn(async move {
                    execute_polling_request(client, polling_url, polling_token).await
                });
                metrics.peak_in_flight = metrics.peak_in_flight.max(request_tasks.len());
            }
            Some(result) = request_tasks.join_next() => {
                handle_completed_task(result, &mut metrics);
            }
            _ = &mut first_ctrl_c => {
                tracing::info!("stopping request scheduling; waiting for in-flight requests");
                break;
            }
            _ = &mut shutdown_timer => {
                tracing::info!("test duration elapsed; waiting for in-flight requests");
                break;
            }
        }
    }

    drain_request_tasks(&mut request_tasks, &mut metrics).await;
    report_final_results(&metrics, &state);
    Ok(())
}

async fn drain_request_tasks(
    request_tasks: &mut JoinSet<RequestResult>,
    metrics: &mut LoadTestMetrics,
) {
    let second_ctrl_c = tokio::signal::ctrl_c();
    tokio::pin!(second_ctrl_c);

    while !request_tasks.is_empty() {
        tokio::select! {
            Some(result) = request_tasks.join_next() => handle_completed_task(result, metrics),
            _ = &mut second_ctrl_c => {
                tracing::warn!("second Ctrl-C received; exiting immediately");
                std::process::exit(130);
            }
        }
    }
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
                    tracing::warn!(%error, "polling request failed at transport level");
                }
            }
        }
        Err(error) => {
            metrics.panicked_tasks += 1;
            tracing::error!(%error, "polling request task failed");
        }
    }
}

fn validate_actors(actors: &[PollingActor]) -> anyhow::Result<()> {
    let mut user_ids = HashSet::with_capacity(actors.len());
    let mut device_ids = HashSet::with_capacity(actors.len());
    let mut polling_tokens = HashSet::with_capacity(actors.len());

    for actor in actors {
        if actor.user_id <= 0 {
            bail!("line {}: user_id must be positive", actor.source_line);
        }
        if actor.device_id <= 0 {
            bail!("line {}: device_id must be positive", actor.source_line);
        }
        if actor.polling_token.is_empty() {
            bail!(
                "line {}: polling_token must not be empty",
                actor.source_line
            );
        }
        if !user_ids.insert(actor.user_id) {
            bail!(
                "line {}: duplicate user_id {}",
                actor.source_line,
                actor.user_id
            );
        }
        if !device_ids.insert(actor.device_id) {
            bail!(
                "line {}: duplicate device_id {}",
                actor.source_line,
                actor.device_id
            );
        }
        if !polling_tokens.insert(&actor.polling_token) {
            bail!("line {}: duplicate polling_token", actor.source_line);
        }
    }

    Ok(())
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
