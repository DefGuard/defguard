// FIXME: actually refactor errors instead
#![allow(clippy::result_large_err)]
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
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
    sync::{broadcast::Sender, mpsc::UnboundedSender, watch::Receiver},
    task::{AbortHandle, JoinSet},
};
use tonic::{Request, service::interceptor::InterceptedService, transport::Channel};

use crate::{error::GatewayError, handler::GatewayHandler};

#[macro_use]
extern crate tracing;

mod certs;
mod error;
mod handler;
// #[cfg(test)]
// mod tests;

#[cfg(test)]
static TONIC_SOCKET: &str = "tonic.sock";
const GATEWAY_TABLE_TRIGGER: &str = "gateway_change";
const GATEWAY_RECONNECT_DELAY: Duration = Duration::from_secs(5);
const TEN_SECS: Duration = Duration::from_secs(10);

type Client = GatewayClient<InterceptedService<Channel, ClientVersionInterceptor>>;

pub struct GatewayManager {
    clients: Arc<Mutex<HashMap<Id, Client>>>,
    pool: PgPool,
    handlers: JoinSet<Result<(), GatewayError>>,
    tx: GatewayTxSet,
}

impl GatewayManager {
    #[must_use]
    pub fn new(pool: PgPool, tx: GatewayTxSet) -> Self {
        Self {
            clients: Arc::default(),
            handlers: JoinSet::new(),
            pool,
            tx,
        }
    }

    /// Bi-directional gRPC stream for communication with Defguard Gateway.
    pub async fn run(&mut self) -> Result<(), anyhow::Error> {
        let (certs_tx, certs_rx) = tokio::sync::watch::channel(Arc::new(HashMap::new()));
        certs::refresh_certs(&self.pool, &certs_tx).await;
        let refresh_pool = self.pool.clone();
        tokio::spawn(async move {
            loop {
                certs::refresh_certs(&refresh_pool, &certs_tx).await;
                tokio::time::sleep(TEN_SECS).await;
            }
        });
        let mut abort_handles = HashMap::new();
        for gateway in Gateway::all(&self.pool).await? {
            let id = gateway.id;
            let abort_handle =
                self.run_handler(gateway, Arc::clone(&self.clients), certs_rx.clone())?;
            abort_handles.insert(id, abort_handle);
        }

        // Observe gateway URL changes.
        let mut listener = PgListener::connect_with(&self.pool).await?;
        listener.listen(GATEWAY_TABLE_TRIGGER).await?;
        while let Ok(notification) = listener.recv().await {
            let payload = notification.payload();
            match serde_json::from_str::<ChangeNotification<Gateway<Id>>>(payload) {
                Ok(gateway_notification) => match gateway_notification.operation {
                    TriggerOperation::Insert => {
                        if let Some(new) = gateway_notification.new {
                            let id = new.id;
                            let abort_handle =
                                self.run_handler(new, Arc::clone(&self.clients), certs_rx.clone())?;
                            abort_handles.insert(id, abort_handle);
                        }
                    }
                    TriggerOperation::Update => {
                        if let (Some(old), Some(new)) =
                            (gateway_notification.old, gateway_notification.new)
                        {
                            if old.address == new.address && old.port == new.port {
                                debug!(
                                    "Gateway URL didn't change. Keeping the current gateway handler"
                                );
                            } else if let Some(abort_handle) = abort_handles.remove(&old.id) {
                                info!(
                                    "Aborting connection to {old}, it has changed in the database"
                                );
                                abort_handle.abort();
                                let id = new.id;
                                let abort_handle = self.run_handler(
                                    new,
                                    Arc::clone(&self.clients),
                                    certs_rx.clone(),
                                )?;
                                abort_handles.insert(id, abort_handle);
                            } else {
                                warn!("Cannot find {old} on the list of connected gateways");
                            }
                        }
                    }
                    TriggerOperation::Delete => {
                        let Some(old) = gateway_notification.old else {
                            continue;
                        };

                        // Send purge request to the gateway.
                        let maybe_client = {
                            self.clients
                                .lock()
                                .expect("Failed to lock GatewayManager::clients")
                                .remove(&old.id)
                        };

                        if let Some(mut client) = maybe_client {
                            debug!("Sending purge request to gateway {old}");
                            if let Err(err) = client.purge(Request::new(())).await {
                                error!("Error sending purge request to gateway {old}: {err}");
                            } else {
                                info!("Sent purge request to gateway {old}");
                            }
                        } else {
                            warn!(
                                "Cannot find gRPC client for gateway {old}, won't send purge request"
                            );
                        }

                        // Kill the `GatewayHandler` and the connection.
                        if let Some(abort_handle) = abort_handles.remove(&old.id) {
                            info!(
                                "Aborting connection to gateway {old}, it has disappeard from the database"
                            );
                            abort_handle.abort();
                        } else {
                            warn!("Cannot find abort handle for gateway {old}");
                        }
                    }
                },
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
        let mut gateway_handler = GatewayHandler::new(
            gateway,
            self.pool.clone(),
            self.tx.events.clone(),
            self.tx.peer_stats.clone(),
            certs_rx.clone(),
        )?;
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
