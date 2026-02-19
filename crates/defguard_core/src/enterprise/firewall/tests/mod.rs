use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use defguard_common::db::{
    Id, NoId,
    models::{
        Device, DeviceType, WireguardNetwork, device::WireguardNetworkDevice, group::Group,
        user::User,
    },
    setup_pool,
};
use defguard_proto::enterprise::firewall::{
    FirewallPolicy, IpAddress, IpRange, IpVersion, Port, PortRange as PortRangeProto, Protocol,
    ip_address::Address, port::Port as PortInner,
};
use ipnetwork::IpNetwork;
use rand::{Rng, rngs::ThreadRng, thread_rng};
use sqlx::{
    PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
    query,
};

use crate::enterprise::{
    db::models::acl::{
        AclAlias, AclRule, AclRuleAlias, AclRuleDestinationRange, AclRuleDevice, AclRuleGroup,
        AclRuleInfo, AclRuleNetwork, AclRuleUser, AliasKind, PortRange, RuleState,
    },
    firewall::try_get_location_firewall_config,
    license::{License, LicenseTier, set_cached_license},
};

mod all_locations;
mod destination;
mod disabled_rules;
mod expired_rules;
mod gh1868;
mod ip_address_handling;
mod source;
mod unapplied_rules;

impl Default for AclRuleDestinationRange<Id> {
    fn default() -> Self {
        Self {
            id: Id::default(),
            rule_id: Id::default(),
            start: IpAddr::V4(Ipv4Addr::UNSPECIFIED),
            end: IpAddr::V4(Ipv4Addr::UNSPECIFIED),
        }
    }
}

fn set_test_license_business() {
    let license = License {
        customer_id: "0c4dcb5400544d47ad8617fcdf2704cb".into(),
        limits: None,
        subscription: false,
        tier: LicenseTier::Business,
        valid_until: None,
        version_date_limit: None,
    };
    set_cached_license(Some(license));
}

fn random_user_with_id<R: Rng>(rng: &mut R, id: Id) -> User<Id> {
    let mut user: User<Id> = rng.r#gen();
    user.id = id;
    user
}

fn random_network_device_with_id<R: Rng>(rng: &mut R, id: Id) -> Device<Id> {
    let device: Device = rng.r#gen();
    let mut device = device.with_id(id);
    device.device_type = DeviceType::Network;
    device
}

async fn create_test_users_and_devices(
    rng: &mut ThreadRng,
    pool: &PgPool,
    test_locations: Vec<&WireguardNetwork<Id>>,
) {
    // create two users
    let user_1: User<NoId> = rng.r#gen();
    let user_1 = user_1.save(pool).await.unwrap();
    let user_2: User<NoId> = rng.r#gen();
    let user_2 = user_2.save(pool).await.unwrap();

    // create two devices for each user and create network configurations for all test locations
    for user in [&user_1, &user_2] {
        // Create 2 devices per user
        for device_num in 1..3 {
            let device = Device {
                id: NoId,
                name: format!("device-{}-{}", user.id, device_num),
                user_id: user.id,
                device_type: DeviceType::User,
                description: None,
                wireguard_pubkey: Default::default(),
                created: Default::default(),
                configured: true,
            };
            let device = device.save(pool).await.unwrap();

            // Add device to locations' VPN network
            for location in test_locations.iter() {
                let wireguard_ips = location
                    .address
                    .iter()
                    .map(|subnet| match subnet {
                        IpNetwork::V4(ipv4_network) => {
                            let octets = ipv4_network.network().octets();
                            IpAddr::V4(Ipv4Addr::new(
                                octets[0],
                                octets[1],
                                user.id as u8,
                                device_num,
                            ))
                        }
                        IpNetwork::V6(ipv6_network) => {
                            let mut octets = ipv6_network.network().octets();
                            // Set the last two octets (bytes 14 and 15)
                            octets[14] = user.id as u8;
                            octets[15] = device_num;
                            IpAddr::V6(Ipv6Addr::from(octets))
                        }
                    })
                    .collect();
                let network_device = WireguardNetworkDevice {
                    device_id: device.id,
                    wireguard_network_id: location.id,
                    wireguard_ips,
                    preshared_key: None,
                    is_authorized: true,
                    authorized_at: None,
                };
                network_device.insert(pool).await.unwrap();
            }
        }
    }
}

async fn create_acl_rule(
    pool: &PgPool,
    rule: AclRule,
    locations: Vec<Id>,
    allowed_users: Vec<Id>,
    denied_users: Vec<Id>,
    allowed_groups: Vec<Id>,
    denied_groups: Vec<Id>,
    allowed_network_devices: Vec<Id>,
    denied_network_devices: Vec<Id>,
    destination_ranges: Vec<(IpAddr, IpAddr)>,
    aliases: Vec<Id>,
) -> AclRuleInfo<Id> {
    let mut conn = pool.acquire().await.unwrap();

    // create base rule
    let rule = rule.save(&mut *conn).await.unwrap();
    let rule_id = rule.id;

    // create related objects
    // locations
    for location_id in locations {
        AclRuleNetwork::new(rule_id, location_id)
            .save(&mut *conn)
            .await
            .unwrap();
    }

    // allowed users
    for user_id in allowed_users {
        AclRuleUser::new(rule_id, user_id, true)
            .save(&mut *conn)
            .await
            .unwrap();
    }

    // denied users
    for user_id in denied_users {
        AclRuleUser::new(rule_id, user_id, false)
            .save(&mut *conn)
            .await
            .unwrap();
    }

    // allowed groups
    for group_id in allowed_groups {
        AclRuleGroup::new(rule_id, group_id, true)
            .save(&mut *conn)
            .await
            .unwrap();
    }

    // denied groups
    for group_id in denied_groups {
        AclRuleGroup::new(rule_id, group_id, false)
            .save(&mut *conn)
            .await
            .unwrap();
    }

    // allowed devices
    for device_id in allowed_network_devices {
        AclRuleDevice::new(rule_id, device_id, true)
            .save(&mut *conn)
            .await
            .unwrap();
    }

    // denied devices
    for device_id in denied_network_devices {
        AclRuleDevice::new(rule_id, device_id, false)
            .save(&mut *conn)
            .await
            .unwrap();
    }

    // destination ranges
    for range in destination_ranges {
        AclRuleDestinationRange {
            id: NoId,
            rule_id,
            start: range.0,
            end: range.1,
        }
        .save(&mut *conn)
        .await
        .unwrap();
    }

    // aliases
    for alias_id in aliases {
        AclRuleAlias::new(rule_id, alias_id)
            .save(&mut *conn)
            .await
            .unwrap();
    }

    // convert to output format
    rule.to_info(&mut conn).await.unwrap()
}

