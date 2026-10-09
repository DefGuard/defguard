use std::net::IpAddr;

use defguard_common::{
    db::{
        Id,
        models::{DeviceNetworkInfo, WireguardNetwork, wireguard::DEFAULT_WIREGUARD_MTU},
    },
    gateway_event::GatewayCommand,
    gateway_types::{FirewallConfig, WireguardPeer},
};
use defguard_proto::{
    enterprise::firewall::FirewallConfig as ProtoFirewallConfig,
    gateway::{Configuration, CoreResponse, Peer, Update, UpdateType, core_response, update},
};
use sqlx::PgPool;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tonic::{Code, Status};

/// Helper struct for handling gateway events.
pub(crate) struct GatewayUpdatesHandler {
    network_id: Id,
    pub(crate) network: WireguardNetwork<Id>,
    gateway_name: String,
    pool: Option<PgPool>,
    pub(crate) session_authorization_required: bool,
    events_rx: UnboundedReceiver<GatewayCommand>,
    tx: UnboundedSender<CoreResponse>,
}

impl GatewayUpdatesHandler {
    #[must_use]
    pub(crate) fn new(
        network_id: Id,
        network: WireguardNetwork<Id>,
        gateway_name: String,
        pool: Option<PgPool>,
        events_rx: UnboundedReceiver<GatewayCommand>,
        tx: UnboundedSender<CoreResponse>,
    ) -> Self {
        Self {
            network_id,
            network,
            gateway_name,
            pool,
            session_authorization_required: false,
            events_rx,
            tx,
        }
    }

    async fn authorization_required_for(
        network: &WireguardNetwork<Id>,
        pool: Option<&PgPool>,
    ) -> bool {
        if network.mfa_enabled {
            return true;
        }

        let Some(pool) = pool else {
            return false;
        };

        match network.has_postures(pool).await {
            Ok(has_postures) => has_postures,
            Err(err) => {
                error!("Failed to fetch postures for location {network}: {err}");
                false
            }
        }
    }

    async fn refresh_session_authorization_required(&mut self) {
        self.session_authorization_required =
            Self::authorization_required_for(&self.network, self.pool.as_ref()).await;
    }

    #[must_use]
    pub(crate) fn runtime_peer_update(
        &self,
        peer_label: &str,
        peer_pubkey: String,
        allowed_ips: Vec<String>,
        is_authorized: bool,
        preshared_key: Option<String>,
    ) -> Option<Peer> {
        if !self.session_authorization_required {
            return Some(Peer {
                pubkey: peer_pubkey,
                allowed_ips,
                preshared_key: None,
                keepalive_interval: Some(self.network.keepalive_interval.cast_unsigned()),
            });
        }

        if !is_authorized {
            debug!(
                "Skipping gateway peer update for WireGuard device {peer_label} in runtime-authorized location {} because there is no active VPN session",
                self.network.name
            );
            return None;
        }

        let Some(preshared_key) = preshared_key else {
            debug!(
                "Skipping gateway peer update for WireGuard device {peer_label} in location {} because the runtime preshared key is missing",
                self.network.name
            );
            return None;
        };

        Some(Peer {
            pubkey: peer_pubkey,
            allowed_ips,
            preshared_key: Some(preshared_key),
            keepalive_interval: Some(self.network.keepalive_interval.cast_unsigned()),
        })
    }

    fn send_runtime_device_update(
        &self,
        peer_label: &str,
        peer_pubkey: String,
        network_info: &DeviceNetworkInfo,
        update_type: UpdateType,
    ) -> Result<(), Status> {
        let allowed_ips = network_info
            .device_wireguard_ips
            .iter()
            .map(IpAddr::to_string)
            .collect();

        let Some(peer) = self.runtime_peer_update(
            peer_label,
            peer_pubkey,
            allowed_ips,
            network_info.is_authorized,
            network_info.preshared_key.clone(),
        ) else {
            return Ok(());
        };

        self.send_peer_update(peer, update_type)
    }

