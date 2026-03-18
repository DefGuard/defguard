use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use defguard_common::{
    db::{ChangeNotification, Id, TriggerOperation, models::gateway::Gateway},
    messages::peer_stats_update::PeerStatsUpdate,
};
use defguard_core::grpc::GatewayEvent;
use defguard_proto::gateway::gateway_client::GatewayClient;
use defguard_version::client::ClientVersionInterceptor;
use sqlx::{PgPool, postgres::PgListener};
use tokio::{
    sync::{Notify, broadcast::Sender, mpsc::UnboundedSender, watch::Receiver},
    task::{AbortHandle, JoinSet},
};
use tonic::{Request, service::interceptor::InterceptedService, transport::Channel};

use crate::{error::GatewayError, handler::GatewayHandler};

#[macro_use]
extern crate tracing;

mod certs;
mod error;
mod handler;

#[doc(hidden)]
pub use handler::TestGatewayHandler;

const GATEWAY_TABLE_TRIGGER: &str = "gateway_change";
const GATEWAY_RECONNECT_DELAY: Duration = Duration::from_secs(5);
const TEN_SECS: Duration = Duration::from_secs(10);

type Client = GatewayClient<InterceptedService<Channel, ClientVersionInterceptor>>;

struct AbortTaskOnDrop<T> {
    handle: Option<tokio::task::JoinHandle<T>>,
}

impl<T> AbortTaskOnDrop<T> {
    fn new(handle: tokio::task::JoinHandle<T>) -> Self {
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

#[derive(Clone, Default)]
struct GatewayManagerTestSupport {
    socket_paths_by_url: Arc<Mutex<HashMap<String, PathBuf>>>,
    handler_spawn_attempts_by_gateway: Arc<Mutex<HashMap<Id, u64>>>,
    handler_spawn_attempt_notify: Arc<Notify>,
    gateway_notifications_by_gateway: Arc<Mutex<HashMap<Id, u64>>>,
    gateway_notification_notify: Arc<Notify>,
    listener_ready: Arc<AtomicBool>,
    listener_ready_notify: Arc<Notify>,
}

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

    fn handler_spawn_attempt_count(&self, gateway_id: Id) -> u64 {
        self.handler_spawn_attempts_by_gateway
            .lock()
            .expect("Failed to lock GatewayManager handler spawn attempts registry")
            .get(&gateway_id)
            .copied()
            .unwrap_or_default()
    }

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

    fn note_gateway_notification(&self, gateway_id: Id) {
        let mut gateway_notifications = self
            .gateway_notifications_by_gateway
            .lock()
            .expect("Failed to lock GatewayManager gateway notification registry");
        *gateway_notifications.entry(gateway_id).or_default() += 1;
        self.gateway_notification_notify.notify_waiters();
    }

    fn gateway_notification_count(&self, gateway_id: Id) -> u64 {
        self.gateway_notifications_by_gateway
            .lock()
            .expect("Failed to lock GatewayManager gateway notification registry")
            .get(&gateway_id)
            .copied()
            .unwrap_or_default()
    }

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
}

#[doc(hidden)]
#[derive(Clone, Default)]
pub struct TestGatewayManagerControl {
    inner: GatewayManagerTestSupport,
}

impl TestGatewayManagerControl {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_gateway_url(&self, gateway_url: String, socket_path: PathBuf) {
        self.inner.register_gateway_url(gateway_url, socket_path);
    }

    #[doc(hidden)]
    pub fn handler_spawn_attempt_count(&self, gateway_id: Id) -> u64 {
        self.inner.handler_spawn_attempt_count(gateway_id)
    }

    #[doc(hidden)]
    pub async fn wait_for_handler_spawn_attempt_count(&self, gateway_id: Id, expected_count: u64) {
        self.inner
            .wait_for_handler_spawn_attempt_count(gateway_id, expected_count)
            .await;
    }

    #[doc(hidden)]
    pub fn gateway_notification_count(&self, gateway_id: Id) -> u64 {
        self.inner.gateway_notification_count(gateway_id)
    }

    #[doc(hidden)]
    pub async fn wait_for_gateway_notification_count(&self, gateway_id: Id, expected_count: u64) {
        self.inner
            .wait_for_gateway_notification_count(gateway_id, expected_count)
            .await;
    }

    pub async fn wait_until_listener_ready(&self) {
        self.inner.wait_until_listener_ready().await;
    }
}

pub struct GatewayManager {
    clients: Arc<Mutex<HashMap<Id, Client>>>,
    pool: PgPool,
    handlers: JoinSet<Result<(), GatewayError>>,
    test_support: GatewayManagerTestSupport,
    tx: GatewayTxSet,
}

impl GatewayManager {
    #[must_use]
    pub fn new(pool: PgPool, tx: GatewayTxSet) -> Self {
        Self {
            clients: Arc::default(),
            handlers: JoinSet::new(),
            pool,
            test_support: GatewayManagerTestSupport::default(),
            tx,
        }
    }

    #[doc(hidden)]
    #[must_use]
    pub fn new_for_test(
        pool: PgPool,
        tx: GatewayTxSet,
        control: TestGatewayManagerControl,
    ) -> Self {
        Self {
            clients: Arc::default(),
            handlers: JoinSet::new(),
            pool,
            test_support: control.inner,
            tx,
        }
    }

