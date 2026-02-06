use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use defguard_proto::enterprise::firewall::{IpAddress, IpVersion, ip_address::Address};
use rand::thread_rng;

use crate::enterprise::firewall::{
    get_source_addrs, get_source_network_devices, get_source_users,
    tests::{random_network_device_with_id, random_user_with_id},
};

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

    let users = get_source_users(allowed_users, &denied_users);
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

    let devices = get_source_network_devices(allowed_devices, &denied_devices);
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
        [
            IpAddress {
                address: Some(Address::Ip("10.0.1.1".to_string()))
            },
            IpAddress {
                address: Some(Address::IpSubnet("10.0.1.2/31".to_string()))
            },
            IpAddress {
                address: Some(Address::IpSubnet("10.0.1.4/31".to_string()))
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
    let empty_addrs = get_source_addrs(Vec::new(), Vec::new(), IpVersion::Ipv4);
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
        [
            IpAddress {
                address: Some(Address::Ip("2001:db8::1".to_string()))
            },
            IpAddress {
                address: Some(Address::IpSubnet("2001:db8::2/127".to_string()))
            },
            IpAddress {
                address: Some(Address::IpSubnet("2001:db8::4/127".to_string()))
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
    let empty_addrs = get_source_addrs(Vec::new(), Vec::new(), IpVersion::Ipv6);
    assert!(empty_addrs.is_empty());

    // Test with only IPv4 addresses - should return empty result for IPv6
    let ipv4_only = get_source_addrs(
        vec![IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1))],
        vec![IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2))],
        IpVersion::Ipv6,
    );
    assert!(ipv4_only.is_empty());
}