    /// Process incoming Gateway events
    ///
    /// Every handler receives all events, so it must skip those for other networks.
    pub(crate) async fn run(&mut self) {
        info!(
            "Starting update stream to gateway: {}, network {}",
            self.gateway_name, self.network
        );
        self.refresh_session_authorization_required().await;
        while let Some(update) = self.events_rx.recv().await {
            debug!("Received WireGuard update: {update:?}");
            let result = match update {
                GatewayCommand::NetworkCreated(network_id, network) => {
                    if network_id == self.network_id {
                        self.send_network_update(&network, Vec::new(), None, UpdateType::Create)
                    } else {
                        Ok(())
                    }
                }
                GatewayCommand::NetworkModified(
                    network_id,
                    network,
                    peers,
                    maybe_firewall_config,
                ) => {
                    if network_id == self.network_id {
                        let result = self.send_network_update(
                            &network,
                            peers,
                            maybe_firewall_config,
                            UpdateType::Modify,
                        );
                        // update stored network data
                        self.network = network;
                        self.refresh_session_authorization_required().await;
                        result
                    } else {
                        Ok(())
                    }
                }
                GatewayCommand::NetworkDeleted(network_id, network_name) => {
                    if network_id == self.network_id {
                        self.send_network_delete(&network_name)
                    } else {
                        Ok(())
                    }
                }
                GatewayCommand::DeviceCreated(device) => {
                    // check if a peer has to be added in the current network
                    match device
                        .network_info
                        .iter()
                        .find(|info| info.network_id == self.network_id)
                    {
                        Some(network_info) => self.send_runtime_device_update(
                            &device.device.name,
                            device.device.wireguard_pubkey,
                            network_info,
                            UpdateType::Create,
                        ),
                        None => Ok(()),
                    }
                }
                GatewayCommand::DeviceModified(device) => {
                    // check if a peer has to be updated in the current network
                    match device
                        .network_info
                        .iter()
                        .find(|info| info.network_id == self.network_id)
                    {
                        Some(network_info) => self.send_runtime_device_update(
                            &device.device.name,
                            device.device.wireguard_pubkey,
                            network_info,
                            UpdateType::Modify,
                        ),
                        None => Ok(()),
                    }
                }
                GatewayCommand::DeviceDeleted(device) => {
                    // check if a peer has to be updated in the current network
                    match device
                        .network_info
                        .iter()
                        .find(|info| info.network_id == self.network_id)
                    {
                        Some(_) => self.send_peer_delete(&device.device.wireguard_pubkey),
                        None => Ok(()),
                    }
                }
                GatewayCommand::FirewallConfigChanged(location_id, firewall_config) => {
                    if location_id == self.network_id {
                        self.send_firewall_update(firewall_config)
                    } else {
                        Ok(())
                    }
                }
                GatewayCommand::FirewallDisabled(location_id) => {
                    if location_id == self.network_id {
                        self.send_firewall_disable()
                    } else {
                        Ok(())
                    }
                }
                GatewayCommand::VpnSessionDeauthorized(location_id, device) => {
                    if location_id == self.network_id {
                        self.send_peer_delete(&device.wireguard_pubkey)
                    } else {
                        Ok(())
                    }
                }
                GatewayCommand::VpnSessionAuthorized(location_id, device, network_info) => {
                    if location_id == self.network_id {
                        if network_info.network_id != location_id {
                            error!(
                                "Received VPN authorization success event for location {location_id} with invalid runtime network info: {network_info:?}"
                            );
                            continue;
                        }

                        self.send_runtime_device_update(
                            &device.name,
                            device.wireguard_pubkey,
                            &network_info,
                            UpdateType::Create,
                        )
                    } else {
                        Ok(())
                    }
                }
            };
            if result.is_err() {
                error!(
                    "Closing update steam to gateway: {}, network {}",
                    self.gateway_name, self.network
                );
                break;
            }
        }
    }

    /// Sends updated network configuration
    fn send_network_update(
        &self,
        network: &WireguardNetwork<Id>,
        peers: Vec<WireguardPeer>,
        firewall_config: Option<FirewallConfig>,
        update_type: UpdateType,
    ) -> Result<(), Status> {
        debug!("Sending network update for network {network}");
        let proto_peers = peers.into_iter().map(Into::into).collect();
        let proto_firewall = firewall_config.map(ProtoFirewallConfig::from);
        if let Err(err) = self.tx.send(CoreResponse {
            id: 0,
            payload: Some(core_response::Payload::Update(Update {
                update_type: update_type as i32,
                update: Some(update::Update::Network(Configuration {
                    name: network.name.clone(),
                    private_key: network.prvkey.clone(),
                    addresses: network.address().iter().map(ToString::to_string).collect(),
                    port: network.port.cast_unsigned(),
                    peers: proto_peers,
                    firewall_config: proto_firewall,
                    mtu: network.mtu.cast_unsigned(),
                    fwmark: network.fwmark as u32,
                })),
            })),
        }) {
            let msg = format!(
                "Failed to send network update, network {network}, update type: {update_type:?}, \
                error: {err}",
            );
            error!(msg);
            return Err(Status::new(Code::Internal, msg));
        }
        debug!("Network update sent for network {network}");
        Ok(())
    }