#[sqlx::test]
async fn test_generate_firewall_rules_ipv4(_: PgPoolOptions, options: PgConnectOptions) {
    set_test_license_business();
    let pool = setup_pool(options).await;

    let mut rng = thread_rng();

    // Create test location
    let location = WireguardNetwork {
        id: NoId,
        acl_enabled: false,
        ..Default::default()
    };
    let mut location = location.save(&pool).await.unwrap();

    // Setup test users and their devices
    let user_1: User<NoId> = rng.r#gen();
    let user_1 = user_1.save(&pool).await.unwrap();
    let user_2: User<NoId> = rng.r#gen();
    let user_2 = user_2.save(&pool).await.unwrap();
    let user_3: User<NoId> = rng.r#gen();
    let user_3 = user_3.save(&pool).await.unwrap();
    let user_4: User<NoId> = rng.r#gen();
    let user_4 = user_4.save(&pool).await.unwrap();
    let user_5: User<NoId> = rng.r#gen();
    let user_5 = user_5.save(&pool).await.unwrap();

    for user in [&user_1, &user_2, &user_3, &user_4, &user_5] {
        // Create 2 devices per user
        for device_num in 1..3 {
            let device = Device {
                id: NoId,
                name: format!("device-{}-{}", user.id, device_num),
                user_id: user.id,
                device_type: DeviceType::User,
                description: None,
                wireguard_pubkey: Default::default(),
                created: Default::default(),
                configured: true,
            };
            let device = device.save(&pool).await.unwrap();

            // Add device to location's VPN network
            let network_device = WireguardNetworkDevice {
                device_id: device.id,
                wireguard_network_id: location.id,
                wireguard_ips: vec![IpAddr::V4(Ipv4Addr::new(
                    10,
                    0,
                    user.id as u8,
                    device_num as u8,
                ))],
                preshared_key: None,
                is_authorized: true,
                authorized_at: None,
            };
            network_device.insert(&pool).await.unwrap();
        }
    }

    // Setup test groups
    let group_1 = Group {
        id: NoId,
        name: "group_1".into(),
        ..Default::default()
    };
    let group_1 = group_1.save(&pool).await.unwrap();
    let group_2 = Group {
        id: NoId,
        name: "group_2".into(),
        ..Default::default()
    };
    let group_2 = group_2.save(&pool).await.unwrap();

    // Assign users to groups:
    // Group 1: users 1,2
    // Group 2: users 3,4
    let group_assignments = vec![
        (&group_1, vec![&user_1, &user_2]),
        (&group_2, vec![&user_3, &user_4]),
    ];

    for (group, users) in group_assignments {
        for user in users {
            query!(
                "INSERT INTO group_user (user_id, group_id) VALUES ($1, $2)",
                user.id,
                group.id
            )
            .execute(&pool)
            .await
            .unwrap();
        }
    }

    // Create some network devices
    let network_device_1 = Device {
        id: NoId,
        name: "network-device-1".into(),
        user_id: user_1.id, // Owned by user 1
        device_type: DeviceType::Network,
        description: Some("Test network device 1".into()),
        wireguard_pubkey: Default::default(),
        created: Default::default(),
        configured: true,
    };
    let network_device_1 = network_device_1.save(&pool).await.unwrap();

    let network_device_2 = Device {
        id: NoId,
        name: "network-device-2".into(),
        user_id: user_2.id, // Owned by user 2
        device_type: DeviceType::Network,
        description: Some("Test network device 2".into()),
        wireguard_pubkey: Default::default(),
        created: Default::default(),
        configured: true,
    };
    let network_device_2 = network_device_2.save(&pool).await.unwrap();

    let network_device_3 = Device {
        id: NoId,
        name: "network-device-3".into(),
        user_id: user_3.id, // Owned by user 3
        device_type: DeviceType::Network,
        description: Some("Test network device 3".into()),
        wireguard_pubkey: Default::default(),
        created: Default::default(),
        configured: true,
    };
    let network_device_3 = network_device_3.save(&pool).await.unwrap();

    // Add network devices to location's VPN network
    let network_devices = vec![
        (
            network_device_1.id,
            IpAddr::V4(Ipv4Addr::new(10, 0, 100, 1)),
        ),
        (
            network_device_2.id,
            IpAddr::V4(Ipv4Addr::new(10, 0, 100, 2)),
        ),
        (
            network_device_3.id,
            IpAddr::V4(Ipv4Addr::new(10, 0, 100, 3)),
        ),
    ];

    for (device_id, ip) in network_devices {
        let network_device = WireguardNetworkDevice {
            device_id,
            wireguard_network_id: location.id,
            wireguard_ips: vec![ip],
            preshared_key: None,
            is_authorized: true,
            authorized_at: None,
        };
        network_device.insert(&pool).await.unwrap();
    }

    // Create first ACL rule - Web access
    let acl_rule_1 = AclRule {
        id: NoId,
        name: "Web Access".into(),
        all_locations: false,
        expires: None,
        allow_all_users: false,
        deny_all_users: false,
        allow_all_network_devices: false,
        deny_all_network_devices: false,
        addresses: vec!["192.168.1.0/24".parse().unwrap()],
        ports: vec![
            PortRange::new(80, 80).into(),
            PortRange::new(443, 443).into(),
        ],
        protocols: vec![Protocol::Tcp.into()],
        enabled: true,
        parent_id: None,
        state: RuleState::Applied,
        any_address: false,
        any_port: false,
        any_protocol: false,
        use_manual_destination_settings: true,
        ..Default::default()
    };
    let locations = vec![location.id];
    let allowed_users = vec![user_1.id, user_2.id]; // First two users can access web
    let denied_users = vec![user_3.id]; // Third user explicitly denied
    let allowed_groups = vec![group_1.id]; // First group allowed
    let denied_groups = Vec::new();
    let allowed_devices = vec![network_device_1.id];
    let denied_devices = vec![network_device_2.id, network_device_3.id];
    let destination_ranges = Vec::new();
    let aliases = Vec::new();

    let _acl_rule_1 = create_acl_rule(
        &pool,
        acl_rule_1,
        locations,
        allowed_users,
        denied_users,
        allowed_groups,
        denied_groups,
        allowed_devices,
        denied_devices,
        destination_ranges,
        aliases,
    )
    .await;

    // Create second ACL rule - DNS access
    let acl_rule_2 = AclRule {
        id: NoId,
        name: "DNS Access".into(),
        all_locations: false,
        expires: None,
        allow_all_users: true, // Allow all users
        deny_all_users: false,
        allow_all_network_devices: false,
        deny_all_network_devices: false,
        addresses: Vec::new(), // Will use destination ranges instead
        ports: vec![PortRange::new(53, 53).into()],
        protocols: vec![Protocol::Udp.into(), Protocol::Tcp.into()],
        enabled: true,
        parent_id: None,
        state: RuleState::Applied,
        any_address: false,
        any_port: false,
        any_protocol: false,
        use_manual_destination_settings: true,
        ..Default::default()
    };
    let locations_2 = vec![location.id];
    let allowed_users_2 = Vec::new();
    let denied_users_2 = vec![user_5.id]; // Fifth user denied DNS
    let allowed_groups_2 = Vec::new();
    let denied_groups_2 = vec![group_2.id];
    let allowed_devices_2 = vec![network_device_1.id, network_device_2.id]; // First two network devices allowed
    let denied_devices_2 = vec![network_device_3.id]; // Third network device denied
    let destination_ranges_2 = vec![
        ("10.0.1.13".parse().unwrap(), "10.0.1.43".parse().unwrap()),
        ("10.0.1.52".parse().unwrap(), "10.0.2.43".parse().unwrap()),
    ];
    let aliases_2 = Vec::new();

    let _acl_rule_2 = create_acl_rule(
        &pool,
        acl_rule_2,
        locations_2,
        allowed_users_2,
        denied_users_2,
        allowed_groups_2,
        denied_groups_2,
        allowed_devices_2,
        denied_devices_2,
        destination_ranges_2,
        aliases_2,
    )
    .await;

    let mut conn = pool.acquire().await.unwrap();

    // try to generate firewall config with ACL disabled
    location.acl_enabled = false;
    let generated_firewall_config = try_get_location_firewall_config(&location, &mut conn)
        .await
        .unwrap();
    assert!(generated_firewall_config.is_none());

    // generate firewall config with default policy Allow
    location.acl_enabled = true;
    location.acl_default_allow = true;
    let generated_firewall_config = try_get_location_firewall_config(&location, &mut conn)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        generated_firewall_config.default_policy,
        i32::from(FirewallPolicy::Allow)
    );

    let generated_firewall_rules = generated_firewall_config.rules;

    assert_eq!(generated_firewall_rules.len(), 4);

    // First ACL - Web Access ALLOW
    let web_allow_rule = &generated_firewall_rules[0];
    assert_eq!(web_allow_rule.verdict, i32::from(FirewallPolicy::Allow));
    assert_eq!(web_allow_rule.protocols, vec![i32::from(Protocol::Tcp)]);
    assert_eq!(
        web_allow_rule.destination_addrs,
        [IpAddress {
            address: Some(Address::IpSubnet("192.168.1.0/24".to_string())),
        }]
    );
    assert_eq!(
        web_allow_rule.destination_ports,
        [
            Port {
                port: Some(PortInner::SinglePort(80))
            },
            Port {
                port: Some(PortInner::SinglePort(443))
            }
        ]
    );
    // Source addresses should include devices of users 1,2 and network_device_1
    assert_eq!(
        web_allow_rule.source_addrs,
        [
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
            IpAddress {
                address: Some(Address::Ip("10.0.100.1".to_string())),
            },
        ]
    );

    // First ACL - Web Access DENY
    let web_deny_rule = &generated_firewall_rules[2];
    assert_eq!(web_deny_rule.verdict, i32::from(FirewallPolicy::Deny));
    assert!(web_deny_rule.protocols.is_empty());
    assert!(web_deny_rule.destination_ports.is_empty());
    assert!(web_deny_rule.source_addrs.is_empty());
    assert_eq!(
        web_deny_rule.destination_addrs,
        [IpAddress {
            address: Some(Address::IpSubnet("192.168.1.0/24".to_string())),
        }]
    );

    // Second ACL - DNS Access ALLOW
    let dns_allow_rule = &generated_firewall_rules[1];
    assert_eq!(dns_allow_rule.verdict, i32::from(FirewallPolicy::Allow));
    assert_eq!(
        dns_allow_rule.protocols,
        [i32::from(Protocol::Tcp), i32::from(Protocol::Udp)]
    );
    assert_eq!(
        dns_allow_rule.destination_ports,
        [Port {
            port: Some(PortInner::SinglePort(53))
        }]
    );
    // Source addresses should include network_devices 1,2
    assert_eq!(
        dns_allow_rule.source_addrs,
        [
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
            IpAddress {
                address: Some(Address::IpRange(IpRange {
                    start: "10.0.100.1".to_string(),
                    end: "10.0.100.2".to_string(),
                })),
            },
        ]
    );

    let expected_destination_addrs = vec![
        IpAddress {
            address: Some(Address::Ip("10.0.1.13".to_string())),
        },
        IpAddress {
            address: Some(Address::IpSubnet("10.0.1.14/31".to_string())),
        },
        IpAddress {
            address: Some(Address::IpSubnet("10.0.1.16/28".to_string())),
        },
        IpAddress {
            address: Some(Address::IpSubnet("10.0.1.32/29".to_string())),
        },
        IpAddress {
            address: Some(Address::IpSubnet("10.0.1.40/30".to_string())),
        },
        IpAddress {
            address: Some(Address::IpSubnet("10.0.1.52/30".to_string())),
        },
        IpAddress {
            address: Some(Address::IpSubnet("10.0.1.56/29".to_string())),
        },
        IpAddress {
            address: Some(Address::IpSubnet("10.0.1.64/26".to_string())),
        },
        IpAddress {
            address: Some(Address::IpSubnet("10.0.1.128/25".to_string())),
        },
        IpAddress {
            address: Some(Address::IpSubnet("10.0.2.0/27".to_string())),
        },
        IpAddress {
            address: Some(Address::IpSubnet("10.0.2.32/29".to_string())),
        },
        IpAddress {
            address: Some(Address::IpSubnet("10.0.2.40/30".to_string())),
        },
    ];

    assert_eq!(dns_allow_rule.destination_addrs, expected_destination_addrs);

    // Second ACL - DNS Access DENY
    let dns_deny_rule = &generated_firewall_rules[3];
    assert_eq!(dns_deny_rule.verdict, i32::from(FirewallPolicy::Deny));
    assert!(dns_deny_rule.protocols.is_empty(),);
    assert!(dns_deny_rule.destination_ports.is_empty(),);
    assert!(dns_deny_rule.source_addrs.is_empty(),);
    assert_eq!(dns_deny_rule.destination_addrs, expected_destination_addrs);
}

