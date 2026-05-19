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
    MfaSessionAuthorized(Id, Device<Id>, DeviceNetworkInfo),
    MfaSessionDisconnected(Id, Device<Id>),
}

/// Sends a [`GatewayCommand`] to the gateway manager service.
///
/// In API handler context prefer `AppState::send_gateway_command`.
pub fn send_gateway_command(event: GatewayCommand, wg_tx: &Sender<GatewayCommand>) {
    debug!("Sending the following command to Gateway Manager: {event:?}");
    if let Err(err) = wg_tx.send(event) {
        error!("Error sending Gateway command: {err}");
    }
}

/// Sends multiple [`GatewayCommand`]s to the gateway manager service.
///
/// In API handler context prefer `AppState::send_multiple_gateway_commands`.
pub fn send_multiple_gateway_commands(events: Vec<GatewayCommand>, wg_tx: &Sender<GatewayCommand>) {
    debug!("Sending {} gateway commands", events.len());
    for event in events {
        send_gateway_command(event, wg_tx);
    }
}
