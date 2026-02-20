use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use defguard_proto::enterprise::firewall::{ip_address::Address, IpAddress, IpRange};

use crate::firewall::process_destination_addrs;
use crate::firewall::tests::default_destination_range_with_values;
use defguard_common::db::Id;

#[test]
fn test_process_destination_addrs_v4() {
    // Test data with mixed IPv4 and IPv6 networks
    let destination_ips = [
        "10.0.1.0/24".parse().unwrap(),
        "10.0.2.0/24".parse().unwrap(),
        "2001:db8::/64".parse().unwrap(), // Should be filtered out
        "192.168.1.0/24".parse().unwrap(),
    ];

    let destination_ranges = [
        default_destination_range_with_values(
            Id::default(),
            IpAddr::V4(Ipv4Addr::new(10, 0, 3, 255)),
            IpAddr::V4(Ipv4Addr::new(10, 0, 4, 0)),
        ),
        default_destination_range_with_values(
            Id::default(),
            IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1)),
            IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 100)),
        ),
    ];

    let destination_addrs = process_destination_addrs(&destination_ips, &destination_ranges);

    assert_eq!(
        destination_addrs.0,
        [
            IpAddress {
                address: Some(Address::IpSubnet("10.0.1.0/24".to_string())),
            },
            IpAddress {
                address: Some(Address::IpSubnet("10.0.2.0/24".to_string())),
            },
            IpAddress {
                address: Some(Address::IpRange(IpRange {
                    start: "10.0.3.255".to_string(),
                    end: "10.0.4.0".to_string(),
                })),
            },
            IpAddress {
                address: Some(Address::IpSubnet("192.168.1.0/24".to_string())),
            },
        ]
    );

    // Test with empty input
    let empty_addrs = process_destination_addrs(&[], &[]);
    assert!(empty_addrs.0.is_empty());

    // Test with only IPv6 addresses - should return empty result for IPv4
    let ipv6_only = process_destination_addrs(&["2001:db8::/64".parse().unwrap()], &[]);
    assert!(ipv6_only.0.is_empty());
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
        default_destination_range_with_values(
            Id::default(),
            IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 4, 0, 0, 0, 0, 1)),
            IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 4, 0, 0, 0, 0, 3)),
        ),
        default_destination_range_with_values(
            Id::default(),
            IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)),
            IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)),
        ),
    ];

    let destination_addrs = process_destination_addrs(&destination_ips, &destination_ranges);

    assert_eq!(
        destination_addrs.1,
        [
            IpAddress {
                address: Some(Address::IpSubnet("2001:db8:1::/64".to_string())),
            },
            IpAddress {
                address: Some(Address::IpSubnet("2001:db8:2::/64".to_string())),
            },
            IpAddress {
                address: Some(Address::IpSubnet("2001:db8:3::/64".to_string())),
            },
            IpAddress {
                address: Some(Address::Ip("2001:db8:4::1".to_string()))
            },
            IpAddress {
                address: Some(Address::IpSubnet("2001:db8:4::2/127".to_string()))
            }
        ]
    );

    // Test with empty input
    let empty_addrs = process_destination_addrs(&[], &[]);
    assert!(empty_addrs.1.is_empty());

    // Test with only IPv4 addresses - should return empty result for IPv6
    let ipv4_only = process_destination_addrs(&["192.168.1.0/24".parse().unwrap()], &[]);
    assert!(ipv4_only.1.is_empty());
}
