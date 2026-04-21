use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};
#[cfg(test)]
use std::{
    path::PathBuf,
    sync::atomic::{AtomicBool, Ordering},
};

use defguard_common::{
    db::{ChangeNotification, Id, TriggerOperation, models::gateway::Gateway},
    messages::peer_stats_update::PeerStatsUpdate,
};
use defguard_core::grpc::GatewayEvent;
use defguard_proto::gateway::gateway_client::GatewayClient;
use defguard_version::client::ClientVersionInterceptor;
use sqlx::{PgPool, postgres::PgListener};
#[cfg(test)]
use tokio::sync::Notify;
use tokio::{
    sync::{broadcast::Sender, mpsc::UnboundedSender, watch::Receiver},
    task::{AbortHandle, JoinHandle, JoinSet},
    time::sleep,
};
use tonic::{Request, service::interceptor::InterceptedService, transport::Channel};

use crate::{error::GatewayError, handler::GatewayHandler};

#[macro_use]
extern crate tracing;

mod certs;
mod error;
mod handler;

#[cfg(test)]
mod tests;

const GATEWAY_TABLE_TRIGGER: &str = "gateway_change";
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

#[cfg(test)]
#[derive(Clone, Default)]
struct GatewayManagerTestSupport {
    socket_paths_by_url: Arc<Mutex<HashMap<String, PathBuf>>>,
    handler_spawn_attempts_by_gateway: Arc<Mutex<HashMap<Id, u64>>>,
    handler_spawn_attempt_notify: Arc<Notify>,
    handler_connection_attempts_by_gateway: Arc<Mutex<HashMap<Id, u64>>>,
    handler_connection_attempt_notify: Arc<Notify>,
    gateway_notifications_by_gateway: Arc<Mutex<HashMap<Id, u64>>>,
    gateway_notification_notify: Arc<Notify>,
    listener_ready: Arc<AtomicBool>,
    listener_ready_notify: Arc<Notify>,
    retry_delay_override: Arc<Mutex<Option<Duration>>>,
}

#[cfg(test)]
impl GatewayManagerTestSupport {
    fn register_gateway_url(&self, gateway_url: String, socket_path: PathBuf) {
        self.socket_paths_by_url
            .lock()
            .expect("Failed to lock GatewayManager test socket registry")
            .insert(gateway_url, socket_path);
    }

    fn socket_path_for(&self, gateway: &Gateway<Id>) -> Option<PathBuf> {
        self.socket_paths_by_url
            .lock()
            .expect("Failed to lock GatewayManager test socket registry")
            .get(&gateway.url())
            .cloned()
    }

    fn note_handler_spawn_attempt(&self, gateway_id: Id) {
        let mut handler_spawn_attempts = self
            .handler_spawn_attempts_by_gateway
            .lock()
            .expect("Failed to lock GatewayManager handler spawn attempts registry");
        *handler_spawn_attempts.entry(gateway_id).or_default() += 1;
        self.handler_spawn_attempt_notify.notify_waiters();
    }

    #[cfg(test)]
    fn handler_spawn_attempt_count(&self, gateway_id: Id) -> u64 {
        self.handler_spawn_attempts_by_gateway
            .lock()
            .expect("Failed to lock GatewayManager handler spawn attempts registry")
            .get(&gateway_id)
            .copied()
            .unwrap_or_default()
    }

    #[cfg(test)]
    async fn wait_for_handler_spawn_attempt_count(&self, gateway_id: Id, expected_count: u64) {
        loop {
            if self.handler_spawn_attempt_count(gateway_id) >= expected_count {
                return;
            }

            let notified = self.handler_spawn_attempt_notify.notified();
            if self.handler_spawn_attempt_count(gateway_id) >= expected_count {
                return;
            }

            notified.await;
        }
    }

    fn note_handler_connection_attempt(&self, gateway_id: Id) {
        let mut handler_connection_attempts = self
            .handler_connection_attempts_by_gateway
            .lock()
            .expect("Failed to lock GatewayManager handler connection attempts registry");
        *handler_connection_attempts.entry(gateway_id).or_default() += 1;
        self.handler_connection_attempt_notify.notify_waiters();
    }

