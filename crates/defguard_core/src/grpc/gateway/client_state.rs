use std::collections::HashMap;

use thiserror::Error;
use tonic::Status;

use crate::db::{Device, Id};

#[derive(Debug, Error)]
pub enum ClientMapError {
    #[error("VPN client {public_key} is already connected to location {location_id}")]
    ClientAlreadyConnected { public_key: String, location_id: Id },
}

impl From<ClientMapError> for Status {
    fn from(value: ClientMapError) -> Self {
        todo!()
    }
}

/// Represents current information about a connected VPN client
#[derive(Debug, Serialize, Clone)]
pub struct ClientState {
    pub device: Device<Id>,
}

impl ClientState {
    pub fn new(device: Device<Id>) -> Self {
        Self { device }
    }
    pub fn update_client_state(&mut self, new_device: Device<Id>) {
        self.device = new_device;
    }
}

/// Helper struct used to handle connected VPN clients state
/// Clients are grouped by location ID
type ClientPubKey = String;
#[derive(Debug, Serialize, Clone)]
pub struct ClientMap(HashMap<Id, HashMap<ClientPubKey, ClientState>>);

impl ClientMap {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn get_vpn_client(
        &mut self,
        location_id: Id,
        client_pubkey: &str,
    ) -> Option<&mut ClientState> {
        self.0
            .get_mut(&location_id)
            .map(|location_map| location_map.get_mut(client_pubkey))
            .flatten()
    }

    pub fn connect_vpn_client(
        &mut self,
        location_id: Id,
        gateway_hostname: String,
        public_key: String,
        device: Device<Id>,
    ) -> Result<(), ClientMapError> {
        info!(
            "VPN client {} connected to location {location_id} through gateway {gateway_hostname}",
            device.name
        );

        // initialize location map if it doesn't exist yet
        let location_map = match self.0.get_mut(&location_id) {
            Some(location_map) => location_map,
            None => {
                // initialize new map for location and immediately return a mutable reference
                self.0.insert(location_id, HashMap::new());
                self.0.get_mut(&location_id).unwrap()
            }
        };

        // check if client is already connected
        if location_map.contains_key(&public_key) {
            return Err(ClientMapError::ClientAlreadyConnected {
                public_key,
                location_id,
            });
        };

        // add client state to location map
        let client_state = ClientState::new(device);
        location_map.insert(public_key, client_state);

        Ok(())
    }

    pub fn disconnect_vpn_client(&self, location_id: Id, device: Device<Id>) {
        info!(
            "VPN client {} disconnected from location {location_id}",
            device.name
        );
        unimplemented!()
    }
}