#[sqlx::test]
async fn test_generate_firewall_rules_ipv6(_: PgPoolOptions, options: PgConnectOptions) {
    set_test_license_business();
    let pool = setup_pool(options).await;
    let mut rng = thread_rng();

    // Create test location
    let location = WireguardNetwork {
        id: NoId,
        acl_enabled: false,
        address: vec![IpNetwork::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), 0).unwrap()],
        ..Default::default()
    };
    let mut location = location.save(&pool).await.unwrap();

    // Setup test users and their devices
    let user_1: User<NoId> = rng.r#gen();
    let user_1 = user_1.save(&pool).await.unwrap();
    let user_2: User<NoId> = rng.r#gen();
    let user_2 = user_2.save(&pool).await.unwrap();
    let user_3: User<NoId> = rng.r#gen();
    let user_3 = user_3.save(&pool).await.unwrap();
    let user_4: User<NoId> = rng.r#gen();
    let user_4 = user_4.save(&pool).await.unwrap();
    let user_5: User<NoId> = rng.r#gen();
    let user_5 = user_5.save(&pool).await.unwrap();

    for user in [&user_1, &user_2, &user_3, &user_4, &user_5] {
        // Create 2 devices per user
        for device_num in 1..3 {
            let device = Device {
                id: NoId,
                name: format!("device-{}-{}", user.id, device_num),
                user_id: user.id,
                device_type: DeviceType::User,
                description: None,
                wireguard_pubkey: Default::default(),
                created: Default::default(),
                configured: true,
            };
            let device = device.save(&pool).await.unwrap();

            // Add device to location's VPN network
            let network_device = WireguardNetworkDevice {
                device_id: device.id,
                wireguard_network_id: location.id,
                wireguard_ips: vec![IpAddr::V6(Ipv6Addr::new(
                    0xff00,
                    0,
                    0,
                    0,
                    0,
                    0,
                    user.id as u16,
                    device_num as u16,
                ))],
                preshared_key: None,
                is_authorized: true,
                authorized_at: None,
            };
            network_device.insert(&pool).await.unwrap();
        }
    }

    // Setup test groups
    let group_1 = Group {
        id: NoId,
        name: "group_1".into(),
        ..Default::default()
    };
    let group_1 = group_1.save(&pool).await.unwrap();
    let group_2 = Group {
        id: NoId,
        name: "group_2".into(),
        ..Default::default()
    };
    let group_2 = group_2.save(&pool).await.unwrap();

    // Assign users to groups:
    // Group 1: users 1,2
    // Group 2: users 3,4
    let group_assignments = vec![
        (&group_1, vec![&user_1, &user_2]),
        (&group_2, vec![&user_3, &user_4]),
    ];

    for (group, users) in group_assignments {
        for user in users {
            query!(
                "INSERT INTO group_user (user_id, group_id) VALUES ($1, $2)",
                user.id,
                group.id
            )
            .execute(&pool)
            .await
            .unwrap();
        }
    }

    // Create some network devices
    let network_device_1 = Device {
        id: NoId,
        name: "network-device-1".into(),
        user_id: user_1.id, // Owned by user 1
        device_type: DeviceType::Network,
        description: Some("Test network device 1".into()),
        wireguard_pubkey: Default::default(),
        created: Default::default(),
        configured: true,
    };
    let network_device_1 = network_device_1.save(&pool).await.unwrap();

    let network_device_2 = Device {
        id: NoId,
        name: "network-device-2".into(),
        user_id: user_2.id, // Owned by user 2
        device_type: DeviceType::Network,
        description: Some("Test network device 2".into()),
        wireguard_pubkey: Default::default(),
        created: Default::default(),
        configured: true,
    };
    let network_device_2 = network_device_2.save(&pool).await.unwrap();

    let network_device_3 = Device {
        id: NoId,
        name: "network-device-3".into(),
        user_id: user_3.id, // Owned by user 3
        device_type: DeviceType::Network,
        description: Some("Test network device 3".into()),
        wireguard_pubkey: Default::default(),
        created: Default::default(),
        configured: true,
    };
    let network_device_3 = network_device_3.save(&pool).await.unwrap();

    // Add network devices to location's VPN network
    let network_devices = vec![
        (
            network_device_1.id,
            IpAddr::V6(Ipv6Addr::new(0xff00, 0, 0, 0, 0, 0, 0x0100, 1)),
        ),
        (
            network_device_2.id,
            IpAddr::V6(Ipv6Addr::new(0xff00, 0, 0, 0, 0, 0, 0x0100, 2)),
        ),
        (
            network_device_3.id,
            IpAddr::V6(Ipv6Addr::new(0xff00, 0, 0, 0, 0, 0, 0x0100, 3)),
        ),
    ];

    for (device_id, ip) in network_devices {
        let network_device = WireguardNetworkDevice {
            device_id,
            wireguard_network_id: location.id,
            wireguard_ips: vec![ip],
            preshared_key: None,
            is_authorized: true,
            authorized_at: None,
        };
        network_device.insert(&pool).await.unwrap();
    }

    // Create first ACL rule - Web access
    let acl_rule_1 = AclRule {
        id: NoId,
        name: "Web Access".into(),
        all_locations: false,
        expires: None,
        allow_all_users: false,
        deny_all_users: false,
        allow_all_network_devices: false,
        deny_all_network_devices: false,
        addresses: vec!["fc00::0/112".parse().unwrap()],
        ports: vec![
            PortRange::new(80, 80).into(),
            PortRange::new(443, 443).into(),
        ],
        protocols: vec![Protocol::Tcp.into()],
        enabled: true,
        parent_id: None,
        state: RuleState::Applied,
        any_address: false,
        any_port: false,
        any_protocol: false,
        use_manual_destination_settings: true,
        ..Default::default()
    };
    let locations = vec![location.id];
    let allowed_users = vec![user_1.id, user_2.id]; // First two users can access web
    let denied_users = vec![user_3.id]; // Third user explicitly denied
    let allowed_groups = vec![group_1.id]; // First group allowed
    let denied_groups = Vec::new();
    let allowed_devices = vec![network_device_1.id];
    let denied_devices = vec![network_device_2.id, network_device_3.id];
    let destination_ranges = Vec::new();
    let aliases = Vec::new();

    let _acl_rule_1 = create_acl_rule(
        &pool,
        acl_rule_1,
        locations,
        allowed_users,
        denied_users,
        allowed_groups,
        denied_groups,
        allowed_devices,
        denied_devices,
        destination_ranges,
        aliases,
    )
    .await;

    // Create second ACL rule - DNS access
    let acl_rule_2 = AclRule {
        id: NoId,
        name: "DNS Access".into(),
        all_locations: false,
        expires: None,
        allow_all_users: true, // Allow all users
        deny_all_users: false,
        allow_all_network_devices: false,
        deny_all_network_devices: false,
        addresses: Vec::new(), // Will use destination ranges instead
        ports: vec![PortRange::new(53, 53).into()],
        protocols: vec![Protocol::Udp.into(), Protocol::Tcp.into()],
        enabled: true,
        parent_id: None,
        state: RuleState::Applied,
        any_address: false,
        any_port: false,
        any_protocol: false,
        use_manual_destination_settings: true,
        ..Default::default()
    };
    let locations_2 = vec![location.id];
    let allowed_users_2 = Vec::new();
    let denied_users_2 = vec![user_5.id]; // Fifth user denied DNS
    let allowed_groups_2 = Vec::new();
    let denied_groups_2 = vec![group_2.id];
    let allowed_devices_2 = vec![network_device_1.id, network_device_2.id]; // First two network devices allowed
    let denied_devices_2 = vec![network_device_3.id]; // Third network device denied
    let destination_ranges_2 = vec![
        ("fc00::1:13".parse().unwrap(), "fc00::1:43".parse().unwrap()),
        ("fc00::1:52".parse().unwrap(), "fc00::2:43".parse().unwrap()),
    ];
    let aliases_2 = Vec::new();

    let _acl_rule_2 = create_acl_rule(
        &pool,
        acl_rule_2,
        locations_2,
        allowed_users_2,
        denied_users_2,
        allowed_groups_2,
        denied_groups_2,
        allowed_devices_2,
        denied_devices_2,
        destination_ranges_2,
        aliases_2,
    )
    .await;

    let mut conn = pool.acquire().await.unwrap();

    // try to generate firewall config with ACL disabled
    location.acl_enabled = false;
    let generated_firewall_config = try_get_location_firewall_config(&location, &mut conn)
        .await
        .unwrap();
    assert!(generated_firewall_config.is_none());

    // generate firewall config with default policy Allow
    location.acl_enabled = true;
    location.acl_default_allow = true;
    let generated_firewall_config = try_get_location_firewall_config(&location, &mut conn)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        generated_firewall_config.default_policy,
        i32::from(FirewallPolicy::Allow)
    );

    let generated_firewall_rules = generated_firewall_config.rules;

    assert_eq!(generated_firewall_rules.len(), 4);

    // First ACL - Web Access ALLOW
    let web_allow_rule = &generated_firewall_rules[0];
    assert_eq!(web_allow_rule.verdict, i32::from(FirewallPolicy::Allow));
    assert_eq!(web_allow_rule.protocols, vec![i32::from(Protocol::Tcp)]);
    assert_eq!(
        web_allow_rule.destination_addrs,
        [IpAddress {
            address: Some(Address::IpSubnet("fc00::/112".to_string())),
        }]
    );
    assert_eq!(
        web_allow_rule.destination_ports,
        [
            Port {
                port: Some(PortInner::SinglePort(80))
            },
            Port {
                port: Some(PortInner::SinglePort(443))
            }
        ]
    );
    // Source addresses should include devices of users 1,2 and network_device_1
    assert_eq!(
        web_allow_rule.source_addrs,
        [
            IpAddress {
                address: Some(Address::IpRange(IpRange {
                    start: "ff00::1:1".to_string(),
                    end: "ff00::1:2".to_string(),
                })),
            },
            IpAddress {
                address: Some(Address::IpRange(IpRange {
                    start: "ff00::2:1".to_string(),
                    end: "ff00::2:2".to_string(),
                })),
            },
            IpAddress {
                address: Some(Address::Ip("ff00::100:1".to_string())),
            },
        ]
    );

    // First ACL - Web Access DENY
    let web_deny_rule = &generated_firewall_rules[2];
    assert_eq!(web_deny_rule.verdict, i32::from(FirewallPolicy::Deny));
    assert!(web_deny_rule.protocols.is_empty());
    assert!(web_deny_rule.destination_ports.is_empty());
    assert!(web_deny_rule.source_addrs.is_empty());
    assert_eq!(
        web_deny_rule.destination_addrs,
        [IpAddress {
            address: Some(Address::IpSubnet("fc00::/112".to_string())),
        }]
    );

    // Second ACL - DNS Access ALLOW
    let dns_allow_rule = &generated_firewall_rules[1];
    assert_eq!(dns_allow_rule.verdict, i32::from(FirewallPolicy::Allow));
    assert_eq!(
        dns_allow_rule.protocols,
        [i32::from(Protocol::Tcp), i32::from(Protocol::Udp)]
    );
    assert_eq!(
        dns_allow_rule.destination_ports,
        [Port {
            port: Some(PortInner::SinglePort(53))
        }]
    );

    let expected_destination_addrs = vec![
        IpAddress {
            address: Some(Address::Ip("fc00::1:13".to_string())),
        },
        IpAddress {
            address: Some(Address::IpSubnet("fc00::1:14/126".to_string())),
        },
        IpAddress {
            address: Some(Address::IpSubnet("fc00::1:18/125".to_string())),
        },
        IpAddress {
            address: Some(Address::IpSubnet("fc00::1:20/123".to_string())),
        },
        IpAddress {
            address: Some(Address::IpSubnet("fc00::1:40/126".to_string())),
        },
        IpAddress {
            address: Some(Address::IpSubnet("fc00::1:52/127".to_string())),
        },
        IpAddress {
            address: Some(Address::IpSubnet("fc00::1:54/126".to_string())),
        },
        IpAddress {
            address: Some(Address::IpSubnet("fc00::1:58/125".to_string())),
        },
        IpAddress {
            address: Some(Address::IpSubnet("fc00::1:60/123".to_string())),
        },
        IpAddress {
            address: Some(Address::IpSubnet("fc00::1:80/121".to_string())),
        },
        IpAddress {
            address: Some(Address::IpSubnet("fc00::1:100/120".to_string())),
        },
        IpAddress {
            address: Some(Address::IpSubnet("fc00::1:200/119".to_string())),
        },
        IpAddress {
            address: Some(Address::IpSubnet("fc00::1:400/118".to_string())),
        },
        IpAddress {
            address: Some(Address::IpSubnet("fc00::1:800/117".to_string())),
        },
        IpAddress {
            address: Some(Address::IpSubnet("fc00::1:1000/116".to_string())),
        },
        IpAddress {
            address: Some(Address::IpSubnet("fc00::1:2000/115".to_string())),
        },
        IpAddress {
            address: Some(Address::IpSubnet("fc00::1:4000/114".to_string())),
        },
        IpAddress {
            address: Some(Address::IpSubnet("fc00::1:8000/113".to_string())),
        },
        IpAddress {
            address: Some(Address::IpSubnet("fc00::2:0/122".to_string())),
        },
        IpAddress {
            address: Some(Address::IpSubnet("fc00::2:40/126".to_string())),
        },
    ];

    // Source addresses should include network_devices 1,2
    assert_eq!(
        dns_allow_rule.source_addrs,
        [
            IpAddress {
                address: Some(Address::IpRange(IpRange {
                    start: "ff00::1:1".to_string(),
                    end: "ff00::1:2".to_string(),
                })),
            },
            IpAddress {
                address: Some(Address::IpRange(IpRange {
                    start: "ff00::2:1".to_string(),
                    end: "ff00::2:2".to_string(),
                })),
            },
            IpAddress {
                address: Some(Address::IpRange(IpRange {
                    start: "ff00::100:1".to_string(),
                    end: "ff00::100:2".to_string(),
                })),
            },
        ]
    );
    assert_eq!(dns_allow_rule.destination_addrs, expected_destination_addrs);

    // Second ACL - DNS Access DENY
    let dns_deny_rule = &generated_firewall_rules[3];
    assert_eq!(dns_deny_rule.verdict, i32::from(FirewallPolicy::Deny));
    assert!(dns_deny_rule.protocols.is_empty(),);
    assert!(dns_deny_rule.destination_ports.is_empty(),);
    assert!(dns_deny_rule.source_addrs.is_empty(),);
    assert_eq!(dns_deny_rule.destination_addrs, expected_destination_addrs);
}

