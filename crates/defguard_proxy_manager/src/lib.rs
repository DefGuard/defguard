use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::Duration,
};

use defguard_common::{db::models::proxy::Proxy, types::proxy::ProxyControlMessage};
use defguard_core::{
    events::BidiStreamEvent, grpc::gateway::events::GatewayEvent, version::IncompatibleComponents,
};
use defguard_mail::Mail;
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

use crate::{certs::refresh_certs, error::ProxyError, proxy_handler::ProxyHandler};

mod certs;
mod error;
mod proxy_handler;
mod servers;

#[macro_use]
extern crate tracing;

const TEN_SECS: Duration = Duration::from_secs(10);

/// Coordinates communication between the Core and multiple proxy instances.
///
/// Responsibilities include:
/// - instantiating and supervising proxy connections,
/// - routing responses to the appropriate proxy based on correlation state,
/// - providing shared infrastructure (database access, outbound channels),
pub struct ProxyManager {
    pool: PgPool,
    tx: ProxyTxSet,
    incompatible_components: Arc<RwLock<IncompatibleComponents>>,
    proxy_control: Receiver<ProxyControlMessage>,
}

impl ProxyManager {
    pub fn new(
        pool: PgPool,
        tx: ProxyTxSet,
        incompatible_components: Arc<RwLock<IncompatibleComponents>>,
        proxy_control_rx: Receiver<ProxyControlMessage>,
    ) -> Self {
        Self {
            pool,
            tx,
            incompatible_components,
            proxy_control: proxy_control_rx,
        }
    }

    /// Spawns and supervises asynchronous tasks for all configured proxies.
    ///
    /// Each proxy runs in its own task and shares Core-side infrastructure
    /// such as routing state and compatibility tracking.
    pub async fn run(mut self) -> Result<(), ProxyError> {
        debug!("ProxyManager starting");
        let remote_mfa_responses = Arc::default();
        let sessions = Arc::default();
        let (certs_tx, certs_rx) = watch::channel(Arc::new(HashMap::new()));
        let refresh_pool = self.pool.clone();
        tokio::spawn(async move {
            loop {
                refresh_certs(&refresh_pool, &certs_tx).await;
                tokio::time::sleep(TEN_SECS).await;
            }
        });
        // Retrieve proxies from DB.
        let mut shutdown_channels = HashMap::new();
        let proxies = Proxy::all(&self.pool)
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
                    Arc::new(Mutex::new(Some(shutdown_rx))),
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        debug!("Retrieved {} proxies from the DB", proxies.len());

        // Connect to all proxies.
        let mut tasks = JoinSet::<Result<(), ProxyError>>::new();
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
                                let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<bool>();
                                shutdown_channels.insert(id, shutdown_tx);
                                match ProxyHandler::from_proxy(
                                    &proxy_model,
                                    self.pool.clone(),
                                    &self.tx,
                                    Arc::clone(&remote_mfa_responses),
                                    Arc::clone(&sessions),
                                    Arc::new(Mutex::new(Some(shutdown_rx))),
                                ) {
                                    Ok(proxy) => {
                                        debug!("Spawning proxy task for proxy {}", proxy.url);
                                        tasks.spawn(proxy.run(self.tx.clone(), self.incompatible_components.clone(), certs_rx.clone()));
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
    mail: UnboundedSender<Mail>,
    bidi_events: UnboundedSender<BidiStreamEvent>,
}

impl ProxyTxSet {
    #[must_use]
    pub const fn new(
        wireguard: Sender<GatewayEvent>,
        mail: UnboundedSender<Mail>,
        bidi_events: UnboundedSender<BidiStreamEvent>,
    ) -> Self {
        Self {
            wireguard,
            mail,
            bidi_events,
        }
    }
}
