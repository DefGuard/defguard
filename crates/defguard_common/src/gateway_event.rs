//! Gateway command types and helpers for communicating with the gateway manager service.
//!
//! [`GatewayCommand`] is the primary type sent from core to the gateway manager over
//! an in-process broadcast channel. The gateway manager converts each command to the
//! appropriate protobuf wire message before forwarding it to the gateway daemon.

use tokio::sync::broadcast::Sender;
use tracing::{debug, error};

use crate::{
    db::{
        Id,
        models::{
            Device, WireguardNetwork,
            device::{DeviceInfo, DeviceNetworkInfo},
        },
    },
    gateway_types::{FirewallConfig, WireguardPeer},
};

/// A command sent from core to the gateway manager service.
///
/// Each variant instructs the gateway daemon to update its WireGuard state or
/// firewall configuration. Native Rust types are used throughout; conversion to
/// protobuf wire types happens at the serialization boundary in the gateway manager.
#[derive(Clone, Debug)]
pub enum GatewayCommand {
    NetworkCreated(Id, WireguardNetwork<Id>),
    NetworkModified(
        Id,
        WireguardNetwork<Id>,
        Vec<WireguardPeer>,
        Option<FirewallConfig>,
    ),
    NetworkDeleted(Id, String),
    DeviceCreated(DeviceInfo),
    DeviceModified(DeviceInfo),
    DeviceDeleted(DeviceInfo),
    FirewallConfigChanged(Id, FirewallConfig),
    FirewallDisabled(Id),
    VpnSessionAuthorized(Id, Device<Id>, DeviceNetworkInfo),
    VpnSessionDeauthorized(Id, Device<Id>),
}

/// Sends a [`GatewayCommand`] to the gateway manager service.
///
/// In API handler context prefer `AppState::send_gateway_command`.
pub fn send_gateway_command(command: GatewayCommand, gateway_tx: &Sender<GatewayCommand>) {
    debug!("Sending the following command to Gateway Manager: {command:?}");
    if let Err(err) = gateway_tx.send(command) {
        error!("Error sending gateway command: {err}");
    }
}

/// Sends multiple [`GatewayCommand`]s to the gateway manager service.
///
/// In API handler context prefer `AppState::send_multiple_gateway_commands`.
pub fn send_multiple_gateway_commands(
    commands: Vec<GatewayCommand>,
    gateway_tx: &Sender<GatewayCommand>,
) {
    debug!("Sending {} gateway commands", commands.len());
    for command in commands {
        send_gateway_command(command, gateway_tx);
    }
}
