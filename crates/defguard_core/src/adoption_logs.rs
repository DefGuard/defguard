use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, LazyLock, Mutex},
    time::{Duration, Instant},
};

use tracing::{Event, Subscriber};
use tracing_subscriber::{
    Layer,
    layer::Context,
    registry::{LookupSpan, SpanRef},
};

const MAX_LOG_LINES_PER_ADOPTION: usize = 200;
const ADOPTION_LOG_TTL: Duration = Duration::from_secs(20 * 60);

#[derive(Clone, Default)]
pub struct AdoptionLogRegistry {
    inner: Arc<Mutex<HashMap<String, AdoptionLogState>>>,
}

#[derive(Default)]
struct AdoptionLogState {
    lines: VecDeque<String>,
    touched_at: Option<Instant>,
}

impl AdoptionLogRegistry {
    pub fn start(&self, adoption_id: &str) {
        self.with_map(|map| {
            self.cleanup_locked(map);
            map.entry(adoption_id.to_owned())
                .or_insert_with(AdoptionLogState::default)
                .touch();
        });
    }

    pub fn record(&self, adoption_id: &str, line: String) {
        self.with_map(|map| {
            self.cleanup_locked(map);
            let state = map
                .entry(adoption_id.to_owned())
                .or_insert_with(AdoptionLogState::default);
            state.push_line(line);
        });
    }

    pub fn take(&self, adoption_id: &str) -> Vec<String> {
        self.with_map(|map| {
            self.cleanup_locked(map);
            map.remove(adoption_id)
                .map(AdoptionLogState::into_vec)
                .unwrap_or_default()
        })
    }

    fn with_map<R>(&self, f: impl FnOnce(&mut HashMap<String, AdoptionLogState>) -> R) -> R {
        let mut guard = self.inner.lock().expect("adoption log mutex poisoned");
        f(&mut guard)
    }

    fn cleanup_locked(&self, map: &mut HashMap<String, AdoptionLogState>) {
        let now = Instant::now();
        map.retain(|_, state| state.is_fresh(now));
    }
}

impl AdoptionLogState {
    fn touch(&mut self) {
        self.touched_at = Some(Instant::now());
    }

    fn push_line(&mut self, line: String) {
        self.touch();
        self.lines.push_back(line);
        if self.lines.len() > MAX_LOG_LINES_PER_ADOPTION {
            self.lines.pop_front();
        }
    }

    fn is_fresh(&self, now: Instant) -> bool {
        self.touched_at
            .map(|touched_at| now.duration_since(touched_at) <= ADOPTION_LOG_TTL)
            .unwrap_or(true)
    }

    fn into_vec(self) -> Vec<String> {
        self.lines.into_iter().collect()
    }
}

#[derive(Clone)]
pub struct CoreAdoptionLogLayer {
    registry: AdoptionLogRegistry,
}

impl CoreAdoptionLogLayer {
    #[must_use]
    pub fn new(registry: AdoptionLogRegistry) -> Self {
        Self { registry }
    }
}

#[derive(Clone)]
struct AdoptionIdExtension(String);

impl<S> Layer<S> for CoreAdoptionLogLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_new_span(
        &self,
        attrs: &tracing::span::Attributes<'_>,
        id: &tracing::span::Id,
        ctx: Context<'_, S>,
    ) {
        let mut visitor = FieldVisitor::default();
        attrs.record(&mut visitor);

        let Some(adoption_id) = visitor.adoption_id else {
            return;
        };

        if let Some(span) = ctx.span(id) {
            span.extensions_mut()
                .insert(AdoptionIdExtension(adoption_id));
        }
    }

    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        let mut visitor = FieldVisitor::default();
        event.record(&mut visitor);

        let adoption_id = visitor
            .adoption_id
            .or_else(|| find_adoption_id_in_scope(&ctx, event));

        let Some(adoption_id) = adoption_id else {
            return;
        };

        let metadata = event.metadata();
        let message = visitor.message.unwrap_or_default();
        let line = format!("{} {}: {}", metadata.level(), metadata.target(), message);
        self.registry.record(&adoption_id, line);
    }
}

fn find_adoption_id_in_scope<S>(ctx: &Context<'_, S>, event: &Event<'_>) -> Option<String>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    let scope = ctx.event_scope(event)?;
    scope.from_root().filter_map(adoption_id_from_span).last()
}

fn adoption_id_from_span<S>(span: SpanRef<'_, S>) -> Option<String>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    span.extensions()
        .get::<AdoptionIdExtension>()
        .map(|id| id.0.clone())
}

#[derive(Default)]
struct FieldVisitor {
    adoption_id: Option<String>,
    message: Option<String>,
}

impl tracing::field::Visit for FieldVisitor {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        match field.name() {
            "adoption_id" => {
                self.adoption_id = Some(value.to_owned());
            }
            "message" => {
                self.message = Some(value.to_owned());
            }
            _ => {}
        }
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        let value = format!("{value:?}");
        self.record_str(field, &value);
    }
}

static REGISTRY: LazyLock<AdoptionLogRegistry> = LazyLock::new(AdoptionLogRegistry::default);

pub fn start_adoption(adoption_id: &str) {
    REGISTRY.start(adoption_id);
}

#[must_use]
pub fn take_logs(adoption_id: &str) -> Vec<String> {
    REGISTRY.take(adoption_id)
}

#[must_use]
pub fn core_adoption_log_layer() -> CoreAdoptionLogLayer {
    CoreAdoptionLogLayer::new(REGISTRY.clone())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use tracing::info;
    use tracing_subscriber::{layer::SubscriberExt, registry::Registry};

    use super::{AdoptionLogRegistry, CoreAdoptionLogLayer};

    #[test]
    fn captures_only_events_inside_adoption_span() {
        let registry = AdoptionLogRegistry::default();
        let layer = CoreAdoptionLogLayer::new(registry.clone());
        let subscriber = Registry::default().with(layer);
        let dispatch = tracing::Dispatch::new(subscriber);

        tracing::dispatcher::with_default(&dispatch, || {
            let span = tracing::info_span!("proxy_adoption", adoption_id = "adoption-1");
            let _entered = span.enter();
            info!("captured");
            drop(_entered);

            info!("not captured");
        });

        let logs = registry.take("adoption-1");
        assert_eq!(logs.len(), 1);
        assert!(logs[0].contains("captured"));
    }

    #[test]
    fn separates_logs_for_concurrent_adoptions() {
        let registry = AdoptionLogRegistry::default();
        let layer = CoreAdoptionLogLayer::new(registry.clone());
        let subscriber = Registry::default().with(layer);
        let dispatch = tracing::Dispatch::new(subscriber);
        let dispatch = Arc::new(dispatch);

        let mut handles = Vec::new();
        for (adoption_id, message) in [("a-1", "one"), ("a-2", "two")] {
            let dispatch = Arc::clone(&dispatch);
            handles.push(std::thread::spawn(move || {
                tracing::dispatcher::with_default(&dispatch, || {
                    let span = tracing::info_span!("proxy_adoption", adoption_id = adoption_id);
                    let _entered = span.enter();
                    info!("{message}");
                });
            }));
        }

        for handle in handles {
            handle.join().expect("log thread should finish cleanly");
        }

        let logs_a1 = registry.take("a-1");
        let logs_a2 = registry.take("a-2");

        assert_eq!(logs_a1.len(), 1);
        assert_eq!(logs_a2.len(), 1);
        assert!(logs_a1[0].contains("one"));
        assert!(logs_a2[0].contains("two"));
    }
}