#[sqlx::test]
async fn test_generate_firewall_rules_ipv4_and_ipv6(_: PgPoolOptions, options: PgConnectOptions) {
    set_test_license_business();
    let pool = setup_pool(options).await;

    let mut rng = thread_rng();

    // Create test location
    let location = WireguardNetwork {
        id: NoId,
        acl_enabled: false,
        address: vec![
            IpNetwork::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0).unwrap(),
            IpNetwork::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), 0).unwrap(),
        ],
        ..Default::default()
    };
    let mut location = location.save(&pool).await.unwrap();

    // Setup test users and their devices
    let user_1: User<NoId> = rng.r#gen();
    let user_1 = user_1.save(&pool).await.unwrap();
    let user_2: User<NoId> = rng.r#gen();
    let user_2 = user_2.save(&pool).await.unwrap();
    let user_3: User<NoId> = rng.r#gen();
    let user_3 = user_3.save(&pool).await.unwrap();
    let user_4: User<NoId> = rng.r#gen();
    let user_4 = user_4.save(&pool).await.unwrap();
    let user_5: User<NoId> = rng.r#gen();
    let user_5 = user_5.save(&pool).await.unwrap();

    for user in [&user_1, &user_2, &user_3, &user_4, &user_5] {
        // Create 2 devices per user
        for device_num in 1..3 {
            let device = Device {
                id: NoId,
                name: format!("device-{}-{}", user.id, device_num),
                user_id: user.id,
                device_type: DeviceType::User,
                description: None,
                wireguard_pubkey: Default::default(),
                created: Default::default(),
                configured: true,
            };
            let device = device.save(&pool).await.unwrap();

            // Add device to location's VPN network
            let network_device = WireguardNetworkDevice {
                device_id: device.id,
                wireguard_network_id: location.id,
                wireguard_ips: vec![
                    IpAddr::V4(Ipv4Addr::new(10, 0, user.id as u8, device_num as u8)),
                    IpAddr::V6(Ipv6Addr::new(
                        0xff00,
                        0,
                        0,
                        0,
                        0,
                        0,
                        user.id as u16,
                        device_num as u16,
                    )),
                ],
                preshared_key: None,
                is_authorized: true,
                authorized_at: None,
            };
            network_device.insert(&pool).await.unwrap();
        }
    }

    // Setup test groups
    let group_1 = Group {
        id: NoId,
        name: "group_1".into(),
        ..Default::default()
    };
    let group_1 = group_1.save(&pool).await.unwrap();
    let group_2 = Group {
        id: NoId,
        name: "group_2".into(),
        ..Default::default()
    };
    let group_2 = group_2.save(&pool).await.unwrap();

    // Assign users to groups:
    // Group 1: users 1,2
    // Group 2: users 3,4
    let group_assignments = vec![
        (&group_1, vec![&user_1, &user_2]),
        (&group_2, vec![&user_3, &user_4]),
    ];

    for (group, users) in group_assignments {
        for user in users {
            query!(
                "INSERT INTO group_user (user_id, group_id) VALUES ($1, $2)",
                user.id,
                group.id
            )
            .execute(&pool)
            .await
            .unwrap();
        }
    }

    // Create some network devices
    let network_device_1 = Device {
        id: NoId,
        name: "network-device-1".into(),
        user_id: user_1.id, // Owned by user 1
        device_type: DeviceType::Network,
        description: Some("Test network device 1".into()),
        wireguard_pubkey: Default::default(),
        created: Default::default(),
        configured: true,
    };
    let network_device_1 = network_device_1.save(&pool).await.unwrap();

    let network_device_2 = Device {
        id: NoId,
        name: "network-device-2".into(),
        user_id: user_2.id, // Owned by user 2
        device_type: DeviceType::Network,
        description: Some("Test network device 2".into()),
        wireguard_pubkey: Default::default(),
        created: Default::default(),
        configured: true,
    };
    let network_device_2 = network_device_2.save(&pool).await.unwrap();

    let network_device_3 = Device {
        id: NoId,
        name: "network-device-3".into(),
        user_id: user_3.id, // Owned by user 3
        device_type: DeviceType::Network,
        description: Some("Test network device 3".into()),
        wireguard_pubkey: Default::default(),
        created: Default::default(),
        configured: true,
    };
    let network_device_3 = network_device_3.save(&pool).await.unwrap();

    // Add network devices to location's VPN network
    let network_devices = vec![
        (
            network_device_1.id,
            vec![
                IpAddr::V4(Ipv4Addr::new(10, 0, 100, 1)),
                IpAddr::V6(Ipv6Addr::new(0xff00, 0, 0, 0, 0, 0, 0x0100, 1)),
            ],
        ),
        (
            network_device_2.id,
            vec![
                IpAddr::V4(Ipv4Addr::new(10, 0, 100, 2)),
                IpAddr::V6(Ipv6Addr::new(0xff00, 0, 0, 0, 0, 0, 0x0100, 2)),
            ],
        ),
        (
            network_device_3.id,
            vec![
                IpAddr::V4(Ipv4Addr::new(10, 0, 100, 3)),
                IpAddr::V6(Ipv6Addr::new(0xff00, 0, 0, 0, 0, 0, 0x0100, 3)),
            ],
        ),
    ];

    for (device_id, ips) in network_devices {
        let network_device = WireguardNetworkDevice {
            device_id,
            wireguard_network_id: location.id,
            wireguard_ips: ips,
            preshared_key: None,
            is_authorized: true,
            authorized_at: None,
        };
        network_device.insert(&pool).await.unwrap();
    }

    // Create first ACL rule - Web access
    let acl_rule_1 = AclRule {
        id: NoId,
        name: "Web Access".into(),
        all_locations: false,
        expires: None,
        allow_all_users: false,
        deny_all_users: false,
        allow_all_network_devices: false,
        deny_all_network_devices: false,
        addresses: vec![
            "192.168.1.0/24".parse().unwrap(),
            "fc00::0/112".parse().unwrap(),
        ],
        ports: vec![
            PortRange::new(80, 80).into(),
            PortRange::new(443, 443).into(),
        ],
        protocols: vec![Protocol::Tcp.into()],
        enabled: true,
        parent_id: None,
        state: RuleState::Applied,
        any_address: false,
        any_port: false,
        any_protocol: false,
        use_manual_destination_settings: true,
        ..Default::default()
    };
    let locations = vec![location.id];
    let allowed_users = vec![user_1.id, user_2.id]; // First two users can access web
    let denied_users = vec![user_3.id]; // Third user explicitly denied
    let allowed_groups = vec![group_1.id]; // First group allowed
    let denied_groups = Vec::new();
    let allowed_devices = vec![network_device_1.id];
    let denied_devices = vec![network_device_2.id, network_device_3.id];
    let destination_ranges = Vec::new();
    let aliases = Vec::new();

    let _acl_rule_1 = create_acl_rule(
        &pool,
        acl_rule_1,
        locations,
        allowed_users,
        denied_users,
        allowed_groups,
        denied_groups,
        allowed_devices,
        denied_devices,
        destination_ranges,
        aliases,
    )
    .await;

    // Create second ACL rule - DNS access
    let acl_rule_2 = AclRule {
        id: NoId,
        name: "DNS Access".into(),
        all_locations: false,
        expires: None,
        allow_all_users: true, // Allow all users
        deny_all_users: false,
        allow_all_network_devices: false,
        deny_all_network_devices: false,
        addresses: Vec::new(), // Will use destination ranges instead
        ports: vec![PortRange::new(53, 53).into()],
        protocols: vec![Protocol::Udp.into(), Protocol::Tcp.into()],
        enabled: true,
        parent_id: None,
        state: RuleState::Applied,
        any_address: false,
        any_port: false,
        any_protocol: false,
        use_manual_destination_settings: true,
        ..Default::default()
    };
    let locations_2 = vec![location.id];
    let allowed_users_2 = Vec::new();
    let denied_users_2 = vec![user_5.id]; // Fifth user denied DNS
    let allowed_groups_2 = Vec::new();
    let denied_groups_2 = vec![group_2.id];
    let allowed_devices_2 = vec![network_device_1.id, network_device_2.id]; // First two network devices allowed
    let denied_devices_2 = vec![network_device_3.id]; // Third network device denied
    let destination_ranges_2 = vec![
        ("10.0.1.13".parse().unwrap(), "10.0.1.43".parse().unwrap()),
        ("10.0.1.52".parse().unwrap(), "10.0.2.43".parse().unwrap()),
        ("fc00::1:13".parse().unwrap(), "fc00::1:43".parse().unwrap()),
        ("fc00::1:52".parse().unwrap(), "fc00::2:43".parse().unwrap()),
    ];
    let aliases_2 = Vec::new();

    let _acl_rule_2 = create_acl_rule(
        &pool,
        acl_rule_2,
        locations_2,
        allowed_users_2,
        denied_users_2,
        allowed_groups_2,
        denied_groups_2,
        allowed_devices_2,
        denied_devices_2,
        destination_ranges_2,
        aliases_2,
    )
    .await;

    let mut conn = pool.acquire().await.unwrap();

    // try to generate firewall config with ACL disabled
    location.acl_enabled = false;
    let generated_firewall_config = try_get_location_firewall_config(&location, &mut conn)
        .await
        .unwrap();
    assert!(generated_firewall_config.is_none());

    // generate firewall config with default policy Allow
    location.acl_enabled = true;
    location.acl_default_allow = true;
    let generated_firewall_config = try_get_location_firewall_config(&location, &mut conn)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        generated_firewall_config.default_policy,
        i32::from(FirewallPolicy::Allow)
    );

    let generated_firewall_rules = generated_firewall_config.rules;

    assert_eq!(generated_firewall_rules.len(), 8);

    // First ACL - Web Access ALLOW
    let web_allow_rule_ipv4 = &generated_firewall_rules[0];
    assert_eq!(
        web_allow_rule_ipv4.verdict,
        i32::from(FirewallPolicy::Allow)
    );
    assert_eq!(
        web_allow_rule_ipv4.protocols,
        vec![i32::from(Protocol::Tcp)]
    );
    assert_eq!(
        web_allow_rule_ipv4.destination_addrs,
        vec![IpAddress {
            address: Some(Address::IpSubnet("192.168.1.0/24".to_string())),
        }]
    );
    assert_eq!(
        web_allow_rule_ipv4.destination_ports,
        vec![
            Port {
                port: Some(PortInner::SinglePort(80))
            },
            Port {
                port: Some(PortInner::SinglePort(443))
            }
        ]
    );
    // Source addresses should include devices of users 1,2 and network_device_1
    assert_eq!(
        web_allow_rule_ipv4.source_addrs,
        vec![
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
            IpAddress {
                address: Some(Address::Ip("10.0.100.1".to_string())),
            },
        ]
    );

    let web_allow_rule_ipv6 = &generated_firewall_rules[1];
    assert_eq!(
        web_allow_rule_ipv6.verdict,
        i32::from(FirewallPolicy::Allow)
    );
    assert_eq!(web_allow_rule_ipv6.protocols, [i32::from(Protocol::Tcp)]);
    assert_eq!(
        web_allow_rule_ipv6.destination_addrs,
        [IpAddress {
            address: Some(Address::IpSubnet("fc00::/112".to_string())),
        }]
    );
    assert_eq!(
        web_allow_rule_ipv6.destination_ports,
        [
            Port {
                port: Some(PortInner::SinglePort(80))
            },
            Port {
                port: Some(PortInner::SinglePort(443))
            }
        ]
    );
    // Source addresses should include devices of users 1,2 and network_device_1
    assert_eq!(
        web_allow_rule_ipv6.source_addrs,
        [
            IpAddress {
                address: Some(Address::IpRange(IpRange {
                    start: "ff00::1:1".to_string(),
                    end: "ff00::1:2".to_string(),
                })),
            },
            IpAddress {
                address: Some(Address::IpRange(IpRange {
                    start: "ff00::2:1".to_string(),
                    end: "ff00::2:2".to_string(),
                })),
            },
            IpAddress {
                address: Some(Address::Ip("ff00::100:1".to_string())),
            },
        ]
    );

    // First ACL - Web Access DENY
    let web_deny_rule_ipv4 = &generated_firewall_rules[4];
    assert_eq!(web_deny_rule_ipv4.verdict, i32::from(FirewallPolicy::Deny));
    assert!(web_deny_rule_ipv4.protocols.is_empty());
    assert!(web_deny_rule_ipv4.destination_ports.is_empty());
    assert!(web_deny_rule_ipv4.source_addrs.is_empty());
    assert_eq!(
        web_deny_rule_ipv4.destination_addrs,
        [IpAddress {
            address: Some(Address::IpSubnet("192.168.1.0/24".to_string())),
        }]
    );

    let web_deny_rule_ipv6 = &generated_firewall_rules[5];
    assert_eq!(web_deny_rule_ipv6.verdict, i32::from(FirewallPolicy::Deny));
    assert!(web_deny_rule_ipv6.protocols.is_empty());
    assert!(web_deny_rule_ipv6.destination_ports.is_empty());
    assert!(web_deny_rule_ipv6.source_addrs.is_empty());
    assert_eq!(
        web_deny_rule_ipv6.destination_addrs,
        [IpAddress {
            address: Some(Address::IpSubnet("fc00::/112".to_string())),
        }]
    );

    // Second ACL - DNS Access ALLOW
    let dns_allow_rule_ipv4 = &generated_firewall_rules[2];
    assert_eq!(
        dns_allow_rule_ipv4.verdict,
        i32::from(FirewallPolicy::Allow)
    );
    assert_eq!(
        dns_allow_rule_ipv4.protocols,
        [i32::from(Protocol::Tcp), i32::from(Protocol::Udp)]
    );
    assert_eq!(
        dns_allow_rule_ipv4.destination_ports,
        [Port {
            port: Some(PortInner::SinglePort(53))
        }]
    );
    // Source addresses should include network_devices 1,2
    assert_eq!(
        dns_allow_rule_ipv4.source_addrs,
        [
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
            IpAddress {
                address: Some(Address::IpRange(IpRange {
                    start: "10.0.100.1".to_string(),
                    end: "10.0.100.2".to_string(),
                })),
            },
        ]
    );

    let expected_destination_addrs_v4 = vec![
        IpAddress {
            address: Some(Address::Ip("10.0.1.13".to_string())),
        },
        IpAddress {
            address: Some(Address::IpSubnet("10.0.1.14/31".to_string())),
        },
        IpAddress {
            address: Some(Address::IpSubnet("10.0.1.16/28".to_string())),
        },
        IpAddress {
            address: Some(Address::IpSubnet("10.0.1.32/29".to_string())),
        },
        IpAddress {
            address: Some(Address::IpSubnet("10.0.1.40/30".to_string())),
        },
        IpAddress {
            address: Some(Address::IpSubnet("10.0.1.52/30".to_string())),
        },
        IpAddress {
            address: Some(Address::IpSubnet("10.0.1.56/29".to_string())),
        },
        IpAddress {
            address: Some(Address::IpSubnet("10.0.1.64/26".to_string())),
        },
        IpAddress {
            address: Some(Address::IpSubnet("10.0.1.128/25".to_string())),
        },
        IpAddress {
            address: Some(Address::IpSubnet("10.0.2.0/27".to_string())),
        },
        IpAddress {
            address: Some(Address::IpSubnet("10.0.2.32/29".to_string())),
        },
        IpAddress {
            address: Some(Address::IpSubnet("10.0.2.40/30".to_string())),
        },
    ];

    assert_eq!(
        dns_allow_rule_ipv4.destination_addrs,
        expected_destination_addrs_v4
    );

    let dns_allow_rule_ipv6 = &generated_firewall_rules[3];
    assert_eq!(
        dns_allow_rule_ipv6.verdict,
        i32::from(FirewallPolicy::Allow)
    );
    assert_eq!(
        dns_allow_rule_ipv6.protocols,
        [i32::from(Protocol::Tcp), i32::from(Protocol::Udp)]
    );
    assert_eq!(
        dns_allow_rule_ipv6.destination_ports,
        [Port {
            port: Some(PortInner::SinglePort(53))
        }]
    );
    // Source addresses should include network_devices 1,2
    assert_eq!(
        dns_allow_rule_ipv6.source_addrs,
        [
            IpAddress {
                address: Some(Address::IpRange(IpRange {
                    start: "ff00::1:1".to_string(),
                    end: "ff00::1:2".to_string(),
                })),
            },
            IpAddress {
                address: Some(Address::IpRange(IpRange {
                    start: "ff00::2:1".to_string(),
                    end: "ff00::2:2".to_string(),
                })),
            },
            IpAddress {
                address: Some(Address::IpRange(IpRange {
                    start: "ff00::100:1".to_string(),
                    end: "ff00::100:2".to_string(),
                })),
            },
        ]
    );

    let expected_destination_addrs_v6 = vec![
        IpAddress {
            address: Some(Address::Ip("fc00::1:13".to_string())),
        },
        IpAddress {
            address: Some(Address::IpSubnet("fc00::1:14/126".to_string())),
        },
        IpAddress {
            address: Some(Address::IpSubnet("fc00::1:18/125".to_string())),
        },
        IpAddress {
            address: Some(Address::IpSubnet("fc00::1:20/123".to_string())),
        },
        IpAddress {
            address: Some(Address::IpSubnet("fc00::1:40/126".to_string())),
        },
        IpAddress {
            address: Some(Address::IpSubnet("fc00::1:52/127".to_string())),
        },
        IpAddress {
            address: Some(Address::IpSubnet("fc00::1:54/126".to_string())),
        },
        IpAddress {
            address: Some(Address::IpSubnet("fc00::1:58/125".to_string())),
        },
        IpAddress {
            address: Some(Address::IpSubnet("fc00::1:60/123".to_string())),
        },
        IpAddress {
            address: Some(Address::IpSubnet("fc00::1:80/121".to_string())),
        },
        IpAddress {
            address: Some(Address::IpSubnet("fc00::1:100/120".to_string())),
        },
        IpAddress {
            address: Some(Address::IpSubnet("fc00::1:200/119".to_string())),
        },
        IpAddress {
            address: Some(Address::IpSubnet("fc00::1:400/118".to_string())),
        },
        IpAddress {
            address: Some(Address::IpSubnet("fc00::1:800/117".to_string())),
        },
        IpAddress {
            address: Some(Address::IpSubnet("fc00::1:1000/116".to_string())),
        },
        IpAddress {
            address: Some(Address::IpSubnet("fc00::1:2000/115".to_string())),
        },
        IpAddress {
            address: Some(Address::IpSubnet("fc00::1:4000/114".to_string())),
        },
        IpAddress {
            address: Some(Address::IpSubnet("fc00::1:8000/113".to_string())),
        },
        IpAddress {
            address: Some(Address::IpSubnet("fc00::2:0/122".to_string())),
        },
        IpAddress {
            address: Some(Address::IpSubnet("fc00::2:40/126".to_string())),
        },
    ];

    assert_eq!(
        dns_allow_rule_ipv6.destination_addrs,
        expected_destination_addrs_v6
    );

    // Second ACL - DNS Access DENY
    let dns_deny_rule_ipv4 = &generated_firewall_rules[6];
    assert_eq!(dns_deny_rule_ipv4.verdict, i32::from(FirewallPolicy::Deny));
    assert!(dns_deny_rule_ipv4.protocols.is_empty(),);
    assert!(dns_deny_rule_ipv4.destination_ports.is_empty(),);
    assert!(dns_deny_rule_ipv4.source_addrs.is_empty(),);
    assert_eq!(
        dns_deny_rule_ipv4.destination_addrs,
        expected_destination_addrs_v4
    );

    let dns_deny_rule_ipv6 = &generated_firewall_rules[7];
    assert_eq!(dns_deny_rule_ipv6.verdict, i32::from(FirewallPolicy::Deny));
    assert!(dns_deny_rule_ipv6.protocols.is_empty(),);
    assert!(dns_deny_rule_ipv6.destination_ports.is_empty(),);
    assert!(dns_deny_rule_ipv6.source_addrs.is_empty(),);
    assert_eq!(
        dns_deny_rule_ipv6.destination_addrs,
        expected_destination_addrs_v6
    );
}

