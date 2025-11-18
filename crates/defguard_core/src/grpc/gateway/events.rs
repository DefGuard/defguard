use defguard_common::db::Id;
use defguard_proto::{enterprise::firewall::FirewallConfig, gateway::Peer};

use crate::db::{WireguardNetwork, models::device::DeviceInfo};

#[derive(Clone, Debug)]
pub enum GatewayEvent {
    NetworkCreated(Id, WireguardNetwork<Id>),
    NetworkModified(Id, WireguardNetwork<Id>, Vec<Peer>, Option<FirewallConfig>),
    NetworkDeleted(Id, String),
    DeviceCreated(DeviceInfo),
    DeviceModified(DeviceInfo),
    DeviceDeleted(DeviceInfo),
    FirewallConfigChanged(Id, FirewallConfig),
    FirewallDisabled(Id),
}