    #[cfg(test)]
    fn handler_connection_attempt_count(&self, gateway_id: Id) -> u64 {
        self.handler_connection_attempts_by_gateway
            .lock()
            .expect("Failed to lock GatewayManager handler connection attempts registry")
            .get(&gateway_id)
            .copied()
            .unwrap_or_default()
    }

    #[cfg(test)]
    async fn wait_for_handler_connection_attempt_count(&self, gateway_id: Id, expected_count: u64) {
        loop {
            if self.handler_connection_attempt_count(gateway_id) >= expected_count {
                return;
            }

            let notified = self.handler_connection_attempt_notify.notified();
            if self.handler_connection_attempt_count(gateway_id) >= expected_count {
                return;
            }

            notified.await;
        }
    }

    fn note_gateway_notification(&self, gateway_id: Id) {
        let mut gateway_notifications = self
            .gateway_notifications_by_gateway
            .lock()
            .expect("Failed to lock GatewayManager gateway notification registry");
        *gateway_notifications.entry(gateway_id).or_default() += 1;
        self.gateway_notification_notify.notify_waiters();
    }

    #[cfg(test)]
    fn gateway_notification_count(&self, gateway_id: Id) -> u64 {
        self.gateway_notifications_by_gateway
            .lock()
            .expect("Failed to lock GatewayManager gateway notification registry")
            .get(&gateway_id)
            .copied()
            .unwrap_or_default()
    }

    #[cfg(test)]
    async fn wait_for_gateway_notification_count(&self, gateway_id: Id, expected_count: u64) {
        loop {
            if self.gateway_notification_count(gateway_id) >= expected_count {
                return;
            }

            let notified = self.gateway_notification_notify.notified();
            if self.gateway_notification_count(gateway_id) >= expected_count {
                return;
            }

            notified.await;
        }
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
}

pub struct GatewayManager {
    clients: Arc<Mutex<HashMap<Id, Client>>>,
    pool: PgPool,
    handlers: JoinSet<Result<(), GatewayError>>,
    #[cfg(test)]
    test_support: GatewayManagerTestSupport,
    tx: GatewayTxSet,
}

impl GatewayManager {
    #[cfg(not(test))]
    #[must_use]
    pub fn new(pool: PgPool, tx: GatewayTxSet) -> Self {
        Self {
            clients: Arc::default(),
            handlers: JoinSet::new(),
            pool,
            tx,
        }
    }

    #[cfg(test)]
    #[must_use]
    fn new(pool: PgPool, tx: GatewayTxSet, test_support: GatewayManagerTestSupport) -> Self {
        Self {
            clients: Arc::default(),
            handlers: JoinSet::new(),
            pool,
            test_support,
            tx,
        }
    }

    /// Records that the manager finished processing a pg_notify for the given gateway.
    /// Tests block on `wait_for_gateway_notification_count` to synchronize against the
    /// manager's async loop before asserting side effects.
    #[cfg(test)]
    fn note_gateway_notification_for_tests(&self, gateway_id: Id) {
        self.test_support.note_gateway_notification(gateway_id);
    }

    fn manager_reconnect_delay(&self) -> Duration {
        #[cfg(test)]
        {
            self.test_support.manager_reconnect_delay()
        }

        #[cfg(not(test))]
        {
            GATEWAY_RECONNECT_DELAY
        }
    }

