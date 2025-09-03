use std::collections::HashMap;

use chrono::Utc;
use defguard_version::tracing::VersionInfo;
use semver::Version;
use sqlx::PgPool;
use thiserror::Error;
use tokio::sync::mpsc::UnboundedSender;
use uuid::Uuid;

use super::state::GatewayState;
use crate::{db::Id, mail::Mail};

/// Helper struct used to handle gateway state. Gateways are grouped by network.
type GatewayHostname = String;
#[derive(Debug, Serialize)]
pub struct GatewayMap(HashMap<Id, HashMap<GatewayHostname, GatewayState>>);

#[derive(Debug, Error)]
pub enum GatewayMapError {
    #[error("Gateway {1} for network {0} not found")]
    NotFound(i64, GatewayHostname),
    #[error("Network {0} not found")]
    NetworkNotFound(i64),
    #[error("Gateway with UID {0} not found")]
    UidNotFound(Uuid),
    #[error("Cannot remove. Gateway with UID {0} is still active")]
    RemoveActive(Uuid),
    #[error("Config missing")]
    ConfigError,
    #[error("Failed to get current settings")]
    SettingsError,
}

impl GatewayMap {
    #[must_use]
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Add a new gateway to the map.
    /// This is meant to be called when Gateway requests a config as a sort of "registration".
    pub(crate) fn add_gateway(
        &mut self,
        network_id: Id,
        network_name: &str,
        hostname: String,
        name: Option<String>,
        mail_tx: UnboundedSender<Mail>,
        version: Version,
    ) {
        info!("Adding gateway {hostname} with to gateway map for network {network_id}",);
        let gateway_state =
            GatewayState::new(network_id, network_name, &hostname, name, mail_tx, version);

        if let Some(network_gateway_map) = self.0.get_mut(&network_id) {
            network_gateway_map.entry(hostname).or_insert(gateway_state);
        } else {
            // no map for a given network exists yet
            let mut network_gateway_map = HashMap::new();
            network_gateway_map.insert(hostname, gateway_state);
            self.0.insert(network_id, network_gateway_map);
        }
    }

    /// Remove gateway from the map.
    pub(crate) fn remove_gateway(
        &mut self,
        network_id: Id,
        uid: Uuid,
    ) -> Result<(), GatewayMapError> {
        debug!("Removing gateway from network {network_id}");
        if let Some(network_gateway_map) = self.0.get_mut(&network_id) {
            // find gateway by uuid
            let hostname = match network_gateway_map
                .iter()
                .find(|(_address, state)| state.uid == uid)
            {
                None => {
                    error!("Failed to find gateway with UID {uid}");
                    return Err(GatewayMapError::UidNotFound(uid));
                }
                Some((hostname, state)) => {
                    if state.connected {
                        error!("Cannot remove. Gateway with UID {uid} is still active");
                        return Err(GatewayMapError::RemoveActive(uid));
                    }
                    hostname.clone()
                }
            };
            // remove matching gateway
            network_gateway_map.remove(&hostname)
        } else {
            // no map for a given network exists yet
            error!("Network {network_id} not found in gateway map");
            return Err(GatewayMapError::NetworkNotFound(network_id));
        };
        info!("Gateway with UID {uid} removed from network {network_id}");
        Ok(())
    }

    /// Change gateway status to connected.
    /// Assume that the gateway is already present in the map.
    pub(crate) fn connect_gateway(
        &mut self,
        network_id: Id,
        hostname: &str,
        pool: &PgPool,
    ) -> Result<(), GatewayMapError> {
        debug!("Connecting gateway {hostname} in network {network_id}");
        if let Some(network_gateway_map) = self.0.get_mut(&network_id) {
            if let Some(state) = network_gateway_map.get_mut(hostname) {
                // check if a gateway is reconnecting to avoid sending notifications on initial
                // connection
                let is_reconnecting = state.disconnected_at.is_some();
                state.connected = true;
                state.disconnected_at = None;
                state.connected_at = Some(Utc::now().naive_utc());
                state.cancel_pending_disconnect_notification();
                if is_reconnecting {
                    state.handle_reconnect_notification(pool);
                }
                debug!(
                    "Gateway {hostname} found in gateway map, current state: {:?}",
                    state
                );
            } else {
                error!("Gateway {hostname} not found in gateway map for network {network_id}");
                return Err(GatewayMapError::NotFound(network_id, hostname.into()));
            }
        } else {
            // no map for a given network exists yet
            error!("Network {network_id} not found in gateway map");
            return Err(GatewayMapError::NetworkNotFound(network_id));
        }
        info!("Gateway {hostname} connected in network {network_id}");
        Ok(())
    }

    /// Change gateway status to disconnected.
    pub(crate) fn disconnect_gateway(
        &mut self,
        network_id: Id,
        hostname: String,
        pool: &PgPool,
    ) -> Result<(), GatewayMapError> {
        debug!("Disconnecting gateway {hostname} in network {network_id}");
        if let Some(network_gateway_map) = self.0.get_mut(&network_id) {
            if let Some(state) = network_gateway_map.get_mut(&hostname) {
                state.connected = false;
                state.disconnected_at = Some(Utc::now().naive_utc());
                state.handle_disconnect_notification(pool);
                debug!("Gateway {hostname} found in gateway map, current state: {state:?}");
                info!("Gateway {hostname} disconnected in network {network_id}");
                return Ok(());
            }
        }
        let err = GatewayMapError::NotFound(network_id, hostname);
        error!("Gateway disconnect failed: {err}");
        Err(err)
    }

    /// Return `true` if at least one gateway in a given network is connected.
    #[must_use]
    pub(crate) fn connected(&self, network_id: Id) -> bool {
        match self.0.get(&network_id) {
            Some(network_gateway_map) => network_gateway_map
                .values()
                .any(|gateway| gateway.connected),
            None => false,
        }
    }

    /// Return a list of all statuses of all gateways for a given network.
    #[must_use]
    pub fn get_network_gateway_status(&self, network_id: Id) -> Vec<GatewayState> {
        if let Some(network_gateway_map) = self.0.get(&network_id) {
            network_gateway_map.values().cloned().collect()
        } else {
            Vec::new()
        }
    }

    /// Flattens the inner `HashMap` into `Vec`.
    ///
    /// Since key information in inner HashMap is within `GatewayState` it's simpler to consume it
    /// as `Vec` on web.
    ///
    /// # Returns
    /// `HashMap<i64, Vec<GatewayState>>` from `GatewayMap`
    #[must_use]
    pub(crate) fn as_flattened(&self) -> HashMap<Id, Vec<GatewayState>> {
        self.0
            .iter()
            .map(|(id, inner_map)| {
                let states: Vec<GatewayState> = inner_map.values().cloned().collect();
                (*id, states)
            })
            .collect()
    }

    #[must_use]
    pub(crate) fn all_states_as_version_info(&self) -> Vec<VersionInfo> {
        self.0
            .values()
            .flat_map(|inner_map| inner_map.values().map(GatewayState::as_version_info))
            .collect()
    }
}

impl Default for GatewayMap {
    fn default() -> Self {
        Self::new()
    }
}
