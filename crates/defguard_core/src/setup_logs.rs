use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use tracing::{Event as TracingEvent, Subscriber};
use tracing_subscriber::{Layer, layer::Context};

pub const MAX_CORE_LOG_LINES: usize = 200;

#[derive(Clone)]
pub struct CoreSetupLogLayer;

tokio::task_local! {
    static CORE_SETUP_LOGS: Arc<Mutex<VecDeque<String>>>;
}

pub async fn scope_setup_logs<F, T>(buffer: Arc<Mutex<VecDeque<String>>>, future: F) -> T
where
    F: std::future::Future<Output = T>,
{
    CORE_SETUP_LOGS.scope(buffer, future).await
}

impl<S> Layer<S> for CoreSetupLogLayer
where
    S: Subscriber,
{
    fn on_event(&self, event: &TracingEvent<'_>, _ctx: Context<'_, S>) {
        let Some(buffer) = CORE_SETUP_LOGS.try_with(Clone::clone).ok() else {
            return;
        };

        let mut visitor = MessageVisitor::default();
        event.record(&mut visitor);

        let metadata = event.metadata();
        let message = visitor.message.unwrap_or_default();
        let Ok(mut guard) = buffer.lock() else {
            return;
        };
        if guard.len() >= MAX_CORE_LOG_LINES {
            guard.pop_front();
        }
        guard.push_back(format!(
            "{} {}: {}",
            metadata.level(),
            metadata.target(),
            message
        ));
    }
}

#[derive(Default)]
struct MessageVisitor {
    message: Option<String>,
}

impl tracing::field::Visit for MessageVisitor {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.message = Some(value.to_owned());
        }
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = Some(format!("{value:?}"));
        }
    }
}
