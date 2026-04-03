use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::Duration,
};

#[cfg(test)]
use std::{path::PathBuf, str::FromStr, sync::Mutex as StdMutex};

use axum_extra::extract::cookie::Key;
use defguard_common::{
    db::{Id, models::proxy::Proxy},
    types::proxy::ProxyControlMessage,
};
use defguard_core::{
    events::BidiStreamEvent,
    grpc::{GatewayEvent, proxy::client_mfa::ClientLoginSession},
    version::IncompatibleComponents,
};
use defguard_proto::proxy::{CoreResponse, HttpsCerts, core_response};
use sqlx::PgPool;
use tokio::{
    select,
    sync::{
        Mutex,
        broadcast::Sender,
        mpsc::{Receiver, UnboundedSender},
        oneshot, watch,
    },
    task::JoinSet,
    time::sleep,
};

#[cfg(test)]
use tokio::sync::Notify;

use crate::{certs::refresh_certs, error::ProxyError, handler::ProxyHandler};

mod certs;
mod error;
mod handler;
mod servers;

#[cfg(test)]
mod tests;

#[macro_use]
extern crate tracing;

const TEN_SECS: Duration = Duration::from_secs(10);

#[cfg(test)]
#[derive(Clone, Default)]
pub(crate) struct ProxyManagerTestSupport {
    socket_paths_by_url: Arc<StdMutex<HashMap<String, PathBuf>>>,
    handler_spawn_attempts_by_proxy: Arc<StdMutex<HashMap<Id, u64>>>,
    handler_spawn_attempt_notify: Arc<Notify>,
    retry_delay_override: Arc<StdMutex<Option<Duration>>>,
}

#[cfg(test)]
impl ProxyManagerTestSupport {
    fn register_proxy_url(&self, proxy_url: String, socket_path: PathBuf) {
        self.socket_paths_by_url
            .lock()
            .expect("Failed to lock ProxyManager test socket registry")
            .insert(proxy_url, socket_path);
    }

    pub(crate) fn socket_path_for_url(&self, url: &str) -> Option<PathBuf> {
        self.socket_paths_by_url
            .lock()
            .expect("Failed to lock ProxyManager test socket registry")
            .get(url)
            .cloned()
    }

    fn note_handler_spawn_attempt(&self, proxy_id: Id) {
        let mut handler_spawn_attempts = self
            .handler_spawn_attempts_by_proxy
            .lock()
            .expect("Failed to lock ProxyManager handler spawn attempts registry");
        *handler_spawn_attempts.entry(proxy_id).or_default() += 1;
        self.handler_spawn_attempt_notify.notify_waiters();
    }

    pub(crate) fn handler_spawn_attempt_count(&self, proxy_id: Id) -> u64 {
        self.handler_spawn_attempts_by_proxy
            .lock()
            .expect("Failed to lock ProxyManager handler spawn attempts registry")
            .get(&proxy_id)
            .copied()
            .unwrap_or_default()
    }

    pub(crate) async fn wait_for_handler_spawn_attempt_count(
        &self,
        proxy_id: Id,
        expected_count: u64,
    ) {
        loop {
            if self.handler_spawn_attempt_count(proxy_id) >= expected_count {
                return;
            }

            let notified = self.handler_spawn_attempt_notify.notified();
            if self.handler_spawn_attempt_count(proxy_id) >= expected_count {
                return;
            }

            notified.await;
        }
    }

    pub(crate) fn set_retry_delay(&self, retry_delay: Duration) {
        *self
            .retry_delay_override
            .lock()
            .expect("Failed to lock ProxyManager retry delay override") = Some(retry_delay);
    }

    #[allow(dead_code)]
    pub(crate) fn manager_reconnect_delay(&self) -> Duration {
        self.retry_delay_override
            .lock()
            .expect("Failed to lock ProxyManager retry delay override")
            .unwrap_or(TEN_SECS)
    }

    pub(crate) fn handler_reconnect_delay(&self) -> Duration {
        self.retry_delay_override
            .lock()
            .expect("Failed to lock ProxyManager retry delay override")
            .unwrap_or(TEN_SECS)
    }
}

/// Map from proxy ID to the `CoreResponse` sender for that handler's active stream.
///
/// Populated by each `ProxyHandler` when it establishes a stream; used by the manager
/// to push messages to a specific proxy.
pub(crate) type HandlerTxMap = Arc<RwLock<HashMap<i64, UnboundedSender<CoreResponse>>>>;

