use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    ops::RangeInclusive,
};

use defguard_common::db::{NoId, models::WireguardNetwork, setup_pool};
use defguard_proto::enterprise::firewall::{
    FirewallPolicy, IpAddress, IpRange, ip_address::Address,
};
use rand::thread_rng;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

use super::{create_acl_rule, create_test_users_and_devices, set_test_license_business};
use crate::enterprise::{
    db::models::acl::{
        AclAlias, AclAliasDestinationRange, AclRule, AclRuleDestinationRange, AliasKind, RuleState,
    },
    firewall::{process_destination_addrs, try_get_location_firewall_config},
};

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
        AclRuleDestinationRange {
            start: IpAddr::V4(Ipv4Addr::new(10, 0, 3, 255)),
            end: IpAddr::V4(Ipv4Addr::new(10, 0, 4, 0)),
            ..Default::default()
        },
        AclRuleDestinationRange {
            start: IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1)), // Should be filtered out
            end: IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 100)),
            ..Default::default()
        },
    ];

    let destination_addrs = process_destination_addrs(
        &destination_ips,
        destination_ranges.iter().map(RangeInclusive::from),
    );

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
    let empty_addrs = process_destination_addrs(&[], std::iter::empty::<RangeInclusive<IpAddr>>());
    assert!(empty_addrs.0.is_empty());

    // Test with only IPv6 addresses - should return empty result for IPv4
    let ipv6_only = process_destination_addrs(
        &["2001:db8::/64".parse().unwrap()],
        std::iter::empty::<RangeInclusive<IpAddr>>(),
    );
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

    let destination_ranges = [
        AclRuleDestinationRange {
            start: IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 4, 0, 0, 0, 0, 1)),
            end: IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 4, 0, 0, 0, 0, 3)),
            ..Default::default()
        },
        AclRuleDestinationRange {
            start: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)), // Should be filtered out
            end: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)),
            ..Default::default()
        },
    ];

    let destination_addrs = process_destination_addrs(
        &destination_ips,
        destination_ranges.iter().map(RangeInclusive::from),
    );

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
    let empty_addrs = process_destination_addrs(&[], std::iter::empty::<RangeInclusive<IpAddr>>());
    assert!(empty_addrs.1.is_empty());

    // Test with only IPv4 addresses - should return empty result for IPv6
    let ipv4_only = process_destination_addrs(
        &["192.168.1.0/24".parse().unwrap()],
        std::iter::empty::<RangeInclusive<IpAddr>>(),
    );
    assert!(ipv4_only.1.is_empty());
}

#[sqlx::test]
async fn test_any_address_overwrites_manual_destination(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    set_test_license_business();
    let pool = setup_pool(options).await;

    let mut rng = thread_rng();

    let location = WireguardNetwork {
        acl_enabled: true,
        address: vec!["10.0.0.0/16".parse().unwrap()],
        ..Default::default()
    }
    .save(&pool)
    .await
    .unwrap();

    create_test_users_and_devices(&mut rng, &pool, vec![&location]).await;

    let acl_rule = AclRule {
        name: "any destination rule".to_string(),
        state: RuleState::Applied,
        allow_all_users: true,
        any_address: true,
        addresses: vec!["192.168.1.0/24".parse().unwrap()],
        use_manual_destination_settings: true,
        ..Default::default()
    };

    create_acl_rule(
        &pool,
        acl_rule,
        vec![location.id],
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        vec![(
            IpAddr::V4(Ipv4Addr::new(192, 168, 1, 10)),
            IpAddr::V4(Ipv4Addr::new(192, 168, 1, 20)),
        )],
        Vec::new(),
    )
    .await;

    let mut conn = pool.acquire().await.unwrap();
    let generated_firewall_rules = try_get_location_firewall_config(&location, &mut conn)
        .await
        .unwrap()
        .unwrap()
        .rules;

    let expected_source_addrs = [
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
    ];

    assert_eq!(generated_firewall_rules.len(), 2);

    let allow_rule = &generated_firewall_rules[0];
    assert_eq!(allow_rule.verdict, i32::from(FirewallPolicy::Allow));
    assert_eq!(allow_rule.source_addrs, expected_source_addrs);
    assert!(allow_rule.destination_addrs.is_empty());

    let deny_rule = &generated_firewall_rules[1];
    assert_eq!(deny_rule.verdict, i32::from(FirewallPolicy::Deny));
    assert!(deny_rule.source_addrs.is_empty());
    assert!(deny_rule.destination_addrs.is_empty());
}