#[sqlx::test]
async fn test_alias_kinds(_: PgPoolOptions, options: PgConnectOptions) {
    set_test_license_business();
    let pool = setup_pool(options).await;

    let mut rng = thread_rng();

    // Create test location
    let location = WireguardNetwork {
        id: NoId,
        acl_enabled: true,
        address: vec!["10.0.0.0/16".parse().unwrap()],
        ..Default::default()
    }
    .save(&pool)
    .await
    .unwrap();

    // Setup some test users and their devices
    create_test_users_and_devices(&mut rng, &pool, vec![&location]).await;

    // create ACL rule
    let acl_rule = AclRule {
        id: NoId,
        name: "test rule".to_string(),
        expires: None,
        enabled: true,
        state: RuleState::Applied,
        addresses: vec!["192.168.1.0/24".parse().unwrap()],
        allow_all_users: true,
        use_manual_destination_settings: true,
        ..Default::default()
    }
    .save(&pool)
    .await
    .unwrap();

    // create different kinds of aliases and add them to the rule
    let destination_alias = AclAlias {
        id: NoId,
        name: "destination alias".to_string(),
        kind: AliasKind::Destination,
        ports: vec![PortRange::new(100, 200).into()],
        any_address: true,
        any_protocol: true,
        ..Default::default()
    }
    .save(&pool)
    .await
    .unwrap();
    let component_alias = AclAlias {
        id: NoId,
        kind: AliasKind::Component,
        addresses: vec!["10.0.2.3".parse().unwrap()],
        ..Default::default()
    }
    .save(&pool)
    .await
    .unwrap();
    for alias in [&destination_alias, &component_alias] {
        AclRuleAlias::new(acl_rule.id, alias.id)
            .save(&pool)
            .await
            .unwrap();
    }

    // assign rule to location
    AclRuleNetwork::new(acl_rule.id, location.id)
        .save(&pool)
        .await
        .unwrap();

    let mut conn = pool.acquire().await.unwrap();
    let generated_firewall_rules = try_get_location_firewall_config(&location, &mut conn)
        .await
        .unwrap()
        .unwrap()
        .rules;

    // check generated rules
    assert_eq!(generated_firewall_rules.len(), 4);
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
            address: Some(Address::Ip("10.0.2.3".to_string())),
        },
        IpAddress {
            address: Some(Address::IpSubnet("192.168.1.0/24".to_string())),
        },
    ];

    let allow_rule = &generated_firewall_rules[0];
    assert_eq!(allow_rule.verdict, i32::from(FirewallPolicy::Allow));
    assert_eq!(allow_rule.source_addrs, expected_source_addrs);
    assert_eq!(allow_rule.destination_addrs, expected_destination_addrs);
    assert!(allow_rule.destination_ports.is_empty());
    assert!(allow_rule.protocols.is_empty());
    assert_eq!(
        allow_rule.comment,
        Some("ACL 1 - test rule ALLOW".to_string())
    );

    let alias_allow_rule = &generated_firewall_rules[1];
    assert_eq!(alias_allow_rule.verdict, i32::from(FirewallPolicy::Allow));
    assert_eq!(alias_allow_rule.source_addrs, expected_source_addrs);
    assert!(alias_allow_rule.destination_addrs.is_empty());
    assert_eq!(
        alias_allow_rule.destination_ports,
        vec![Port {
            port: Some(PortInner::PortRange(PortRangeProto {
                start: 100,
                end: 200,
            }))
        }]
    );
    assert!(alias_allow_rule.protocols.is_empty());
    assert_eq!(
        alias_allow_rule.comment,
        Some("ACL 1 - test rule, ALIAS 1 - destination alias ALLOW".to_string())
    );

    let deny_rule = &generated_firewall_rules[2];
    assert_eq!(deny_rule.verdict, i32::from(FirewallPolicy::Deny));
    assert!(deny_rule.source_addrs.is_empty());
    assert_eq!(deny_rule.destination_addrs, expected_destination_addrs);
    assert!(deny_rule.destination_ports.is_empty());
    assert!(deny_rule.protocols.is_empty());
    assert_eq!(
        deny_rule.comment,
        Some("ACL 1 - test rule DENY".to_string())
    );

    let alias_deny_rule = &generated_firewall_rules[3];
    assert_eq!(alias_deny_rule.verdict, i32::from(FirewallPolicy::Deny));
    assert!(alias_deny_rule.source_addrs.is_empty());
    assert!(alias_deny_rule.destination_addrs.is_empty());
    assert!(alias_deny_rule.destination_ports.is_empty());
    assert!(alias_deny_rule.protocols.is_empty());
    assert_eq!(
        alias_deny_rule.comment,
        Some("ACL 1 - test rule, ALIAS 1 - destination alias DENY".to_string())
    );
}

