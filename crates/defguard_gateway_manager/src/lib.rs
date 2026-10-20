use std::time::Duration;
#[cfg(test)]
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

#[cfg(test)]
use defguard_common::db::{Id, models::gateway::Gateway};
use defguard_common::messages::peer_stats_update::PeerStatsUpdate;
use defguard_core::events::GatewayConnectionEvent;
use defguard_proto::gateway::gateway_client::GatewayClient;
use defguard_version::client::ClientVersionInterceptor;
#[cfg(test)]
use tokio::sync::Notify;
use tokio::{
    sync::mpsc::{UnboundedSender, unbounded_channel},
    task::JoinHandle,
};
use tonic::{service::interceptor::InterceptedService, transport::Channel};

#[macro_use]
extern crate tracing;

mod certs;
mod error;
mod handler;
pub mod manager;
mod updates;

#[cfg(test)]
mod tests;

const GATEWAY_RECONNECT_DELAY: Duration = Duration::from_secs(5);
const TEN_SECS: Duration = Duration::from_secs(10);

type Client = GatewayClient<InterceptedService<Channel, ClientVersionInterceptor>>;

struct AbortTaskOnDrop<T> {
    handle: Option<JoinHandle<T>>,
}

impl<T> AbortTaskOnDrop<T> {
    fn new(handle: JoinHandle<T>) -> Self {
        Self {
            handle: Some(handle),
        }
    }
}

impl<T> Drop for AbortTaskOnDrop<T> {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            handle.abort();
        }
    }
}

/// A per-Gateway counter that tests can increment, read, and block on. The map and its
/// `Notify` always travel together, so they are bundled into one type.
#[cfg(test)]
#[derive(Clone, Default)]
struct NotificationCounter {
    counts: Arc<Mutex<HashMap<Id, u64>>>,
    notify: Arc<Notify>,
}

#[cfg(test)]
impl NotificationCounter {
    fn note(&self, gateway_id: Id) {
        let mut counts = self
            .counts
            .lock()
            .expect("Failed to lock GatewayManager test notification counter");
        *counts.entry(gateway_id).or_default() += 1;
        self.notify.notify_waiters();
    }

    fn count(&self, gateway_id: Id) -> u64 {
        self.counts
            .lock()
            .expect("Failed to lock GatewayManager test notification counter")
            .get(&gateway_id)
            .copied()
            .unwrap_or_default()
    }

    async fn wait_for(&self, gateway_id: Id, expected_count: u64) {
        loop {
            if self.count(gateway_id) >= expected_count {
                return;
            }

            let notified = self.notify.notified();
            if self.count(gateway_id) >= expected_count {
                return;
            }

            notified.await;
        }
    }
}

#[cfg(test)]
#[derive(Clone, Default)]
struct GatewayManagerTestSupport {
    socket_paths_by_url: Arc<Mutex<HashMap<String, PathBuf>>>,
    handler_spawn_attempts: NotificationCounter,
    handler_connection_attempts: NotificationCounter,
    gateway_notifications: NotificationCounter,
    disconnect_notifications: NotificationCounter,
    reconnect_notifications: NotificationCounter,
    listener_ready: Arc<AtomicBool>,
    listener_ready_notify: Arc<Notify>,
    retry_delay_override: Arc<Mutex<Option<Duration>>>,
    disconnect_notification_delay_override: Arc<Mutex<Option<Duration>>>,
    event_senders: manager::EventSenders,
}

#[cfg(test)]
impl GatewayManagerTestSupport {
    fn register_gateway_url(&self, gateway_url: String, socket_path: PathBuf) {
        self.socket_paths_by_url
            .lock()
            .expect("Failed to lock GatewayManager test socket registry")
            .insert(gateway_url, socket_path);
    }

    fn has_event_sender(&self, gateway_id: Id) -> bool {
        self.event_senders
            .lock()
            .expect("Failed to lock GatewayManager::event_senders")
            .contains_key(&gateway_id)
    }

    fn socket_path_for(&self, gateway: &Gateway<Id>) -> Option<PathBuf> {
        self.socket_paths_by_url
            .lock()
            .expect("Failed to lock GatewayManager test socket registry")
            .get(&gateway.url())
            .cloned()
    }

    fn note_handler_spawn_attempt(&self, gateway_id: Id) {
        self.handler_spawn_attempts.note(gateway_id);
    }

    #[cfg(test)]
    fn handler_spawn_attempt_count(&self, gateway_id: Id) -> u64 {
        self.handler_spawn_attempts.count(gateway_id)
    }

    #[cfg(test)]
    async fn wait_for_handler_spawn_attempt_count(&self, gateway_id: Id, expected_count: u64) {
        self.handler_spawn_attempts
            .wait_for(gateway_id, expected_count)
            .await;
    }

    fn note_handler_connection_attempt(&self, gateway_id: Id) {
        self.handler_connection_attempts.note(gateway_id);
    }