#[sqlx::test]
async fn test_any_address_overwrites_destination_alias_addrs(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    set_test_license_business();
    let pool = setup_pool(options).await;

    let mut rng = thread_rng();

    let location = WireguardNetwork {
        acl_enabled: true,
        address: vec!["10.0.0.0/16".parse().unwrap()],
        ..Default::default()
    }
    .save(&pool)
    .await
    .unwrap();

    create_test_users_and_devices(&mut rng, &pool, vec![&location]).await;

    let destination_alias = AclAlias {
        name: "any destination alias".to_string(),
        kind: AliasKind::Destination,
        any_address: true,
        any_port: true,
        any_protocol: true,
        addresses: vec!["10.1.0.0/24".parse().unwrap()],
        ..Default::default()
    }
    .save(&pool)
    .await
    .unwrap();

    AclAliasDestinationRange {
        id: NoId,
        alias_id: destination_alias.id,
        start: IpAddr::V4(Ipv4Addr::new(10, 2, 0, 10)),
        end: IpAddr::V4(Ipv4Addr::new(10, 2, 0, 20)),
    }
    .save(&pool)
    .await
    .unwrap();

    let acl_rule = AclRule {
        name: "any destination alias rule".to_string(),
        state: RuleState::Applied,
        allow_all_users: true,
        use_manual_destination_settings: false,
        ..Default::default()
    };

    create_acl_rule(
        &pool,
        acl_rule,
        vec![location.id],
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        vec![destination_alias.id],
    )
    .await;

    let mut conn = pool.acquire().await.unwrap();
    let generated_firewall_rules = try_get_location_firewall_config(&location, &mut conn)
        .await
        .unwrap()
        .unwrap()
        .rules;

    let expected_source_addrs = [
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
    ];

    assert_eq!(generated_firewall_rules.len(), 2);

    let allow_rule = &generated_firewall_rules[0];
    assert_eq!(allow_rule.verdict, i32::from(FirewallPolicy::Allow));
    assert_eq!(allow_rule.source_addrs, expected_source_addrs);
    assert!(allow_rule.destination_addrs.is_empty());

    let deny_rule = &generated_firewall_rules[1];
    assert_eq!(deny_rule.verdict, i32::from(FirewallPolicy::Deny));
    assert!(deny_rule.source_addrs.is_empty());
    assert!(deny_rule.destination_addrs.is_empty());
}

#[sqlx::test]
async fn test_manual_destination_includes_component_alias_address_range(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    set_test_license_business();
    let pool = setup_pool(options).await;

    let mut rng = thread_rng();

    let location = WireguardNetwork {
        acl_enabled: true,
        address: vec!["10.0.0.0/16".parse().unwrap()],
        ..Default::default()
    }
    .save(&pool)
    .await
    .unwrap();

    create_test_users_and_devices(&mut rng, &pool, vec![&location]).await;

    let component_alias = AclAlias {
        name: "component alias with destination range".to_string(),
        kind: AliasKind::Component,
        ..Default::default()
    }
    .save(&pool)
    .await
    .unwrap();

    AclAliasDestinationRange {
        id: NoId,
        alias_id: component_alias.id,
        start: IpAddr::V4(Ipv4Addr::new(10, 2, 0, 255)),
        end: IpAddr::V4(Ipv4Addr::new(10, 2, 1, 0)),
    }
    .save(&pool)
    .await
    .unwrap();

    let acl_rule = AclRule {
        name: "manual destination component alias range rule".to_string(),
        state: RuleState::Applied,
        allow_all_users: true,
        use_manual_destination_settings: true,
        any_address: false,
        ..Default::default()
    };

    create_acl_rule(
        &pool,
        acl_rule,
        vec![location.id],
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        vec![component_alias.id],
    )
    .await;

    let mut conn = pool.acquire().await.unwrap();
    let generated_firewall_rules = try_get_location_firewall_config(&location, &mut conn)
        .await
        .unwrap()
        .unwrap()
        .rules;

    let expected_source_addrs = [
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
    ];
    let expected_destination_addrs = [IpAddress {
        address: Some(Address::IpRange(IpRange {
            start: "10.2.0.255".to_string(),
            end: "10.2.1.0".to_string(),
        })),
    }];

    assert_eq!(generated_firewall_rules.len(), 2);

    let allow_rule = &generated_firewall_rules[0];
    assert_eq!(allow_rule.verdict, i32::from(FirewallPolicy::Allow));
    assert_eq!(allow_rule.source_addrs, expected_source_addrs);
    assert_eq!(allow_rule.destination_addrs, expected_destination_addrs);

    let deny_rule = &generated_firewall_rules[1];
    assert_eq!(deny_rule.verdict, i32::from(FirewallPolicy::Deny));
    assert!(deny_rule.source_addrs.is_empty());
    assert_eq!(deny_rule.destination_addrs, expected_destination_addrs);
}