    fn build_handler(
        &self,
        gateway: Gateway<Id>,
        certs_rx: Receiver<Arc<HashMap<Id, String>>>,
    ) -> Result<GatewayHandler, GatewayError> {
        #[cfg(test)]
        {
            self.test_support.note_handler_spawn_attempt(gateway.id);

            let mut gateway_handler =
                if let Some(socket_path) = self.test_support.socket_path_for(&gateway) {
                    GatewayHandler::new_with_test_socket(
                        gateway,
                        self.pool.clone(),
                        self.tx.events.clone(),
                        self.tx.peer_stats.clone(),
                        certs_rx,
                        socket_path,
                    )?
                } else {
                    GatewayHandler::new(
                        gateway,
                        self.pool.clone(),
                        self.tx.events.clone(),
                        self.tx.peer_stats.clone(),
                        certs_rx,
                    )?
                };
            gateway_handler.attach_test_support(self.test_support.clone());

            Ok(gateway_handler)
        }

        #[cfg(not(test))]
        GatewayHandler::new(
            gateway,
            self.pool.clone(),
            self.tx.events.clone(),
            self.tx.peer_stats.clone(),
            certs_rx,
        )
    }

    /// Bi-directional gRPC stream for communication with Defguard Gateway.
    pub async fn run(&mut self) -> Result<(), anyhow::Error> {
        let (certs_tx, certs_rx) = tokio::sync::watch::channel(Arc::new(HashMap::new()));
        certs::refresh_certs(&self.pool, &certs_tx).await;
        let refresh_pool = self.pool.clone();
        let _refresh_certs_task = AbortTaskOnDrop::new(tokio::spawn(async move {
            loop {
                certs::refresh_certs(&refresh_pool, &certs_tx).await;
                sleep(TEN_SECS).await;
            }
        }));
        // Stores the abort handle and a snapshot of the gateway at the time the handler was last
        // started. The snapshot is used by the Update arm to detect connection-relevant changes.
        let mut abort_handles: HashMap<Id, (AbortHandle, Gateway<Id>)> = HashMap::new();
        for gateway in Gateway::all(&self.pool).await? {
            if !gateway.enabled {
                debug!("Existing Gateway is disabled, so it won't be handled");
                continue;
            }

            let id = gateway.id;
            let snapshot = gateway.clone();
            let abort_handle =
                self.run_handler(gateway, Arc::clone(&self.clients), certs_rx.clone())?;
            abort_handles.insert(id, (abort_handle, snapshot));
        }

        // Observe gateway changes.
        let mut listener = PgListener::connect_with(&self.pool).await?;
        listener.listen(GATEWAY_TABLE_TRIGGER).await?;

        #[cfg(test)]
        self.test_support.mark_listener_ready();

        while let Ok(notification) = listener.recv().await {
            let payload = notification.payload();
            match serde_json::from_str::<ChangeNotification>(payload) {
                Ok(gateway_notification) => {
                    let gateway_id = gateway_notification.id;

                    match gateway_notification.operation {
                        TriggerOperation::Insert => {
                            let gateway = match Gateway::find_by_id(&self.pool, gateway_id).await {
                                Ok(Some(gateway)) => gateway,
                                Ok(None) => {
                                    warn!(
                                        "Received Insert notification for Gateway \
                                            id={gateway_id} but it was not found in the database"
                                    );
                                    #[cfg(test)]
                                    self.note_gateway_notification_for_tests(gateway_id);
                                    continue;
                                }
                                Err(err) => {
                                    error!("Failed to fetch Gateway id={gateway_id}: {err}");
                                    continue;
                                }
                            };

                            if gateway.enabled {
                                let snapshot = gateway.clone();
                                let abort_handle = self.run_handler(
                                    gateway,
                                    Arc::clone(&self.clients),
                                    certs_rx.clone(),
                                )?;
                                abort_handles.insert(gateway_id, (abort_handle, snapshot));
                            } else {
                                debug!(
                                    "New Gateway id={gateway_id} is disabled, so it won't be \
                                    handled"
                                );
                            }

                            #[cfg(test)]
                            self.note_gateway_notification_for_tests(gateway_id);
                        }
                        TriggerOperation::Update => {
                            let mut gateway =
                                match Gateway::find_by_id(&self.pool, gateway_id).await {
                                    Ok(Some(gateway)) => gateway,
                                    Ok(None) => {
                                        warn!(
                                            "Received Update notification for Gateway \
                                            id={gateway_id} but it was not found in the database"
                                        );
                                        #[cfg(test)]
                                        self.note_gateway_notification_for_tests(gateway_id);
                                        continue;
                                    }
                                    Err(err) => {
                                        error!("Failed to fetch Gateway id={gateway_id}: {err}");
                                        continue;
                                    }
                                };

                            // Only restart the handler when connection-relevant fields have actually changed
                            let should_restart = match abort_handles.get(&gateway_id) {
                                Some((_, snapshot)) => needs_restart(snapshot, &gateway),
                                // Gateway not currently handled - treat as needing a (re)start.
                                None => true,
                            };

                            if should_restart {
                                self.remove_client(gateway_id);
                                if let Some((abort_handle, _)) = abort_handles.remove(&gateway_id) {
                                    info!(
                                        "Aborting connection to Gateway id={gateway_id}, \
                                        connection-relevant fields have changed"
                                    );
                                    abort_handle.abort();
                                }

                                // Only mark disconnected if the gateway was actually connected
                                if gateway.is_connected() {
                                    if let Err(err) = gateway.touch_disconnected(&self.pool).await {
                                        error!(
                                            "Failed to update disconnection time for Gateway \
                                            id={gateway_id} after database change: {err}"
                                        );
                                    }
                                }

                                if gateway.enabled {
                                    let snapshot = gateway.clone();
                                    let abort_handle = self.run_handler(
                                        gateway,
                                        Arc::clone(&self.clients),
                                        certs_rx.clone(),
                                    )?;
                                    abort_handles.insert(gateway_id, (abort_handle, snapshot));
                                } else {
                                    debug!(
                                        "Updated Gateway id={gateway_id} is disabled, so it \
                                        won't be handled"
                                    );
                                }
                            } else {
                                // Non-connection-relevant update (e.g. version bump from handler
                                // save). Refresh the stored snapshot so future comparisons use
                                // up-to-date baseline values.
                                if let Some((_, snapshot)) = abort_handles.get_mut(&gateway_id) {
                                    *snapshot = gateway;
                                }
                            }

                            #[cfg(test)]
                            self.note_gateway_notification_for_tests(gateway_id);
                        }
                        TriggerOperation::Delete => {
                            // Send purge request to Gateway.
                            let maybe_client = self.remove_client(gateway_id);

                            if let Some(mut client) = maybe_client {
                                debug!("Sending purge request to Gateway id={gateway_id}");
                                if let Err(err) = client.purge(Request::new(())).await {
                                    error!(
                                        "Error sending purge request to Gateway id={gateway_id}: \
                                        {err}"
                                    );
                                } else {
                                    info!("Sent purge request to Gateway id={gateway_id}");
                                }
                            } else {
                                warn!(
                                    "Cannot find gRPC client for Gateway id={gateway_id}; \
                                    skipping purge request"
                                );
                            }

                            // Kill the `GatewayHandler` and the connection.
                            if let Some((abort_handle, _)) = abort_handles.remove(&gateway_id) {
                                info!(
                                    "Aborting connection to Gateway id={gateway_id}, it has \
                                    disappeared from the database"
                                );
                                abort_handle.abort();
                            } else {
                                warn!(
                                    "Cannot find Gateway id={gateway_id} on the list of \
                                    connected gateways"
                                );
                            }

                            #[cfg(test)]
                            self.note_gateway_notification_for_tests(gateway_id);
                        }
                    };
                }
                Err(err) => error!("Failed to de-serialize database notification object: {err}"),
            }
        }

        while let Some(Ok(_result)) = self.handlers.join_next().await {
            debug!("Gateway gRPC task has ended");
        }

        Ok(())
    }

