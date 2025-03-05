use std::net::IpAddr;

use ipnetwork::{IpNetwork, Ipv6Network};
use sqlx::{query_as, query_scalar, Error as SqlxError, PgPool};

use super::db::models::acl::{
    AclAliasDestinationRange, AclRule, AclRuleDestinationRange, AclRuleInfo, PortRange,
};

use crate::{
    db::{models::error::ModelError, Device, Id, User, WireguardNetwork},
    grpc::proto::enterprise::firewall::{
        ip_address::Address, port::Port as PortInner, FirewallConfig, FirewallPolicy, FirewallRule,
        IpAddress, IpRange, IpVersion, Port, PortRange as PortRangeProto,
    },
};

#[derive(Debug, thiserror::Error)]
pub enum FirewallError {
    #[error("Database error")]
    DbError(#[from] sqlx::Error),
    #[error(transparent)]
    ModelError(#[from] ModelError),
}

pub async fn generate_firewall_rules_from_acls(
    location_id: Id,
    default_location_policy: FirewallPolicy,
    ip_version: IpVersion,
    acl_rules: Vec<AclRuleInfo<Id>>,
    pool: &PgPool,
) -> Result<Vec<FirewallRule>, FirewallError> {
    debug!("Generating firewall rules for location {location_id} with default policy {default_location_policy:?} and IP version {ip_version:?}");

    // initialize empty result Vec
    let mut firewall_rules = Vec::new();

    // we only create rules which are opposite to the default policy
    // for example if by default the firewall denies all traffic it only makes sense to add allow
    // rules
    let firewall_rule_verdict = match default_location_policy {
        FirewallPolicy::Allow => FirewallPolicy::Deny,
        FirewallPolicy::Deny => FirewallPolicy::Allow,
    };

    // convert each ACL into a corresponding `FirewallRule`
    for acl in acl_rules {
        debug!("Processing ACL rule: {acl:?}");
        // fetch allowed users
        let allowed_users = acl.get_all_allowed_users(pool).await?;

        // fetch denied users
        let denied_users = acl.get_all_denied_users(pool).await?;

        // get relevant users for determining source IPs
        let users = get_source_users(allowed_users, denied_users, default_location_policy);

        // get network IPs for devices belonging to those users
        let user_device_ips = get_user_device_ips(&users, location_id, pool).await?;

        // get network device IPs for rule source
        let network_devices = get_source_network_devices(
            acl.allowed_devices,
            acl.denied_devices,
            default_location_policy,
        );
        let network_device_ips =
            get_network_device_ips(&network_devices, location_id, pool).await?;

        // convert device IPs into source addresses for a firewall rule
        let source_addrs = get_source_addrs(user_device_ips, network_device_ips, ip_version);

        // extract destination parameters from ACL rule
        let AclRuleInfo {
            mut destination,
            destination_ranges,
            mut ports,
            mut protocols,
            aliases,
            ..
        } = acl;

        // store alias ranges separately since they use a different struct
        let mut alias_destination_ranges = Vec::new();

        // process aliases by appending destination parameters from each of them to existing lists
        for alias in aliases {
            // fetch destination ranges for a fiven alias
            alias_destination_ranges.extend(alias.get_destination_ranges(pool).await?);

            // extend existing parameter lists
            destination.extend(alias.destination);
            ports.extend(
                alias
                    .ports
                    .into_iter()
                    .map(|port_range| port_range.into())
                    .collect::<Vec<PortRange>>(),
            );
            protocols.extend(alias.protocols);
        }

        // prepare destination addresses
        let destination_addrs = process_destination_addrs(
            destination,
            destination_ranges,
            alias_destination_ranges,
            ip_version,
        );

        // prepare destination ports
        let destination_ports = merge_port_ranges(ports);

        // remove duplicates protocol entries
        protocols.sort();
        protocols.dedup();

        // prepare comment
        let comment = Some(format!("ACL {} - {}", acl.id, acl.name));

        // prepare firewall rule for this ACL
        let firewall_rule = FirewallRule {
            id: acl.id,
            source_addrs,
            destination_addrs,
            destination_ports,
            protocols,
            verdict: firewall_rule_verdict.into(),
            comment,
        };
        debug!("Firewall rule generated from ACL: {firewall_rule:?}");
        firewall_rules.push(firewall_rule)
    }
    Ok(firewall_rules)
}

/// Prepares a list of all relevant users whose device IPs we'll need to prepare
/// source config for a firewall rule.
///
/// Depending on the default policy we either need:
/// - allowed users if default policy is `Deny`
/// - denied users if default policy is `Allow`
fn get_source_users(
    allowed_users: Vec<User<Id>>,
    denied_users: Vec<User<Id>>,
    default_location_policy: FirewallPolicy,
) -> Vec<User<Id>> {
    match default_location_policy {
        // start with allowed users and remove those explicitly denied
        FirewallPolicy::Deny => allowed_users
            .into_iter()
            .filter(|user| !denied_users.contains(user))
            .collect(),
        // start with denied users and remove those explicitly allowed
        FirewallPolicy::Allow => denied_users
            .into_iter()
            .filter(|user| !allowed_users.contains(user))
            .collect(),
    }
}

/// Fetches all IPs of devices belonging to specified users within a given location's VPN subnet.
// We specifically only fetch user devices since network devices are handled separately.
async fn get_user_device_ips(
    users: &[User<Id>],
    location_id: Id,
    pool: &PgPool,
) -> Result<Vec<IpAddr>, SqlxError> {
    // prepeare a list of user IDs
    let user_ids: Vec<Id> = users.iter().map(|user| user.id).collect();

    // fetch network IPs
    query_scalar!(
            "SELECT wireguard_ip \"wireguard_ip: IpAddr\" \
            FROM wireguard_network_device wnd \
            JOIN device d ON d.id = wnd.device_id \
            WHERE wnd.wireguard_network_id = $1 AND d.device_type = 'user'::device_type AND d.user_id = ANY($2)",
            location_id,
            &user_ids
        )
        .fetch_all(pool)
        .await
}

/// Prepares a list of all relevant network devices whose IPs we'll need to prepare
/// source config for a firewall rule.
///
/// Depending on the default policy we either need:
/// - allowed devices if default policy is `Deny`
/// - denied devices if default policy is `Allow`
fn get_source_network_devices(
    allowed_devices: Vec<Device<Id>>,
    denied_devices: Vec<Device<Id>>,
    default_location_policy: FirewallPolicy,
) -> Vec<Device<Id>> {
    match default_location_policy {
        // start with allowed devices and remove those explicitly denied
        FirewallPolicy::Deny => allowed_devices
            .into_iter()
            .filter(|device| !denied_devices.contains(device))
            .collect(),
        // start with denied devices and remove those explicitly allowed
        FirewallPolicy::Allow => denied_devices
            .into_iter()
            .filter(|device| !allowed_devices.contains(device))
            .collect(),
    }
}

/// Fetches all IPs of specified network devices within a given location's VPN subnet.
async fn get_network_device_ips(
    network_devices: &[Device<Id>],
    location_id: Id,
    pool: &PgPool,
) -> Result<Vec<IpAddr>, SqlxError> {
    // prepare a list of IDs
    let network_device_ids: Vec<Id> = network_devices.iter().map(|device| device.id).collect();

    // fetch network IPs
    query_scalar!(
        "SELECT wireguard_ip \"wireguard_ip: IpAddr\" \
            FROM wireguard_network_device wnd \
            WHERE wnd.wireguard_network_id = $1 AND wnd.device_id = ANY($2)",
        location_id,
        &network_device_ids,
    )
    .fetch_all(pool)
    .await
}

/// Combines user device IPs and network device IPs into a list of source addresses which can be
/// used by a firewall rule.
fn get_source_addrs(
    user_device_ips: Vec<IpAddr>,
    network_device_ips: Vec<IpAddr>,
    ip_version: IpVersion,
) -> Vec<IpAddress> {
    // combine both lists into a single iterator
    let source_ips = user_device_ips.into_iter().chain(network_device_ips);

    // prepare source addrs by removing incompatible IP version elements
    // and converting them to expected gRPC format
    let source_addrs = source_ips
        .filter_map(|ip| match ip_version {
            IpVersion::Ipv4 => {
                if ip.is_ipv4() {
                    Some((ip, ip))
                } else {
                    None
                }
            }
            IpVersion::Ipv6 => {
                if ip.is_ipv6() {
                    Some((ip, ip))
                } else {
                    None
                }
            }
        })
        .collect();

    // merge address ranges into non-overlapping elements
    merge_addrs(source_addrs)
}

/// Convert destination networks and ranges configured in an ACL rule
/// into the correct format for a firewall rule. This includes:
/// - combining both lists
/// - converting to gRPC IpAddress struct
/// - filtering out incompatible IP version
/// - merging into the smallest possible list of non-overlapping ranges,
///   subnets and addresses
fn process_destination_addrs(
    destination_ips: Vec<IpNetwork>,
    destination_ranges: Vec<AclRuleDestinationRange<Id>>,
    alias_destination_ranges: Vec<AclAliasDestinationRange<Id>>,
    ip_version: IpVersion,
) -> Vec<IpAddress> {
    // filter out destinations with incompatible IP version and convert to intermediate
    // tuple representation for merging
    let destination_iterator = destination_ips.iter().filter_map(|dst| match ip_version {
        IpVersion::Ipv4 => {
            if dst.is_ipv4() {
                Some((dst.network(), dst.broadcast()))
            } else {
                None
            }
        }
        IpVersion::Ipv6 => {
            if let IpNetwork::V6(subnet) = dst {
                let range_start = subnet.network().into();
                let range_end = get_last_ip_in_v6_subnet(subnet);
                Some((range_start, range_end))
            } else {
                None
            }
        }
    });

    // filter out destination ranges with incompatible IP version and convert to intermediate
    // tuple representation for merging
    let destination_range_iterator = destination_ranges
        .iter()
        .filter_map(|dst| match ip_version {
            IpVersion::Ipv4 => {
                if dst.start.is_ipv4() && dst.end.is_ipv4() {
                    Some((dst.start, dst.end))
                } else {
                    None
                }
            }
            IpVersion::Ipv6 => {
                if dst.start.is_ipv6() && dst.end.is_ipv6() {
                    Some((dst.start, dst.end))
                } else {
                    None
                }
            }
        });
    let alias_destination_range_iterator =
        alias_destination_ranges
            .iter()
            .filter_map(|dst| match ip_version {
                IpVersion::Ipv4 => {
                    if dst.start.is_ipv4() && dst.end.is_ipv4() {
                        Some((dst.start, dst.end))
                    } else {
                        None
                    }
                }
                IpVersion::Ipv6 => {
                    if dst.start.is_ipv6() && dst.end.is_ipv6() {
                        Some((dst.start, dst.end))
                    } else {
                        None
                    }
                }
            });

    // combine both iterators to return a single list
    let destination_addrs = destination_iterator
        .chain(destination_range_iterator)
        .chain(alias_destination_range_iterator)
        .collect();

    // merge address ranges into non-overlapping elements
    merge_addrs(destination_addrs)
}

fn get_last_ip_in_v6_subnet(subnet: &Ipv6Network) -> IpAddr {
    // get subnet IP portion as u128
    let first_ip = subnet.ip().to_bits();

    let last_ip = first_ip | (!u128::from(subnet.mask()));

    IpAddr::V6(last_ip.into())
}

/// Converts an arbitrary list of ip address ranges into the smallest possible list
/// of non-overlapping elements which can be used in a firewall rule.
/// It assumes that all ranges with an invalid IP version have already been filtered out.
fn merge_addrs(addr_ranges: Vec<(IpAddr, IpAddr)>) -> Vec<IpAddress> {
    // merge into non-overlapping ranges
    let addr_ranges = merge_ranges(addr_ranges);

    // convert to gRPC format
    let mut result = Vec::new();
    for (range_start, range_end) in addr_ranges {
        if range_start == range_end {
            // single IP address
            result.push(IpAddress {
                address: Some(Address::Ip(range_start.to_string())),
            });
        } else {
            // TODO: find largest subnet in range
            // address range
            result.push(IpAddress {
                address: Some(Address::IpRange(IpRange {
                    start: range_start.to_string(),
                    end: range_end.to_string(),
                })),
            });
        }
    }

    result
}

// Returns the largest subnet in given address range and the remaining address range.
// TODO: figure out an implementation
fn find_largest_subnet_in_range(
    range_start: IpAddr,
    range_end: IpAddr,
) -> (Option<IpNetwork>, Option<(IpAddr, IpAddr)>) {
    todo!()
}

/// Takes a list of port ranges and returns the smallest possible non-overlapping list of `Port`s.
fn merge_port_ranges(port_ranges: Vec<PortRange>) -> Vec<Port> {
    // convert ranges to a list of tuples for merging
    let port_ranges = port_ranges
        .into_iter()
        .map(|range| (range.start(), range.end()))
        .collect();

    // merge into non-overlapping ranges
    let port_ranges = merge_ranges(port_ranges);

    // convert resulting ranges into gRPC format
    port_ranges
        .into_iter()
        .map(|(range_start, range_end)| {
            if range_start == range_end {
                Port {
                    port: Some(PortInner::SinglePort(range_start as u32)),
                }
            } else {
                Port {
                    port: Some(PortInner::PortRange(PortRangeProto {
                        start: range_start as u32,
                        end: range_end as u32,
                    })),
                }
            }
        })
        .collect()
}

/// Helper function which implements merging a set of ranges of arbitrary elements
/// into the smallest possible set of non-overlapping ranges.
/// It can then be reused for merging port and address ranges.
fn merge_ranges<T: Ord>(mut ranges: Vec<(T, T)>) -> Vec<(T, T)> {
    // return early if list is empty
    if ranges.is_empty() {
        return Vec::new();
    }

    // sort elements by range start
    ranges.sort_by(|a, b| {
        let a_start = &a.0;
        let b_start = &b.0;
        a_start.cmp(b_start)
    });

    // initialize result vector
    let mut merged_ranges = Vec::new();

    // start with first range
    let current_range = ranges.remove(0);
    let mut current_range_start = current_range.0;
    let mut current_range_end = current_range.1;

    // iterate over remaining ranges
    for range in ranges {
        let range_start = range.0;
        let range_end = range.1;

        // compare with current range
        if range_start <= current_range_end {
            // ranges are overlapping, merge them
            // if range is not contained within current range
            if range_end > current_range_end {
                current_range_end = range_end;
            }
        } else {
            // ranges are not overlapping, add current range to result
            merged_ranges.push((current_range_start, current_range_end));
            current_range_start = range_start;
            current_range_end = range_end;
        }
    }

    // add last remaining range
    merged_ranges.push((current_range_start, current_range_end));

    // return resulting list
    merged_ranges
}

impl WireguardNetwork<Id> {
    /// Fetches all active ACL rules for a given location.
    /// Filters out rules which are disabled, expired or have not been deployed yet.
    /// TODO: actually filter out unwanted rules once drafts etc are implemented
    pub(crate) async fn get_active_acl_rules(
        &self,
        pool: &PgPool,
    ) -> Result<Vec<AclRuleInfo<Id>>, SqlxError> {
        debug!("Fetching active ACL rules for location {self}");
        let rules = query_as!(
            AclRule,
            "SELECT a.id, name, allow_all_users, deny_all_users, all_networks, \
                destination, ports, protocols, expires \
                FROM aclrule a \
                JOIN aclrulenetwork an \
                ON a.id = an.rule_id \
                WHERE an.network_id = $1",
            self.id,
        )
        .fetch_all(pool)
        .await?;

        // convert to `AclRuleInfo`
        let mut rules_info = Vec::new();
        for rule in rules {
            let rule_info = rule.to_info(pool).await?;
            rules_info.push(rule_info);
        }
        Ok(rules_info)
    }

    /// Prepares firewall configuration for a gateway based on location config and ACLs
    /// Returns `None` if firewall management is disabled for a given location.
    /// TODO: actually determine if a config should be generated
    pub async fn try_get_firewall_config(
        &self,
        pool: &PgPool,
    ) -> Result<Option<FirewallConfig>, FirewallError> {
        info!("Generating firewall config for location {self}");
        // fetch all active ACLs for location
        let location_acls = self.get_active_acl_rules(pool).await?;
        debug!(
            "Found {0} active ACL rules for location {self}",
            location_acls.len()
        );

        // determine IP version based on location subnet
        let ip_version = self.get_ip_version();

        // FIXME: add default policy to location model
        let default_policy = FirewallPolicy::Deny;
        let firewall_rules = generate_firewall_rules_from_acls(
            self.id,
            default_policy,
            ip_version,
            location_acls,
            pool,
        )
        .await?;

        let firewall_config = FirewallConfig {
            ip_version: ip_version.into(),
            default_policy: default_policy.into(),
            rules: firewall_rules,
        };

        debug!("Firewall config generated for location {self}: {firewall_config:?}");
        Ok(Some(firewall_config))
    }

    fn get_ip_version(&self) -> IpVersion {
        // get the subnet from which device IPs are being assigned
        // by default only the first configured subnet is being used
        let vpn_subnet = self
            .address
            .first()
            .expect("WireguardNetwork must have an address");

        match vpn_subnet {
            IpNetwork::V4(_ipv4_network) => IpVersion::Ipv4,
            IpNetwork::V6(_ipv6_network) => IpVersion::Ipv6,
        }
    }
}

#[cfg(test)]
mod test {
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

    use ipnetwork::Ipv6Network;
    use rand::{thread_rng, Rng};

    use crate::{
        db::{models::device::DeviceType, Device, Id, User},
        enterprise::{db::models::acl::PortRange, firewall::get_source_network_devices},
        grpc::proto::enterprise::firewall::{
            ip_address::Address, port::Port as PortInner, FirewallPolicy, IpAddress, IpRange, Port,
            PortRange as PortRangeProto,
        },
    };

    use super::{get_last_ip_in_v6_subnet, get_source_users, merge_addrs, merge_port_ranges};

    fn random_user_with_id<R: Rng>(rng: &mut R, id: Id) -> User<Id> {
        let mut user: User<Id> = rng.gen();
        user.id = id;
        user
    }

    fn random_network_device_with_id<R: Rng>(rng: &mut R, id: Id) -> Device<Id> {
        let mut device: Device<Id> = rng.gen();
        device.id = id;
        device.device_type = DeviceType::Network;
        device
    }

    #[test]
    fn test_get_relevant_users() {
        let mut rng = thread_rng();

        // prepare allowed and denied users lists with shared elements
        let user_1 = random_user_with_id(&mut rng, 1);
        let user_2 = random_user_with_id(&mut rng, 2);
        let user_3 = random_user_with_id(&mut rng, 3);
        let user_4 = random_user_with_id(&mut rng, 4);
        let user_5 = random_user_with_id(&mut rng, 5);
        let allowed_users = vec![user_1.clone(), user_2.clone(), user_4.clone()];
        let denied_users = vec![user_3.clone(), user_4, user_5.clone()];

        // default policy is `Allow`, so we should get explicitly denied users
        let users = get_source_users(
            allowed_users.clone(),
            denied_users.clone(),
            FirewallPolicy::Allow,
        );
        assert_eq!(users, vec![user_3, user_5]);
        //
        // default policy is `Deny`, so we should get explicitly allowed users
        let users = get_source_users(allowed_users, denied_users, FirewallPolicy::Deny);
        assert_eq!(users, vec![user_1, user_2]);
    }

    #[test]
    fn test_get_relevant_network_devices() {
        let mut rng = thread_rng();

        // prepare allowed and denied network devices lists with shared elements
        let device_1 = random_network_device_with_id(&mut rng, 1);
        let device_2 = random_network_device_with_id(&mut rng, 2);
        let device_3 = random_network_device_with_id(&mut rng, 3);
        let device_4 = random_network_device_with_id(&mut rng, 4);
        let device_5 = random_network_device_with_id(&mut rng, 5);
        let allowed_devices = vec![
            device_1.clone(),
            device_3.clone(),
            device_4.clone(),
            device_5.clone(),
        ];
        let denied_devices = vec![device_2.clone(), device_4, device_5.clone()];

        // default policy is `Allow`, so we should get explicitly denied devices
        let devices = get_source_network_devices(
            allowed_devices.clone(),
            denied_devices.clone(),
            FirewallPolicy::Allow,
        );
        assert_eq!(devices, vec![device_2]);
        //
        // default policy is `Deny`, so we should get explicitly allowed devices
        let devices =
            get_source_network_devices(allowed_devices, denied_devices, FirewallPolicy::Deny);
        assert_eq!(devices, vec![device_1, device_3]);
    }

    #[test]
    fn test_process_source_addrs_v4() {
        // Test data with mixed IPv4 and IPv6 addresses
        let user_device_ips = vec![
            IpAddr::V4(Ipv4Addr::new(10, 0, 1, 1)),
            IpAddr::V4(Ipv4Addr::new(10, 0, 1, 2)),
            IpAddr::V4(Ipv4Addr::new(10, 0, 1, 5)),
            IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1)), // Should be filtered out
            IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)),
        ];

        let network_device_ips = vec![
            IpAddr::V4(Ipv4Addr::new(10, 0, 1, 3)),
            IpAddr::V4(Ipv4Addr::new(10, 0, 1, 4)),
            IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 2)), // Should be filtered out
            IpAddr::V4(Ipv4Addr::new(172, 16, 1, 1)),
        ];

        let source_addrs = get_source_addrs(user_device_ips, network_device_ips, IpVersion::Ipv4);

        // Should merge consecutive IPs into ranges and keep separate non-consecutive ranges
        assert_eq!(
            source_addrs,
            vec![
                IpAddress {
                    address: Some(Address::IpRange(IpRange {
                        start: "10.0.1.1".to_string(),
                        end: "10.0.1.5".to_string(),
                    })),
                },
                IpAddress {
                    address: Some(Address::Ip("172.16.1.1".to_string())),
                },
                IpAddress {
                    address: Some(Address::Ip("192.168.1.100".to_string())),
                },
            ]
        );

        // Test with empty input
        let empty_addrs = get_source_addrs(vec![], vec![], IpVersion::Ipv4);
        assert!(empty_addrs.is_empty());

        // Test with only IPv6 addresses - should return empty result for IPv4
        let ipv6_only = get_source_addrs(
            vec![IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1))],
            vec![IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 2))],
            IpVersion::Ipv4,
        );
        assert!(ipv6_only.is_empty());
    }

    #[test]
    fn test_process_source_addrs_v6() {
        unimplemented!()
    }

    #[test]
    fn test_process_destination_addrs_v4() {
        unimplemented!()
    }

    #[test]
    fn test_process_destination_addrs_v6() {
        unimplemented!()
    }

    #[test]
    fn test_merge_v4_addrs() {
        let addr_ranges = vec![
            (
                IpAddr::V4(Ipv4Addr::new(10, 0, 60, 20)),
                IpAddr::V4(Ipv4Addr::new(10, 0, 60, 25)),
            ),
            (
                IpAddr::V4(Ipv4Addr::new(10, 0, 10, 1)),
                IpAddr::V4(Ipv4Addr::new(10, 0, 10, 22)),
            ),
            (
                IpAddr::V4(Ipv4Addr::new(10, 0, 8, 51)),
                IpAddr::V4(Ipv4Addr::new(10, 0, 9, 12)),
            ),
            (
                IpAddr::V4(Ipv4Addr::new(10, 0, 9, 1)),
                IpAddr::V4(Ipv4Addr::new(10, 0, 10, 12)),
            ),
            (
                IpAddr::V4(Ipv4Addr::new(10, 0, 9, 20)),
                IpAddr::V4(Ipv4Addr::new(10, 0, 10, 32)),
            ),
            (
                IpAddr::V4(Ipv4Addr::new(192, 168, 0, 20)),
                IpAddr::V4(Ipv4Addr::new(192, 168, 0, 20)),
            ),
            (
                IpAddr::V4(Ipv4Addr::new(10, 0, 20, 20)),
                IpAddr::V4(Ipv4Addr::new(10, 0, 20, 20)),
            ),
        ];

        let merged_addrs = merge_addrs(addr_ranges);
        assert_eq!(
            merged_addrs,
            vec![
                IpAddress {
                    address: Some(Address::IpRange(IpRange {
                        start: "10.0.8.51".to_string(),
                        end: "10.0.10.32".to_string(),
                    })),
                },
                IpAddress {
                    address: Some(Address::Ip("10.0.20.20".to_string())),
                },
                IpAddress {
                    address: Some(Address::IpRange(IpRange {
                        start: "10.0.60.20".to_string(),
                        end: "10.0.60.25".to_string(),
                    })),
                },
                IpAddress {
                    address: Some(Address::Ip("192.168.0.20".to_string())),
                },
            ]
        );
    }

    #[test]
    fn test_merge_v6_addrs() {
        let addr_ranges = vec![
            (
                IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0x1, 0x0, 0x0, 0x0, 0x0, 0x1)),
                IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0x1, 0x0, 0x0, 0x0, 0x0, 0x5)),
            ),
            (
                IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0x1, 0x0, 0x0, 0x0, 0x0, 0x3)),
                IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0x1, 0x0, 0x0, 0x0, 0x0, 0x8)),
            ),
            (
                IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0x2, 0x0, 0x0, 0x0, 0x0, 0x1)),
                IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0x2, 0x0, 0x0, 0x0, 0x0, 0x1)),
            ),
            (
                IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0x3, 0x0, 0x0, 0x0, 0x0, 0x1)),
                IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0x3, 0x0, 0x0, 0x0, 0x0, 0x3)),
            ),
        ];

        let merged_addrs = merge_addrs(addr_ranges);
        assert_eq!(
            merged_addrs,
            vec![
                IpAddress {
                    address: Some(Address::IpRange(IpRange {
                        start: "2001:db8:1::1".to_string(),
                        end: "2001:db8:1::8".to_string(),
                    })),
                },
                IpAddress {
                    address: Some(Address::Ip("2001:db8:2::1".to_string())),
                },
                IpAddress {
                    address: Some(Address::IpRange(IpRange {
                        start: "2001:db8:3::1".to_string(),
                        end: "2001:db8:3::3".to_string(),
                    })),
                },
            ]
        );
    }

    #[test]
    fn test_merge_port_ranges() {
        // single port
        let input_ranges = vec![PortRange::new(100, 100)];
        let merged = merge_port_ranges(input_ranges);
        assert_eq!(
            merged,
            vec![Port {
                port: Some(PortInner::SinglePort(100))
            }]
        );

        // overlapping ranges
        let input_ranges = vec![
            PortRange::new(100, 200),
            PortRange::new(150, 220),
            PortRange::new(210, 300),
        ];
        let merged = merge_port_ranges(input_ranges);
        assert_eq!(
            merged,
            vec![Port {
                port: Some(PortInner::PortRange(PortRangeProto {
                    start: 100,
                    end: 300
                }))
            }]
        );

        // duplicate ranges
        let input_ranges = vec![
            PortRange::new(100, 200),
            PortRange::new(100, 200),
            PortRange::new(150, 220),
            PortRange::new(150, 220),
            PortRange::new(210, 300),
            PortRange::new(210, 300),
            PortRange::new(350, 400),
            PortRange::new(350, 400),
            PortRange::new(350, 400),
        ];
        let merged = merge_port_ranges(input_ranges);
        assert_eq!(
            merged,
            vec![
                Port {
                    port: Some(PortInner::PortRange(PortRangeProto {
                        start: 100,
                        end: 300
                    }))
                },
                Port {
                    port: Some(PortInner::PortRange(PortRangeProto {
                        start: 350,
                        end: 400
                    }))
                }
            ]
        );

        // non-consecutive ranges
        let input_ranges = vec![
            PortRange::new(501, 699),
            PortRange::new(151, 220),
            PortRange::new(210, 300),
            PortRange::new(800, 800),
            PortRange::new(200, 210),
            PortRange::new(50, 50),
        ];
        let merged = merge_port_ranges(input_ranges);
        assert_eq!(
            merged,
            vec![
                Port {
                    port: Some(PortInner::SinglePort(50))
                },
                Port {
                    port: Some(PortInner::PortRange(PortRangeProto {
                        start: 151,
                        end: 300
                    }))
                },
                Port {
                    port: Some(PortInner::PortRange(PortRangeProto {
                        start: 501,
                        end: 699
                    }))
                },
                Port {
                    port: Some(PortInner::SinglePort(800))
                }
            ]
        );

        // fully contained range
        let input_ranges = vec![PortRange::new(100, 200), PortRange::new(120, 180)];
        let merged = merge_port_ranges(input_ranges);
        assert_eq!(
            merged,
            vec![Port {
                port: Some(PortInner::PortRange(PortRangeProto {
                    start: 100,
                    end: 200
                }))
            }]
        );
    }

    #[test]
    fn test_last_ip_in_v6_subnet() {
        let subnet: Ipv6Network = "2001:db8:85a3::8a2e:370:7334/64".parse().unwrap();
        let last_ip = get_last_ip_in_v6_subnet(&subnet);
        assert_eq!(
            last_ip,
            IpAddr::V6(Ipv6Addr::new(
                0x2001, 0x0db8, 0x85a3, 0x0000, 0xffff, 0xffff, 0xffff, 0xffff
            ))
        );

        let subnet: Ipv6Network = "280b:47f8:c9d7:634c:cb35:11f3:14e1:5016/119"
            .parse()
            .unwrap();
        let last_ip = get_last_ip_in_v6_subnet(&subnet);
        assert_eq!(
            last_ip,
            IpAddr::V6(Ipv6Addr::new(
                0x280b, 0x47f8, 0xc9d7, 0x634c, 0xcb35, 0x11f3, 0x14e1, 0x51ff
            ))
        )
    }

    #[test]
    fn test_process_protocols() {
        unimplemented!()
    }

    #[sqlx::test]
    async fn test_generate_firewall_rules() {
        unimplemented!()
    }
}
