use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    ops::RangeInclusive,
};

/// Return next item.
/// This trait can be replaced by `std::iter::Step` once it becomes stable.
pub(crate) trait Next {
    /// Return next item.
    fn next(&self) -> Self;
}

impl Next for IpAddr {
    /// Returns the next IP address in sequence, handling overflow by wrapping.
    fn next(&self) -> IpAddr {
        match self {
            IpAddr::V4(ipv4) => IpAddr::V4(Ipv4Addr::from_bits(ipv4.to_bits().wrapping_add(1))),
            IpAddr::V6(ipv6) => IpAddr::V6(Ipv6Addr::from_bits(ipv6.to_bits().wrapping_add(1))),
        }
    }
}

impl Next for u16 {
    fn next(&self) -> u16 {
        self.wrapping_add(1)
    }
}

/// Helper function which implements merging a set of ranges of arbitrary elements
/// into the smallest possible set of non-overlapping ranges.
/// It can then be reused for merging port and address ranges.
pub(super) fn merge_ranges<T: Ord + Next>(
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
