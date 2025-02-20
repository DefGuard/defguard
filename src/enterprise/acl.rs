use std::net::IpAddr;

use ip_address::Address;
use ipnetwork::IpNetwork;
use sqlx::{query_as, query_scalar, Error as SqlxError, PgExecutor, PgPool};

use crate::db::{Id, User, WireguardNetwork};

use super::db::models::acl::AclRule;

tonic::include_proto!("acl");

pub enum Policy {
    Allow,
    Deny,
}

pub enum NetworkAddressingType {
    IpV4,
    IpV6,
}

pub enum DestinationPort {
    SinglePort { port: u16 },
    PortRange { start: u16, end: u16 },
}

pub enum DestinationIp {
    SingleIp(IpAddr),
    IpRange { start: IpAddr, end: IpAddr },
    Subnet(IpNetwork),
}

pub async fn generate_firewall_rules_from_acls(
    location_id: Id,
    default_location_policy: Policy,
    acl_rules: Vec<AclRule<Id>>,
    pool: &PgPool,
) -> Result<Vec<FirewallRule>, SqlxError> {
    let mut rules = Vec::new();

    // we only create rules which are opposite to the default policy
    // for example if by default the firewall denies all traffic it only makes sense to add allow
    // rules
    let firewall_rule_verdict = match default_location_policy {
        Policy::Allow => Verdict::Deny,
        Policy::Deny => Verdict::Accept,
    };
    for acl in acl_rules {
        // fetch allowed users
        let allowed_users = acl.get_all_allowed_users(pool).await?;

        // fetch denied users
        let denied_users = acl.get_all_denied_users(pool).await?;

        // prepare a list of all relevant users whose device IPs we'll need to prepare a firewall
        // rule
        //
        // depending on the default policy we either need:
        // - allowed users if default policy is `Deny`
        // - denied users if default policy is `Allow`
        let users: Vec<User<Id>> = match default_location_policy {
            // start with allowed users and remove those explicitly denied
            Policy::Deny => allowed_users
                .into_iter()
                .filter(|user| !denied_users.contains(user))
                .collect(),
            // start with denied users and remove those explicitly allowed
            Policy::Allow => denied_users
                .into_iter()
                .filter(|user| !allowed_users.contains(user))
                .collect(),
        };
        let user_ids: Vec<Id> = users.iter().map(|user| user.id).collect();

        // get network IPs for devices belonging to those users and convert them to source IPs
        // NOTE: only consider user devices since network devices will be handled separately
        let user_device_ips: Vec<IpAddr> = query_scalar!(
            "SELECT wireguard_ip \"wireguard_ip: IpAddr\" \
            FROM wireguard_network_device wnd \
            JOIN device d ON d.id = wnd.device_id \
            WHERE wnd.wireguard_network_id = $1 AND d.device_type = 'user'::device_type AND d.user_id = ANY($2)",
            location_id,
            &user_ids
        )
        .fetch_all(pool)
        .await?;
        let source_addr = user_device_ips
            .into_iter()
            .map(|ip| match ip {
                IpAddr::V4(ipv4_addr) => IpAddress {
                    version: IpVersion::Ipv4.into(),
                    address: Some(Address::Ip(ipv4_addr.to_string())),
                },
                IpAddr::V6(ipv6_addr) => IpAddress {
                    version: IpVersion::Ipv6.into(),
                    address: Some(Address::Ip(ipv6_addr.to_string())),
                },
            })
            .collect();

        // prepare destination addresses
        let destination_addr = acl
            .destination
            .into_iter()
            .map(|dst| match dst {
                IpNetwork::V4(ipv4_network) => IpAddress {
                    version: IpVersion::Ipv4.into(),
                    address: Some(Address::IpSubnet(ipv4_network.to_string())),
                },
                IpNetwork::V6(ipv6_network) => IpAddress {
                    version: IpVersion::Ipv6.into(),
                    address: Some(Address::IpSubnet(ipv6_network.to_string())),
                },
            })
            .collect();

        // prepare destination ports
        let destination_port = acl
            .ports
            .into_iter()
            .map(|port_range| Port {
                port: Some(port::Port::PortRange(PortRange {
                    start: match port_range.start {
                        std::ops::Bound::Included(s) => s as u32,
                        std::ops::Bound::Excluded(s) => s as u32,
                        std::ops::Bound::Unbounded => 0,
                    },
                    end: match port_range.end {
                        std::ops::Bound::Included(e) => e as u32,
                        std::ops::Bound::Excluded(e) => e as u32,
                        std::ops::Bound::Unbounded => u16::MAX as u32,
                    },
                })),
            })
            .collect();

        // TODO: prepare protocols

        // TODO: process aliases

        // FIXME: actually determine the family
        // determine rule family based on network subnet type
        let family = Family::Ip.into();

        // prepare firewall rule for this ACL
        let firewall_rule = FirewallRule {
            family,
            index: None,
            source_addr,
            destination_addr,
            destination_port,
            protocol: Vec::new(),
            verdict: firewall_rule_verdict.into(),
            comment: None,
        };
        rules.push(firewall_rule)
    }
    Ok(rules)
}

impl WireguardNetwork<Id> {
    /// Fetches all active ACL rules for a given location
    pub(crate) async fn get_active_acl_rules<'e, E>(
        &self,
        executor: E,
    ) -> Result<Vec<AclRule<Id>>, SqlxError>
    where
        E: PgExecutor<'e>,
    {
        query_as!(
            AclRule,
            "SELECT a.id, name, allow_all_users, deny_all_users, all_networks, \
                destination, ports, expires \
                FROM aclrule a \
                JOIN aclrulenetwork an \
                ON a.id = an.rule_id \
                WHERE an.network_id = $1",
            self.id,
        )
        .fetch_all(executor)
        .await
    }

    /// Prepares firewall configuration for a gateway based on location config and ACLs
    pub async fn get_firewall_config(&self, pool: &PgPool) -> Result<(), SqlxError> {
        // fetch all active ACLs for location
        let location_acls = self.get_active_acl_rules(pool).await?;

        // TODO: add default policy to location model
        let firewall_rules =
            generate_firewall_rules_from_acls(self.id, Policy::Deny, location_acls, pool).await?;

        Ok(())
    }
}