/// Coordinates communication between the Core and multiple proxy instances.
///
/// Responsibilities include:
/// - instantiating and supervising proxy connections,
/// - providing shared infrastructure (database access, outbound channels),
pub struct ProxyManager {
    pool: PgPool,
    tx: ProxyTxSet,
    incompatible_components: Arc<RwLock<IncompatibleComponents>>,
    proxy_control: Receiver<ProxyControlMessage>,
    proxy_cookie_key: Key,
    #[cfg(test)]
    test_support: Option<ProxyManagerTestSupport>,
}

impl ProxyManager {
    pub fn new(
        pool: PgPool,
        tx: ProxyTxSet,
        incompatible_components: Arc<RwLock<IncompatibleComponents>>,
        proxy_control_rx: Receiver<ProxyControlMessage>,
        core_secret_key: &str,
    ) -> Self {
        Self {
            pool,
            tx,
            incompatible_components,
            proxy_control: proxy_control_rx,
            proxy_cookie_key: Key::derive_from(core_secret_key.as_bytes()),
            #[cfg(test)]
            test_support: None,
        }
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) fn new_for_test(
        pool: PgPool,
        tx: ProxyTxSet,
        incompatible_components: Arc<RwLock<IncompatibleComponents>>,
        proxy_control_rx: Receiver<ProxyControlMessage>,
        core_secret_key: &str,
        test_support: ProxyManagerTestSupport,
    ) -> Self {
        Self {
            pool,
            tx,
            incompatible_components,
            proxy_control: proxy_control_rx,
            proxy_cookie_key: Key::derive_from(core_secret_key.as_bytes()),
            test_support: Some(test_support),
        }
    }

    fn build_handler(
        &self,
        proxy: &Proxy<Id>,
        remote_mfa_responses: Arc<RwLock<HashMap<String, oneshot::Sender<String>>>>,
        sessions: Arc<RwLock<HashMap<String, ClientLoginSession>>>,
        handler_tx_map: HandlerTxMap,
        shutdown_rx: Arc<Mutex<oneshot::Receiver<bool>>>,
        proxy_cookie_key: Key,
    ) -> Result<ProxyHandler, ProxyError> {
        #[cfg(test)]
        if let Some(test_support) = self.test_support.clone() {
            use reqwest::Url;

            test_support.note_handler_spawn_attempt(proxy.id);

            let url = Url::from_str(&format!("http://{}:{}", proxy.address, proxy.port))
                .map_err(ProxyError::from)?;
            let url_str = url.to_string();
            let socket_path = test_support.socket_path_for_url(&url_str);

            // Always construct with the shared handler_tx_map so that
            // BroadcastHttpsCerts (and any other manager-level broadcasts) can
            // reach this handler.
            let mut handler = ProxyHandler::from_proxy(
                proxy,
                self.pool.clone(),
                &self.tx,
                remote_mfa_responses,
                sessions,
                shutdown_rx,
                proxy_cookie_key,
                handler_tx_map,
            )?;

            if let Some(path) = socket_path {
                handler.set_test_socket_path(path);
            }
            handler.attach_test_support(test_support);
            return Ok(handler);
        }

        ProxyHandler::from_proxy(
            proxy,
            self.pool.clone(),
            &self.tx,
            remote_mfa_responses,
            sessions,
            shutdown_rx,
            proxy_cookie_key,
            handler_tx_map,
        )
    }