#[sqlx::test]
async fn test_manual_destination_merges_rule_and_component_alias_address_ranges(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    set_test_license_business();
    let pool = setup_pool(options).await;

    let mut rng = thread_rng();

    let location = WireguardNetwork {
        acl_enabled: true,
        address: vec!["10.0.0.0/16".parse().unwrap()],
        ..Default::default()
    }
    .save(&pool)
    .await
    .unwrap();

    create_test_users_and_devices(&mut rng, &pool, vec![&location]).await;

    let component_alias = AclAlias {
        name: "component alias with destination range".to_string(),
        kind: AliasKind::Component,
        ..Default::default()
    }
    .save(&pool)
    .await
    .unwrap();

    AclAliasDestinationRange {
        id: NoId,
        alias_id: component_alias.id,
        start: IpAddr::V4(Ipv4Addr::new(10, 2, 0, 255)),
        end: IpAddr::V4(Ipv4Addr::new(10, 2, 1, 0)),
    }
    .save(&pool)
    .await
    .unwrap();

    let acl_rule = AclRule {
        name: "manual destination mixed destination ranges rule".to_string(),
        state: RuleState::Applied,
        allow_all_users: true,
        use_manual_destination_settings: true,
        any_address: false,
        ..Default::default()
    };

    create_acl_rule(
        &pool,
        acl_rule,
        vec![location.id],
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        vec![(
            IpAddr::V4(Ipv4Addr::new(10, 3, 0, 255)),
            IpAddr::V4(Ipv4Addr::new(10, 3, 1, 0)),
        )],
        vec![component_alias.id],
    )
    .await;

    let mut conn = pool.acquire().await.unwrap();
    let generated_firewall_rules = try_get_location_firewall_config(&location, &mut conn)
        .await
        .unwrap()
        .unwrap()
        .rules;

    let expected_source_addrs = [
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
    ];
    let expected_destination_addrs = [
        IpAddress {
            address: Some(Address::IpRange(IpRange {
                start: "10.2.0.255".to_string(),
                end: "10.2.1.0".to_string(),
            })),
        },
        IpAddress {
            address: Some(Address::IpRange(IpRange {
                start: "10.3.0.255".to_string(),
                end: "10.3.1.0".to_string(),
            })),
        },
    ];

    assert_eq!(generated_firewall_rules.len(), 2);

    let allow_rule = &generated_firewall_rules[0];
    assert_eq!(allow_rule.verdict, i32::from(FirewallPolicy::Allow));
    assert_eq!(allow_rule.source_addrs, expected_source_addrs);
    assert_eq!(allow_rule.destination_addrs, expected_destination_addrs);

    let deny_rule = &generated_firewall_rules[1];
    assert_eq!(deny_rule.verdict, i32::from(FirewallPolicy::Deny));
    assert!(deny_rule.source_addrs.is_empty());
    assert_eq!(deny_rule.destination_addrs, expected_destination_addrs);
}