    /// Bi-directional gRPC stream for communication with Defguard Gateway.
    pub async fn run(&mut self) -> Result<(), anyhow::Error> {
        let (certs_tx, certs_rx) = tokio::sync::watch::channel(Arc::new(HashMap::new()));
        certs::refresh_certs(&self.pool, &certs_tx).await;
        let refresh_pool = self.pool.clone();
        let _refresh_certs_task = AbortTaskOnDrop::new(tokio::spawn(async move {
            loop {
                certs::refresh_certs(&refresh_pool, &certs_tx).await;
                tokio::time::sleep(TEN_SECS).await;
            }
        }));
        let mut abort_handles = HashMap::new();
        for gateway in Gateway::all(&self.pool).await? {
            if !gateway.enabled {
                debug!("Existing Gateway is disabled, so it won't be handled");
                continue;
            }

            let id = gateway.id;
            let abort_handle =
                self.run_handler(gateway, Arc::clone(&self.clients), certs_rx.clone())?;
            abort_handles.insert(id, abort_handle);
        }

        // Observe gateway URL changes.
        let mut listener = PgListener::connect_with(&self.pool).await?;
        listener.listen(GATEWAY_TABLE_TRIGGER).await?;
        self.test_support.mark_listener_ready();
        while let Ok(notification) = listener.recv().await {
            let payload = notification.payload();
            match serde_json::from_str::<ChangeNotification<Gateway<Id>>>(payload) {
                Ok(gateway_notification) => {
                    let maybe_gateway_id = match gateway_notification.operation {
                        TriggerOperation::Insert => {
                            let Some(new) = gateway_notification.new else {
                                continue;
                            };

                            let id = new.id;
                            if new.enabled {
                                let abort_handle = self.run_handler(
                                    new,
                                    Arc::clone(&self.clients),
                                    certs_rx.clone(),
                                )?;
                                abort_handles.insert(id, abort_handle);
                            } else {
                                debug!("New Gateway is disabled, so it won't be handled");
                            }

                            Some(id)
                        }
                        TriggerOperation::Update => {
                            let (Some(old), Some(new)) =
                                (gateway_notification.old, gateway_notification.new)
                            else {
                                continue;
                            };

                            let id = new.id;
                            if old.address == new.address
                                && old.port == new.port
                                && old.enabled == new.enabled
                            {
                                debug!("Gateway address/port/state didn't change");
                            } else {
                                if let Some(abort_handle) = abort_handles.remove(&old.id) {
                                    info!(
                                        "Aborting connection to Gateway {old}, it has changed in the \
                                        database"
                                    );
                                    abort_handle.abort();
                                } else if old.enabled {
                                    warn!(
                                        "Cannot find Gateway {old} on the list of connected gateways"
                                    );
                                }
                                if new.enabled {
                                    let abort_handle = self.run_handler(
                                        new,
                                        Arc::clone(&self.clients),
                                        certs_rx.clone(),
                                    )?;
                                    abort_handles.insert(id, abort_handle);
                                } else {
                                    debug!("Updated Gateway is disabled, so it won't be handled");
                                }
                            }

                            Some(id)
                        }
                        TriggerOperation::Delete => {
                            let Some(old) = gateway_notification.old else {
                                continue;
                            };

                            // Send purge request to Gateway.
                            let maybe_client = {
                                self.clients
                                    .lock()
                                    .expect("Failed to lock GatewayManager::clients")
                                    .remove(&old.id)
                            };

                            if let Some(mut client) = maybe_client {
                                debug!("Sending purge request to Gateway {old}");
                                if let Err(err) = client.purge(Request::new(())).await {
                                    error!("Error sending purge request to Gateway {old}: {err}");
                                } else {
                                    info!("Sent purge request to Gateway {old}");
                                }
                            } else {
                                warn!(
                                    "Cannot find gRPC client for Gateway {old}; skipping purge request"
                                );
                            }

                            // Kill the `GatewayHandler` and the connection.
                            if let Some(abort_handle) = abort_handles.remove(&old.id) {
                                info!(
                                    "Aborting connection to Gateway {old}, it has disappeard from the \
                                    database"
                                );
                                abort_handle.abort();
                            } else if old.enabled {
                                warn!("Cannot find Gateway {old} on the list of connected gateways");
                            }

                            Some(old.id)
                        }
                    };

                    if let Some(gateway_id) = maybe_gateway_id {
                        self.test_support.note_gateway_notification(gateway_id);
                    }
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
        self.test_support.note_handler_spawn_attempt(gateway.id);
        let mut gateway_handler =
            if let Some(socket_path) = self.test_support.socket_path_for(&gateway) {
                GatewayHandler::new_with_test_socket(
                    gateway,
                    self.pool.clone(),
                    self.tx.events.clone(),
                    self.tx.peer_stats.clone(),
                    certs_rx.clone(),
                    socket_path,
                )?
            } else {
                GatewayHandler::new(
                    gateway,
                    self.pool.clone(),
                    self.tx.events.clone(),
                    self.tx.peer_stats.clone(),
                    certs_rx.clone(),
                )?
            };
        let abort_handle = self.handlers.spawn(async move {
            loop {
                if let Err(err) = gateway_handler
                    .handle_connection(Arc::clone(&clients))
                    .await
                {
                    error!("Gateway connection error: {err}, retrying in 5 seconds...");
                    tokio::time::sleep(GATEWAY_RECONNECT_DELAY).await;
                }
            }
        });
        Ok(abort_handle)
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
