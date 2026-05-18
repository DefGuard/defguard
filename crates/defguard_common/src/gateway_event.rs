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
pub enum GatewayEvent {
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

/// Sends a [`GatewayEvent`] to the gateway manager service.
///
/// In API handler context prefer `AppState::send_wireguard_event`.
pub fn send_wireguard_event(event: GatewayEvent, wg_tx: &Sender<GatewayEvent>) {
    debug!("Sending the following WireGuard event to Defguard Gateway: {event:?}");
    if let Err(err) = wg_tx.send(event) {
        error!("Error sending WireGuard event {err}");
    }
}

/// Sends multiple [`GatewayEvent`]s to the gateway manager service.
///
/// In API handler context prefer `AppState::send_multiple_wireguard_events`.
pub fn send_multiple_wireguard_events(events: Vec<GatewayEvent>, wg_tx: &Sender<GatewayEvent>) {
    debug!("Sending {} WireGuard events", events.len());
    for event in events {
        send_wireguard_event(event, wg_tx);
    }
}
