use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use defguard_proto::enterprise::firewall::{
    IpAddress, IpRange, Port, PortRange as PortRangeProto, ip_address::Address,
    port::Port as PortInner,
};
use ipnetwork::Ipv6Network;

use crate::enterprise::{
    db::models::acl::PortRange,
    firewall::{
        find_largest_subnet_in_range, get_last_ip_in_v6_subnet, merge_addrs, merge_port_ranges,
    },
};

#[test]
fn test_merge_v4_addrs() {
    let addr_ranges = vec![
        IpAddr::V4(Ipv4Addr::new(10, 0, 60, 20))..=IpAddr::V4(Ipv4Addr::new(10, 0, 60, 25)),
        IpAddr::V4(Ipv4Addr::new(10, 0, 10, 1))..=IpAddr::V4(Ipv4Addr::new(10, 0, 10, 22)),
        IpAddr::V4(Ipv4Addr::new(10, 0, 8, 127))..=IpAddr::V4(Ipv4Addr::new(10, 0, 9, 12)),
        IpAddr::V4(Ipv4Addr::new(10, 0, 9, 1))..=IpAddr::V4(Ipv4Addr::new(10, 0, 10, 12)),
        IpAddr::V4(Ipv4Addr::new(10, 0, 9, 20))..=IpAddr::V4(Ipv4Addr::new(10, 0, 10, 31)),
        IpAddr::V4(Ipv4Addr::new(192, 168, 0, 20))..=IpAddr::V4(Ipv4Addr::new(192, 168, 0, 20)),
        IpAddr::V4(Ipv4Addr::new(10, 0, 20, 20))..=IpAddr::V4(Ipv4Addr::new(10, 0, 20, 20)),
    ];

    let merged_addrs = merge_addrs(addr_ranges);

    assert_eq!(
        merged_addrs,
        [
            IpAddress {
                address: Some(Address::Ip("10.0.8.127".to_string())),
            },
            IpAddress {
                address: Some(Address::IpSubnet("10.0.8.128/25".to_string())),
            },
            IpAddress {
                address: Some(Address::IpSubnet("10.0.9.0/24".to_string())),
            },
            IpAddress {
                address: Some(Address::IpSubnet("10.0.10.0/27".to_string())),
            },
            IpAddress {
                address: Some(Address::Ip("10.0.20.20".to_string())),
            },
            IpAddress {
                address: Some(Address::IpSubnet("10.0.60.20/30".to_string())),
            },
            IpAddress {
                address: Some(Address::IpSubnet("10.0.60.24/31".to_string())),
            },
            IpAddress {
                address: Some(Address::Ip("192.168.0.20".to_string())),
            },
        ]
    );

    // merge single IPs into a range
    let addr_ranges = vec![
        IpAddr::V4(Ipv4Addr::new(10, 0, 10, 0))..=IpAddr::V4(Ipv4Addr::new(10, 0, 10, 0)),
        IpAddr::V4(Ipv4Addr::new(10, 0, 10, 1))..=IpAddr::V4(Ipv4Addr::new(10, 0, 10, 1)),
        IpAddr::V4(Ipv4Addr::new(10, 0, 10, 2))..=IpAddr::V4(Ipv4Addr::new(10, 0, 10, 2)),
        IpAddr::V4(Ipv4Addr::new(10, 0, 10, 3))..=IpAddr::V4(Ipv4Addr::new(10, 0, 10, 3)),
        IpAddr::V4(Ipv4Addr::new(10, 0, 10, 20))..=IpAddr::V4(Ipv4Addr::new(10, 0, 10, 20)),
    ];

    let merged_addrs = merge_addrs(addr_ranges);
    assert_eq!(
        merged_addrs,
        [
            IpAddress {
                address: Some(Address::IpSubnet("10.0.10.0/30".to_string())),
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
        IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0x1, 0x0, 0x0, 0x0, 0x0, 0x1))
            ..=IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0x1, 0x0, 0x0, 0x0, 0x0, 0x5)),
        IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0x1, 0x0, 0x0, 0x0, 0x0, 0x3))
            ..=IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0x1, 0x0, 0x0, 0x0, 0x0, 0x8)),
        IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0x2, 0x0, 0x0, 0x0, 0x0, 0x1))
            ..=IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0x2, 0x0, 0x0, 0x0, 0x0, 0x1)),
        IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0x3, 0x0, 0x0, 0x0, 0x0, 0x1))
            ..=IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0x3, 0x0, 0x0, 0x0, 0x0, 0x3)),
    ];

    let merged_addrs = merge_addrs(addr_ranges);
    assert_eq!(
        merged_addrs,
        [
            IpAddress {
                address: Some(Address::Ip("2001:db8:1::1".to_string()))
            },
            IpAddress {
                address: Some(Address::IpSubnet("2001:db8:1::2/127".to_string()))
            },
            IpAddress {
                address: Some(Address::IpSubnet("2001:db8:1::4/126".to_string()))
            },
            IpAddress {
                address: Some(Address::Ip("2001:db8:1::8".to_string()))
            },
            IpAddress {
                address: Some(Address::Ip("2001:db8:2::1".to_string()))
            },
            IpAddress {
                address: Some(Address::Ip("2001:db8:3::1".to_string()))
            },
            IpAddress {
                address: Some(Address::IpSubnet("2001:db8:3::2/127".to_string()))
            }
        ]
    );
}

