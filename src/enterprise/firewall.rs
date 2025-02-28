use std::net::IpAddr;

use ipnetwork::IpNetwork;
use sqlx::{
    postgres::types::PgRange, query_as, query_scalar, Error as SqlxError, PgExecutor, PgPool,
};

use super::db::models::acl::AclRule;

use crate::{
    db::{models::error::ModelError, Id, User, WireguardNetwork},
    grpc::proto::enterprise::firewall::{
        ip_address::Address, port::Port as PortInner, FirewallConfig, FirewallPolicy, FirewallRule,
        IpAddress, IpVersion, Port, PortRange,
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
    acl_rules: Vec<AclRule<Id>>,
    pool: &PgPool,
) -> Result<Vec<FirewallRule>, FirewallError> {
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
        // fetch allowed users
        let allowed_users = acl.get_all_allowed_users(pool).await?;

        // fetch denied users
        let denied_users = acl.get_all_denied_users(pool).await?;

        // fetch aliases used by ACL
        // TODO: prefetch a map to reduce number of queries
        let aliases = acl.get_aliases(pool).await?;

        // prepare a list of all relevant users whose device IPs we'll need to prepare a firewall
        // rule
        //
        // depending on the default policy we either need:
        // - allowed users if default policy is `Deny`
        // - denied users if default policy is `Allow`
        let users: Vec<User<Id>> = match default_location_policy {
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

        // prepare source addrs
        // TODO: convert into non-overlapping elements
        let source_addrs = user_device_ips
            .iter()
            .filter_map(|ip| match ip_version {
                IpVersion::Ipv4 => {
                    if ip.is_ipv4() {
                        Some(IpAddress {
                            address: Some(Address::Ip(ip.to_string())),
                        })
                    } else {
                        None
                    }
                }
                IpVersion::Ipv6 => {
                    if ip.is_ipv6() {
                        Some(IpAddress {
                            address: Some(Address::Ip(ip.to_string())),
                        })
                    } else {
                        None
                    }
                }
            })
            .collect();

        let AclRule {
            mut destination,
            mut ports,
            mut protocols,
            ..
        } = acl;

        // process aliases
        for alias in aliases {
            destination.extend(alias.destination);
            ports.extend(alias.ports);
            protocols.extend(alias.protocols);
        }

        // prepare destination addresses
        // TODO: convert into non-overlapping elements
        let destination_addrs = destination
            .iter()
            .filter_map(|dst| match ip_version {
                IpVersion::Ipv4 => {
                    if dst.is_ipv4() {
                        Some(IpAddress {
                            address: Some(Address::IpSubnet(dst.to_string())),
                        })
                    } else {
                        None
                    }
                }
                IpVersion::Ipv6 => {
                    if dst.is_ipv6() {
                        Some(IpAddress {
                            address: Some(Address::IpSubnet(dst.to_string())),
                        })
                    } else {
                        None
                    }
                }
            })
            .collect();

        // prepare destination ports
        let destination_ports = merge_port_ranges(ports);

        // prepare protocols
        // remove duplicates
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
        firewall_rules.push(firewall_rule)
    }
    Ok(firewall_rules)
}

/// TODO: Implement once data model is finalized and address processing can be generalized
/// Helper function which prepares a list of IP addresses by doing the following:
/// - filter out addresses of different type than the VPN subnet
/// - remove duplicate elements
/// - transform subnets, ranges, IPs into non-overlapping elements
/// - convert to format expected by `FirewallRule` gRPC struct
fn process_ip_addrs(_addrs: Vec<IpAddr>, _location_ip_version: IpVersion) {
    unimplemented!()
}

/// Helper to extract actual starp port value from `Bound` enum used by `PgRange`.
fn get_start_port_from_range(range: &PgRange<i32>) -> u32 {
    match range.start {
        std::ops::Bound::Included(value) => value as u32,
        std::ops::Bound::Excluded(value) => (value + 1) as u32,
        std::ops::Bound::Unbounded => u32::MIN,
    }
}

/// Helper to extract actual end port value from `Bound` enum used by `PgRange`.
fn get_end_port_from_range(range: &PgRange<i32>) -> u32 {
    match range.end {
        std::ops::Bound::Included(value) => value as u32,
        std::ops::Bound::Excluded(value) => (value - 1) as u32,
        // type casting because protobuf doesn't have u16, but max port number is u16::MAX
        std::ops::Bound::Unbounded => u16::MAX as u32,
    }
}

/// Takes a list of port ranges and returns the smallest possible non-overlapping list of `Port`s.
fn merge_port_ranges(mut port_ranges: Vec<PgRange<i32>>) -> Vec<Port> {
    // return early if list is empty
    if port_ranges.is_empty() {
        return Vec::new();
    }

    // sort elements by range start
    port_ranges.sort_by(|a, b| {
        let a_start = get_start_port_from_range(a);
        let b_start = get_start_port_from_range(b);
        a_start.cmp(&b_start)
    });

    let mut merged_ranges = Vec::new();
    // start with first range
    let current_range = port_ranges.remove(0);
    let mut current_range_start = get_start_port_from_range(&current_range);
    let mut current_range_end = get_end_port_from_range(&current_range);

    // iterate over remaining ranges
    for range in port_ranges {
        let range_start = get_start_port_from_range(&range);
        let range_end = get_end_port_from_range(&range);

        // compare with current range
        if range_start <= current_range_end {
            // ranges are overlapping, merge them
            current_range_end = range_end;
        } else {
            // ranges are not overlapping, add current range to result
            let port_range = PortInner::PortRange(PortRange {
                start: current_range_start,
                end: current_range_end,
            });
            merged_ranges.push(port_range);
            current_range_start = range_start;
            current_range_end = range_end;
        }
    }

    // add last range
    let port_range = PortInner::PortRange(PortRange {
        start: current_range_start,
        end: current_range_end,
    });
    merged_ranges.push(port_range);

    // convert single-element ranges
    merged_ranges
        .into_iter()
        .map(|port| {
            if let PortInner::PortRange(range) = port {
                if range.start == range.end {
                    return Port {
                        port: Some(PortInner::SinglePort(range.start)),
                    };
                }
            }
            Port { port: Some(port) }
        })
        .collect()
}

impl WireguardNetwork<Id> {
    /// Fetches all active ACL rules for a given location.
    /// Filters out rules which are disabled, expired or have not been deployed yet.
    /// TODO: actually filter out unwanted rules once drafts etc are implemented
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
                destination, ports, protocols, expires \
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
    /// Returns `None` if firewall management is disabled for a given location.
    /// TODO: actually determine if a config should be generated
    pub async fn try_get_firewall_config(
        &self,
        pool: &PgPool,
    ) -> Result<Option<FirewallConfig>, FirewallError> {
        // fetch all active ACLs for location
        let location_acls = self.get_active_acl_rules(pool).await?;

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

        Ok(Some(FirewallConfig {
            ip_version: ip_version.into(),
            default_policy: default_policy.into(),
            rules: firewall_rules,
        }))
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
    use std::ops::Bound;

    use sqlx::postgres::types::PgRange;

    use crate::grpc::proto::enterprise::firewall::{port::Port as PortInner, Port, PortRange};

    use super::merge_port_ranges;

    // #[test]
    // fn test_non_overlapping_addrs() {
    //     unimplemented!()
    // }

    // #[test]
    // fn test_get_relevant_users() {
    //     unimplemented!()
    // }

    // #[test]
    // fn test_process_source_addrs_v4() {
    //     unimplemented!()
    // }

    // #[test]
    // fn test_process_source_addrs_v6() {
    //     unimplemented!()
    // }

    // #[test]
    // fn test_process_destination_addrs_v4() {
    //     unimplemented!()
    // }

    // #[test]
    // fn test_process_destination_addrs_v6() {
    //     unimplemented!()
    // }

    #[test]
    fn test_merge_port_ranges() {
        // single port
        let input_ranges = vec![PgRange {
            start: Bound::Included(100),
            end: Bound::Included(100),
        }];
        let merged = merge_port_ranges(input_ranges);
        assert_eq!(
            merged,
            vec![Port {
                port: Some(PortInner::SinglePort(100))
            }]
        );

        // overlapping ranges
        let input_ranges = vec![
            PgRange {
                start: Bound::Included(100),
                end: Bound::Included(200),
            },
            PgRange {
                start: Bound::Included(150),
                end: Bound::Included(220),
            },
            PgRange {
                start: Bound::Included(210),
                end: Bound::Included(300),
            },
        ];
        let merged = merge_port_ranges(input_ranges);
        assert_eq!(
            merged,
            vec![Port {
                port: Some(PortInner::PortRange(PortRange {
                    start: 100,
                    end: 300
                }))
            }]
        );

        // duplicate ranges
        let input_ranges = vec![
            PgRange {
                start: Bound::Included(100),
                end: Bound::Included(200),
            },
            PgRange {
                start: Bound::Included(100),
                end: Bound::Included(200),
            },
            PgRange {
                start: Bound::Included(150),
                end: Bound::Included(220),
            },
            PgRange {
                start: Bound::Included(150),
                end: Bound::Included(220),
            },
            PgRange {
                start: Bound::Included(210),
                end: Bound::Included(300),
            },
            PgRange {
                start: Bound::Included(210),
                end: Bound::Included(300),
            },
            PgRange {
                start: Bound::Included(350),
                end: Bound::Included(400),
            },
            PgRange {
                start: Bound::Included(350),
                end: Bound::Included(400),
            },
            PgRange {
                start: Bound::Included(350),
                end: Bound::Included(400),
            },
        ];
        let merged = merge_port_ranges(input_ranges);
        assert_eq!(
            merged,
            vec![
                Port {
                    port: Some(PortInner::PortRange(PortRange {
                        start: 100,
                        end: 300
                    }))
                },
                Port {
                    port: Some(PortInner::PortRange(PortRange {
                        start: 350,
                        end: 400
                    }))
                }
            ]
        );

        // non-consecutive ranges
        let input_ranges = vec![
            PgRange {
                start: Bound::Excluded(500),
                end: Bound::Excluded(700),
            },
            PgRange {
                start: Bound::Excluded(150),
                end: Bound::Included(220),
            },
            PgRange {
                start: Bound::Included(210),
                end: Bound::Included(300),
            },
            PgRange {
                start: Bound::Included(800),
                end: Bound::Included(800),
            },
            PgRange {
                start: Bound::Included(200),
                end: Bound::Included(210),
            },
            PgRange {
                start: Bound::Included(50),
                end: Bound::Excluded(51),
            },
        ];
        let merged = merge_port_ranges(input_ranges);
        assert_eq!(
            merged,
            vec![
                Port {
                    port: Some(PortInner::SinglePort(50))
                },
                Port {
                    port: Some(PortInner::PortRange(PortRange {
                        start: 151,
                        end: 300
                    }))
                },
                Port {
                    port: Some(PortInner::PortRange(PortRange {
                        start: 501,
                        end: 699
                    }))
                },
                Port {
                    port: Some(PortInner::SinglePort(800))
                }
            ]
        );
    }

    // #[test]
    // fn test_process_protocols() {
    //     unimplemented!()
    // }

    // #[sqlx::test]
    // async fn test_generate_firewall_rules() {
    //     unimplemented!()
    // }
}
