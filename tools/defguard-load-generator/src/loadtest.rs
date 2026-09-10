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
use tokio::time::{Interval, MissedTickBehavior};

use crate::config::ConfigPollingArgs;

const POLLING_PATH: &str = "/api/v1/poll";

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
}

pub struct ConfigPollingLoadTest {
    proxy_url: String,
    actors_file: PathBuf,
    requests_per_second: NonZeroU64,
}

impl ConfigPollingLoadTest {
    #[must_use]
    pub fn new(args: ConfigPollingArgs) -> Self {
        Self {
            proxy_url: args.proxy_url,
            actors_file: args.devices_file,
            requests_per_second: args.requests_per_second,
        }
    }

    pub async fn run(self) -> anyhow::Result<()> {
        let state = self.initialize_state()?;
        tracing::info!(
            polling_url = %state.polling_url,
            actors = state.actors.len(),
            requests_per_second = self.requests_per_second.get(),
            "starting config-polling load test"
        );

        run_load_loop(state).await
    }

    fn initialize_state(&self) -> anyhow::Result<SharedLoadTestState> {
        let actors = load_actors(&self.actors_file)?;
        if actors.is_empty() {
            bail!("actors file contains no actors");
        }

        let mut request_interval = tokio::time::interval(Duration::from_secs_f64(
            1.0 / self.requests_per_second.get() as f64,
        ));
        request_interval.set_missed_tick_behavior(MissedTickBehavior::Burst);

        Ok(SharedLoadTestState {
            http_client: Client::new(),
            polling_url: format!("{}{}", self.proxy_url.trim_end_matches('/'), POLLING_PATH),
            actors,
            request_interval,
            next_actor_index: 0,
            started_at: Instant::now(),
        })
    }
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
    loop {
        state.request_interval.tick().await;

        let actor = &state.actors[state.next_actor_index];
        state.next_actor_index = (state.next_actor_index + 1) % state.actors.len();

        tracing::debug!(
            user_id = actor.user_id,
            device_id = actor.device_id,
            elapsed = ?state.started_at.elapsed(),
            "scheduling polling request"
        );

        // TODO: spawn the request task and update the shared result metrics.
        let _ = (&state.http_client, &state.polling_url, &actor.polling_token);
        todo!();
    }
}
