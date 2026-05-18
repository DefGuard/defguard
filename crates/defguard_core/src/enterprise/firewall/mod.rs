use std::{net::IpAddr, ops::RangeInclusive};

use defguard_common::db::{
    Id,
    models::{Device, ModelError, WireguardNetwork, user::User},
};
use defguard_proto::enterprise::firewall::{
    FirewallConfig, FirewallPolicy, FirewallRule, IpAddress, IpRange, IpVersion, Port,
    PortRange as PortRangeProto, SnatBinding as SnatBindingProto, ip_address::Address,
    port::Port as PortInner,
};
use ipnetwork::IpNetwork;
use sqlx::{PgConnection, query_as, query_scalar};

use super::{
    db::models::acl::{AclRule, AclRuleDestinationRange, AclRuleInfo, PortRange, Protocol},
    utils::{extract_subnets_from_range, get_last_ip_in_v6_subnet, merge_ranges},
};
use crate::enterprise::{
    db::models::{acl::AclAlias, snat::UserSnatBinding},
    is_business_license_active,
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
/// Additionally a separate set of rules is created for each pre-defined `Destination` used
/// as part of the rule.
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
    let has_ipv4_addresses = location.address().iter().any(IpNetwork::is_ipv4);
    let has_ipv6_addresses = location.address().iter().any(IpNetwork::is_ipv6);

    // convert each ACL into a corresponding `FirewallRule`s
    for acl in acl_rules {
        debug!("Processing ACL rule: {acl:?}");
        // prepare source IPs
        let (ipv4_source_addrs, ipv6_source_addrs) =
            get_source_ips(&mut *conn, location_id, &acl).await?;

        // extract destination parameters from ACL rule
        let AclRuleInfo {
            id,
            name: rule_name,
            addresses,
            address_ranges,
            ports,
            protocols,
            aliases,
            destinations,
            any_address,
            any_port,
            any_protocol,
            use_manual_destination_settings,
            ..
        } = acl;

        // check if we need to add rules for manually defined destination
        if use_manual_destination_settings {
            let (manual_destination_allow_rules, manual_destination_deny_rules) =
                get_manual_destination_rules(
                    &mut *conn,
                    id,
                    &rule_name,
                    has_ipv4_addresses,
                    has_ipv6_addresses,
                    (&ipv4_source_addrs, &ipv6_source_addrs),
                    aliases,
                    addresses,
                    address_ranges,
                    ports,
                    protocols,
                    any_address,
                    any_port,
                    any_protocol,
                )
                .await?;

            // append generated rules to output
            allow_rules.extend(manual_destination_allow_rules);
            deny_rules.extend(manual_destination_deny_rules);
        }

        // process destination aliases by creating a dedicated set of rules for each of them
        if !destinations.is_empty() {
            debug!(
                "Generating firewall rules for {} pre-defined destinations used in ACL rule {id:?}",
                destinations.len()
            );
        }
        for destination in destinations {
            debug!("Processing ACL pre-defined destination: {destination:?}");
            let (destination_allow_rules, destination_deny_rules) =
                get_predefined_destination_rules(
                    &mut *conn,
                    destination,
                    acl.id,
                    &rule_name,
                    has_ipv4_addresses,
                    has_ipv6_addresses,
                    (&ipv4_source_addrs, &ipv6_source_addrs),
                )
                .await?;

            // append generated rules to output
            allow_rules.extend(destination_allow_rules);
            deny_rules.extend(destination_deny_rules);
        }
    }

    // combine both rule lists
    Ok(allow_rules.into_iter().chain(deny_rules).collect())
}

