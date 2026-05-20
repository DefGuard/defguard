use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    ops::RangeInclusive,
};

use ipnetwork::IpNetwork;

/// Return next item.
/// This trait can be replaced by `std::iter::Step` once it becomes stable.
pub(crate) trait Next {
    /// Return next item.
    fn next(&self) -> Self;
}

impl Next for IpAddr {
    /// Returns the next IP address in sequence, handling overflow by wrapping.
    fn next(&self) -> Self {
        match self {
            Self::V4(ipv4) => Self::V4(Ipv4Addr::from_bits(ipv4.to_bits().wrapping_add(1))),
            Self::V6(ipv6) => Self::V6(Ipv6Addr::from_bits(ipv6.to_bits().wrapping_add(1))),
        }
    }
}

impl Next for u16 {
    fn next(&self) -> Self {
        self.wrapping_add(1)
    }
}

/// Returns the last IP address in an IPv6 subnet.
pub(crate) fn get_last_ip_in_v6_subnet(subnet: &ipnetwork::Ipv6Network) -> IpAddr {
    let first_ip = subnet.ip().to_bits();
    let last_ip = first_ip | (!subnet.mask().to_bits());
    IpAddr::V6(last_ip.into())
}

/// Finds the largest subnet that fits within the given IP address range.
/// Returns None if no valid subnet can be found.
pub(crate) fn find_largest_subnet_in_range(start: IpAddr, end: IpAddr) -> Option<IpNetwork> {
    if start > end {
        return None;
    }

    match (start, end) {
        (IpAddr::V4(start_v4), IpAddr::V4(end_v4)) => {
            find_largest_ipv4_subnet_in_range(start_v4, end_v4)
        }
        (IpAddr::V6(start_v6), IpAddr::V6(end_v6)) => {
            find_largest_ipv6_subnet_in_range(start_v6, end_v6)
        }
        _ => None, // Mixed IP versions
    }
}

/// Finds the largest IPv4 subnet that fits within the given range.
/// The subnet must contain more than one IP address since single IPs have their own
/// representation.
fn find_largest_ipv4_subnet_in_range(start: Ipv4Addr, end: Ipv4Addr) -> Option<IpNetwork> {
    let start_bits = start.to_bits();
    let end_bits = end.to_bits();

    // Find the largest prefix length where the subnet fits in the range.
    // We make some reasonable assumptions here and skip /0 and /32 networks.
    for prefix_len in 1..=31 {
        let mask = u32::MAX << (32 - prefix_len);

        // number of IPs in subnet
        let subnet_size = 1u32 << (32 - prefix_len);

        // try to find first and last address in subnet
        // in case the subnet does not align with first address in range
        // try next potential subnet start
        let network_addr = start_bits & mask;
        let network_addr = if network_addr < start_bits {
            // try next aligned address and handle overflow
            let next_network_addr = network_addr.wrapping_add(subnet_size);
            if next_network_addr < network_addr {
                // overflow occurred, no valid network of this size
                continue;
            }
            next_network_addr
        } else {
            network_addr
        };

        let broadcast_addr = network_addr | !mask;

        if network_addr >= start_bits && broadcast_addr <= end_bits {
            if let Ok(network) =
                IpNetwork::new(IpAddr::V4(Ipv4Addr::from(network_addr)), prefix_len)
            {
                return Some(network);
            }
        }
    }

    None
}

/// Finds the largest IPv6 subnet that fits within the given range.
/// The subnet must contain more than one IP address since single IPs have their own
/// representation.
fn find_largest_ipv6_subnet_in_range(start: Ipv6Addr, end: Ipv6Addr) -> Option<IpNetwork> {
    let start_bits = start.to_bits();
    let end_bits = end.to_bits();

    // Find the largest prefix length where the subnet fits in the range.
    // We make some reasonable assumptions here and skip /0 and /128 networks.
    for prefix_len in 1..=127 {
        let mask = u128::MAX << (128 - prefix_len);

        // number of IPs in subnet
        let subnet_size = 1u128 << (128 - prefix_len);

        // try to find first and last address in subnet
        // in case the subnet does not align with first address in range
        // try next potential subnet start
        let network_addr = start_bits & mask;
        let network_addr = if network_addr < start_bits {
            // try next aligned address and handle overflow
            let next_network_addr = network_addr.wrapping_add(subnet_size);
            if next_network_addr < network_addr {
                // overflow occurred, no valid network of this size
                continue;
            }
            next_network_addr
        } else {
            network_addr
        };

        let broadcast_addr = network_addr | !mask;

        if network_addr >= start_bits && broadcast_addr <= end_bits {
            if let Ok(network) =
                IpNetwork::new(IpAddr::V6(Ipv6Addr::from(network_addr)), prefix_len)
            {
                return Some(network);
            }
        }
    }

    None
}