    /// Sends delete network command to gateway
    fn send_network_delete(&self, network_name: &str) -> Result<(), Status> {
        debug!(
            "Sending network delete command for network {}",
            self.network
        );
        if let Err(err) = self.tx.send(CoreResponse {
            id: 0,
            payload: Some(core_response::Payload::Update(Update {
                update_type: UpdateType::Delete as i32,
                update: Some(update::Update::Network(Configuration {
                    name: network_name.to_owned(),
                    private_key: String::new(),
                    addresses: Vec::new(),
                    port: 0,
                    peers: Vec::new(),
                    firewall_config: None,
                    mtu: DEFAULT_WIREGUARD_MTU.cast_unsigned(),
                    fwmark: 0,
                })),
            })),
        }) {
            let msg = format!(
                "Failed to send network update, network {}, update type: 3 (DELETE), error: {err}",
                self.network,
            );
            error!(msg);
            return Err(Status::new(Code::Internal, msg));
        }
        debug!("Network delete command sent for network {}", self.network);
        Ok(())
    }

    /// Send update peer command to gateway
    fn send_peer_update(&self, peer: Peer, update_type: UpdateType) -> Result<(), Status> {
        debug!("Sending peer update for network {}", self.network);
        if let Err(err) = self.tx.send(CoreResponse {
            id: 0,
            payload: Some(core_response::Payload::Update(Update {
                update_type: update_type as i32,
                update: Some(update::Update::Peer(peer)),
            })),
        }) {
            let msg = format!(
                "Failed to send peer update for network {}, update type: {update_type:?}, \
                error: {err}",
                self.network,
            );
            error!(msg);
            return Err(Status::new(Code::Internal, msg));
        }
        debug!("Peer update sent for network {}", self.network);
        Ok(())
    }

    /// Send delete peer command to gateway
    fn send_peer_delete(&self, peer_pubkey: &str) -> Result<(), Status> {
        debug!("Sending peer delete for network {}", self.network);
        if let Err(err) = self.tx.send(CoreResponse {
            id: 0,
            payload: Some(core_response::Payload::Update(Update {
                update_type: UpdateType::Delete as i32,
                update: Some(update::Update::Peer(Peer {
                    pubkey: peer_pubkey.into(),
                    allowed_ips: Vec::new(),
                    preshared_key: None,
                    keepalive_interval: None,
                })),
            })),
        }) {
            let msg = format!(
                "Failed to send peer update for network {}, peer {peer_pubkey}, update type: 3 \
                (DELETE), error: {err}",
                self.network,
            );
            error!(msg);
            return Err(Status::new(Code::Internal, msg));
        }
        debug!("Peer delete command sent for network {}", self.network);
        Ok(())
    }

    /// Send firewall config update command to gateway
    fn send_firewall_update(&self, firewall_config: FirewallConfig) -> Result<(), Status> {
        debug!(
            "Sending firewall config update for network {} with config {firewall_config:?}",
            self.network
        );
        let proto_firewall: ProtoFirewallConfig = firewall_config.into();
        if let Err(err) = self.tx.send(CoreResponse {
            id: 0,
            payload: Some(core_response::Payload::Update(Update {
                update_type: UpdateType::Modify as i32,
                update: Some(update::Update::FirewallConfig(proto_firewall)),
            })),
        }) {
            let msg = format!(
                "Failed to send firewall config update for network {}, error: {err}",
                self.network,
            );
            error!(msg);
            return Err(Status::new(Code::Internal, msg));
        }
        debug!("Firewall config update sent for network {}", self.network);
        Ok(())
    }

    /// Send firewall disable command to gateway
    fn send_firewall_disable(&self) -> Result<(), Status> {
        debug!(
            "Sending firewall disable command for network {}",
            self.network
        );
        if let Err(err) = self.tx.send(CoreResponse {
            id: 0,
            payload: Some(core_response::Payload::Update(Update {
                update_type: UpdateType::Delete as i32,
                update: Some(update::Update::DisableFirewall(())),
            })),
        }) {
            let msg = format!(
                "Failed to send firewall disable command for network {}, error: {err}",
                self.network,
            );
            error!(msg);
            return Err(Status::new(Code::Internal, msg));
        }
        debug!("Firewall disable command sent for network {}", self.network);
        Ok(())
    }
}
