use defguard_common::db::{
    Id,
    models::{Device, WireguardNetwork, device::DeviceInfo},
};
use defguard_proto::{enterprise::firewall::FirewallConfig, gateway::Peer};

type LocationId = Id;

// TODO: move this to common crate
#[derive(Clone, Debug)]
pub enum GatewayEvent {
    NetworkCreated(LocationId, WireguardNetwork<Id>),
    NetworkModified(
        LocationId,
        WireguardNetwork<Id>,
        Vec<Peer>,
        Option<FirewallConfig>,
    ),
    NetworkDeleted(LocationId, String),
    DeviceCreated(DeviceInfo),
    DeviceModified(DeviceInfo),
    DeviceDeleted(DeviceInfo),
    FirewallConfigChanged(LocationId, FirewallConfig),
    FirewallDisabled(LocationId),
    MfaSessionDisconnected(LocationId, Device<Id>),
}
