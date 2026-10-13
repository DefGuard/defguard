use std::{
    collections::HashMap,
    sync::{Arc, Mutex, atomic::AtomicBool},
    time::Duration,
};

use defguard_common::{
    db::{ChangeNotification, Id, TriggerOperation, models::gateway::Gateway},
    gateway_event::GatewayCommand,
};
use sqlx::{PgPool, postgres::PgListener};
use tokio::{
    sync::{
        mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel},
        watch,
    },
    task::{AbortHandle, JoinSet},
    time::sleep,
};
use tonic::Request;

#[cfg(test)]
use crate::GatewayManagerTestSupport;
use crate::{
    AbortTaskOnDrop, Client, GatewayTxSet, TEN_SECS, certs, error::GatewayError,
    handler::GatewayHandler, needs_restart,
};

const GATEWAY_TABLE_TRIGGER: &str = "gateway_change";

/// Per-Gateway channels feeding each `GatewayHandler`.
pub(crate) type EventSenders = Arc<Mutex<HashMap<Id, UnboundedSender<GatewayCommand>>>>;

pub struct GatewayManager {
    clients: Arc<Mutex<HashMap<Id, Client>>>,
    pool: PgPool,
    handlers: JoinSet<Result<(), GatewayError>>,
    /// Per-Gateway flag tracking whether a disconnect email went out and is still awaiting its
    /// matching reconnect email. Owned by the manager rather than the handler so it survives a
    /// handler rebuild when connection-relevant fields change.
    disconnect_notification_sent_by_gateway: Arc<Mutex<HashMap<Id, Arc<AtomicBool>>>>,
    #[cfg(test)]
    test_support: GatewayManagerTestSupport,
    tx: GatewayTxSet,
    /// Taken by [`Self::run`].
    events_rx: Option<UnboundedReceiver<GatewayCommand>>,
    event_senders: EventSenders,
}

impl GatewayManager {
    #[cfg(not(test))]
    #[must_use]
    pub fn new(
        pool: PgPool,
        tx: GatewayTxSet,
        events_rx: UnboundedReceiver<GatewayCommand>,
    ) -> Self {
        Self {
            clients: Arc::default(),
            handlers: JoinSet::new(),
            pool,
            disconnect_notification_sent_by_gateway: Arc::default(),
            tx,
            events_rx: Some(events_rx),
            event_senders: Arc::default(),
        }
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) fn new(
        pool: PgPool,
        tx: GatewayTxSet,
        events_rx: UnboundedReceiver<GatewayCommand>,
        test_support: GatewayManagerTestSupport,
    ) -> Self {
        Self {
            clients: Arc::default(),
            handlers: JoinSet::new(),
            pool,
            disconnect_notification_sent_by_gateway: Arc::default(),
            event_senders: Arc::clone(&test_support.event_senders),
            test_support,
            tx,
            events_rx: Some(events_rx),
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
            crate::GATEWAY_RECONNECT_DELAY
        }
    }

    /// Returns the per-Gateway "disconnect email sent, awaiting reconnect" flag, creating it if
    /// necessary.
    fn disconnect_notification_flag(&self, gateway_id: Id) -> Arc<AtomicBool> {
        self.disconnect_notification_sent_by_gateway
            .lock()
            .expect("Failed to lock GatewayManager disconnect notification flags")
            .entry(gateway_id)
            .or_insert_with(|| Arc::new(AtomicBool::new(false)))
            .clone()
    }

    /// Removes the per-Gateway "disconnect email sent" flag (on Gateway deletion).
    fn clear_disconnect_notification_flag(&self, gateway_id: Id) {
        self.disconnect_notification_sent_by_gateway
            .lock()
            .expect("Failed to lock GatewayManager disconnect notification flags")
            .remove(&gateway_id);
    }

    fn build_handler(
        &self,
        gateway: Gateway<Id>,
        events_rx: UnboundedReceiver<GatewayCommand>,
        certs_rx: watch::Receiver<Arc<HashMap<Id, String>>>,
    ) -> Result<GatewayHandler, GatewayError> {
        // Reuse any outstanding "disconnect sent" flag for this Gateway so a reconnect email is
        // not lost when the handler is rebuilt after connection-relevant fields change.
        let disconnect_notification_sent = self.disconnect_notification_flag(gateway.id);

        #[cfg(test)]
        {
            self.test_support.note_handler_spawn_attempt(gateway.id);

            let mut gateway_handler =
                if let Some(socket_path) = self.test_support.socket_path_for(&gateway) {
                    GatewayHandler::new_with_test_socket(
                        gateway,
                        self.pool.clone(),
                        events_rx,
                        self.tx.connection_events.clone(),
                        self.tx.peer_stats.clone(),
                        certs_rx,
                        socket_path,
                        disconnect_notification_sent,
                    )?
                } else {
                    GatewayHandler::new(
                        gateway,
                        self.pool.clone(),
                        events_rx,
                        self.tx.connection_events.clone(),
                        self.tx.peer_stats.clone(),
                        certs_rx,
                        disconnect_notification_sent,
                    )?
                };
            gateway_handler.attach_test_support(self.test_support.clone());

            Ok(gateway_handler)
        }

        #[cfg(not(test))]
        GatewayHandler::new(
            gateway,
            self.pool.clone(),
            events_rx,
            self.tx.connection_events.clone(),
            self.tx.peer_stats.clone(),
            certs_rx,
            disconnect_notification_sent,
        )
    }

