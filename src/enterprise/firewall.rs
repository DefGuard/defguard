use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    ops::Range,
};

use ipnetwork::{IpNetwork, Ipv6Network};
use sqlx::{query_as, query_scalar, Error as SqlxError, PgConnection};

use super::db::models::acl::{
    AclAliasDestinationRange, AclRule, AclRuleDestinationRange, AclRuleInfo, PortRange,
};

use crate::{
    db::{models::error::ModelError, Device, Id, User, WireguardNetwork},
    enterprise::is_enterprise_enabled,
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

/// Converts ACLs into firewall rules which can be sent to a gateway over gRPC.
///
/// Each ACL is translated into two rules (in this specific order):
/// - ALLOW which determines which devices can access a destination
/// - DENY which stops all other traffic to a given destination
pub async fn generate_firewall_rules_from_acls(
    location_id: Id,
    ip_version: IpVersion,
    acl_rules: Vec<AclRuleInfo<Id>>,
    conn: &mut PgConnection,
) -> Result<Vec<FirewallRule>, FirewallError> {
    debug!("Generating firewall rules for location {location_id} with IP version {ip_version:?}");
    // initialize empty result Vec
    let mut firewall_rules = Vec::new();

    // convert each ACL into a corresponding `FirewallRule`s
    for acl in acl_rules {
        debug!("Processing ACL rule: {acl:?}");
        // FIXME: use `allow_all_users` and `deny_all_users` values to avoid processing excessive
        // number of records
        //
        // fetch allowed users
        let allowed_users = acl.get_all_allowed_users(&mut *conn).await?;

        // fetch denied users
        let denied_users = acl.get_all_denied_users(&mut *conn).await?;

        // get relevant users for determining source IPs
        let users = get_source_users(allowed_users, denied_users);

        // get network IPs for devices belonging to those users
        let user_device_ips = get_user_device_ips(&users, location_id, &mut *conn).await?;

        // get network device IPs for rule source
        let network_devices = get_source_network_devices(acl.allowed_devices, acl.denied_devices);
        let network_device_ips =
            get_network_device_ips(&network_devices, location_id, &mut *conn).await?;

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
            alias_destination_ranges.extend(alias.get_destination_ranges(&mut *conn).await?);

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

        // prepare ALLOW rule for this ACL
        let allow_rule = FirewallRule {
            id: acl.id,
            source_addrs,
            destination_addrs: destination_addrs.clone(),
            destination_ports,
            protocols,
            verdict: i32::from(FirewallPolicy::Allow),
            comment: Some(format!("ACL {} - {} ALLOW", acl.id, acl.name)),
        };
        debug!("ALLOW rule generated from ACL: {allow_rule:?}");
        firewall_rules.push(allow_rule);

        // prepare DENY rule for this ACL
        //
        // it should specify only the destination addrs to block all remaining traffic
        let deny_rule = FirewallRule {
            id: acl.id,
            source_addrs: Vec::new(),
            destination_addrs,
            destination_ports: Vec::new(),
            protocols: Vec::new(),
            verdict: i32::from(FirewallPolicy::Deny),
            comment: Some(format!("ACL {} - {} DENY", acl.id, acl.name)),
        };
        debug!("DENY rule generated from ACL: {deny_rule:?}");
        firewall_rules.push(deny_rule)
    }
    Ok(firewall_rules)
}

/// Prepares a list of all relevant users whose device IPs we'll need to prepare
/// source config for a firewall rule.
///
/// Source addrs are only needed for the ALLOW rule, so we need to take the allowed users and
/// remove any explicitly denied users.
fn get_source_users(allowed_users: Vec<User<Id>>, denied_users: Vec<User<Id>>) -> Vec<User<Id>> {
    // start with allowed users and remove those explicitly denied
    allowed_users
        .into_iter()
        .filter(|user| !denied_users.contains(user))
        .collect()
}

/// Fetches all IPs of devices belonging to specified users within a given location's VPN subnet.
// We specifically only fetch user devices since network devices are handled separately.
async fn get_user_device_ips<'e, E: sqlx::PgExecutor<'e>>(
    users: &[User<Id>],
    location_id: Id,
    executor: E,
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
        .fetch_all(executor)
        .await
}

/// Prepares a list of all relevant network devices whose IPs we'll need to prepare
/// source config for a firewall rule.
///
/// Source addrs are only needed for the ALLOW rule, so we need to take the allowed devices and
/// remove any explicitly denied devices.
fn get_source_network_devices(
    allowed_devices: Vec<Device<Id>>,
    denied_devices: Vec<Device<Id>>,
) -> Vec<Device<Id>> {
    // start with allowed devices and remove those explicitly denied
    allowed_devices
        .into_iter()
        .filter(|device| !denied_devices.contains(device))
        .collect()
}

/// Fetches all IPs of specified network devices within a given location's VPN subnet.
async fn get_network_device_ips(
    network_devices: &[Device<Id>],
    location_id: Id,
    conn: &mut PgConnection,
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
    .fetch_all(conn)
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
                    Some(ip_to_range(ip, ip))
                } else {
                    None
                }
            }
            IpVersion::Ipv6 => {
                if ip.is_ipv6() {
                    Some(ip_to_range(ip, ip))
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
/// - combining all addr lists
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
    // range representation for merging
    let destination_iterator = destination_ips.iter().filter_map(|dst| match ip_version {
        IpVersion::Ipv4 => {
            if dst.is_ipv4() {
                Some(ip_to_range(dst.network(), dst.broadcast()))
            } else {
                None
            }
        }
        IpVersion::Ipv6 => {
            if let IpNetwork::V6(subnet) = dst {
                let range_start = subnet.network().into();
                let range_end = get_last_ip_in_v6_subnet(subnet);
                Some(ip_to_range(range_start, range_end))
            } else {
                None
            }
        }
    });

    // filter out destination ranges with incompatible IP version and convert to intermediate
    // range representation for merging
    let destination_range_iterator = destination_ranges
        .iter()
        .filter_map(|dst| match ip_version {
            IpVersion::Ipv4 => {
                if dst.start.is_ipv4() && dst.end.is_ipv4() {
                    Some(ip_to_range(dst.start, dst.end))
                } else {
                    None
                }
            }
            IpVersion::Ipv6 => {
                if dst.start.is_ipv6() && dst.end.is_ipv6() {
                    Some(ip_to_range(dst.start, dst.end))
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
                        Some(ip_to_range(dst.start, dst.end))
                    } else {
                        None
                    }
                }
                IpVersion::Ipv6 => {
                    if dst.start.is_ipv6() && dst.end.is_ipv6() {
                        Some(ip_to_range(dst.start, dst.end))
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

fn ip_to_range(first_ip: IpAddr, last_ip: IpAddr) -> Range<IpAddr> {
    first_ip..next_ip(last_ip)
}

fn range_to_ip(ip_range: Range<IpAddr>) -> (IpAddr, IpAddr) {
    let first_ip = ip_range.start;
    let last_ip = previous_ip(ip_range.end);

    (first_ip, last_ip)
}

/// Returns the next IP address in sequence, handling overflow via wrapping
fn next_ip(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V4(ipv4) => {
            let octets = ipv4.octets();
            let mut num: u32 = ((octets[0] as u32) << 24)
                | ((octets[1] as u32) << 16)
                | ((octets[2] as u32) << 8)
                | octets[3] as u32;
            num = num.wrapping_add(1);
            IpAddr::V4(Ipv4Addr::from(num))
        }
        IpAddr::V6(ipv6) => {
            let segments = ipv6.segments();
            let mut num: u128 = ((segments[0] as u128) << 112)
                | ((segments[1] as u128) << 96)
                | ((segments[2] as u128) << 80)
                | ((segments[3] as u128) << 64)
                | ((segments[4] as u128) << 48)
                | ((segments[5] as u128) << 32)
                | ((segments[6] as u128) << 16)
                | segments[7] as u128;
            num = num.wrapping_add(1);
            IpAddr::V6(Ipv6Addr::from(num))
        }
    }
}

/// Returns the previous IP address in sequence, handling underflow via wrapping
fn previous_ip(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V4(ipv4) => {
            let octets = ipv4.octets();
            let mut num: u32 = ((octets[0] as u32) << 24)
                | ((octets[1] as u32) << 16)
                | ((octets[2] as u32) << 8)
                | octets[3] as u32;
            num = num.wrapping_sub(1);
            IpAddr::V4(Ipv4Addr::from(num))
        }
        IpAddr::V6(ipv6) => {
            let segments = ipv6.segments();
            let mut num: u128 = ((segments[0] as u128) << 112)
                | ((segments[1] as u128) << 96)
                | ((segments[2] as u128) << 80)
                | ((segments[3] as u128) << 64)
                | ((segments[4] as u128) << 48)
                | ((segments[5] as u128) << 32)
                | ((segments[6] as u128) << 16)
                | segments[7] as u128;
            num = num.wrapping_sub(1);
            IpAddr::V6(Ipv6Addr::from(num))
        }
    }
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
fn merge_addrs(addr_ranges: Vec<Range<IpAddr>>) -> Vec<IpAddress> {
    // merge into non-overlapping ranges
    let addr_ranges = merge_ranges(addr_ranges);

    // convert to gRPC format
    let mut result = Vec::new();
    for range in addr_ranges {
        let (range_start, range_end) = range_to_ip(range);
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

/// Takes a list of port ranges and returns the smallest possible non-overlapping list of `Port`s.
fn merge_port_ranges(port_ranges: Vec<PortRange>) -> Vec<Port> {
    // convert ranges to a list of tuples for merging
    let port_ranges = port_ranges.into_iter().map(|range| range.0).collect();

    // merge into non-overlapping ranges
    let port_ranges = merge_ranges(port_ranges);

    // convert resulting ranges into gRPC format
    port_ranges
        .into_iter()
        .map(|range| {
            let range_start = range.start;
            let range_end = range.end - 1;
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
fn merge_ranges<T: Ord + std::fmt::Debug>(mut ranges: Vec<Range<T>>) -> Vec<Range<T>> {
    // return early if list is empty
    if ranges.is_empty() {
        return Vec::new();
    }

    // sort elements by range start
    ranges.sort_by(|a, b| {
        let a_start = &a.start;
        let b_start = &b.start;
        a_start.cmp(b_start)
    });

    // initialize result vector
    let mut merged_ranges = Vec::new();

    // start with first range
    let current_range = ranges.remove(0);
    let mut current_range_start = current_range.start;
    let mut current_range_end = current_range.end;

    // iterate over remaining ranges
    for range in ranges {
        let range_start = range.start;
        let range_end = range.end;

        // compare with current range
        if range_start <= current_range_end {
            // ranges are overlapping, merge them
            // if range is not contained within current range
            if range_end > current_range_end {
                current_range_end = range_end;
            }
        } else {
            // ranges are not overlapping, add current range to result
            merged_ranges.push(current_range_start..current_range_end);
            current_range_start = range_start;
            current_range_end = range_end;
        }
    }

    // add last remaining range
    merged_ranges.push(Range {
        start: current_range_start,
        end: current_range_end,
    });

    // return resulting list
    merged_ranges
}

impl WireguardNetwork<Id> {
    /// Fetches all active ACL rules for a given location.
    /// Filters out rules which are disabled, expired or have not been deployed yet.
    pub(crate) async fn get_active_acl_rules(
        &self,
        conn: &mut PgConnection,
    ) -> Result<Vec<AclRuleInfo<Id>>, SqlxError> {
        debug!("Fetching active ACL rules for location {self}");
        let rules: Vec<AclRule<Id>> = query_as(
            "SELECT a.id, name, allow_all_users, deny_all_users, all_networks, \
                destination, ports, protocols, expires, enabled, parent_id, state \
                FROM aclrule a \
                JOIN aclrulenetwork an \
                ON a.id = an.rule_id \
                WHERE an.network_id = $1 AND enabled = true \
                AND state = 'applied'::aclrule_state \
                AND (expires IS NULL OR expires > NOW())",
        )
        .bind(self.id)
        .fetch_all(&mut *conn)
        .await?;
        debug!("Found {} active ACL rules for location {self}", rules.len());

        // convert to `AclRuleInfo`
        let mut rules_info = Vec::new();
        for rule in rules {
            let rule_info = rule.to_info(&mut *conn).await?;
            rules_info.push(rule_info);
        }
        Ok(rules_info)
    }

    /// Prepares firewall configuration for a gateway based on location config and ACLs
    /// Returns `None` if firewall management is disabled for a given location.
    pub async fn try_get_firewall_config(
        &self,
        conn: &mut PgConnection,
    ) -> Result<Option<FirewallConfig>, FirewallError> {
        // do a license check
        if !is_enterprise_enabled() {
            debug!(
                "Enterprise features are disabled, skipping generating firewall config for location {self}"
            );
            return Ok(None);
        }

        // check if ACLs are enabled
        if !self.acl_enabled {
            debug!(
                "ACL rules are disabled for location {self}, skipping generating firewall config"
            );
            return Ok(None);
        }

        info!("Generating firewall config for location {self}");
        // fetch all active ACLs for location
        let location_acls = self.get_active_acl_rules(&mut *conn).await?;

        // determine IP version based on location subnet
        let ip_version = self.get_ip_version();

        let default_policy = match self.acl_default_allow {
            true => FirewallPolicy::Allow,
            false => FirewallPolicy::Deny,
        };
        let firewall_rules =
            generate_firewall_rules_from_acls(self.id, ip_version, location_acls, &mut *conn)
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

        let ip_version = match vpn_subnet {
            IpNetwork::V4(_ipv4_network) => IpVersion::Ipv4,
            IpNetwork::V6(_ipv6_network) => IpVersion::Ipv6,
        };
        debug!("VPN subnet {vpn_subnet:?} for location {self} has IP version {ip_version:?}");

        ip_version
    }
}

#[cfg(test)]
mod test {
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

    use chrono::NaiveDateTime;
    use ipnetwork::Ipv6Network;
    use rand::{thread_rng, Rng};
    use sqlx::{query, PgPool};

    use super::process_destination_addrs;
    use crate::{
        db::{
            models::device::{DeviceType, WireguardNetworkDevice},
            Device, Group, Id, NoId, User, WireguardNetwork,
        },
        enterprise::{
            db::models::acl::{
                AclAliasDestinationRange, AclRule, AclRuleAlias, AclRuleDestinationRange,
                AclRuleDevice, AclRuleGroup, AclRuleInfo, AclRuleNetwork, AclRuleUser, PortRange,
                RuleState,
            },
            firewall::{
                get_source_addrs, get_source_network_devices, ip_to_range, next_ip, previous_ip,
            },
        },
        grpc::proto::enterprise::firewall::{
            ip_address::Address, port::Port as PortInner, FirewallPolicy, IpAddress, IpRange,
            IpVersion, Port, PortRange as PortRangeProto, Protocol,
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

        let users = get_source_users(allowed_users, denied_users);
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

        let devices = get_source_network_devices(allowed_devices, denied_devices);
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
        // Test data with mixed IPv4 and IPv6 addresses
        let user_device_ips = vec![
            IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1)),
            IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 2)),
            IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 5)),
            IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)), // Should be filtered out
            IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 1, 0, 0, 0, 1)),
        ];

        let network_device_ips = vec![
            IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 3)),
            IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 4)),
            IpAddr::V4(Ipv4Addr::new(10, 0, 1, 1)), // Should be filtered out
            IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 2, 0, 0, 0, 1)),
        ];

        let source_addrs = get_source_addrs(user_device_ips, network_device_ips, IpVersion::Ipv6);

        // Should merge consecutive IPs into ranges and keep separate non-consecutive ranges
        assert_eq!(
            source_addrs,
            vec![
                IpAddress {
                    address: Some(Address::IpRange(IpRange {
                        start: "2001:db8::1".to_string(),
                        end: "2001:db8::5".to_string(),
                    })),
                },
                IpAddress {
                    address: Some(Address::Ip("2001:db8:0:1::1".to_string())),
                },
                IpAddress {
                    address: Some(Address::Ip("2001:db8:0:2::1".to_string())),
                },
            ]
        );

        // Test with empty input
        let empty_addrs = get_source_addrs(vec![], vec![], IpVersion::Ipv6);
        assert!(empty_addrs.is_empty());

        // Test with only IPv4 addresses - should return empty result for IPv6
        let ipv4_only = get_source_addrs(
            vec![IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1))],
            vec![IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2))],
            IpVersion::Ipv6,
        );
        assert!(ipv4_only.is_empty());
    }

    #[test]
    fn test_process_destination_addrs_v4() {
        // Test data with mixed IPv4 and IPv6 networks
        let destination_ips = vec![
            "10.0.1.0/24".parse().unwrap(),
            "10.0.2.0/24".parse().unwrap(),
            "2001:db8::/64".parse().unwrap(), // Should be filtered out
            "192.168.1.0/24".parse().unwrap(),
        ];

        let destination_ranges = vec![
            AclRuleDestinationRange {
                start: IpAddr::V4(Ipv4Addr::new(10, 0, 3, 1)),
                end: IpAddr::V4(Ipv4Addr::new(10, 0, 3, 100)),
                ..Default::default()
            },
            AclRuleDestinationRange {
                start: IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1)), // Should be filtered out
                end: IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 100)),
                ..Default::default()
            },
        ];

        let alias_destination_ranges = vec![
            AclAliasDestinationRange {
                start: IpAddr::V4(Ipv4Addr::new(10, 0, 4, 1)),
                end: IpAddr::V4(Ipv4Addr::new(10, 0, 4, 50)),
                ..Default::default()
            },
            AclAliasDestinationRange {
                start: IpAddr::V4(Ipv4Addr::new(10, 0, 4, 40)),
                end: IpAddr::V4(Ipv4Addr::new(10, 0, 4, 100)),
                ..Default::default()
            },
        ];

        let destination_addrs = process_destination_addrs(
            destination_ips,
            destination_ranges,
            alias_destination_ranges,
            IpVersion::Ipv4,
        );

        assert_eq!(
            destination_addrs,
            vec![
                IpAddress {
                    address: Some(Address::IpRange(IpRange {
                        start: "10.0.1.0".to_string(),
                        end: "10.0.2.255".to_string(),
                    })),
                },
                IpAddress {
                    address: Some(Address::IpRange(IpRange {
                        start: "10.0.3.1".to_string(),
                        end: "10.0.3.100".to_string(),
                    })),
                },
                IpAddress {
                    address: Some(Address::IpRange(IpRange {
                        start: "10.0.4.1".to_string(),
                        end: "10.0.4.100".to_string(),
                    })),
                },
                IpAddress {
                    address: Some(Address::IpRange(IpRange {
                        start: "192.168.1.0".to_string(),
                        end: "192.168.1.255".to_string(),
                    })),
                },
            ]
        );

        // Test with empty input
        let empty_addrs = process_destination_addrs(vec![], vec![], vec![], IpVersion::Ipv4);
        assert!(empty_addrs.is_empty());

        // Test with only IPv6 addresses - should return empty result for IPv4
        let ipv6_only = process_destination_addrs(
            vec!["2001:db8::/64".parse().unwrap()],
            vec![],
            vec![],
            IpVersion::Ipv4,
        );
        assert!(ipv6_only.is_empty());
    }

    #[test]
    fn test_process_destination_addrs_v6() {
        // Test data with mixed IPv4 and IPv6 networks
        let destination_ips = vec![
            "2001:db8:1::/64".parse().unwrap(),
            "2001:db8:2::/64".parse().unwrap(),
            "10.0.1.0/24".parse().unwrap(), // Should be filtered out
            "2001:db8:3::/64".parse().unwrap(),
        ];

        let destination_ranges = vec![
            AclRuleDestinationRange {
                start: IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 4, 0, 0, 0, 0, 1)),
                end: IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 4, 0, 0, 0, 0, 100)),
                ..Default::default()
            },
            AclRuleDestinationRange {
                start: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)), // Should be filtered out
                end: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)),
                ..Default::default()
            },
        ];

        let alias_destination_ranges = vec![
            AclAliasDestinationRange {
                start: IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 5, 0, 0, 0, 0, 1)),
                end: IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 5, 0, 0, 0, 0, 50)),
                ..Default::default()
            },
            AclAliasDestinationRange {
                start: IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 5, 0, 0, 0, 0, 40)),
                end: IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 5, 0, 0, 0, 0, 100)),
                ..Default::default()
            },
        ];

        let destination_addrs = process_destination_addrs(
            destination_ips,
            destination_ranges,
            alias_destination_ranges,
            IpVersion::Ipv6,
        );

        assert_eq!(
            destination_addrs,
            vec![
                IpAddress {
                    address: Some(Address::IpRange(IpRange {
                        start: "2001:db8:1::".to_string(),
                        end: "2001:db8:1:0:ffff:ffff:ffff:ffff".to_string(),
                    })),
                },
                IpAddress {
                    address: Some(Address::IpRange(IpRange {
                        start: "2001:db8:2::".to_string(),
                        end: "2001:db8:2:0:ffff:ffff:ffff:ffff".to_string(),
                    })),
                },
                IpAddress {
                    address: Some(Address::IpRange(IpRange {
                        start: "2001:db8:3::".to_string(),
                        end: "2001:db8:3:0:ffff:ffff:ffff:ffff".to_string(),
                    })),
                },
                IpAddress {
                    address: Some(Address::IpRange(IpRange {
                        start: "2001:db8:4::1".to_string(),
                        end: "2001:db8:4::64".to_string(),
                    })),
                },
                IpAddress {
                    address: Some(Address::IpRange(IpRange {
                        start: "2001:db8:5::1".to_string(),
                        end: "2001:db8:5::64".to_string(),
                    })),
                },
            ]
        );

        // Test with empty input
        let empty_addrs = process_destination_addrs(vec![], vec![], vec![], IpVersion::Ipv6);
        assert!(empty_addrs.is_empty());

        // Test with only IPv4 addresses - should return empty result for IPv6
        let ipv4_only = process_destination_addrs(
            vec!["192.168.1.0/24".parse().unwrap()],
            vec![],
            vec![],
            IpVersion::Ipv6,
        );
        assert!(ipv4_only.is_empty());
    }

    #[test]
    fn test_merge_v4_addrs() {
        let addr_ranges = vec![
            ip_to_range(
                IpAddr::V4(Ipv4Addr::new(10, 0, 60, 20)),
                IpAddr::V4(Ipv4Addr::new(10, 0, 60, 25)),
            ),
            ip_to_range(
                IpAddr::V4(Ipv4Addr::new(10, 0, 10, 1)),
                IpAddr::V4(Ipv4Addr::new(10, 0, 10, 22)),
            ),
            ip_to_range(
                IpAddr::V4(Ipv4Addr::new(10, 0, 8, 51)),
                IpAddr::V4(Ipv4Addr::new(10, 0, 9, 12)),
            ),
            ip_to_range(
                IpAddr::V4(Ipv4Addr::new(10, 0, 9, 1)),
                IpAddr::V4(Ipv4Addr::new(10, 0, 10, 12)),
            ),
            ip_to_range(
                IpAddr::V4(Ipv4Addr::new(10, 0, 9, 20)),
                IpAddr::V4(Ipv4Addr::new(10, 0, 10, 32)),
            ),
            ip_to_range(
                IpAddr::V4(Ipv4Addr::new(192, 168, 0, 20)),
                IpAddr::V4(Ipv4Addr::new(192, 168, 0, 20)),
            ),
            ip_to_range(
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

        // merge single IPs into a range
        let addr_ranges = vec![
            ip_to_range(
                IpAddr::V4(Ipv4Addr::new(10, 0, 10, 1)),
                IpAddr::V4(Ipv4Addr::new(10, 0, 10, 1)),
            ),
            ip_to_range(
                IpAddr::V4(Ipv4Addr::new(10, 0, 10, 2)),
                IpAddr::V4(Ipv4Addr::new(10, 0, 10, 2)),
            ),
            ip_to_range(
                IpAddr::V4(Ipv4Addr::new(10, 0, 10, 3)),
                IpAddr::V4(Ipv4Addr::new(10, 0, 10, 3)),
            ),
            ip_to_range(
                IpAddr::V4(Ipv4Addr::new(10, 0, 10, 4)),
                IpAddr::V4(Ipv4Addr::new(10, 0, 10, 4)),
            ),
            ip_to_range(
                IpAddr::V4(Ipv4Addr::new(10, 0, 10, 20)),
                IpAddr::V4(Ipv4Addr::new(10, 0, 10, 20)),
            ),
        ];

        let merged_addrs = merge_addrs(addr_ranges);
        assert_eq!(
            merged_addrs,
            vec![
                IpAddress {
                    address: Some(Address::IpRange(IpRange {
                        start: "10.0.10.1".to_string(),
                        end: "10.0.10.4".to_string(),
                    })),
                },
                IpAddress {
                    address: Some(Address::Ip("10.0.10.20".to_string())),
                },
            ]
        );
    }

    #[test]
    fn test_merge_v6_addrs() {
        let addr_ranges = vec![
            ip_to_range(
                IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0x1, 0x0, 0x0, 0x0, 0x0, 0x1)),
                IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0x1, 0x0, 0x0, 0x0, 0x0, 0x5)),
            ),
            ip_to_range(
                IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0x1, 0x0, 0x0, 0x0, 0x0, 0x3)),
                IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0x1, 0x0, 0x0, 0x0, 0x0, 0x8)),
            ),
            ip_to_range(
                IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0x2, 0x0, 0x0, 0x0, 0x0, 0x1)),
                IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0x2, 0x0, 0x0, 0x0, 0x0, 0x1)),
            ),
            ip_to_range(
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
    fn test_next_ip() {
        // Test IPv4
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));
        assert_eq!(next_ip(ip), IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2)));
        let ip = IpAddr::V4(Ipv4Addr::new(10, 0, 2, 255));
        assert_eq!(next_ip(ip), IpAddr::V4(Ipv4Addr::new(10, 0, 3, 0)));

        // Test IPv4 overflow
        let ip = IpAddr::V4(Ipv4Addr::new(255, 255, 255, 255));
        assert_eq!(next_ip(ip), IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)));

        // Test IPv6
        let ip = IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1));
        assert_eq!(
            next_ip(ip),
            IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 2))
        );

        // Test IPv6 overflow
        let ip = IpAddr::V6(Ipv6Addr::new(
            0xffff, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff,
        ));
        assert_eq!(
            next_ip(ip),
            IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0))
        );
    }

    #[test]
    fn test_previous_ip() {
        // Test IPv4
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2));
        assert_eq!(previous_ip(ip), IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)));
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 2, 0));
        assert_eq!(previous_ip(ip), IpAddr::V4(Ipv4Addr::new(192, 168, 1, 255)));

        // Test IPv4 underflow
        let ip = IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0));
        assert_eq!(
            previous_ip(ip),
            IpAddr::V4(Ipv4Addr::new(255, 255, 255, 255))
        );

        // Test IPv6
        let ip = IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 2));
        assert_eq!(
            previous_ip(ip),
            IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1))
        );

        // Test IPv6 underflow
        let ip = IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0));
        assert_eq!(
            previous_ip(ip),
            IpAddr::V6(Ipv6Addr::new(
                0xffff, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff
            ))
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

    async fn create_acl_rule(
        pool: &PgPool,
        rule: AclRule,
        locations: Vec<Id>,
        allowed_users: Vec<Id>,
        denied_users: Vec<Id>,
        allowed_groups: Vec<Id>,
        denied_groups: Vec<Id>,
        allowed_network_devices: Vec<Id>,
        denied_network_devices: Vec<Id>,
        destination_ranges: Vec<(IpAddr, IpAddr)>,
        aliases: Vec<Id>,
    ) -> AclRuleInfo<Id> {
        let mut conn = pool.acquire().await.unwrap();

        // create base rule
        let rule = rule.save(&mut *conn).await.unwrap();
        let rule_id = rule.id;

        // create related objects
        // locations
        for location_id in locations {
            let obj = AclRuleNetwork {
                id: NoId,
                rule_id,
                network_id: location_id,
            };
            obj.save(&mut *conn).await.unwrap();
        }

        // allowed users
        for user_id in allowed_users {
            let obj = AclRuleUser {
                id: NoId,
                allow: true,
                rule_id,
                user_id,
            };
            obj.save(&mut *conn).await.unwrap();
        }

        // denied users
        for user_id in denied_users {
            let obj = AclRuleUser {
                id: NoId,
                allow: false,
                rule_id,
                user_id,
            };
            obj.save(&mut *conn).await.unwrap();
        }

        // allowed groups
        for group_id in allowed_groups {
            let obj = AclRuleGroup {
                id: NoId,
                allow: true,
                rule_id,
                group_id,
            };
            obj.save(&mut *conn).await.unwrap();
        }

        // denied groups
        for group_id in denied_groups {
            let obj = AclRuleGroup {
                id: NoId,
                allow: false,
                rule_id,
                group_id,
            };
            obj.save(&mut *conn).await.unwrap();
        }

        // allowed devices
        for device_id in allowed_network_devices {
            let obj = AclRuleDevice {
                id: NoId,
                allow: true,
                rule_id,
                device_id,
            };
            obj.save(&mut *conn).await.unwrap();
        }

        // denied devices
        for device_id in denied_network_devices {
            let obj = AclRuleDevice {
                id: NoId,
                allow: false,
                rule_id,
                device_id,
            };
            obj.save(&mut *conn).await.unwrap();
        }

        // destination ranges
        for range in destination_ranges {
            let obj = AclRuleDestinationRange {
                id: NoId,
                rule_id,
                start: range.0,
                end: range.1,
            };
            obj.save(&mut *conn).await.unwrap();
        }

        // aliases
        for alias_id in aliases {
            let obj = AclRuleAlias {
                id: NoId,
                rule_id,
                alias_id,
            };
            obj.save(&mut *conn).await.unwrap();
        }

        // convert to output format
        rule.to_info(&mut conn).await.unwrap()
    }

    #[sqlx::test]
    async fn test_generate_firewall_rules(pool: PgPool) {
        let mut rng = thread_rng();

        // Create test location
        let location = WireguardNetwork {
            id: NoId,
            acl_enabled: false,
            ..Default::default()
        };
        let mut location = location.save(&pool).await.unwrap();

        // Setup test users and their devices
        let user_1: User<NoId> = rng.gen();
        let user_1 = user_1.save(&pool).await.unwrap();
        let user_2: User<NoId> = rng.gen();
        let user_2 = user_2.save(&pool).await.unwrap();
        let user_3: User<NoId> = rng.gen();
        let user_3 = user_3.save(&pool).await.unwrap();
        let user_4: User<NoId> = rng.gen();
        let user_4 = user_4.save(&pool).await.unwrap();
        let user_5: User<NoId> = rng.gen();
        let user_5 = user_5.save(&pool).await.unwrap();

        for user in [&user_1, &user_2, &user_3, &user_4, &user_5] {
            // Create 2 devices per user
            for device_num in 1..3 {
                let device = Device {
                    id: NoId,
                    name: format!("device-{}-{}", user.id, device_num),
                    user_id: user.id,
                    device_type: DeviceType::User,
                    description: None,
                    wireguard_pubkey: Default::default(),
                    created: Default::default(),
                    configured: true,
                };
                let device = device.save(&pool).await.unwrap();

                // Add device to location's VPN network
                let network_device = WireguardNetworkDevice {
                    device_id: device.id,
                    wireguard_network_id: location.id,
                    wireguard_ip: IpAddr::V4(Ipv4Addr::new(10, 0, user.id as u8, device_num as u8)),
                    preshared_key: None,
                    is_authorized: true,
                    authorized_at: None,
                };
                network_device.insert(&pool).await.unwrap();
            }
        }

        // Setup test groups
        let group_1 = Group {
            id: NoId,
            name: "group_1".into(),
            ..Default::default()
        };
        let group_1 = group_1.save(&pool).await.unwrap();
        let group_2 = Group {
            id: NoId,
            name: "group_2".into(),
            ..Default::default()
        };
        let group_2 = group_2.save(&pool).await.unwrap();

        // Assign users to groups:
        // Group 1: users 1,2
        // Group 2: users 3,4
        let group_assignments = vec![
            (&group_1, vec![&user_1, &user_2]),
            (&group_2, vec![&user_3, &user_4]),
        ];

        for (group, users) in group_assignments {
            for user in users {
                query!(
                    "INSERT INTO group_user (user_id, group_id) VALUES ($1, $2)",
                    user.id,
                    group.id
                )
                .execute(&pool)
                .await
                .unwrap();
            }
        }

        // Create some network devices
        let network_device_1 = Device {
            id: NoId,
            name: "network-device-1".into(),
            user_id: user_1.id, // Owned by user 1
            device_type: DeviceType::Network,
            description: Some("Test network device 1".into()),
            wireguard_pubkey: Default::default(),
            created: Default::default(),
            configured: true,
        };
        let network_device_1 = network_device_1.save(&pool).await.unwrap();

        let network_device_2 = Device {
            id: NoId,
            name: "network-device-2".into(),
            user_id: user_2.id, // Owned by user 2
            device_type: DeviceType::Network,
            description: Some("Test network device 2".into()),
            wireguard_pubkey: Default::default(),
            created: Default::default(),
            configured: true,
        };
        let network_device_2 = network_device_2.save(&pool).await.unwrap();

        let network_device_3 = Device {
            id: NoId,
            name: "network-device-3".into(),
            user_id: user_3.id, // Owned by user 3
            device_type: DeviceType::Network,
            description: Some("Test network device 3".into()),
            wireguard_pubkey: Default::default(),
            created: Default::default(),
            configured: true,
        };
        let network_device_3 = network_device_3.save(&pool).await.unwrap();

        // Add network devices to location's VPN network
        let network_devices = vec![
            (
                network_device_1.id,
                IpAddr::V4(Ipv4Addr::new(10, 0, 100, 1)),
            ),
            (
                network_device_2.id,
                IpAddr::V4(Ipv4Addr::new(10, 0, 100, 2)),
            ),
            (
                network_device_3.id,
                IpAddr::V4(Ipv4Addr::new(10, 0, 100, 3)),
            ),
        ];

        for (device_id, ip) in network_devices {
            let network_device = WireguardNetworkDevice {
                device_id,
                wireguard_network_id: location.id,
                wireguard_ip: ip,
                preshared_key: None,
                is_authorized: true,
                authorized_at: None,
            };
            network_device.insert(&pool).await.unwrap();
        }

        // Create first ACL rule - Web access
        let acl_rule_1 = AclRule {
            id: NoId,
            name: "Web Access".into(),
            all_networks: false,
            expires: None,
            allow_all_users: false,
            deny_all_users: false,
            destination: vec!["192.168.1.0/24".parse().unwrap()],
            ports: vec![
                PortRange::new(80, 80).into(),
                PortRange::new(443, 443).into(),
            ],
            protocols: vec![Protocol::Tcp.into()],
            enabled: true,
            parent_id: None,
            state: RuleState::Applied,
        };
        let locations = vec![location.id];
        let allowed_users = vec![user_1.id, user_2.id]; // First two users can access web
        let denied_users = vec![user_3.id]; // Third user explicitly denied
        let allowed_groups = vec![group_1.id]; // First group allowed
        let denied_groups = vec![];
        let allowed_devices = vec![network_device_1.id];
        let denied_devices = vec![network_device_2.id, network_device_3.id];
        let destination_ranges = vec![];
        let aliases = vec![];

        let _acl_rule_1 = create_acl_rule(
            &pool,
            acl_rule_1,
            locations,
            allowed_users,
            denied_users,
            allowed_groups,
            denied_groups,
            allowed_devices,
            denied_devices,
            destination_ranges,
            aliases,
        )
        .await;

        // Create second ACL rule - DNS access
        let acl_rule_2 = AclRule {
            id: NoId,
            name: "DNS Access".into(),
            all_networks: false,
            expires: None,
            allow_all_users: true, // Allow all users
            deny_all_users: false,
            destination: vec![], // Will use destination ranges instead
            ports: vec![PortRange::new(53, 53).into()],
            protocols: vec![Protocol::Udp.into(), Protocol::Tcp.into()],
            enabled: true,
            parent_id: None,
            state: RuleState::Applied,
        };
        let locations_2 = vec![location.id];
        let allowed_users_2 = vec![];
        let denied_users_2 = vec![user_5.id]; // Fifth user denied DNS
        let allowed_groups_2 = vec![];
        let denied_groups_2 = vec![group_2.id];
        let allowed_devices_2 = vec![network_device_1.id, network_device_2.id]; // First two network devices allowed
        let denied_devices_2 = vec![network_device_3.id]; // Third network device denied
        let destination_ranges_2 = vec![
            ("10.0.1.13".parse().unwrap(), "10.0.1.43".parse().unwrap()),
            ("10.0.1.52".parse().unwrap(), "10.0.2.43".parse().unwrap()),
        ];
        let aliases_2 = vec![];

        let _acl_rule_2 = create_acl_rule(
            &pool,
            acl_rule_2,
            locations_2,
            allowed_users_2,
            denied_users_2,
            allowed_groups_2,
            denied_groups_2,
            allowed_devices_2,
            denied_devices_2,
            destination_ranges_2,
            aliases_2,
        )
        .await;

        let mut conn = pool.acquire().await.unwrap();

        // try to generate firewall config with ACL disabled
        location.acl_enabled = false;
        let generated_firewall_config = location.try_get_firewall_config(&mut conn).await.unwrap();
        assert!(generated_firewall_config.is_none());

        // generate firewall config with default policy Allow
        location.acl_enabled = true;
        location.acl_default_allow = true;
        let generated_firewall_config = location
            .try_get_firewall_config(&mut conn)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            generated_firewall_config.default_policy,
            i32::from(FirewallPolicy::Allow)
        );
        assert_eq!(
            generated_firewall_config.ip_version,
            i32::from(IpVersion::Ipv4)
        );

        let generated_firewall_rules = generated_firewall_config.rules;

        assert_eq!(generated_firewall_rules.len(), 4);

        // First ACL - Web Access ALLOW
        // Should allow access for users 1,2 and network_device_1 to web ports
        let web_allow_rule = &generated_firewall_rules[0];
        assert_eq!(web_allow_rule.verdict, i32::from(FirewallPolicy::Allow));
        assert_eq!(web_allow_rule.protocols, vec![i32::from(Protocol::Tcp)]);
        assert_eq!(
            web_allow_rule.destination_addrs,
            vec![IpAddress {
                address: Some(Address::IpRange(IpRange {
                    start: "192.168.1.0".to_string(),
                    end: "192.168.1.255".to_string(),
                })),
            }]
        );
        assert_eq!(
            web_allow_rule.destination_ports,
            vec![
                Port {
                    port: Some(PortInner::SinglePort(80))
                },
                Port {
                    port: Some(PortInner::SinglePort(443))
                }
            ]
        );
        // Source addresses should include devices of users 1,2 and network_device_1
        assert_eq!(
            web_allow_rule.source_addrs,
            vec![
                IpAddress {
                    address: Some(Address::IpRange(IpRange {
                        start: "10.0.1.1".to_string(),
                        end: "10.0.1.2".to_string(),
                    })),
                },
                IpAddress {
                    address: Some(Address::IpRange(IpRange {
                        start: "10.0.2.1".to_string(),
                        end: "10.0.2.2".to_string(),
                    })),
                },
                IpAddress {
                    address: Some(Address::Ip("10.0.100.1".to_string())),
                },
            ]
        );

        // First ACL - Web Access DENY
        // Should allow access for users 1,2 and network_device_1 to web ports
        let web_deny_rule = &generated_firewall_rules[1];
        assert_eq!(web_deny_rule.verdict, i32::from(FirewallPolicy::Deny));
        assert!(web_deny_rule.protocols.is_empty());
        assert!(web_deny_rule.destination_ports.is_empty());
        assert!(web_deny_rule.source_addrs.is_empty());
        assert_eq!(
            web_deny_rule.destination_addrs,
            vec![IpAddress {
                address: Some(Address::IpRange(IpRange {
                    start: "192.168.1.0".to_string(),
                    end: "192.168.1.255".to_string(),
                })),
            }]
        );

        // Second ACL - DNS Access ALLOW
        // Should allow access for all users except user_5 and groups 1,2 members
        // plus network_devices 1,2
        let dns_allow_rule = &generated_firewall_rules[2];
        assert_eq!(dns_allow_rule.verdict, i32::from(FirewallPolicy::Allow));
        assert_eq!(
            dns_allow_rule.protocols,
            vec![i32::from(Protocol::Tcp), i32::from(Protocol::Udp)]
        );
        assert_eq!(
            dns_allow_rule.destination_ports,
            vec![Port {
                port: Some(PortInner::SinglePort(53))
            }]
        );
        // Source addresses should include network_devices 1,2
        assert_eq!(
            dns_allow_rule.source_addrs,
            vec![
                IpAddress {
                    address: Some(Address::IpRange(IpRange {
                        start: "10.0.1.1".to_string(),
                        end: "10.0.1.2".to_string(),
                    })),
                },
                IpAddress {
                    address: Some(Address::IpRange(IpRange {
                        start: "10.0.2.1".to_string(),
                        end: "10.0.2.2".to_string(),
                    })),
                },
                IpAddress {
                    address: Some(Address::IpRange(IpRange {
                        start: "10.0.100.1".to_string(),
                        end: "10.0.100.2".to_string(),
                    })),
                },
            ]
        );
        assert_eq!(
            dns_allow_rule.destination_addrs,
            vec![
                IpAddress {
                    address: Some(Address::IpRange(IpRange {
                        start: "10.0.1.13".to_string(),
                        end: "10.0.1.43".to_string(),
                    })),
                },
                IpAddress {
                    address: Some(Address::IpRange(IpRange {
                        start: "10.0.1.52".to_string(),
                        end: "10.0.2.43".to_string(),
                    })),
                }
            ]
        );

        // Second ACL - DNS Access DENY
        // Should allow access for all users except user_5 and groups 1,2 members
        // plus network_devices 1,2
        let dns_deny_rule = &generated_firewall_rules[3];
        assert_eq!(dns_deny_rule.verdict, i32::from(FirewallPolicy::Deny));
        assert!(dns_deny_rule.protocols.is_empty(),);
        assert!(dns_deny_rule.destination_ports.is_empty(),);
        assert!(dns_deny_rule.source_addrs.is_empty(),);
        assert_eq!(
            dns_deny_rule.destination_addrs,
            vec![
                IpAddress {
                    address: Some(Address::IpRange(IpRange {
                        start: "10.0.1.13".to_string(),
                        end: "10.0.1.43".to_string(),
                    })),
                },
                IpAddress {
                    address: Some(Address::IpRange(IpRange {
                        start: "10.0.1.52".to_string(),
                        end: "10.0.2.43".to_string(),
                    })),
                }
            ]
        );
    }

    #[sqlx::test]
    async fn test_expired_acl_rules(pool: PgPool) {
        // Create test location
        let location = WireguardNetwork {
            id: NoId,
            acl_enabled: true,
            ..Default::default()
        };
        let location = location.save(&pool).await.unwrap();

        // create expired ACL rules
        let mut acl_rule_1 = AclRule {
            id: NoId,
            expires: Some(NaiveDateTime::UNIX_EPOCH),
            enabled: true,
            state: RuleState::Applied,
            ..Default::default()
        }
        .save(&pool)
        .await
        .unwrap();
        let mut acl_rule_2 = AclRule {
            id: NoId,
            expires: Some(NaiveDateTime::UNIX_EPOCH),
            enabled: true,
            state: RuleState::Applied,
            ..Default::default()
        }
        .save(&pool)
        .await
        .unwrap();

        // assign rules to location
        for rule in [&acl_rule_1, &acl_rule_2] {
            let obj = AclRuleNetwork {
                id: NoId,
                rule_id: rule.id,
                network_id: location.id,
            };
            obj.save(&pool).await.unwrap();
        }

        let mut conn = pool.acquire().await.unwrap();
        let generated_firewall_rules = location
            .try_get_firewall_config(&mut conn)
            .await
            .unwrap()
            .unwrap()
            .rules;

        // both rules were expired
        assert_eq!(generated_firewall_rules.len(), 0);

        // make both rules not expired
        acl_rule_1.expires = None;
        acl_rule_1.save(&pool).await.unwrap();

        acl_rule_2.expires = Some(NaiveDateTime::MAX);
        acl_rule_2.save(&pool).await.unwrap();

        let generated_firewall_rules = location
            .try_get_firewall_config(&mut conn)
            .await
            .unwrap()
            .unwrap()
            .rules;
        assert_eq!(generated_firewall_rules.len(), 4);
    }

    #[sqlx::test]
    async fn test_disabled_acl_rules(pool: PgPool) {
        // Create test location
        let location = WireguardNetwork {
            id: NoId,
            acl_enabled: true,
            ..Default::default()
        };
        let location = location.save(&pool).await.unwrap();

        // create disabled ACL rules
        let mut acl_rule_1 = AclRule {
            id: NoId,
            expires: None,
            enabled: false,
            state: RuleState::Applied,
            ..Default::default()
        }
        .save(&pool)
        .await
        .unwrap();
        let mut acl_rule_2 = AclRule {
            id: NoId,
            expires: None,
            enabled: false,
            state: RuleState::Applied,
            ..Default::default()
        }
        .save(&pool)
        .await
        .unwrap();

        // assign rules to location
        for rule in [&acl_rule_1, &acl_rule_2] {
            let obj = AclRuleNetwork {
                id: NoId,
                rule_id: rule.id,
                network_id: location.id,
            };
            obj.save(&pool).await.unwrap();
        }

        let mut conn = pool.acquire().await.unwrap();
        let generated_firewall_rules = location
            .try_get_firewall_config(&mut conn)
            .await
            .unwrap()
            .unwrap()
            .rules;

        // both rules were disabled
        assert_eq!(generated_firewall_rules.len(), 0);

        // make both rules enabled
        acl_rule_1.enabled = true;
        acl_rule_1.save(&pool).await.unwrap();

        acl_rule_2.enabled = true;
        acl_rule_2.save(&pool).await.unwrap();

        let generated_firewall_rules = location
            .try_get_firewall_config(&mut conn)
            .await
            .unwrap()
            .unwrap()
            .rules;
        assert_eq!(generated_firewall_rules.len(), 4);
    }

    #[sqlx::test]
    async fn test_unapplied_acl_rules(pool: PgPool) {
        // Create test location
        let location = WireguardNetwork {
            id: NoId,
            acl_enabled: true,
            ..Default::default()
        };
        let location = location.save(&pool).await.unwrap();

        // create unapplied ACL rules
        let mut acl_rule_1 = AclRule {
            id: NoId,
            expires: None,
            enabled: true,
            state: RuleState::New,
            ..Default::default()
        }
        .save(&pool)
        .await
        .unwrap();
        let mut acl_rule_2 = AclRule {
            id: NoId,
            expires: None,
            enabled: true,
            state: RuleState::Modified,
            ..Default::default()
        }
        .save(&pool)
        .await
        .unwrap();

        // assign rules to location
        for rule in [&acl_rule_1, &acl_rule_2] {
            let obj = AclRuleNetwork {
                id: NoId,
                rule_id: rule.id,
                network_id: location.id,
            };
            obj.save(&pool).await.unwrap();
        }

        let mut conn = pool.acquire().await.unwrap();
        let generated_firewall_rules = location
            .try_get_firewall_config(&mut conn)
            .await
            .unwrap()
            .unwrap()
            .rules;

        // both rules were not applied
        assert_eq!(generated_firewall_rules.len(), 0);

        // make both rules applied
        acl_rule_1.state = RuleState::Applied;
        acl_rule_1.save(&pool).await.unwrap();

        acl_rule_2.state = RuleState::Applied;
        acl_rule_2.save(&pool).await.unwrap();

        let generated_firewall_rules = location
            .try_get_firewall_config(&mut conn)
            .await
            .unwrap()
            .unwrap()
            .rules;
        assert_eq!(generated_firewall_rules.len(), 4);
    }
}