/// Recursively decomposes an IP address range into the smallest possible set of
/// non-overlapping [`IpNetwork`]s (CIDRs). Single host addresses are returned as
/// /32 (IPv4) or /128 (IPv6) networks.
///
/// If no fitting subnet can be found for a sub-range (an overflow edge case that
/// is unreachable in practice), that sub-range is silently dropped.
pub(crate) fn extract_subnets_from_range(range_start: IpAddr, range_end: IpAddr) -> Vec<IpNetwork> {
    let mut result = Vec::new();

    // Single IP address - return as host network.
    if range_start == range_end {
        let prefix = match range_start {
            IpAddr::V4(_) => 32,
            IpAddr::V6(_) => 128,
        };
        if let Ok(network) = IpNetwork::new(range_start, prefix) {
            result.push(network);
        }
        return result;
    }

    // Try to find the largest subnet that fits in the range.
    if let Some(subnet) = find_largest_subnet_in_range(range_start, range_end) {
        let subnet_start = subnet.network();
        let subnet_end = match subnet {
            IpNetwork::V4(_) => subnet.broadcast(),
            IpNetwork::V6(net6) => get_last_ip_in_v6_subnet(&net6),
        };

        // Subnet covers the entire range - use it directly.
        if subnet_start == range_start && subnet_end == range_end {
            result.push(subnet);
        } else {
            // Add range before subnet (if any).
            if range_start < subnet_start {
                let prev_ip = match subnet_start {
                    IpAddr::V4(ip) => {
                        let ip_u32 = ip.to_bits();
                        if ip_u32 > 0 {
                            IpAddr::V4(Ipv4Addr::from(ip_u32 - 1))
                        } else {
                            range_start // shouldn't happen in practice
                        }
                    }
                    IpAddr::V6(ip) => {
                        let ip_u128 = ip.to_bits();
                        if ip_u128 > 0 {
                            IpAddr::V6(Ipv6Addr::from(ip_u128 - 1))
                        } else {
                            range_start // shouldn't happen in practice
                        }
                    }
                };
                result.extend(extract_subnets_from_range(range_start, prev_ip));
            }

            result.push(subnet);

            // Add range after subnet (if any).
            if subnet_end < range_end {
                let next_ip = match subnet_end {
                    IpAddr::V4(ip) => {
                        let ip_u32 = ip.to_bits();
                        if ip_u32 < u32::MAX {
                            IpAddr::V4(Ipv4Addr::from(ip_u32 + 1))
                        } else {
                            range_end // shouldn't happen in practice
                        }
                    }
                    IpAddr::V6(ip) => {
                        let ip_u128 = ip.to_bits();
                        if ip_u128 < u128::MAX {
                            IpAddr::V6(Ipv6Addr::from(ip_u128 + 1))
                        } else {
                            range_end // shouldn't happen in practice
                        }
                    }
                };
                result.extend(extract_subnets_from_range(next_ip, range_end));
            }
        }
    }
    // If no subnet fits (overflow edge case, unreachable in practice) - drop silently.

    result
}

/// Helper function which implements merging a set of ranges of arbitrary elements
/// into the smallest possible set of non-overlapping ranges.
/// It can then be reused for merging port and address ranges.
pub(crate) fn merge_ranges<T: Ord + Next>(
    mut ranges: Vec<RangeInclusive<T>>,
) -> Vec<RangeInclusive<T>> {
    // Return early if the list is empty.
    if ranges.is_empty() {
        return Vec::new();
    }

    // Sort elements by range start.
    ranges.sort_unstable_by(|a, b| {
        let a_start = a.start();
        let b_start = b.start();
        a_start.cmp(b_start)
    });

    // Initialize result vector.
    let mut merged_ranges = Vec::new();

    // Start with the first range.
    let (mut current_range_start, mut current_range_end) = ranges.remove(0).into_inner();
    let mut next_up = current_range_end.next();

    // Iterate over remaining ranges.
    for range in ranges {
        let (range_start, range_end) = range.into_inner();

        // Compare with the current range.
        if next_up >= range_start {
            // Ranges are overlapping, so merge them
            // if range is not contained within the current range.
            if range_end >= current_range_end {
                next_up = range_end.next();
                current_range_end = range_end;
            }
        } else {
            // ranges are not overlapping, add current range to result
            merged_ranges.push(current_range_start..=current_range_end);
            current_range_start = range_start;
            next_up = range_end.next();
            current_range_end = range_end;
        }
    }

    // Add last remaining range.
    merged_ranges.push(current_range_start..=current_range_end);

    merged_ranges
}