    #[cfg(test)]
    fn handler_connection_attempt_count(&self, gateway_id: Id) -> u64 {
        self.handler_connection_attempts.count(gateway_id)
    }

    #[cfg(test)]
    async fn wait_for_handler_connection_attempt_count(&self, gateway_id: Id, expected_count: u64) {
        self.handler_connection_attempts
            .wait_for(gateway_id, expected_count)
            .await;
    }

    fn note_gateway_notification(&self, gateway_id: Id) {
        self.gateway_notifications.note(gateway_id);
    }

    #[cfg(test)]
    fn gateway_notification_count(&self, gateway_id: Id) -> u64 {
        self.gateway_notifications.count(gateway_id)
    }

    #[cfg(test)]
    async fn wait_for_gateway_notification_count(&self, gateway_id: Id, expected_count: u64) {
        self.gateway_notifications
            .wait_for(gateway_id, expected_count)
            .await;
    }

    /// Records that a Gateway disconnect email notification was actually sent. Tests use this
    /// instead of a mock SMTP server, because mail delivery itself is fire-and-forget.
    fn note_disconnect_notification_sent(&self, gateway_id: Id) {
        self.disconnect_notifications.note(gateway_id);
    }

    #[cfg(test)]
    fn disconnect_notification_count(&self, gateway_id: Id) -> u64 {
        self.disconnect_notifications.count(gateway_id)
    }

    #[cfg(test)]
    async fn wait_for_disconnect_notification_count(&self, gateway_id: Id, expected_count: u64) {
        self.disconnect_notifications
            .wait_for(gateway_id, expected_count)
            .await;
    }

    /// Records that a Gateway reconnect email notification was actually sent.
    fn note_reconnect_notification_sent(&self, gateway_id: Id) {
        self.reconnect_notifications.note(gateway_id);
    }

    #[cfg(test)]
    fn reconnect_notification_count(&self, gateway_id: Id) -> u64 {
        self.reconnect_notifications.count(gateway_id)
    }

    #[cfg(test)]
    async fn wait_for_reconnect_notification_count(&self, gateway_id: Id, expected_count: u64) {
        self.reconnect_notifications
            .wait_for(gateway_id, expected_count)
            .await;
    }

    fn mark_listener_ready(&self) {
        self.listener_ready.store(true, Ordering::Release);
        self.listener_ready_notify.notify_waiters();
    }

    #[cfg(test)]
    async fn wait_until_listener_ready(&self) {
        loop {
            if self.listener_ready.load(Ordering::Acquire) {
                return;
            }

            let notified = self.listener_ready_notify.notified();
            if self.listener_ready.load(Ordering::Acquire) {
                return;
            }

            notified.await;
        }
    }

    #[cfg(test)]
    fn set_retry_delay(&self, retry_delay: Duration) {
        *self
            .retry_delay_override
            .lock()
            .expect("Failed to lock GatewayManager retry delay override") = Some(retry_delay);
    }

    fn manager_reconnect_delay(&self) -> Duration {
        self.retry_delay_override
            .lock()
            .expect("Failed to lock GatewayManager retry delay override")
            .unwrap_or(GATEWAY_RECONNECT_DELAY)
    }

    fn handler_reconnect_delay(&self) -> Duration {
        self.retry_delay_override
            .lock()
            .expect("Failed to lock GatewayManager retry delay override")
            .unwrap_or(TEN_SECS)
    }

    /// Overrides the inactivity threshold delay so tests do not have to wait out whole minutes.
    #[cfg(test)]
    fn set_disconnect_notification_delay(&self, delay: Duration) {
        *self
            .disconnect_notification_delay_override
            .lock()
            .expect("Failed to lock GatewayManager disconnect notification delay override") =
            Some(delay);
    }

    fn disconnect_notification_delay(&self, configured_delay: Duration) -> Duration {
        self.disconnect_notification_delay_override
            .lock()
            .expect("Failed to lock GatewayManager disconnect notification delay override")
            .unwrap_or(configured_delay)
    }
}

/// Shared set of outbound channels that gateway instances use to forward events, notifications,
/// and side effects to Core components.
#[derive(Clone)]
pub struct GatewayTxSet {
    peer_stats: UnboundedSender<PeerStatsUpdate>,
    connection_events: UnboundedSender<GatewayConnectionEvent>,
}

impl GatewayTxSet {
    #[must_use]
    pub fn new(peer_stats: UnboundedSender<PeerStatsUpdate>) -> Self {
        let (connection_events, _) = unbounded_channel();
        Self {
            peer_stats,
            connection_events,
        }
    }

    #[must_use]
    pub fn with_connection_events(
        mut self,
        connection_events: UnboundedSender<GatewayConnectionEvent>,
    ) -> Self {
        self.connection_events = connection_events;
        self
    }
}