#[test]
fn test_merge_addrs_extracts_ipv4_subnets() {
    let ranges = vec![
        IpAddr::V4(Ipv4Addr::new(192, 168, 1, 0))..=IpAddr::V4(Ipv4Addr::new(192, 168, 2, 255)),
    ];

    let result = merge_addrs(ranges);

    assert_eq!(
        result,
        [
            IpAddress {
                address: Some(Address::IpSubnet("192.168.1.0/24".to_string()))
            },
            IpAddress {
                address: Some(Address::IpSubnet("192.168.2.0/24".to_string()))
            },
        ]
    );
}

#[test]
fn test_merge_addrs_extracts_ipv6_subnets() {
    let start = "2001:db8::".parse::<Ipv6Addr>().unwrap();
    let end = "2001:db9::ffff".parse::<Ipv6Addr>().unwrap();
    let ranges = vec![IpAddr::V6(start)..=IpAddr::V6(end)];

    let result = merge_addrs(ranges);

    assert_eq!(
        result,
        [
            IpAddress {
                address: Some(Address::IpSubnet("2001:db8::/32".to_string()))
            },
            IpAddress {
                address: Some(Address::IpSubnet("2001:db9::/112".to_string()))
            },
        ]
    );
}

#[test]
fn test_merge_addrs_falls_back_to_range_when_no_subnet_fits() {
    let ranges = vec![
        IpAddr::V4(Ipv4Addr::new(192, 168, 1, 255))..=IpAddr::V4(Ipv4Addr::new(192, 168, 2, 0)),
    ];

    let result = merge_addrs(ranges);

    assert_eq!(
        result,
        [IpAddress {
            address: Some(Address::IpRange(IpRange {
                start: "192.168.1.255".to_string(),
                end: "192.168.2.0".to_string(),
            })),
        },]
    );

    let start = "2001:db8:ffff:ffff:ffff:ffff:ffff:ffff"
        .parse::<Ipv6Addr>()
        .unwrap();
    let end = "2001:db9::".parse::<Ipv6Addr>().unwrap();
    let ranges = vec![IpAddr::V6(start)..=IpAddr::V6(end)];

    let result = merge_addrs(ranges);

    assert_eq!(
        result,
        [IpAddress {
            address: Some(Address::IpRange(IpRange {
                start: "2001:db8:ffff:ffff:ffff:ffff:ffff:ffff".to_string(),
                end: "2001:db9::".to_string(),
            })),
        },]
    );
}

#[test]
fn test_merge_addrs_handles_single_ip() {
    // Test case: single IP should remain as IP
    let ranges =
        vec![IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1))..=IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1))];

    let result = merge_addrs(ranges);

    assert_eq!(
        result,
        [IpAddress {
            address: Some(Address::Ip("192.168.1.1".to_string())),
        },]
    );

    let start = "2001:db8::".parse::<Ipv6Addr>().unwrap();
    let end = "2001:db8::".parse::<Ipv6Addr>().unwrap();
    let ranges = vec![IpAddr::V6(start)..=IpAddr::V6(end)];

    let result = merge_addrs(ranges);

    assert_eq!(
        result,
        [IpAddress {
            address: Some(Address::Ip("2001:db8::".to_string())),
        },]
    );
}

