use std::{
    fs::File,
    io::{BufRead, BufReader},
    num::NonZeroU64,
    path::PathBuf,
    time::{Duration, Instant},
};

use anyhow::{Context, bail};
use reqwest::Client;
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
    user_id: i64,
    device_id: i64,
    polling_token: String,
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
        })
    }
}

async fn execute_polling_request(
    client: Client,
    polling_url: String,
    polling_token: String,
) -> anyhow::Result<()> {
    let response = client
        .post(polling_url)
        .header("defguard-client-version", CLIENT_VERSION)
        .header("defguard-client-platform", CLIENT_PLATFORM)
        .header("user-agent", USER_AGENT)
        .json(&serde_json::json!({ "token": polling_token }))
        .send()
        .await?;

    let status = response.status();
    response.error_for_status()?.bytes().await?;
    tracing::debug!(%status, "polling request completed");
    Ok(())
}

fn load_actors(path: &PathBuf) -> anyhow::Result<Vec<PollingActor>> {
    let file = File::open(path)
        .with_context(|| format!("failed to open actors file {}", path.display()))?;
    BufReader::new(file)
        .lines()
        .enumerate()
        .map(|(line_number, line)| {
            let line = line?;
            serde_json::from_str(&line).with_context(|| {
                format!(
                    "invalid actor JSON at {}:{}",
                    path.display(),
                    line_number + 1
                )
            })
        })
        .collect()
}

async fn run_load_loop(mut state: SharedLoadTestState) -> anyhow::Result<()> {
    let mut request_tasks = JoinSet::new();
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
                    tracing::warn!(
                        max_in_flight = state.max_in_flight,
                        "maximum number of in-flight requests reached; skipping request"
                    );
                    continue;
                }

                let actor = &state.actors[state.next_actor_index];
                state.next_actor_index = (state.next_actor_index + 1) % state.actors.len();

                tracing::debug!(
                    user_id = actor.user_id,
                    device_id = actor.device_id,
                    elapsed = ?state.started_at.elapsed(),
                    "scheduling polling request"
                );

                let client = state.http_client.clone();
                let polling_url = state.polling_url.clone();
                let polling_token = actor.polling_token.clone();
                request_tasks.spawn(async move {
                    execute_polling_request(client, polling_url, polling_token).await
                });
            }
            Some(result) = request_tasks.join_next() => {
                handle_completed_task(result);
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

    drain_request_tasks(&mut request_tasks).await;
    report_final_results();
    Ok(())
}

async fn drain_request_tasks(request_tasks: &mut JoinSet<anyhow::Result<()>>) {
    let second_ctrl_c = tokio::signal::ctrl_c();
    tokio::pin!(second_ctrl_c);

    while !request_tasks.is_empty() {
        tokio::select! {
            Some(result) = request_tasks.join_next() => handle_completed_task(result),
            _ = &mut second_ctrl_c => {
                tracing::warn!("second Ctrl-C received; exiting immediately");
                std::process::exit(130);
            }
        }
    }
}

fn handle_completed_task(result: Result<anyhow::Result<()>, JoinError>) {
    // TODO: update metrics and classify successful responses, HTTP errors, timeouts, and panics.
    let _ = result;
}

fn validate_actors(actors: &[PollingActor]) -> anyhow::Result<()> {
    // TODO: validate actor fields and report the source JSONL line for malformed input.
    let _ = actors;
    Ok(())
}

fn report_final_results() {
    // TODO: print the final load-test report and latency statistics.
}
