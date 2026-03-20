use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::Duration,
};

use axum_extra::extract::cookie::Key;
use defguard_common::{
    db::models::{Certificates, proxy::Proxy},
    types::proxy::ProxyControlMessage,
};
use defguard_core::{events::BidiStreamEvent, grpc::GatewayEvent, version::IncompatibleComponents};
use defguard_proto::proxy::{AcmeChallenge, CoreResponse, core_response};
use sqlx::PgPool;
use tokio::{
    select,
    sync::{
        Mutex,
        broadcast::Sender,
        mpsc::{Receiver, UnboundedSender},
        watch,
    },
    task::JoinSet,
};

use crate::{certs::refresh_certs, error::ProxyError, handler::ProxyHandler};

mod certs;
mod error;
mod handler;
mod servers;

#[macro_use]
extern crate tracing;

const TEN_SECS: Duration = Duration::from_secs(10);

/// Map from proxy ID to the `CoreResponse` sender for that handler's active stream.
///
/// Populated by each `ProxyHandler` when it establishes a stream; used by the manager
/// to push messages (e.g. `AcmeChallenge`) to a specific proxy.
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
        }
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
                tokio::time::sleep(TEN_SECS).await;
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
                let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<bool>();
                shutdown_channels.insert(proxy.id, shutdown_tx);
                ProxyHandler::from_proxy(
                    proxy,
                    self.pool.clone(),
                    &self.tx,
                    Arc::clone(&remote_mfa_responses),
                    Arc::clone(&sessions),
                    Arc::new(Mutex::new(shutdown_rx)),
                    self.proxy_cookie_key.clone(),
                    Arc::clone(&handler_tx_map),
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
                                    tokio::sync::oneshot::channel::<bool>();
                                shutdown_channels.insert(id, shutdown_tx);
                                match ProxyHandler::from_proxy(
                                    &proxy_model,
                                    self.pool.clone(),
                                    &self.tx,
                                    Arc::clone(&remote_mfa_responses),
                                    Arc::clone(&sessions),
                                    Arc::new(Mutex::new(shutdown_rx)),
                                    self.proxy_cookie_key.clone(),
                                    Arc::clone(&handler_tx_map),
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
                        Some(ProxyControlMessage::TriggerAcme { proxy_id, domain, use_staging }) => {
                            debug!("Triggering ACME issuance on proxy ID: {proxy_id}");
                            let certs = Certificates::get_or_default(&self.pool).await.unwrap_or_default();
                            let account_credentials_json = certs
                                .acme_account_credentials
                                .unwrap_or_default();
                            let challenge = AcmeChallenge {
                                domain,
                                use_staging,
                                account_credentials_json,
                            };
                            let msg = CoreResponse {
                                id: 0,
                                payload: Some(core_response::Payload::AcmeChallenge(challenge)),
                            };
                            let sent = handler_tx_map
                                .read()
                                .map(|map| {
                                    if let Some(tx) = map.get(&proxy_id) {
                                        let _ = tx.send(msg);
                                        true
                                    } else {
                                        false
                                    }
                                })
                                .unwrap_or(false);
                            if !sent {
                                warn!("No connected handler found for proxy ID {proxy_id} to send AcmeChallenge");
                            }
                        }
                        Some(ProxyControlMessage::BroadcastHttpsCerts { cert_pem, key_pem }) => {
                            debug!("Broadcasting HttpsCerts to all connected proxies");
                            let msg = CoreResponse {
                                id: 0,
                                payload: Some(core_response::Payload::HttpsCerts(
                                    defguard_proto::proxy::HttpsCerts { cert_pem, key_pem },
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