#[sqlx::test]
async fn test_destination_alias_only_acl(_: PgPoolOptions, options: PgConnectOptions) {
    set_test_license_business();
    let pool = setup_pool(options).await;

    let mut rng = thread_rng();

    // Create test location
    let location = WireguardNetwork {
        id: NoId,
        acl_enabled: true,
        address: vec!["10.0.0.0/16".parse().unwrap()],
        ..Default::default()
    }
    .save(&pool)
    .await
    .unwrap();

    // Setup some test users and their devices
    create_test_users_and_devices(&mut rng, &pool, vec![&location]).await;

    // create ACL rule without manually configured destination
    let acl_rule = AclRule {
        id: NoId,
        name: "test rule".to_string(),
        expires: None,
        enabled: true,
        state: RuleState::Applied,
        addresses: Vec::new(),
        allow_all_users: true,
        use_manual_destination_settings: false,
        ..Default::default()
    }
    .save(&pool)
    .await
    .unwrap();

    // create different kinds of aliases and add them to the rule
    let destination_alias_1 = AclAlias {
        id: NoId,
        name: "postgres".to_string(),
        kind: AliasKind::Destination,
        addresses: vec!["10.0.2.3".parse().unwrap()],
        ports: vec![PortRange::new(5432, 5432).into()],
        ..Default::default()
    }
    .save(&pool)
    .await
    .unwrap();
    let destination_alias_2 = AclAlias {
        id: NoId,
        name: "redis".to_string(),
        kind: AliasKind::Destination,
        addresses: vec!["10.0.2.4".parse().unwrap()],
        ports: vec![PortRange::new(6379, 6379).into()],
        ..Default::default()
    }
    .save(&pool)
    .await
    .unwrap();
    for alias in [&destination_alias_1, &destination_alias_2] {
        AclRuleAlias::new(acl_rule.id, alias.id)
            .save(&pool)
            .await
            .unwrap();
    }

    // assign rule to location
    AclRuleNetwork::new(acl_rule.id, location.id)
        .save(&pool)
        .await
        .unwrap();

    let mut conn = pool.acquire().await.unwrap();
    let generated_firewall_rules = try_get_location_firewall_config(&location, &mut conn)
        .await
        .unwrap()
        .unwrap()
        .rules;

    // check generated rules
    assert_eq!(generated_firewall_rules.len(), 4);
    let expected_source_addrs = vec![
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

    let alias_allow_rule_1 = &generated_firewall_rules[0];
    assert_eq!(alias_allow_rule_1.verdict, i32::from(FirewallPolicy::Allow));
    assert_eq!(alias_allow_rule_1.source_addrs, expected_source_addrs);
    assert_eq!(
        alias_allow_rule_1.destination_addrs,
        vec![IpAddress {
            address: Some(Address::Ip("10.0.2.3".to_string())),
        },]
    );
    assert_eq!(
        alias_allow_rule_1.destination_ports,
        vec![Port {
            port: Some(PortInner::SinglePort(5432))
        }]
    );
    assert!(alias_allow_rule_1.protocols.is_empty());
    assert_eq!(
        alias_allow_rule_1.comment,
        Some("ACL 1 - test rule, ALIAS 1 - postgres ALLOW".to_string())
    );

    let alias_allow_rule_2 = &generated_firewall_rules[1];
    assert_eq!(alias_allow_rule_2.verdict, i32::from(FirewallPolicy::Allow));
    assert_eq!(alias_allow_rule_2.source_addrs, expected_source_addrs);
    assert_eq!(
        alias_allow_rule_2.destination_addrs,
        vec![IpAddress {
            address: Some(Address::Ip("10.0.2.4".to_string())),
        },]
    );
    assert_eq!(
        alias_allow_rule_2.destination_ports,
        vec![Port {
            port: Some(PortInner::SinglePort(6379))
        }]
    );
    assert!(alias_allow_rule_2.protocols.is_empty());
    assert_eq!(
        alias_allow_rule_2.comment,
        Some("ACL 1 - test rule, ALIAS 2 - redis ALLOW".to_string())
    );

    let alias_deny_rule_1 = &generated_firewall_rules[2];
    assert_eq!(alias_deny_rule_1.verdict, i32::from(FirewallPolicy::Deny));
    assert!(alias_deny_rule_1.source_addrs.is_empty());
    assert_eq!(
        alias_deny_rule_1.destination_addrs,
        vec![IpAddress {
            address: Some(Address::Ip("10.0.2.3".to_string())),
        },]
    );
    assert!(alias_deny_rule_1.destination_ports.is_empty());
    assert!(alias_deny_rule_1.protocols.is_empty());
    assert_eq!(
        alias_deny_rule_1.comment,
        Some("ACL 1 - test rule, ALIAS 1 - postgres DENY".to_string())
    );

    let alias_deny_rule_2 = &generated_firewall_rules[3];
    assert_eq!(alias_deny_rule_2.verdict, i32::from(FirewallPolicy::Deny));
    assert!(alias_deny_rule_2.source_addrs.is_empty());
    assert_eq!(
        alias_deny_rule_2.destination_addrs,
        vec![IpAddress {
            address: Some(Address::Ip("10.0.2.4".to_string())),
        },]
    );
    assert!(alias_deny_rule_2.destination_ports.is_empty());
    assert!(alias_deny_rule_2.protocols.is_empty());
    assert_eq!(
        alias_deny_rule_2.comment,
        Some("ACL 1 - test rule, ALIAS 2 - redis DENY".to_string())
    );
}

#[sqlx::test]
async fn test_no_allowed_users_ipv4(_: PgPoolOptions, options: PgConnectOptions) {
    set_test_license_business();
    let pool = setup_pool(options).await;

    // Create test location
    let location = WireguardNetwork {
        id: NoId,
        acl_enabled: true,
        address: vec![IpNetwork::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0).unwrap()],
        ..Default::default()
    };
    let location = location.save(&pool).await.unwrap();

    // create ACL rules
    let acl_rule_1 = AclRule {
        id: NoId,
        expires: None,
        enabled: true,
        state: RuleState::Applied,
        allow_all_users: true,
        use_manual_destination_settings: true,
        ..Default::default()
    }
    .save(&pool)
    .await
    .unwrap();
    let acl_rule_2 = AclRule {
        id: NoId,
        expires: None,
        enabled: true,
        state: RuleState::Applied,
        allow_all_users: true,
        use_manual_destination_settings: true,
        ..Default::default()
    }
    .save(&pool)
    .await
    .unwrap();

    // assign rules to location
    for rule in [&acl_rule_1, &acl_rule_2] {
        AclRuleNetwork::new(rule.id, location.id)
            .save(&pool)
            .await
            .unwrap();
    }

    let mut conn = pool.acquire().await.unwrap();
    let generated_firewall_rules = try_get_location_firewall_config(&location, &mut conn)
        .await
        .unwrap()
        .unwrap()
        .rules;

    // only deny rules are generated
    assert_eq!(generated_firewall_rules.len(), 2);
    for rule in generated_firewall_rules {
        assert_eq!(rule.verdict(), FirewallPolicy::Deny);
    }
}