/// Prepare two lists of source IPs split between IPv4 and IPv6.
///
/// This is achieved on first determining allowed users and network devices
/// and then getting assigned IP addresses of their devices.
async fn get_source_ips(
    conn: &mut PgConnection,
    location_id: Id,
    acl: &AclRuleInfo<Id>,
) -> Result<(Vec<IpAddress>, Vec<IpAddress>), FirewallError> {
    // fetch allowed users
    let allowed_users = acl.get_all_allowed_users(&mut *conn).await?;

    // fetch denied users
    let denied_users = acl.get_all_denied_users(&mut *conn).await?;

    // get relevant users for determining source IPs
    let source_users = get_source_users(allowed_users, &denied_users);

    // prepare a list of user IDs
    let source_user_ids: Vec<Id> = source_users.iter().map(|user| user.id).collect();

    // get network IPs for devices belonging to those users
    let source_user_device_ips =
        get_user_device_ips(&source_user_ids, location_id, &mut *conn).await?;
    // separate IPv4 and IPv6 user-device addresses
    let source_user_device_ips = source_user_device_ips
        .iter()
        .flatten()
        .partition(|ip| ip.is_ipv4());

    // fetch allowed network devices
    let allowed_network_devices = acl.get_all_allowed_devices(&mut *conn, location_id).await?;

    // fetch denied network devices
    let denied_network_devices = acl.get_all_denied_devices(&mut *conn, location_id).await?;

    // get network device IPs for rule source
    let source_network_devices =
        get_source_network_devices(allowed_network_devices, &denied_network_devices);
    let source_network_device_ips =
        get_network_device_ips(&source_network_devices, location_id, &mut *conn).await?;

    // separate IPv4 and IPv6 network-device addresses
    let source_network_device_ips = source_network_device_ips
        .iter()
        .flatten()
        .partition(|ip| ip.is_ipv4());

    // convert device IPs into source addresses for a firewall rule
    let ipv4_source_addrs = get_source_addrs(
        source_user_device_ips.0,
        source_network_device_ips.0,
        IpVersion::Ipv4,
    );
    let ipv6_source_addrs = get_source_addrs(
        source_user_device_ips.1,
        source_network_device_ips.1,
        IpVersion::Ipv6,
    );
    Ok((ipv4_source_addrs, ipv6_source_addrs))
}

/// Generates firewall rules for destination manually specified in ACL rule.
async fn get_manual_destination_rules(
    conn: &mut PgConnection,
    rule_id: Id,
    rule_name: &str,
    location_has_ipv4_addresses: bool,
    location_has_ipv6_addresses: bool,
    source_addrs: (&[IpAddress], &[IpAddress]),
    aliases: Vec<AclAlias<Id>>,
    mut addresses: Vec<IpNetwork>,
    address_ranges: Vec<AclRuleDestinationRange<i64>>,
    mut ports: Vec<PortRange>,
    mut protocols: Vec<i32>,
    any_address: bool,
    any_port: bool,
    any_protocol: bool,
) -> Result<(Vec<FirewallRule>, Vec<FirewallRule>), FirewallError> {
    debug!("Generating firewall rules for manually configured destination in ACL rule {rule_id}");
    // store alias ranges separately since they use a different struct
    let mut alias_destination_ranges = Vec::new();

    // process aliases by appending destination parameters from each of them to
    // existing lists
    for alias in aliases {
        // fetch destination ranges for a given alias
        alias_destination_ranges.extend(alias.get_destination_ranges(&mut *conn).await?);

        // extend existing parameter lists
        addresses.extend(alias.addresses);
        ports.extend(alias.ports.into_iter().map(Into::into).collect::<Vec<_>>());
        protocols.extend(alias.protocols);
    }

    // prepare destination addresses
    let (dest_addrs_v4, dest_addrs_v6) = if any_address {
        (Vec::new(), Vec::new())
    } else {
        process_destination_addrs(
            &addresses,
            address_ranges
                .iter()
                .map(RangeInclusive::from)
                .chain(alias_destination_ranges.iter().map(RangeInclusive::from)),
        )
    };

    // prepare destination ports
    let destination_ports = if any_port {
        Vec::new()
    } else {
        merge_port_ranges(ports)
    };

    // remove duplicate protocol entries
    let destination_protocols = if any_protocol {
        Vec::new()
    } else {
        protocols.sort_unstable();
        protocols.dedup();
        protocols
    };

    let (ipv4_source_addrs, ipv6_source_addrs) = source_addrs;

    // only generate rules for a given IP version if there is a destination address of a given type
    // or any destination toggle is enabled and location uses addresses of a given type
    let has_ipv4_destination =
        !dest_addrs_v4.is_empty() || (location_has_ipv4_addresses && any_address);
    let has_ipv6_destination =
        !dest_addrs_v6.is_empty() || (location_has_ipv6_addresses && any_address);

    let comment = format!("RULE {rule_id} - {rule_name}");
    let mut allow_rules = Vec::new();
    let mut deny_rules = Vec::new();
    if has_ipv4_destination {
        // create IPv4 rules
        let ipv4_rules = create_rules(
            rule_id,
            IpVersion::Ipv4,
            ipv4_source_addrs,
            &dest_addrs_v4,
            &destination_ports,
            &destination_protocols,
            &comment,
        );
        if let Some(rule) = ipv4_rules.0 {
            allow_rules.push(rule);
        }
        deny_rules.push(ipv4_rules.1);
    }

    if has_ipv6_destination {
        // create IPv6 rules
        let ipv6_rules = create_rules(
            rule_id,
            IpVersion::Ipv6,
            ipv6_source_addrs,
            &dest_addrs_v6,
            &destination_ports,
            &destination_protocols,
            &comment,
        );
        if let Some(rule) = ipv6_rules.0 {
            allow_rules.push(rule);
        }
        deny_rules.push(ipv6_rules.1);
    }

    Ok((allow_rules, deny_rules))
}