    fn run_handler(
        &mut self,
        gateway: Gateway<Id>,
        clients: Arc<Mutex<HashMap<Id, Client>>>,
        certs_rx: Receiver<Arc<HashMap<Id, String>>>,
    ) -> Result<AbortHandle, GatewayError> {
        let mut gateway_handler = self.build_handler(gateway, certs_rx)?;
        let manager_reconnect_delay = self.manager_reconnect_delay();
        let abort_handle = self.handlers.spawn(async move {
            gateway_handler
                .handle_connection(clients, manager_reconnect_delay)
                .await
        });
        Ok(abort_handle)
    }

    fn remove_client(&self, gateway_id: Id) -> Option<Client> {
        self.clients
            .lock()
            .expect("Failed to lock GatewayManager::clients")
            .remove(&gateway_id)
    }
}

/// Returns true if the change from `old` to `new` requires the gateway handler to be
/// restarted - i.e. if any field that directly affects the gRPC connection or TLS identity
/// has changed.
///
/// Fields that do NOT trigger a restart (version, timestamps, audit fields, cert expiry)
/// are intentionally excluded so that the handler-internal `gateway.save()` call, which
/// bumps those fields, does not cause an infinite restart loop.
fn needs_restart(old: &Gateway<Id>, new: &Gateway<Id>) -> bool {
    old.address != new.address
        || old.port != new.port
        || old.enabled != new.enabled
        || old.core_client_cert_der != new.core_client_cert_der
        || old.core_client_cert_key_der != new.core_client_cert_key_der
}

#[cfg(test)]
mod unit_tests {
    use chrono::Utc;
    use defguard_common::db::{Id, models::gateway::Gateway};

