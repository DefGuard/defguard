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
    handler_connection_attempts_by_gateway: Arc<Mutex<HashMap<Id, u64>>>,
    handler_connection_attempt_notify: Arc<Notify>,
    gateway_notifications_by_gateway: Arc<Mutex<HashMap<Id, u64>>>,
    gateway_notification_notify: Arc<Notify>,
    listener_ready: Arc<AtomicBool>,
    listener_ready_notify: Arc<Notify>,
    retry_delay_override: Arc<Mutex<Option<Duration>>>,
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

    fn note_handler_connection_attempt(&self, gateway_id: Id) {
        let mut handler_connection_attempts = self
            .handler_connection_attempts_by_gateway
            .lock()
            .expect("Failed to lock GatewayManager handler connection attempts registry");
        *handler_connection_attempts.entry(gateway_id).or_default() += 1;
        self.handler_connection_attempt_notify.notify_waiters();
    }

    fn handler_connection_attempt_count(&self, gateway_id: Id) -> u64 {
        self.handler_connection_attempts_by_gateway
            .lock()
            .expect("Failed to lock GatewayManager handler connection attempts registry")
            .get(&gateway_id)
            .copied()
            .unwrap_or_default()
    }

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

#[derive(Clone, Default)]
struct TestGatewayManagerControl {
    inner: GatewayManagerTestSupport,
}

impl TestGatewayManagerControl {
    #[must_use]
    fn new() -> Self {
        Self::default()
    }

    fn register_gateway_url(&self, gateway_url: String, socket_path: PathBuf) {
        self.inner.register_gateway_url(gateway_url, socket_path);
    }

    fn handler_spawn_attempt_count(&self, gateway_id: Id) -> u64 {
        self.inner.handler_spawn_attempt_count(gateway_id)
    }

    async fn wait_for_handler_spawn_attempt_count(&self, gateway_id: Id, expected_count: u64) {
        self.inner
            .wait_for_handler_spawn_attempt_count(gateway_id, expected_count)
            .await;
    }

    fn handler_connection_attempt_count(&self, gateway_id: Id) -> u64 {
        self.inner.handler_connection_attempt_count(gateway_id)
    }

    async fn wait_for_handler_connection_attempt_count(&self, gateway_id: Id, expected_count: u64) {
        self.inner
            .wait_for_handler_connection_attempt_count(gateway_id, expected_count)
            .await;
    }

    fn gateway_notification_count(&self, gateway_id: Id) -> u64 {
        self.inner.gateway_notification_count(gateway_id)
    }

    async fn wait_for_gateway_notification_count(&self, gateway_id: Id, expected_count: u64) {
        self.inner
            .wait_for_gateway_notification_count(gateway_id, expected_count)
            .await;
    }

    async fn wait_until_listener_ready(&self) {
        self.inner.wait_until_listener_ready().await;
    }

    fn set_retry_delay(&self, retry_delay: Duration) {
        self.inner.set_retry_delay(retry_delay);
    }
}

#[doc(hidden)]
pub mod test_support {
    use std::{collections::HashMap, path::PathBuf, sync::Arc, time::Duration};

    use defguard_common::{
        db::{Id, models::gateway::Gateway},
        messages::peer_stats_update::PeerStatsUpdate,
    };
    use defguard_core::grpc::GatewayEvent;
    use sqlx::PgPool;
    use tokio::sync::{broadcast::Sender, mpsc::UnboundedSender, watch::Receiver};

    use crate::{
        GatewayManager, GatewayTxSet, TestGatewayManagerControl, handler::TestGatewayHandler,
    };

    #[derive(Clone, Default)]
    pub struct GatewayManagerControl {
        inner: TestGatewayManagerControl,
    }

    impl GatewayManagerControl {
        #[must_use]
        pub fn new() -> Self {
            Self {
                inner: TestGatewayManagerControl::new(),
            }
        }

        pub fn register_gateway_url(&self, gateway_url: String, socket_path: PathBuf) {
            self.inner.register_gateway_url(gateway_url, socket_path);
        }

        #[must_use]
        pub fn new_manager(&self, pool: PgPool, tx: GatewayTxSet) -> GatewayManager {
            GatewayManager::new_for_test(pool, tx, self.inner.clone())
        }

        pub fn handler_spawn_attempt_count(&self, gateway_id: Id) -> u64 {
            self.inner.handler_spawn_attempt_count(gateway_id)
        }

        pub async fn wait_for_handler_spawn_attempt_count(
            &self,
            gateway_id: Id,
            expected_count: u64,
        ) {
            self.inner
                .wait_for_handler_spawn_attempt_count(gateway_id, expected_count)
                .await;
        }

        pub fn handler_connection_attempt_count(&self, gateway_id: Id) -> u64 {
            self.inner.handler_connection_attempt_count(gateway_id)
        }

