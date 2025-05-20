use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    ops::RangeInclusive,
};

use ipnetwork::IpNetwork;
use sqlx::{query_as, query_scalar, Error as SqlxError, PgConnection};

use super::{
    db::models::acl::{
        AclAliasDestinationRange, AclRule, AclRuleDestinationRange, AclRuleInfo, PortRange,
        Protocol,
    },
    utils::merge_ranges,
};
use crate::{
    db::{models::error::ModelError, Device, Id, User, WireguardNetwork},
    enterprise::{db::models::acl::AliasKind, is_enterprise_enabled},
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
/// Each ACL is translated into two rules:
/// - ALLOW which determines which devices can access a destination
/// - DENY which stops all other traffic to a given destination
///
/// In the resulting list all ALLOW rules are placed first and then DENY rules are added to the
/// end. This way we can avoid conflicts when some ACLs are overlapping.
pub async fn generate_firewall_rules_from_acls(
    location_id: Id,
    acl_rules: Vec<AclRuleInfo<Id>>,
    conn: &mut PgConnection,
) -> Result<Vec<FirewallRule>, FirewallError> {
    debug!("Generating firewall rules for location {location_id}");
    // initialize empty rules Vec
    let mut allow_rules = Vec::new();
    let mut deny_rules = Vec::new();
    let location = WireguardNetwork::find_by_id(&mut *conn, location_id)
        .await?
        .ok_or(ModelError::NotFound)?;
    let has_ipv4_addresses = location.address.iter().any(IpNetwork::is_ipv4);
    let has_ipv6_addresses = location.address.iter().any(IpNetwork::is_ipv6);

    // convert each ACL into a corresponding `FirewallRule`s
    for acl in acl_rules {
        debug!("Processing ACL rule: {acl:?}");
        // fetch allowed users
        let allowed_users = acl.get_all_allowed_users(&mut *conn).await?;

        // fetch denied users
        let denied_users = acl.get_all_denied_users(&mut *conn).await?;

        // get relevant users for determining source IPs
        let users = get_source_users(allowed_users, &denied_users);

        // get network IPs for devices belonging to those users
        let user_device_ips = get_user_device_ips(&users, location_id, &mut *conn).await?;
        // separate IPv4 and IPv6 user-device addresses
        let user_device_ips = user_device_ips
            .iter()
            .flatten()
            .partition(|ip| ip.is_ipv4());

        // fetch allowed network devices
        let allowed_network_devices = acl.get_all_allowed_devices(&mut *conn, location_id).await?;

        // fetch denied network devices
        let denied_network_devices = acl.get_all_denied_devices(&mut *conn, location_id).await?;

        // get network device IPs for rule source
        let network_devices =
            get_source_network_devices(allowed_network_devices, &denied_network_devices);
        let network_device_ips =
            get_network_device_ips(&network_devices, location_id, &mut *conn).await?;
        // separate IPv4 and IPv6 network-device addresses
        let network_device_ips = network_device_ips
            .iter()
            .flatten()
            .partition(|ip| ip.is_ipv4());

        // convert device IPs into source addresses for a firewall rule
        let ipv4_source_addrs =
            get_source_addrs(user_device_ips.0, network_device_ips.0, IpVersion::Ipv4);
        let ipv6_source_addrs =
            get_source_addrs(user_device_ips.1, network_device_ips.1, IpVersion::Ipv6);

        // extract destination parameters from ACL rule
        let AclRuleInfo {
            id,
            mut destination,
            destination_ranges,
            mut ports,
            mut protocols,
            aliases,
            ..
        } = acl;

        // split aliases into types
        let (destination_aliases, component_aliases): (Vec<_>, Vec<_>) = aliases
            .into_iter()
            .partition(|alias| alias.kind == AliasKind::Destination);

        // store alias ranges separately since they use a different struct
        let mut alias_destination_ranges = Vec::new();

        // process component aliases by appending destination parameters from each of them to
        // existing lists
        for alias in component_aliases {
            // fetch destination ranges for a given alias
            alias_destination_ranges.extend(alias.get_destination_ranges(&mut *conn).await?);

            // extend existing parameter lists
            destination.extend(alias.destination);
            ports.extend(alias.ports.into_iter().map(Into::into).collect::<Vec<_>>());
            protocols.extend(alias.protocols);
        }

        // prepare destination addresses
        let (dest_addrs_v4, dest_addrs_v6) =
            process_destination_addrs(&destination, destination_ranges);

        // prepare destination ports
        let destination_ports = merge_port_ranges(ports);

        // remove duplicate protocol entries
        protocols.sort_unstable();
        protocols.dedup();

        let comment = format!("ACL {} - {}", acl.id, acl.name);
        if has_ipv4_addresses {
            // create IPv4 rules
            let ipv4_rules = create_rules(
                acl.id,
                IpVersion::Ipv4,
                &ipv4_source_addrs,
                &dest_addrs_v4,
                &destination_ports,
                &protocols,
                &comment,
            );
            if let Some(rule) = ipv4_rules.0 {
                allow_rules.push(rule);
            }
            deny_rules.push(ipv4_rules.1);
        }

        if has_ipv6_addresses {
            // create IPv6 rules
            let ipv6_rules = create_rules(
                acl.id,
                IpVersion::Ipv6,
                &ipv6_source_addrs,
                &dest_addrs_v6,
                &destination_ports,
                &protocols,
                &comment,
            );
            if let Some(rule) = ipv6_rules.0 {
                allow_rules.push(rule);
            }
            deny_rules.push(ipv6_rules.1);
        }
        // process destination aliases by creating a dedicated set of rules for each of them
        if !destination_aliases.is_empty() {
            debug!(
                "Generating firewall rules for {} aliases used in ACL rule {id:?}",
                destination_aliases.len()
            );
        }
        for alias in destination_aliases {
            debug!("Processing ACL alias: {alias:?}");

            // alias.simplify()

            // fetch destination ranges for a given alias
            let alias_destination_ranges = alias.get_destination_ranges(&mut *conn).await?;

            // combine destination addrs
            let (dest_addrs_v4, dest_addrs_v6) =
                process_alias_destination_addrs(&alias.destination, alias_destination_ranges);

            // process alias ports
            let alias_ports = alias.ports.into_iter().map(Into::into).collect::<Vec<_>>();
            let destination_ports = merge_port_ranges(alias_ports);

            // remove duplicate protocol entries
            let mut protocols = alias.protocols;
            protocols.sort_unstable();
            protocols.dedup();

            let comment = format!(
                "ACL {} - {}, ALIAS {} - {}",
                acl.id, acl.name, alias.id, alias.name
            );
            if has_ipv4_addresses {
                // create IPv4 rules
                let ipv4_rules = create_rules(
                    alias.id,
                    IpVersion::Ipv4,
                    &ipv4_source_addrs,
                    &dest_addrs_v4,
                    &destination_ports,
                    &protocols,
                    &comment,
                );
                if let Some(rule) = ipv4_rules.0 {
                    allow_rules.push(rule);
                }
                deny_rules.push(ipv4_rules.1);
            }

            if has_ipv6_addresses {
                // create IPv6 rules
                let ipv6_rules = create_rules(
                    alias.id,
                    IpVersion::Ipv6,
                    &ipv6_source_addrs,
                    &dest_addrs_v6,
                    &destination_ports,
                    &protocols,
                    &comment,
                );
                if let Some(rule) = ipv6_rules.0 {
                    allow_rules.push(rule);
                }
                deny_rules.push(ipv6_rules.1);
            }
        }
    }

    // combine both rule lists
    Ok(allow_rules.into_iter().chain(deny_rules).collect())
}

/// Creates ALLOW and DENY rules for given set of source, destination
/// addresses, ports and protocols. The DENY rule should block all
/// remaining traffic to the destination from sources other than specified.
///
/// Returs a 2-tuple where the first field is an `Option` with the ALLOW
/// rule if it should be created and the second field is the DENY rule.
fn create_rules(
    id: Id,
    ip_version: IpVersion,
    source_addrs: &[IpAddress],
    destination_addrs: &[IpAddress],
    destination_ports: &[Port],
    protocols: &[Protocol],
    comment: &str,
) -> (Option<FirewallRule>, FirewallRule) {
    let ip_version = i32::from(ip_version);
    let allow = if source_addrs.is_empty() {
        debug!("Source address list is empty. Skipping generating the ALLOW rule for this ACL");
        None
    } else {
        // prepare ALLOW rule
        let rule = FirewallRule {
            id,
            source_addrs: source_addrs.to_vec(),
            destination_addrs: destination_addrs.to_vec(),
            destination_ports: destination_ports.to_vec(),
            protocols: protocols.to_vec(),
            verdict: i32::from(FirewallPolicy::Allow),
            comment: Some(format!("{comment} ALLOW")),
            ip_version,
        };
        debug!("ALLOW rule generated from ACL: {rule:?}");
        Some(rule)
    };
    // prepare DENY rule
    // it should specify only the destination addrs to block all remaining traffic
    let deny = FirewallRule {
        id,
        source_addrs: Vec::new(),
        destination_addrs: destination_addrs.to_vec(),
        destination_ports: Vec::new(),
        protocols: Vec::new(),
        verdict: i32::from(FirewallPolicy::Deny),
        comment: Some(format!("{comment} DENY")),
        ip_version,
    };
    debug!("DENY rule generated from ACL: {deny:?}");

    (allow, deny)
}

/// Prepares a list of all relevant users whose device IPs we'll need to prepare
/// source config for a firewall rule.
///
/// Source addrs are only needed for the ALLOW rule, so we need to take the allowed users and
/// remove any explicitly denied users.
fn get_source_users(allowed_users: Vec<User<Id>>, denied_users: &[User<Id>]) -> Vec<User<Id>> {
    // start with allowed users and remove those explicitly denied
    allowed_users
        .into_iter()
        .filter(|user| !denied_users.contains(user))
        .collect()
}

/// Fetches all IPs of devices belonging to specified users within a given location's VPN subnet.
/// We specifically only fetch user devices since network devices are handled separately.
async fn get_user_device_ips<'e, E: sqlx::PgExecutor<'e>>(
    users: &[User<Id>],
    location_id: Id,
    executor: E,
) -> Result<Vec<Vec<IpAddr>>, SqlxError> {
    // prepare a list of user IDs
    let user_ids: Vec<Id> = users.iter().map(|user| user.id).collect();

    // fetch network IPs
    query_scalar!(
            "SELECT wireguard_ips \"wireguard_ips: Vec<IpAddr>\" \
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
    denied_devices: &[Device<Id>],
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
) -> Result<Vec<Vec<IpAddr>>, SqlxError> {
    // prepare a list of IDs
    let network_device_ids: Vec<Id> = network_devices.iter().map(|device| device.id).collect();

    // fetch network IPs
    query_scalar!(
        "SELECT wireguard_ips \"wireguard_ips: Vec<IpAddr>\" \
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
                    Some(ip..=ip)
                } else {
                    None
                }
            }
            IpVersion::Ipv6 => {
                if ip.is_ipv6() {
                    Some(ip..=ip)
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
/// - merging into the smallest possible list of non-overlapping ranges,
///   subnets and addresses
///
/// Return a 2-tuple of `Vec<IpAddress>` with all IPv4 addresses in the
/// first field and IPv6 addresses in the second.
fn process_destination_addrs(
    dest_ipnets: &[IpNetwork],
    mut dest_ranges: Vec<AclRuleDestinationRange<Id>>,
) -> (Vec<IpAddress>, Vec<IpAddress>) {
    // Remove all IP address ranges that fit in the networks.
    for dest in dest_ipnets {
        dest_ranges.retain(|range| !range.fits_in_network(dest));
    }

    // Separate IP v4 and v6 addresses.
    let mut ipv4_dest_addrs = dest_ipnets
        .iter()
        .filter(|dst| dst.is_ipv4())
        .map(|addr| IpAddress {
            address: Some(if u32::from(addr.prefix()) == Ipv4Addr::BITS {
                Address::Ip(addr.ip().to_string())
            } else {
                Address::IpSubnet(addr.to_string())
            }),
        })
        .collect::<Vec<_>>();
    let mut ipv6_dest_addrs = dest_ipnets
        .iter()
        .filter(|dst| dst.is_ipv6())
        .map(|addr| {
            let addr_string = addr.to_string();
            IpAddress {
                address: Some(if u32::from(addr.prefix()) == Ipv6Addr::BITS {
                    Address::Ip(addr.ip().to_string())
                } else {
                    Address::IpSubnet(addr_string)
                }),
            }
        })
        .collect::<Vec<_>>();

    // Separate IP v4 and v6 ranges.
    let ipv4_dest_ranges = dest_ranges
        .iter()
        .filter(|dst| dst.start.is_ipv4() && dst.end.is_ipv4())
        .map(RangeInclusive::from)
        .collect();
    let ipv6_dest_ranges = dest_ranges
        .iter()
        .filter(|dst| dst.start.is_ipv6() && dst.end.is_ipv6())
        .map(RangeInclusive::from)
        .collect();

    ipv4_dest_addrs.append(&mut merge_addrs(ipv4_dest_ranges));
    ipv6_dest_addrs.append(&mut merge_addrs(ipv6_dest_ranges));

    (ipv4_dest_addrs, ipv6_dest_addrs)
}

/// Convert destination networks and ranges configured in an ACL alias
/// into the correct format for a firewall rule. This includes:
/// - combining all addr lists
/// - converting to gRPC IpAddress struct
/// - merging into the smallest possible list of non-overlapping ranges,
///   subnets and addresses
///
/// Return a 2-tuple of `Vec<IpAddress>` with all IPv4 addresses in the
/// first field and IPv6 addresses in the second.
fn process_alias_destination_addrs(
    dest_ipnets: &[IpNetwork],
    mut dest_ranges: Vec<AclAliasDestinationRange<Id>>,
) -> (Vec<IpAddress>, Vec<IpAddress>) {
    // Remove all IP address ranges that fit in the networks.
    for dest in dest_ipnets {
        dest_ranges.retain(|range| !range.fits_in_network(dest));
    }

    // Separate IP v4 and v6 addresses.
    let mut ipv4_dest_addrs = dest_ipnets
        .iter()
        .filter(|dst| dst.is_ipv4())
        .map(|addr| IpAddress {
            address: Some(if u32::from(addr.prefix()) == Ipv4Addr::BITS {
                Address::Ip(addr.ip().to_string())
            } else {
                Address::IpSubnet(addr.to_string())
            }),
        })
        .collect::<Vec<_>>();
    let mut ipv6_dest_addrs = dest_ipnets
        .iter()
        .filter(|dst| dst.is_ipv6())
        .map(|addr| {
            let addr_string = addr.to_string();
            IpAddress {
                address: Some(if u32::from(addr.prefix()) == Ipv6Addr::BITS {
                    Address::Ip(addr.ip().to_string())
                } else {
                    Address::IpSubnet(addr_string)
                }),
            }
        })
        .collect::<Vec<_>>();

    // Separate IP v4 and v6 ranges.
    let ipv4_dest_ranges = dest_ranges
        .iter()
        .filter(|dst| dst.start.is_ipv4() && dst.end.is_ipv4())
        .map(RangeInclusive::from)
        .collect();
    let ipv6_dest_ranges = dest_ranges
        .iter()
        .filter(|dst| dst.start.is_ipv6() && dst.end.is_ipv6())
        .map(RangeInclusive::from)
        .collect();

    ipv4_dest_addrs.append(&mut merge_addrs(ipv4_dest_ranges));
    ipv6_dest_addrs.append(&mut merge_addrs(ipv6_dest_ranges));

    (ipv4_dest_addrs, ipv6_dest_addrs)
}

#[cfg(test)]
fn get_last_ip_in_v6_subnet(subnet: &ipnetwork::Ipv6Network) -> IpAddr {
    // get subnet IP portion as u128
    let first_ip = subnet.ip().to_bits();

    let last_ip = first_ip | (!u128::from(subnet.mask()));

    IpAddr::V6(last_ip.into())
}

/// Converts an arbitrary list of IP address ranges into the smallest possible list
/// of non-overlapping elements which can be used in a firewall rule.
/// It assumes that all ranges with an invalid IP version have already been filtered out.
fn merge_addrs(addr_ranges: Vec<RangeInclusive<IpAddr>>) -> Vec<IpAddress> {
    // merge into non-overlapping ranges
    let addr_ranges = merge_ranges(addr_ranges);

    // convert to gRPC format
    let mut result = Vec::new();
    for range in addr_ranges {
        let (range_start, range_end) = range.into_inner();
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
            let range_start = *range.start();
            let range_end = *range.end();
            if range_start == range_end {
                Port {
                    port: Some(PortInner::SinglePort(u32::from(range_start))),
                }
            } else {
                Port {
                    port: Some(PortInner::PortRange(PortRangeProto {
                        start: u32::from(range_start),
                        end: u32::from(range_end),
                    })),
                }
            }
        })
        .collect()
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
            "SELECT DISTINCT ON (a.id) a.id, name, allow_all_users, deny_all_users, all_networks, \
            allow_all_network_devices, deny_all_network_devices, destination, ports, protocols, \
            expires, enabled, parent_id, state \
            FROM aclrule a \
            LEFT JOIN aclrulenetwork an \
            ON a.id = an.rule_id \
            WHERE (an.network_id = $1 OR a.all_networks = true) AND enabled = true \
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
                "Enterprise features are disabled, skipping generating firewall config for \
                location {self}"
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

        let default_policy = if self.acl_default_allow {
            FirewallPolicy::Allow
        } else {
            FirewallPolicy::Deny
        };
        let firewall_rules =
            generate_firewall_rules_from_acls(self.id, location_acls, &mut *conn).await?;
        let firewall_config = FirewallConfig {
            default_policy: default_policy.into(),
            rules: firewall_rules,
        };

        debug!("Firewall config generated for location {self}: {firewall_config:?}");
        Ok(Some(firewall_config))
    }
}

#[cfg(test)]
mod tests;