#[test]
fn test_find_largest_ipv4_subnet_perfect_match() {
    // Test /24 subnet
    let start = Ipv4Addr::new(192, 168, 1, 0);
    let end = Ipv4Addr::new(192, 168, 1, 255);

    let result = find_largest_subnet_in_range(IpAddr::V4(start), IpAddr::V4(end));

    assert!(result.is_some());
    let subnet = result.unwrap();
    assert_eq!(subnet.to_string(), "192.168.1.0/24");

    // Test /28 subnet (16 addresses)
    let start = Ipv4Addr::new(192, 168, 1, 0);
    let end = Ipv4Addr::new(192, 168, 1, 15);

    let result = find_largest_subnet_in_range(IpAddr::V4(start), IpAddr::V4(end));

    assert!(result.is_some());
    let subnet = result.unwrap();
    assert_eq!(subnet.to_string(), "192.168.1.0/28");
}

#[test]
fn test_find_largest_ipv6_subnet_perfect_match() {
    // Test /112 subnet
    let start = "2001:db8::".parse::<Ipv6Addr>().unwrap();
    let end = "2001:db8::ffff".parse::<Ipv6Addr>().unwrap();

    let result = find_largest_subnet_in_range(IpAddr::V6(start), IpAddr::V6(end));

    assert!(result.is_some());
    let subnet = result.unwrap();
    assert_eq!(subnet.to_string(), "2001:db8::/112");
}

#[test]
fn test_find_largest_subnet_mixed_ip_versions() {
    // Test mixed IP versions should return None
    let start = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 0));
    let end = IpAddr::V6("2001:db8::1".parse().unwrap());

    let result = find_largest_subnet_in_range(start, end);

    assert!(result.is_none());
}

#[test]
fn test_find_largest_subnet_invalid_range() {
    // Test invalid range (start > end) should return None
    let start = Ipv4Addr::new(192, 168, 1, 10);
    let end = Ipv4Addr::new(192, 168, 1, 5);

    let result = find_largest_subnet_in_range(IpAddr::V4(start), IpAddr::V4(end));

    assert!(result.is_none());
}

#[test]
fn test_merge_addrs_subnet_at_start_of_range() {
    let ranges = vec![
        IpAddr::V4(Ipv4Addr::new(192, 168, 1, 0))..=IpAddr::V4(Ipv4Addr::new(192, 168, 1, 64)),
    ];

    let result = merge_addrs(ranges);

    assert_eq!(
        result,
        [
            IpAddress {
                address: Some(Address::IpSubnet("192.168.1.0/26".to_string())),
            },
            IpAddress {
                address: Some(Address::Ip("192.168.1.64".to_string())),
            },
        ]
    );

    let ranges = vec![
        IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 0))
            ..=IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 0x40)),
    ];

    let result = merge_addrs(ranges);

    assert_eq!(
        result,
        [
            IpAddress {
                address: Some(Address::IpSubnet("2001:db8::/122".to_string())),
            },
            IpAddress {
                address: Some(Address::Ip("2001:db8::40".to_string())),
            },
        ]
    );
}

#[test]
fn test_merge_addrs_subnet_at_end_of_range() {
    let ranges = vec![
        IpAddr::V4(Ipv4Addr::new(192, 168, 1, 15))..=IpAddr::V4(Ipv4Addr::new(192, 168, 1, 31)),
    ];

    let result = merge_addrs(ranges);

    assert_eq!(
        result,
        [
            IpAddress {
                address: Some(Address::Ip("192.168.1.15".to_string())),
            },
            IpAddress {
                address: Some(Address::IpSubnet("192.168.1.16/28".to_string())),
            },
        ]
    );

    let ranges = vec![
        IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 0x0f))
            ..=IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 0x1f)),
    ];

    let result = merge_addrs(ranges);

    assert_eq!(
        result,
        [
            IpAddress {
                address: Some(Address::Ip("2001:db8::f".to_string())),
            },
            IpAddress {
                address: Some(Address::IpSubnet("2001:db8::10/124".to_string())),
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
        [Port {
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
        [Port {
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
        [
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
        [
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
        [Port {
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
    );
}