        pub async fn wait_for_handler_connection_attempt_count(
            &self,
            gateway_id: Id,
            expected_count: u64,
        ) {
            self.inner
                .wait_for_handler_connection_attempt_count(gateway_id, expected_count)
                .await;
        }

        pub fn gateway_notification_count(&self, gateway_id: Id) -> u64 {
            self.inner.gateway_notification_count(gateway_id)
        }

        pub async fn wait_for_gateway_notification_count(
            &self,
            gateway_id: Id,
            expected_count: u64,
        ) {
            self.inner
                .wait_for_gateway_notification_count(gateway_id, expected_count)
                .await;
        }

        pub async fn wait_until_listener_ready(&self) {
            self.inner.wait_until_listener_ready().await;
        }

        pub fn set_retry_delay(&self, retry_delay: Duration) {
            self.inner.set_retry_delay(retry_delay);
        }
    }

    pub struct GatewayHandler {
        inner: TestGatewayHandler,
    }

    impl GatewayHandler {
        pub fn new(
            gateway: Gateway<Id>,
            pool: PgPool,
            events_tx: Sender<GatewayEvent>,
            peer_stats_tx: UnboundedSender<PeerStatsUpdate>,
            certs_rx: Receiver<Arc<HashMap<Id, String>>>,
            socket_path: PathBuf,
        ) -> anyhow::Result<Self> {
            let inner = TestGatewayHandler::new(
                gateway,
                pool,
                events_tx,
                peer_stats_tx,
                certs_rx,
                socket_path,
            )?;
            Ok(Self { inner })
        }

        pub async fn handle_connection_once(&mut self) -> anyhow::Result<()> {
            self.inner.handle_connection_once().await
        }
    }
}

pub struct GatewayManager {
    clients: Arc<Mutex<HashMap<Id, Client>>>,
    pool: PgPool,
    handlers: JoinSet<Result<(), GatewayError>>,
    test_support: Option<GatewayManagerTestSupport>,
    tx: GatewayTxSet,
}

impl GatewayManager {
    #[must_use]
    pub fn new(pool: PgPool, tx: GatewayTxSet) -> Self {
        Self {
            clients: Arc::default(),
            handlers: JoinSet::new(),
            pool,
            test_support: None,
            tx,
        }
    }

    #[must_use]
    fn new_for_test(pool: PgPool, tx: GatewayTxSet, control: TestGatewayManagerControl) -> Self {
        Self {
            clients: Arc::default(),
            handlers: JoinSet::new(),
            pool,
            test_support: Some(control.inner),
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
        if let Some(test_support) = &self.test_support {
            test_support.mark_listener_ready();
        }
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
                            let mut old = old;

                            let id = new.id;
                            if old.address == new.address
                                && old.port == new.port
                                && old.enabled == new.enabled
                            {
                                debug!("Gateway address/port/state didn't change");
                            } else {
                                self.remove_client(old.id);
                                if let Some(abort_handle) = abort_handles.remove(&old.id) {
                                    if let Err(err) = old.touch_disconnected(&self.pool).await {
                                        error!(
                                            "Failed to update disconnection time for Gateway {old} after database change: {err}"
                                        );
                                    }
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
                            let maybe_client = self.remove_client(old.id);

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
                                    "Aborting connection to Gateway {old}, it has disappeared from the \
                                    database"
                                );
                                abort_handle.abort();
                            } else if old.enabled {
                                warn!(
                                    "Cannot find Gateway {old} on the list of connected gateways"
                                );
                            }

                            Some(old.id)
                        }
                    };

                    if let (Some(gateway_id), Some(test_support)) =
                        (maybe_gateway_id, self.test_support.as_ref())
                    {
                        test_support.note_gateway_notification(gateway_id);
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
        let maybe_test_support = self.test_support.clone();

        if let Some(test_support) = &maybe_test_support {
            test_support.note_handler_spawn_attempt(gateway.id);
        }

        let mut gateway_handler = if let Some(socket_path) = maybe_test_support
            .as_ref()
            .and_then(|test_support| test_support.socket_path_for(&gateway))
        {
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

        if let Some(test_support) = maybe_test_support {
            gateway_handler.attach_test_support(test_support);
        }

        let manager_reconnect_delay = self.test_support.as_ref().map_or(
            GATEWAY_RECONNECT_DELAY,
            GatewayManagerTestSupport::manager_reconnect_delay,
        );
        let abort_handle = self.handlers.spawn(async move {
            loop {
                if let Err(err) = gateway_handler
                    .handle_connection(Arc::clone(&clients))
                    .await
                {
                    error!(
                        "Gateway connection error: {err}, retrying in {manager_reconnect_delay:?}..."
                    );
                    tokio::time::sleep(manager_reconnect_delay).await;
                }
            }
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