#[sqlx::test]
async fn test_empty_manual_destination_only_acl(_: PgPoolOptions, options: PgConnectOptions) {
    set_test_license_business();
    let pool = setup_pool(options).await;

    let mut rng = thread_rng();

    // Create test locations with IPv4 and IPv6 addresses
    let location_ipv4 = WireguardNetwork {
        id: NoId,
        acl_enabled: true,
        address: vec![IpNetwork::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0).unwrap()],
        ..Default::default()
    }
    .save(&pool)
    .await
    .unwrap();
    let location_ipv6 = WireguardNetwork {
        id: NoId,
        acl_enabled: true,
        address: vec![IpNetwork::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), 0).unwrap()],
        ..Default::default()
    }
    .save(&pool)
    .await
    .unwrap();
    let location_ipv4_and_ipv6 = WireguardNetwork {
        id: NoId,
        acl_enabled: true,
        address: vec![
            IpNetwork::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0).unwrap(),
            IpNetwork::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), 0).unwrap(),
        ],
        ..Default::default()
    }
    .save(&pool)
    .await
    .unwrap();

    // Setup some test users and their devices
    let user_1: User<NoId> = rng.r#gen();
    let user_1 = user_1.save(&pool).await.unwrap();
    let user_2: User<NoId> = rng.r#gen();
    let user_2 = user_2.save(&pool).await.unwrap();

    for user in [&user_1, &user_2] {
        // Create 2 devices per user
        for device_num in 1..3 {
            let device = Device {
                id: NoId,
                name: format!("device-{}-{device_num}", user.id),
                user_id: user.id,
                device_type: DeviceType::User,
                description: None,
                wireguard_pubkey: Default::default(),
                created: Default::default(),
                configured: true,
            };
            let device = device.save(&pool).await.unwrap();

            // Add device to all locations' VPN networks
            let network_device = WireguardNetworkDevice {
                device_id: device.id,
                wireguard_network_id: location_ipv4.id,
                wireguard_ips: vec![IpAddr::V4(Ipv4Addr::new(
                    10,
                    0,
                    user.id as u8,
                    device_num as u8,
                ))],
                preshared_key: None,
                is_authorized: true,
                authorized_at: None,
            };
            network_device.insert(&pool).await.unwrap();
            let network_device = WireguardNetworkDevice {
                device_id: device.id,
                wireguard_network_id: location_ipv6.id,
                wireguard_ips: vec![IpAddr::V6(Ipv6Addr::new(
                    0xff00,
                    0,
                    0,
                    0,
                    0,
                    0,
                    user.id as u16,
                    device_num as u16,
                ))],
                preshared_key: None,
                is_authorized: true,
                authorized_at: None,
            };
            network_device.insert(&pool).await.unwrap();
            let network_device = WireguardNetworkDevice {
                device_id: device.id,
                wireguard_network_id: location_ipv4_and_ipv6.id,
                wireguard_ips: vec![
                    IpAddr::V4(Ipv4Addr::new(10, 0, user.id as u8, device_num as u8)),
                    IpAddr::V6(Ipv6Addr::new(
                        0xff00,
                        0,
                        0,
                        0,
                        0,
                        0,
                        user.id as u16,
                        device_num as u16,
                    )),
                ],
                preshared_key: None,
                is_authorized: true,
                authorized_at: None,
            };
            network_device.insert(&pool).await.unwrap();
        }
    }

    // create ACL rule without manually configured destination and no aliases
    let acl_rule = AclRule {
        id: NoId,
        name: "test rule".to_string(),
        expires: None,
        enabled: true,
        state: RuleState::Applied,
        addresses: Vec::new(),
        allow_all_users: true,
        ..Default::default()
    }
    .save(&pool)
    .await
    .unwrap();

    // assign rule to all locations
    for location in [&location_ipv4, &location_ipv6, &location_ipv4_and_ipv6] {
        AclRuleNetwork::new(acl_rule.id, location.id)
            .save(&pool)
            .await
            .unwrap();
    }

    let mut conn = pool.acquire().await.unwrap();

    // check generated rules for IPv4 only location
    let generated_firewall_rules_ipv4 = try_get_location_firewall_config(&location_ipv4, &mut conn)
        .await
        .unwrap()
        .unwrap()
        .rules;

    assert_eq!(generated_firewall_rules_ipv4.len(), 2);
    let expected_source_addrs_ipv4 = vec![
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
    let allow_rule_ipv4 = &generated_firewall_rules_ipv4[0];
    assert_eq!(allow_rule_ipv4.ip_version, i32::from(IpVersion::Ipv4));
    assert_eq!(allow_rule_ipv4.verdict, i32::from(FirewallPolicy::Allow));
    assert_eq!(allow_rule_ipv4.source_addrs, expected_source_addrs_ipv4);
    assert!(allow_rule_ipv4.destination_addrs.is_empty());

    let deny_rule_ipv4 = &generated_firewall_rules_ipv4[1];
    assert_eq!(deny_rule_ipv4.ip_version, i32::from(IpVersion::Ipv4));
    assert_eq!(deny_rule_ipv4.verdict, i32::from(FirewallPolicy::Deny));
    assert!(deny_rule_ipv4.source_addrs.is_empty());
    assert!(deny_rule_ipv4.destination_addrs.is_empty());

    // check generated rules for IPv6 only location
    let generated_firewall_rules_ipv6 = try_get_location_firewall_config(&location_ipv6, &mut conn)
        .await
        .unwrap()
        .unwrap()
        .rules;

    assert_eq!(generated_firewall_rules_ipv6.len(), 2);
    let expected_source_addrs_ipv6 = vec![
        IpAddress {
            address: Some(Address::IpRange(IpRange {
                start: "ff00::1:1".to_string(),
                end: "ff00::1:2".to_string(),
            })),
        },
        IpAddress {
            address: Some(Address::IpRange(IpRange {
                start: "ff00::2:1".to_string(),
                end: "ff00::2:2".to_string(),
            })),
        },
    ];
    let allow_rule_ipv6 = &generated_firewall_rules_ipv6[0];
    assert_eq!(allow_rule_ipv6.ip_version, i32::from(IpVersion::Ipv6));
    assert_eq!(allow_rule_ipv6.verdict, i32::from(FirewallPolicy::Allow));
    assert_eq!(allow_rule_ipv6.source_addrs, expected_source_addrs_ipv6);
    assert!(allow_rule_ipv6.destination_addrs.is_empty());

    let deny_rule_ipv6 = &generated_firewall_rules_ipv6[1];
    assert_eq!(deny_rule_ipv6.ip_version, i32::from(IpVersion::Ipv6));
    assert_eq!(deny_rule_ipv6.verdict, i32::from(FirewallPolicy::Deny));
    assert!(deny_rule_ipv6.source_addrs.is_empty());
    assert!(deny_rule_ipv6.destination_addrs.is_empty());

    // check generated rules for IPv4 and IPv6 location
    let generated_firewall_rules_ipv4_and_ipv6 =
        try_get_location_firewall_config(&location_ipv4_and_ipv6, &mut conn)
            .await
            .unwrap()
            .unwrap()
            .rules;

    assert_eq!(generated_firewall_rules_ipv4_and_ipv6.len(), 4);
    let allow_rule_ipv4 = &generated_firewall_rules_ipv4_and_ipv6[0];
    assert_eq!(allow_rule_ipv4.ip_version, i32::from(IpVersion::Ipv4));
    assert_eq!(allow_rule_ipv4.verdict, i32::from(FirewallPolicy::Allow));
    assert_eq!(allow_rule_ipv4.source_addrs, expected_source_addrs_ipv4);
    assert!(allow_rule_ipv4.destination_addrs.is_empty());

    let allow_rule_ipv6 = &generated_firewall_rules_ipv4_and_ipv6[1];
    assert_eq!(allow_rule_ipv6.ip_version, i32::from(IpVersion::Ipv6));
    assert_eq!(allow_rule_ipv6.verdict, i32::from(FirewallPolicy::Allow));
    assert_eq!(allow_rule_ipv6.source_addrs, expected_source_addrs_ipv6);
    assert!(allow_rule_ipv6.destination_addrs.is_empty());

    let deny_rule_ipv4 = &generated_firewall_rules_ipv4_and_ipv6[2];
    assert_eq!(deny_rule_ipv4.ip_version, i32::from(IpVersion::Ipv4));
    assert_eq!(deny_rule_ipv4.verdict, i32::from(FirewallPolicy::Deny));
    assert!(deny_rule_ipv4.source_addrs.is_empty());
    assert!(deny_rule_ipv4.destination_addrs.is_empty());

    let deny_rule_ipv6 = &generated_firewall_rules_ipv4_and_ipv6[3];
    assert_eq!(deny_rule_ipv6.ip_version, i32::from(IpVersion::Ipv6));
    assert_eq!(deny_rule_ipv6.verdict, i32::from(FirewallPolicy::Deny));
    assert!(deny_rule_ipv6.source_addrs.is_empty());
    assert!(deny_rule_ipv6.destination_addrs.is_empty());
}
