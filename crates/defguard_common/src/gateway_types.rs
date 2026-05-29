//! Native Rust types for data carried in [`crate::gateway_event::GatewayCommand`] variants.
//!
//! These are domain types; conversion to protobuf wire types happens at the
//! serialization boundary (gateway manager) via `From` impls in `defguard_proto`.

use crate::db::Id;

/// A WireGuard peer entry to be configured on a gateway.
#[derive(Clone, Debug, PartialEq)]
pub struct WireguardPeer {
    pub pubkey: String,
    pub allowed_ips: Vec<String>,
    pub preshared_key: Option<String>,
    pub keepalive_interval: Option<u32>,
}

/// Default firewall action applied to traffic that does not match any rule.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum FirewallPolicy {
    #[default]
    Unspecified,
    Allow,
    Deny,
}

/// IP protocol version a firewall rule applies to.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum IpVersion {
    #[default]
    Unspecified,
    Ipv4,
    Ipv6,
}

/// Network protocol matched by a firewall rule.
#[derive(Clone, Debug, PartialEq)]
pub enum Protocol {
    Unspecified,
    Icmp,
    Tcp,
    Udp,
}

impl From<i32> for Protocol {
    fn from(v: i32) -> Self {
        match v {
            1 => Self::Icmp,
            6 => Self::Tcp,
            17 => Self::Udp,
            _ => Self::Unspecified,
        }
    }
}

/// An inclusive range of IP addresses.
#[derive(Clone, Debug, PartialEq)]
pub struct IpRange {
    pub start: String,
    pub end: String,
}

/// An IP address, range, or subnet.
#[derive(Clone, Debug, PartialEq)]
pub enum IpAddress {
    /// Single IP address string (e.g. `"10.0.0.1"`).
    Ip(String),
    /// Inclusive IP range.
    IpRange(IpRange),
    /// IP subnet in CIDR notation (e.g. `"10.0.0.0/24"`).
    IpSubnet(String),
}

/// An inclusive range of port numbers.
#[derive(Clone, Debug, PartialEq)]
pub struct PortRange {
    pub start: u32,
    pub end: u32,
}

/// A single port or an inclusive port range matched by a firewall rule.
#[derive(Clone, Debug, PartialEq)]
pub enum Port {
    Single(u32),
    Range(PortRange),
}

/// A single ACL-derived firewall rule to be enforced on a gateway.
#[derive(Clone, Debug, PartialEq)]
pub struct FirewallRule {
    pub id: Id,
    pub source_addrs: Vec<IpAddress>,
    pub destination_addrs: Vec<IpAddress>,
    pub destination_ports: Vec<Port>,
    pub protocols: Vec<Protocol>,
    pub verdict: FirewallPolicy,
    pub comment: Option<String>,
    pub ip_version: IpVersion,
}

/// Source NAT binding that rewrites the source IP of matching VPN traffic.
#[derive(Clone, Debug, PartialEq)]
pub struct SnatBinding {
    pub id: Id,
    pub source_addrs: Vec<IpAddress>,
    pub public_ip: String,
    pub comment: Option<String>,
}

/// Full firewall configuration to be applied to a gateway location.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FirewallConfig {
    pub default_policy: FirewallPolicy,
    pub rules: Vec<FirewallRule>,
    pub snat_bindings: Vec<SnatBinding>,
}
