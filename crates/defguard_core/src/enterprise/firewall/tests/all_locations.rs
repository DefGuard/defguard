use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use defguard_common::db::{
    NoId,
    models::{Device, DeviceType, User, WireguardNetwork, device::WireguardNetworkDevice},
    setup_pool,
};
use ipnetwork::IpNetwork;
use rand::{Rng, thread_rng};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

use crate::enterprise::{
    db::models::acl::{AclRule, AclRuleNetwork, RuleState},
    firewall::try_get_location_firewall_config,
};

#[sqlx::test]
async fn test_acl_rules_all_locations_ipv4(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let mut rng = thread_rng();

    // Create test location
    let location_1 = WireguardNetwork {
        id: NoId,
        acl_enabled: true,
        ..Default::default()
    };
    let location_1 = location_1.save(&pool).await.unwrap();

    // Create another test location
    let location_2 = WireguardNetwork {
        id: NoId,
        acl_enabled: true,
        ..Default::default()
    };
    let location_2 = location_2.save(&pool).await.unwrap();
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
                wireguard_network_id: location_1.id,
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
                wireguard_network_id: location_2.id,
                wireguard_ips: vec![IpAddr::V4(Ipv4Addr::new(
                    10,
                    10,
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

    // create ACL rules
    let acl_rule_1 = AclRule {
        id: NoId,
        expires: None,
        enabled: true,
        state: RuleState::Applied,
        destination: vec!["192.168.1.0/24".parse().unwrap()],
        manual_settings: true,
        ..Default::default()
    }
    .save(&pool)
    .await
    .unwrap();

    let acl_rule_2 = AclRule {
        id: NoId,
        expires: None,
        enabled: true,
        all_networks: true,
        state: RuleState::Applied,
        manual_settings: false,
        ..Default::default()
    }
    .save(&pool)
    .await
    .unwrap();

    let _acl_rule_3 = AclRule {
        id: NoId,
        expires: None,
        enabled: true,
        all_networks: true,
        allow_all_users: true,
        state: RuleState::Applied,
        manual_settings: false,
        ..Default::default()
    }
    .save(&pool)
    .await
    .unwrap();

    // assign rules to locations
    for rule in [&acl_rule_1, &acl_rule_2] {
        AclRuleNetwork::new(rule.id, location_1.id)
            .save(&pool)
            .await
            .unwrap();
    }
    for rule in [&acl_rule_2] {
        AclRuleNetwork::new(rule.id, location_2.id)
            .save(&pool)
            .await
            .unwrap();
    }

    let mut conn = pool.acquire().await.unwrap();
    let generated_firewall_rules = try_get_location_firewall_config(&location_1, &mut conn)
        .await
        .unwrap()
        .unwrap()
        .rules;

    // both rules were assigned to this location
    assert_eq!(generated_firewall_rules.len(), 4);

    let generated_firewall_rules = try_get_location_firewall_config(&location_2, &mut conn)
        .await
        .unwrap()
        .unwrap()
        .rules;

    // rule with `all_networks` enabled was used for this location
    assert_eq!(generated_firewall_rules.len(), 3);
}

#[sqlx::test]
async fn test_acl_rules_all_locations_ipv6(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let mut rng = thread_rng();

    // Create test location
    let location_1 = WireguardNetwork {
        id: NoId,
        acl_enabled: true,
        address: vec![IpNetwork::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), 0).unwrap()],
        ..Default::default()
    };
    let location_1 = location_1.save(&pool).await.unwrap();

    // Create another test location
    let location_2 = WireguardNetwork {
        id: NoId,
        acl_enabled: true,
        address: vec![IpNetwork::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), 0).unwrap()],
        ..Default::default()
    };
    let location_2 = location_2.save(&pool).await.unwrap();

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
                wireguard_network_id: location_1.id,
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
                wireguard_network_id: location_2.id,
                wireguard_ips: vec![IpAddr::V6(Ipv6Addr::new(
                    0xff00,
                    0,
                    0,
                    0,
                    10,
                    10,
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

    // create ACL rules
    let acl_rule_1 = AclRule {
        id: NoId,
        expires: None,
        enabled: true,
        state: RuleState::Applied,
        destination: vec!["fc00::0/112".parse().unwrap()],
        ..Default::default()
    }
    .save(&pool)
    .await
    .unwrap();

    let acl_rule_2 = AclRule {
        id: NoId,
        expires: None,
        enabled: true,
        all_networks: true,
        state: RuleState::Applied,
        ..Default::default()
    }
    .save(&pool)
    .await
    .unwrap();

    let _acl_rule_3 = AclRule {
        id: NoId,
        expires: None,
        enabled: true,
        all_networks: true,
        allow_all_users: true,
        state: RuleState::Applied,
        ..Default::default()
    }
    .save(&pool)
    .await
    .unwrap();

    // assign rules to locations
    for rule in [&acl_rule_1, &acl_rule_2] {
        AclRuleNetwork::new(rule.id, location_1.id)
            .save(&pool)
            .await
            .unwrap();
    }
    for rule in [&acl_rule_2] {
        AclRuleNetwork::new(rule.id, location_2.id)
            .save(&pool)
            .await
            .unwrap();
    }

    let mut conn = pool.acquire().await.unwrap();
    let generated_firewall_rules = try_get_location_firewall_config(&location_1, &mut conn)
        .await
        .unwrap()
        .unwrap()
        .rules;

    // both rules were assigned to this location
    assert_eq!(generated_firewall_rules.len(), 4);

    let generated_firewall_rules = try_get_location_firewall_config(&location_2, &mut conn)
        .await
        .unwrap()
        .unwrap()
        .rules;

    // rule with `all_networks` enabled was used for this location
    assert_eq!(generated_firewall_rules.len(), 3);
}

#[sqlx::test]
async fn test_acl_rules_all_locations_ipv4_and_ipv6(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let mut rng = thread_rng();

    // Create test location
    let location_1 = WireguardNetwork {
        id: NoId,
        acl_enabled: true,
        address: vec![
            IpNetwork::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0).unwrap(),
            IpNetwork::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), 0).unwrap(),
        ],
        ..Default::default()
    };
    let location_1 = location_1.save(&pool).await.unwrap();

    // Create another test location
    let location_2 = WireguardNetwork {
        id: NoId,
        acl_enabled: true,
        address: vec![
            IpNetwork::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0).unwrap(),
            IpNetwork::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), 0).unwrap(),
        ],
        ..Default::default()
    };
    let location_2 = location_2.save(&pool).await.unwrap();
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
                wireguard_network_id: location_1.id,
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
            let network_device = WireguardNetworkDevice {
                device_id: device.id,
                wireguard_network_id: location_2.id,
                wireguard_ips: vec![
                    IpAddr::V4(Ipv4Addr::new(10, 10, user.id as u8, device_num as u8)),
                    IpAddr::V6(Ipv6Addr::new(
                        0xff00,
                        0,
                        0,
                        0,
                        10,
                        10,
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

    // create ACL rules
    let acl_rule_1 = AclRule {
        id: NoId,
        expires: None,
        enabled: true,
        allow_all_users: true,
        state: RuleState::Applied,
        manual_settings: true,
        destination: vec![
            "192.168.1.0/24".parse().unwrap(),
            "fc00::0/112".parse().unwrap(),
        ],
        ..Default::default()
    }
    .save(&pool)
    .await
    .unwrap();

    let acl_rule_2 = AclRule {
        id: NoId,
        expires: None,
        enabled: true,
        all_networks: true,
        allow_all_users: true,
        state: RuleState::Applied,
        manual_settings: true,
        destination: vec![
            "192.168.2.0/24".parse().unwrap(),
            "fb00::0/112".parse().unwrap(),
        ],
        ..Default::default()
    }
    .save(&pool)
    .await
    .unwrap();

    let _acl_rule_3 = AclRule {
        id: NoId,
        expires: None,
        enabled: true,
        all_networks: true,
        allow_all_users: true,
        state: RuleState::Applied,
        manual_settings: true,
        destination: vec![
            "192.168.3.0/24".parse().unwrap(),
            "fa00::0/112".parse().unwrap(),
        ],
        ..Default::default()
    }
    .save(&pool)
    .await
    .unwrap();

    // assign rules to locations
    for rule in [&acl_rule_1, &acl_rule_2] {
        AclRuleNetwork::new(rule.id, location_1.id)
            .save(&pool)
            .await
            .unwrap();
    }
    for rule in [&acl_rule_2] {
        AclRuleNetwork::new(rule.id, location_2.id)
            .save(&pool)
            .await
            .unwrap();
    }

    let mut conn = pool.acquire().await.unwrap();
    let generated_firewall_rules = try_get_location_firewall_config(&location_1, &mut conn)
        .await
        .unwrap()
        .unwrap()
        .rules;

    // all rules were used to this location
    assert_eq!(generated_firewall_rules.len(), 12);

    let generated_firewall_rules = try_get_location_firewall_config(&location_2, &mut conn)
        .await
        .unwrap()
        .unwrap()
        .rules;

    // rule with `all_networks` enabled was also used for this location
    assert_eq!(generated_firewall_rules.len(), 8);
}