    /// Bi-directional gRPC stream for communication with Defguard Gateway.
    pub async fn run(&mut self) -> Result<(), anyhow::Error> {
        let (certs_tx, certs_rx) = watch::channel(Arc::new(HashMap::new()));
        certs::refresh_certs(&self.pool, &certs_tx).await;
        let refresh_pool = self.pool.clone();
        let _refresh_certs_task = AbortTaskOnDrop::new(tokio::spawn(async move {
            loop {
                certs::refresh_certs(&refresh_pool, &certs_tx).await;
                sleep(TEN_SECS).await;
            }
        }));
        let _dispatch_events_task = self.events_rx.take().map(|mut events_rx| {
            let event_senders = Arc::clone(&self.event_senders);
            AbortTaskOnDrop::new(tokio::spawn(async move {
                while let Some(ref event) = events_rx.recv().await {
                    // Send the event to all handlers, dropping closed channels.
                    event_senders
                        .lock()
                        .expect("Failed to lock GatewayManager::event_senders")
                        .retain(|_, events_tx| events_tx.send(event.clone()).is_ok());
                }
            }))
        });
        // Stores the abort handle and a snapshot of the gateway at the time the handler was last
        // started. The snapshot is used by the Update arm to detect connection-relevant changes.
        let mut abort_handles = HashMap::new();
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
                                if self.stop_handler(gateway_id, &mut abort_handles) {
                                    info!(
                                        "Aborted connection to Gateway id={gateway_id}, \
                                        connection-relevant fields have changed"
                                    );
                                }

                                // Only mark disconnected if the gateway was actually connected
                                if gateway.is_connected()
                                    && let Err(err) = gateway.touch_disconnected(&self.pool).await
                                {
                                    error!(
                                        "Failed to update disconnection time for Gateway \
                                            id={gateway_id} after database change: {err}"
                                    );
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
                            if self.stop_handler(gateway_id, &mut abort_handles) {
                                info!(
                                    "Aborted connection to Gateway id={gateway_id}, it has \
                                    disappeared from the database"
                                );
                            } else {
                                warn!(
                                    "Cannot find Gateway id={gateway_id} on the list of \
                                    connected gateways"
                                );
                            }

                            // Drop the per-Gateway reconnect-pairing state.
                            self.clear_disconnect_notification_flag(gateway_id);

                            #[cfg(test)]
                            self.note_gateway_notification_for_tests(gateway_id);
                        }
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
        certs_rx: watch::Receiver<Arc<HashMap<Id, String>>>,
    ) -> Result<AbortHandle, GatewayError> {
        let (events_tx, events_rx) = unbounded_channel();
        self.event_senders
            .lock()
            .expect("Failed to lock GatewayManager::event_senders")
            .insert(gateway.id, events_tx);
        let mut gateway_handler = self.build_handler(gateway, events_rx, certs_rx)?;
        let manager_reconnect_delay = self.manager_reconnect_delay();
        let abort_handle = self.handlers.spawn(async move {
            gateway_handler
                .handle_connection(clients, manager_reconnect_delay)
                .await
        });
        Ok(abort_handle)
    }

    /// Aborts the Gateway's handler and drops its event sender. Returns `false` if no handler was
    /// running.
    fn stop_handler(
        &self,
        gateway_id: Id,
        abort_handles: &mut HashMap<Id, (AbortHandle, Gateway<Id>)>,
    ) -> bool {
        self.event_senders
            .lock()
            .expect("Failed to lock GatewayManager::event_senders")
            .remove(&gateway_id);
        let Some((abort_handle, _)) = abort_handles.remove(&gateway_id) else {
            return false;
        };
        abort_handle.abort();
        true
    }

    fn remove_client(&self, gateway_id: Id) -> Option<Client> {
        self.clients
            .lock()
            .expect("Failed to lock GatewayManager::clients")
            .remove(&gateway_id)
    }
}