    /// Spawns and supervises asynchronous tasks for all configured proxies.
    ///
    /// Each proxy runs in its own task and shares Core-side infrastructure
    pub async fn run(mut self) -> Result<(), ProxyError> {
        debug!("ProxyManager starting");
        let remote_mfa_responses = Arc::default();
        let sessions = Arc::default();
        let (certs_tx, certs_rx) = watch::channel(Arc::new(HashMap::new()));
        // Prime the cache to avoid race with connection loop.
        refresh_certs(&self.pool, &certs_tx).await;
        let refresh_pool = self.pool.clone();
        tokio::spawn(async move {
            loop {
                refresh_certs(&refresh_pool, &certs_tx).await;
                sleep(TEN_SECS).await;
            }
        });

        // Shared map: proxy_id → sender for the handler's active gRPC stream.
        let handler_tx_map: HandlerTxMap = Arc::new(RwLock::new(HashMap::new()));

        // Retrieve proxies from DB.
        let mut shutdown_channels = HashMap::new();
        let proxies = Proxy::all_enabled(&self.pool)
            .await?
            .iter()
            .map(|proxy| {
                let (shutdown_tx, shutdown_rx) = oneshot::channel::<bool>();
                shutdown_channels.insert(proxy.id, shutdown_tx);
                self.build_handler(
                    proxy,
                    Arc::clone(&remote_mfa_responses),
                    Arc::clone(&sessions),
                    Arc::clone(&handler_tx_map),
                    Arc::new(Mutex::new(shutdown_rx)),
                    self.proxy_cookie_key.clone(),
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        debug!("Retrieved {} proxies from the DB", proxies.len());

        // Connect to all enabled proxies.
        let mut tasks = JoinSet::new();
        for proxy in proxies {
            debug!("Spawning proxy task for proxy {}", proxy.url);
            tasks.spawn(proxy.run(
                self.tx.clone(),
                self.incompatible_components.clone(),
                certs_rx.clone(),
            ));
        }

        loop {
            select! {
                result = tasks.join_next(), if !tasks.is_empty() => {
                    match result {
                        Some(Ok(Ok(()))) => error!("Proxy task returned prematurely"),
                        Some(Ok(Err(err))) => error!("Proxy task returned with error: {err}"),
                        Some(Err(err)) => error!("Proxy task execution failed: {err}"),
                        None => {
                            debug!("All proxy tasks completed");
                        }
                    }
                }
                msg = self.proxy_control.recv() => {
                    match msg {
                        Some(ProxyControlMessage::StartConnection(id)) => {
                            debug!("Starting proxy with ID: {id}");
                            if let Ok(Some(proxy_model)) = Proxy::find_by_id(&self.pool, id).await {
                                if !proxy_model.enabled {
                                    debug!("Proxy ID {id} is disabled; connecting abandoned");
                                    continue;
                                }
                                let (shutdown_tx, shutdown_rx) =
                                    oneshot::channel::<bool>();
                                shutdown_channels.insert(id, shutdown_tx);
                                match self.build_handler(
                                    &proxy_model,
                                    Arc::clone(&remote_mfa_responses),
                                    Arc::clone(&sessions),
                                    Arc::clone(&handler_tx_map),
                                    Arc::new(Mutex::new(shutdown_rx)),
                                    self.proxy_cookie_key.clone(),
                                ) {
                                    Ok(proxy) => {
                                        debug!("Spawning proxy task for proxy {}", proxy.url);
                                        tasks.spawn(proxy.run(self.tx.clone(),
                                            self.incompatible_components.clone(), certs_rx.clone()));
                                    }
                                    Err(err) => error!("Failed to create proxy server: {err}"),
                                }
                            } else {
                                error!("Failed to find proxy with ID: {id}");
                            }
                        }
                        Some(ProxyControlMessage::ShutdownConnection(id)) => {
                            debug!("Shutting down proxy with ID: {id}");
                            if let Some(shutdown_tx) = shutdown_channels.remove(&id) {
                                let _ = shutdown_tx.send(false);
                            } else {
                                warn!("No shutdown channel found for proxy ID: {id}");
                            }
                        }
                        Some(ProxyControlMessage::Purge(id)) => {
                            debug!("Purging proxy with ID: {id}");
                            if let Some(shutdown_tx) = shutdown_channels.remove(&id) {
                                let _ = shutdown_tx.send(true);
                            } else {
                                warn!("No shutdown channel found for proxy ID: {id}");
                            }
                        }
                        Some(ProxyControlMessage::BroadcastHttpsCerts { cert_pem, key_pem }) => {
                            debug!("Broadcasting HttpsCerts to all connected proxies");
                            let msg = CoreResponse {
                                id: 0,
                                payload: Some(core_response::Payload::HttpsCerts(
                                    HttpsCerts { cert_pem, key_pem },
                                )),
                            };
                            if let Ok(map) = handler_tx_map.read() {
                                for (pid, tx) in map.iter() {
                                    debug!("Sending HttpsCerts to proxy {pid}");
                                    let _ = tx.send(msg.clone());
                                }
                            }
                        }
                        None => {
                            debug!("Proxy control channel closed");
                            break;
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

/// Shared set of outbound channels that proxy instances use to forward
/// events, notifications, and side effects to Core components.
#[derive(Clone)]
pub struct ProxyTxSet {
    wireguard: Sender<GatewayEvent>,
    bidi_events: UnboundedSender<BidiStreamEvent>,
}

impl ProxyTxSet {
    #[must_use]
    pub const fn new(
        wireguard: Sender<GatewayEvent>,
        bidi_events: UnboundedSender<BidiStreamEvent>,
    ) -> Self {
        Self {
            wireguard,
            bidi_events,
        }
    }
}