    use super::needs_restart;

    fn base_gateway() -> Gateway<Id> {
        Gateway {
            id: 1,
            location_id: 1,
            name: "test".to_string(),
            address: "127.0.0.1".to_string(),
            port: 50051,
            connected_at: None,
            disconnected_at: None,
            certificate_serial: None,
            certificate_expiry: None,
            version: None,
            enabled: true,
            modified_at: Utc::now().naive_utc(),
            modified_by: "test".to_string(),
            core_client_cert_der: None,
            core_client_cert_key_der: None,
            core_client_cert_expiry: None,
        }
    }

    #[test]
    fn test_needs_restart_detects_connection_relevant_field_changes() {
        let base = base_gateway();

        // Identical gateways - no restart needed.
        assert!(!needs_restart(&base, &base.clone()));

        // Non-connection-relevant fields - no restart.
        let mut no_restart = base.clone();
        no_restart.version = Some("2.0.0".to_string());
        no_restart.modified_by = "someone-else".to_string();
        no_restart.connected_at = Some(Utc::now().naive_utc());
        no_restart.disconnected_at = Some(Utc::now().naive_utc());
        no_restart.certificate_serial = Some("abc".to_string());
        no_restart.core_client_cert_expiry = Some(Utc::now().naive_utc());
        assert!(!needs_restart(&base, &no_restart));

        // address change - restart required.
        let mut changed = base.clone();
        changed.address = "10.0.0.1".to_string();
        assert!(needs_restart(&base, &changed));

        // port change - restart required.
        let mut changed = base.clone();
        changed.port = 9999;
        assert!(needs_restart(&base, &changed));

        // enabled change - restart required.
        let mut changed = base.clone();
        changed.enabled = false;
        assert!(needs_restart(&base, &changed));

        // core_client_cert_der change - restart required.
        let mut changed = base.clone();
        changed.core_client_cert_der = Some(vec![1, 2, 3]);
        assert!(needs_restart(&base, &changed));

        // core_client_cert_key_der change - restart required.
        let mut changed = base.clone();
        changed.core_client_cert_key_der = Some(vec![4, 5, 6]);
        assert!(needs_restart(&base, &changed));
    }
}

/// Shared set of outbound channels that gateway instances use to forward
/// events, notifications, and side effects to Core components.
#[derive(Clone)]
pub struct GatewayTxSet {
    events: Sender<GatewayEvent>,
    peer_stats: UnboundedSender<PeerStatsUpdate>,
}

impl GatewayTxSet {
    #[must_use]
    pub const fn new(
        events: Sender<GatewayEvent>,
        peer_stats: UnboundedSender<PeerStatsUpdate>,
    ) -> Self {
        Self { events, peer_stats }
    }
}
