use std::{
    future::Future,
    num::NonZeroU64,
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::bail;
use tokio::{
    task::{JoinError, JoinSet},
    time::MissedTickBehavior,
};

const PROGRESS_INTERVAL: Duration = Duration::from_secs(5);

pub(crate) struct LoadLoopConfig {
    pub requests_per_second: NonZeroU64,
    pub duration: Option<Duration>,
    pub max_in_flight: usize,
}

#[derive(Default)]
pub(crate) struct LoadLoopStats {
    pub scheduled: u64,
    pub completed: u64,
    pub dropped: u64,
    pub unavailable: u64,
    pub peak_in_flight: usize,
}

struct ActorSelector {
    next: usize,
    busy: Vec<bool>,
    allow_overlap: bool,
}

impl ActorSelector {
    fn new(actor_count: usize, allow_overlap: bool) -> Self {
        Self {
            next: 0,
            busy: vec![false; actor_count],
            allow_overlap,
        }
    }

    fn select<A: Clone>(&mut self, actors: &[A]) -> Option<(usize, A)> {
        if actors.is_empty() {
            return None;
        }

        for offset in 0..actors.len() {
            let index = (self.next + offset) % actors.len();
            if self.allow_overlap || !self.busy[index] {
                self.next = (index + 1) % actors.len();
                self.busy[index] = true;
                return Some((index, actors[index].clone()));
            }
        }
        None
    }

    fn release(&mut self, index: usize) {
        if !self.allow_overlap {
            self.busy[index] = false;
        }
    }
}

pub(crate) async fn run_load_loop<A, O, F, Fut, C>(
    actors: Vec<A>,
    config: LoadLoopConfig,
    allow_actor_overlap: bool,
    execute: F,
    mut on_completed: C,
) -> anyhow::Result<LoadLoopStats>
where
    A: Clone + Send + Sync + 'static,
    O: Send + 'static,
    F: Fn(A) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = O> + Send + 'static,
    C: FnMut(Result<(usize, O), JoinError>, &mut LoadLoopStats),
{
    if actors.is_empty() {
        bail!("load test has no actors");
    }
    if config.max_in_flight == 0 {
        bail!("max-in-flight must be greater than zero");
    }

    let mut request_interval = tokio::time::interval(Duration::from_secs_f64(
        1.0 / config.requests_per_second.get() as f64,
    ));
    request_interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
    let mut progress_interval = tokio::time::interval_at(
        tokio::time::Instant::now() + PROGRESS_INTERVAL,
        PROGRESS_INTERVAL,
    );
    progress_interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

    let started_at = Instant::now();
    let duration = config.duration;
    let shutdown_timer = async move {
        match duration {
            Some(duration) => tokio::time::sleep(duration).await,
            None => std::future::pending().await,
        }
    };
    tokio::pin!(shutdown_timer);
    let first_ctrl_c = tokio::signal::ctrl_c();
    tokio::pin!(first_ctrl_c);

    let execute = Arc::new(execute);
    let mut selector = ActorSelector::new(actors.len(), allow_actor_overlap);
    let mut tasks = JoinSet::<(usize, O)>::new();
    let mut stats = LoadLoopStats::default();

    loop {
        tokio::select! {
            _ = request_interval.tick() => {
                if tasks.len() >= config.max_in_flight {
                    stats.dropped += 1;
                    continue;
                }
                let Some((index, actor)) = selector.select(&actors) else {
                    stats.unavailable += 1;
                    continue;
                };
                stats.scheduled += 1;
                let execute = Arc::clone(&execute);
                tasks.spawn(async move { (index, execute(actor).await) });
                stats.peak_in_flight = stats.peak_in_flight.max(tasks.len());
            }
            Some(result) = tasks.join_next() => {
                stats.completed += 1;
                if let Ok((index, _)) = &result {
                    selector.release(*index);
                }
                on_completed(result, &mut stats);
            }
            _ = progress_interval.tick() => {
                let elapsed = started_at.elapsed();
                tracing::info!(
                    elapsed = ?elapsed,
                    scheduled = stats.scheduled,
                    completed = stats.completed,
                    in_flight = tasks.len(),
                    dropped = stats.dropped,
                    unavailable = stats.unavailable,
                    "load test progress"
                );
            }
            _ = &mut first_ctrl_c => break,
            _ = &mut shutdown_timer => break,
        }
    }

    while !tasks.is_empty() {
        if let Some(result) = tasks.join_next().await {
            stats.completed += 1;
            if let Ok((index, _)) = &result {
                selector.release(*index);
            }
            on_completed(result, &mut stats);
        }
    }

    Ok(stats)
}
