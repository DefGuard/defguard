use defguard_common::gateway_types::{
    FirewallConfig, FirewallPolicy, FirewallRule, IpAddress, IpRange, IpVersion, Port, PortRange,
    Protocol, SnatBinding, WireguardPeer,
};

use crate::{
    enterprise::firewall::{
        FirewallConfig as ProtoFirewallConfig, FirewallPolicy as ProtoFirewallPolicy,
        FirewallRule as ProtoFirewallRule, IpAddress as ProtoIpAddress, IpRange as ProtoIpRange,
        IpVersion as ProtoIpVersion, Port as ProtoPort, PortRange as ProtoPortRange,
        Protocol as ProtoProtocol, SnatBinding as ProtoSnatBinding, ip_address::Address,
        port::Port as PortInner,
    },
    gateway::Peer,
};

impl From<WireguardPeer> for Peer {
    fn from(p: WireguardPeer) -> Self {
        Self {
            pubkey: p.pubkey,
            allowed_ips: p.allowed_ips,
            preshared_key: p.preshared_key,
            keepalive_interval: p.keepalive_interval,
        }
    }
}

fn policy_to_i32(policy: FirewallPolicy) -> i32 {
    match policy {
        FirewallPolicy::Unspecified => ProtoFirewallPolicy::Unspecified as i32,
        FirewallPolicy::Allow => ProtoFirewallPolicy::Allow as i32,
        FirewallPolicy::Deny => ProtoFirewallPolicy::Deny as i32,
    }
}

fn ip_version_to_i32(v: IpVersion) -> i32 {
    match v {
        IpVersion::Unspecified => ProtoIpVersion::Unspecified as i32,
        IpVersion::Ipv4 => ProtoIpVersion::Ipv4 as i32,
        IpVersion::Ipv6 => ProtoIpVersion::Ipv6 as i32,
    }
}

fn protocol_to_i32(p: Protocol) -> i32 {
    match p {
        Protocol::Unspecified => ProtoProtocol::Unspecified as i32,
        Protocol::Icmp => ProtoProtocol::Icmp as i32,
        Protocol::Tcp => ProtoProtocol::Tcp as i32,
        Protocol::Udp => ProtoProtocol::Udp as i32,
    }
}

impl From<IpRange> for ProtoIpRange {
    fn from(r: IpRange) -> Self {
        Self {
            start: r.start,
            end: r.end,
        }
    }
}

impl From<IpAddress> for ProtoIpAddress {
    fn from(addr: IpAddress) -> Self {
        Self {
            address: Some(match addr {
                IpAddress::Ip(ip) => Address::Ip(ip),
                IpAddress::IpRange(r) => Address::IpRange(r.into()),
                IpAddress::IpSubnet(s) => Address::IpSubnet(s),
            }),
        }
    }
}

impl From<PortRange> for ProtoPortRange {
    fn from(r: PortRange) -> Self {
        Self {
            start: r.start,
            end: r.end,
        }
    }
}

impl From<Port> for ProtoPort {
    fn from(p: Port) -> Self {
        Self {
            port: Some(match p {
                Port::Single(n) => PortInner::SinglePort(n),
                Port::Range(r) => PortInner::PortRange(r.into()),
            }),
        }
    }
}

impl From<FirewallRule> for ProtoFirewallRule {
    fn from(r: FirewallRule) -> Self {
        Self {
            id: r.id,
            source_addrs: r.source_addrs.into_iter().map(Into::into).collect(),
            destination_addrs: r.destination_addrs.into_iter().map(Into::into).collect(),
            destination_ports: r.destination_ports.into_iter().map(Into::into).collect(),
            protocols: r.protocols.into_iter().map(protocol_to_i32).collect(),
            verdict: policy_to_i32(r.verdict),
            comment: r.comment,
            ip_version: ip_version_to_i32(r.ip_version),
        }
    }
}

impl From<SnatBinding> for ProtoSnatBinding {
    fn from(b: SnatBinding) -> Self {
        Self {
            id: b.id,
            source_addrs: b.source_addrs.into_iter().map(Into::into).collect(),
            public_ip: b.public_ip,
            comment: b.comment,
        }
    }
}

impl From<FirewallConfig> for ProtoFirewallConfig {
    fn from(c: FirewallConfig) -> Self {
        Self {
            default_policy: policy_to_i32(c.default_policy),
            rules: c.rules.into_iter().map(Into::into).collect(),
            snat_bindings: c.snat_bindings.into_iter().map(Into::into).collect(),
        }
    }
}
