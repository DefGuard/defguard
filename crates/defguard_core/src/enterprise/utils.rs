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
/// The highest bit at which `start` and `end` differ splits the range in two. Unless both
/// halves are whole, making the range a subnet itself, the largest subnet lies against that
/// split, on the side of the bigger half. We skip /0 networks.
fn find_largest_ipv4_subnet_in_range(start: Ipv4Addr, end: Ipv4Addr) -> Option<IpNetwork> {
    let start_bits = start.to_bits();
    let end_bits = end.to_bits();

    // Bits below the split, empty for a single address.
    let mask = u32::MAX.unbounded_shr((start_bits ^ end_bits).leading_zeros() + 1);
    // First address above the split.
    let boundary = (start_bits | mask).wrapping_add(1);
    // Addresses on each side of the split.
    let lower = (!start_bits & mask) + 1;
    let upper = (end_bits & mask) + 1;

    let (network_addr, prefix_len) = if lower.min(upper) > mask {
        // The range is a subnet itself.
        (start_bits, (start_bits ^ end_bits).leading_zeros().max(1))
    } else if lower >= upper {
        // The subnet ends at the split.
        let prefix_len = lower.leading_zeros() + 1;
        (boundary - (1 << (Ipv4Addr::BITS - prefix_len)), prefix_len)
    } else {
        // The subnet starts at the split.
        (boundary, upper.leading_zeros() + 1)
    };

    IpNetwork::new(
        IpAddr::V4(Ipv4Addr::from_bits(network_addr)),
        prefix_len as u8,
    )
    .ok()
}

/// Finds the largest IPv6 subnet that fits within the given range.
/// Works the same way as [`find_largest_ipv4_subnet_in_range`].
fn find_largest_ipv6_subnet_in_range(start: Ipv6Addr, end: Ipv6Addr) -> Option<IpNetwork> {
    let start_bits = start.to_bits();
    let end_bits = end.to_bits();

    // Bits below the split, empty for a single address.
    let mask = u128::MAX.unbounded_shr((start_bits ^ end_bits).leading_zeros() + 1);
    // First address above the split.
    let boundary = (start_bits | mask).wrapping_add(1);
    // Addresses on each side of the split.
    let lower = (!start_bits & mask) + 1;
    let upper = (end_bits & mask) + 1;

    let (network_addr, prefix_len) = if lower.min(upper) > mask {
        // The range is a subnet itself.
        (start_bits, (start_bits ^ end_bits).leading_zeros().max(1))
    } else if lower >= upper {
        // The subnet ends at the split.
        let prefix_len = lower.leading_zeros() + 1;
        (boundary - (1 << (Ipv6Addr::BITS - prefix_len)), prefix_len)
    } else {
        // The subnet starts at the split.
        (boundary, upper.leading_zeros() + 1)
    };

    IpNetwork::new(
        IpAddr::V6(Ipv6Addr::from_bits(network_addr)),
        prefix_len as u8,
    )
    .ok()
}

/// Appends a subnet to the result, splitting an IPv4 /31 into two /32 host networks,
/// since a /31 leaves no usable host addresses.
fn push_subnet(result: &mut Vec<IpNetwork>, subnet: IpNetwork) {
    if let IpNetwork::V4(subnet_v4) = subnet
        && subnet_v4.prefix() == 31
    {
        // A /31 network address is even, so this can't overflow.
        let network_addr = subnet_v4.network().to_bits();
        for addr_bits in [network_addr, network_addr + 1] {
            if let Ok(host) = IpNetwork::new(IpAddr::V4(Ipv4Addr::from_bits(addr_bits)), 32) {
                result.push(host);
            }
        }
    } else {
        result.push(subnet);
    }
}

/// Recursively decomposes an IP address range into the smallest possible set of
/// non-overlapping [`IpNetwork`]s (CIDRs). Single host addresses are returned as
/// /32 (IPv4) or /128 (IPv6) networks, and IPv4 /31s are split into two /32s.
/// Ranges with endpoints of different IP versions are silently dropped.
pub(crate) fn extract_subnets_from_range(range_start: IpAddr, range_end: IpAddr) -> Vec<IpNetwork> {
    let mut result = Vec::new();

    // Single IP address - return as host network.
    if range_start == range_end {
        let prefix = match range_start {
            IpAddr::V4(_) => Ipv4Addr::BITS as u8,
            IpAddr::V6(_) => Ipv6Addr::BITS as u8,
        };
        if let Ok(network) = IpNetwork::new(range_start, prefix) {
            result.push(network);
        }
        return result;
    }

    // Take out the largest subnet and decompose what is left on either side.
    if let Some(subnet) = find_largest_subnet_in_range(range_start, range_end) {
        let subnet_start = subnet.network();
        let subnet_end = match subnet {
            IpNetwork::V4(_) => subnet.broadcast(),
            IpNetwork::V6(net6) => get_last_ip_in_v6_subnet(&net6),
        };

        if range_start < subnet_start {
            // `subnet_start` is above `range_start`, so this can't wrap.
            let prev_ip = match subnet_start {
                IpAddr::V4(ip) => IpAddr::V4(Ipv4Addr::from_bits(ip.to_bits() - 1)),
                IpAddr::V6(ip) => IpAddr::V6(Ipv6Addr::from_bits(ip.to_bits() - 1)),
            };
            result.extend(extract_subnets_from_range(range_start, prev_ip));
        }

        push_subnet(&mut result, subnet);

        if subnet_end < range_end {
            // `subnet_end` is below `range_end`, so this can't wrap.
            result.extend(extract_subnets_from_range(subnet_end.next(), range_end));
        }
    }
    // If no subnet fits (mixed IP versions) - drop silently.

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