/// Generates firewall rules for pre-defined destination used in ACL rule.
async fn get_predefined_destination_rules(
    conn: &mut PgConnection,
    destination: AclAlias<Id>,
    rule_id: Id,
    rule_name: &str,
    location_has_ipv4_addresses: bool,
    location_has_ipv6_addresses: bool,
    source_addrs: (&[IpAddress], &[IpAddress]),
) -> Result<(Vec<FirewallRule>, Vec<FirewallRule>), FirewallError> {
    // fetch destination ranges for a given destination
    let alias_destination_ranges = destination.get_destination_ranges(&mut *conn).await?;

    // combine destination addrs
    let (dest_addrs_v4, dest_addrs_v6) = if destination.any_address {
        (Vec::new(), Vec::new())
    } else {
        process_destination_addrs(
            &destination.addresses,
            alias_destination_ranges.iter().map(RangeInclusive::from),
        )
    };

    // process alias ports
    let destination_ports = if destination.any_port {
        Vec::new()
    } else {
        let alias_ports = destination
            .ports
            .into_iter()
            .map(Into::into)
            .collect::<Vec<_>>();
        merge_port_ranges(alias_ports)
    };

    // process destination protocols
    let destination_protocols = if destination.any_protocol {
        Vec::new()
    } else {
        let mut protocols = destination.protocols;
        protocols.sort_unstable();
        protocols.dedup();
        protocols
    };

    let (ipv4_source_addrs, ipv6_source_addrs) = source_addrs;

    // only generate rules for a given IP version if there is a destination address of a given type
    // or any destination toggle is enabled and location uses addresses of a given type
    let has_ipv4_destination =
        !dest_addrs_v4.is_empty() || (location_has_ipv4_addresses && destination.any_address);
    let has_ipv6_destination =
        !dest_addrs_v6.is_empty() || (location_has_ipv6_addresses && destination.any_address);

    let comment = format!(
        "RULE {rule_id} - {rule_name}, DESTINATION {} - {}",
        destination.id, destination.name
    );
    let mut allow_rules = Vec::new();
    let mut deny_rules = Vec::new();
    if has_ipv4_destination {
        // create IPv4 rules
        let ipv4_rules = create_rules(
            destination.id,
            IpVersion::Ipv4,
            ipv4_source_addrs,
            &dest_addrs_v4,
            &destination_ports,
            &destination_protocols,
            &comment,
        );
        if let Some(rule) = ipv4_rules.0 {
            allow_rules.push(rule);
        }
        deny_rules.push(ipv4_rules.1);
    }

    if has_ipv6_destination {
        // create IPv6 rules
        let ipv6_rules = create_rules(
            destination.id,
            IpVersion::Ipv6,
            ipv6_source_addrs,
            &dest_addrs_v6,
            &destination_ports,
            &destination_protocols,
            &comment,
        );
        if let Some(rule) = ipv6_rules.0 {
            allow_rules.push(rule);
        }
        deny_rules.push(ipv6_rules.1);
    }

    Ok((allow_rules, deny_rules))
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
    user_ids: &[Id],
    location_id: Id,
    executor: E,
) -> sqlx::Result<Vec<Vec<IpAddr>>> {
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
) -> sqlx::Result<Vec<Vec<IpAddr>>> {
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
            IpVersion::Unspecified => None,
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
fn process_destination_addrs<I>(
    dest_ipnets: &[IpNetwork],
    dest_ranges: I,
) -> (Vec<IpAddress>, Vec<IpAddress>)
where
    I: IntoIterator<Item = RangeInclusive<IpAddr>>,
{
    // Separate IP v4 and v6 addresses and convert networks to intermediate range representation for merging
    let ipv4_dest_net_addrs = dest_ipnets
        .iter()
        .filter(|dst| dst.is_ipv4())
        .map(|dst| dst.network()..=dst.broadcast());
    let ipv6_dest_net_addrs = dest_ipnets.iter().filter_map(|dst| {
        if let IpNetwork::V6(subnet) = dst {
            let range_start = subnet.network().into();
            let range_end = get_last_ip_in_v6_subnet(subnet);
            Some(range_start..=range_end)
        } else {
            None
        }
    });

    // Separate IP v4 and v6 ranges.
    let (ipv4_dest_ranges, ipv6_dest_ranges) =
        dest_ranges
            .into_iter()
            .fold((Vec::new(), Vec::new()), |mut ranges, range| {
                if range.start().is_ipv4() && range.end().is_ipv4() {
                    ranges.0.push(range);
                } else if range.start().is_ipv6() && range.end().is_ipv6() {
                    ranges.1.push(range);
                }
                ranges
            });

    // combine iterators
    let ipv4_dest_addrs = ipv4_dest_net_addrs.chain(ipv4_dest_ranges).collect();
    let ipv6_dest_addrs = ipv6_dest_net_addrs.chain(ipv6_dest_ranges).collect();

    (merge_addrs(ipv4_dest_addrs), merge_addrs(ipv6_dest_addrs))
}

/// Converts an IP address range into `Vec<IpAddress>` for use in firewall rules,
/// delegating decomposition to the shared [`extract_subnets_from_range`] utility and
/// mapping each resulting [`IpNetwork`] to the appropriate gRPC [`IpAddress`] variant.
/// Falls back to [`Address::IpRange`] when the range cannot be expressed as any CIDR.
fn extract_all_subnets_from_range(range_start: IpAddr, range_end: IpAddr) -> Vec<IpAddress> {
    let networks = extract_subnets_from_range(range_start, range_end);

    // If decomposition produced nothing for a multi-IP range, the range straddles
    // a CIDR boundary and cannot be expressed as subnets - fall back to IpRange.
    if networks.is_empty() && range_start != range_end {
        return vec![IpAddress {
            address: Some(Address::IpRange(IpRange {
                start: range_start.to_string(),
                end: range_end.to_string(),
            })),
        }];
    }

    networks
        .into_iter()
        .map(|network| {
            let is_host = (network.is_ipv4() && network.prefix() == 32)
                || (network.is_ipv6() && network.prefix() == 128);
            if is_host {
                IpAddress {
                    address: Some(Address::Ip(network.ip().to_string())),
                }
            } else {
                IpAddress {
                    address: Some(Address::IpSubnet(network.to_string())),
                }
            }
        })
        .collect()
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
        result.extend(extract_all_subnets_from_range(range_start, range_end));
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

/// Converts user SNAT bindings into SNAT config to be sent to a gateway as part of `FirewallConfig`.
///
/// To generate the final SNAT binding we need to find all user devices
/// and get their IPs to generate a list of source addresses for a firewall rule.
async fn generate_user_snat_bindings_for_location(
    location_id: Id,
    conn: &mut PgConnection,
) -> sqlx::Result<Vec<SnatBindingProto>> {
    debug!("Generating SNAT bindings for location {location_id}");

    let user_snat_bindings = UserSnatBinding::all_for_location(&mut *conn, location_id).await?;

    // check if there are any bindings configured for this location
    if user_snat_bindings.is_empty() {
        debug!("No user SNAT bindings configured for location {location_id}");
        return Ok(Vec::new());
    }

    // initialize output list
    let mut bindings = Vec::new();

    // process each user SNAT binding
    for user_binding in user_snat_bindings {
        let user_id = user_binding.user_id;

        debug!(
            "Processing SNAT binding for user {user_id} with public IP {}",
            user_binding.public_ip
        );

        // determine IP protocol version based on public IP
        let is_ipv4 = user_binding.public_ip.is_ipv4();

        // fetch all device IPs for this specific user in the location
        let user_device_ips = get_user_device_ips(&[user_id], location_id, &mut *conn).await?;

        // separate IPv4 and IPv6 user-device addresses
        let (user_device_ips_v4, user_device_ips_v6) = user_device_ips
            .iter()
            .flatten()
            .partition(|ip| ip.is_ipv4());

        // convert device IPs into source addresses for a firewall rule
        let source_addrs = if is_ipv4 {
            get_source_addrs(user_device_ips_v4, Vec::new(), IpVersion::Ipv4)
        } else {
            get_source_addrs(user_device_ips_v6, Vec::new(), IpVersion::Ipv6)
        };

        if source_addrs.is_empty() {
            debug!(
                "No compatible device IPs found for user {user_id} in location {location_id} with \
                public IP {}, skipping SNAT binding",
                user_binding.public_ip
            );
            continue;
        }

        // create the SNAT binding proto
        let snat_binding = SnatBindingProto {
            id: user_binding.id,
            source_addrs,
            public_ip: user_binding.public_ip.to_string(),
            comment: Some(format!("User {user_id} SNAT binding {}", user_binding.id)),
        };

        debug!(
            "Created SNAT binding for user {user_id} in location {location_id}: {snat_binding:?}",
        );

        // add to output list
        bindings.push(snat_binding);
    }

    debug!(
        "Generated {} SNAT bindings for location {location_id}",
        bindings.len(),
    );

    Ok(bindings)
}

/// Fetches all active ACL rules for a given location.
/// Filters out rules which are disabled, expired or have not been deployed yet.
pub(crate) async fn get_location_active_acl_rules(
    location: &WireguardNetwork<Id>,
    conn: &mut PgConnection,
) -> sqlx::Result<Vec<AclRuleInfo<Id>>> {
    debug!("Fetching active ACL rules for location {location}");
    let rules: Vec<AclRule<Id>> = query_as(
        "SELECT DISTINCT ON (a.id) a.id, name, allow_all_users, deny_all_users, all_locations, \
        allow_all_groups, deny_all_groups, \
        allow_all_network_devices, deny_all_network_devices, addresses, ports, protocols, \
        expires, enabled, parent_id, state, any_address, any_port, any_protocol,
        use_manual_destination_settings, modified_at, modified_by \
        FROM aclrule a \
        LEFT JOIN aclrulenetwork an ON a.id = an.rule_id \
        WHERE (an.network_id = $1 OR a.all_locations) AND enabled \
        AND state = 'applied'::aclrule_state \
        AND (expires IS NULL OR expires > NOW())",
    )
    .bind(location.id)
    .fetch_all(&mut *conn)
    .await?;
    debug!(
        "Found {} active ACL rules for location {location}",
        rules.len()
    );

    // convert to `AclRuleInfo`
    let mut rules_info = Vec::new();
    for rule in rules {
        let rule_info = rule.to_info(&mut *conn).await?;
        rules_info.push(rule_info);
    }
    Ok(rules_info)
}

/// Prepares firewall configuration for Gateway based on location config and ACLs.
/// Returns `None` if firewall management is disabled for a given location.
pub async fn try_get_location_firewall_config(
    location: &WireguardNetwork<Id>,
    conn: &mut PgConnection,
) -> Result<Option<FirewallConfig>, FirewallError> {
    // do a license check
    if !is_business_license_active() {
        debug!(
            "Enterprise features are disabled, skipping generating firewall config for \
            location {location}"
        );
        return Ok(None);
    }

    // check if ACLs are enabled
    if !location.acl_enabled {
        debug!(
            "ACL rules are disabled for location {location}, skipping generating firewall config"
        );
        return Ok(None);
    }

    info!("Generating firewall config for location {location}");
    // fetch all active ACLs for location
    let location_acls = get_location_active_acl_rules(location, &mut *conn).await?;

    let default_policy = if location.acl_default_allow {
        FirewallPolicy::Allow
    } else {
        FirewallPolicy::Deny
    };
    let firewall_rules =
        generate_firewall_rules_from_acls(location.id, location_acls, &mut *conn).await?;
    let snat_bindings = generate_user_snat_bindings_for_location(location.id, &mut *conn).await?;
    let firewall_config = FirewallConfig {
        default_policy: default_policy.into(),
        rules: firewall_rules,
        snat_bindings,
    };

    debug!("Firewall config generated for location {location}: {firewall_config:?}");
    Ok(Some(firewall_config))
}

#[cfg(test)]
mod tests;
