use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use defguard_common::db::{NoId, models::WireguardNetwork, setup_pool};
use ipnetwork::IpNetwork;
use rand::thread_rng;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

use crate::enterprise::{
    db::models::acl::{AclRule, AclRuleNetwork, RuleState},
    firewall::{
        tests::{create_test_users_and_devices, set_test_license_business},
        try_get_location_firewall_config,
    },
};

#[sqlx::test]
async fn test_acl_rules_all_locations_ipv4(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let mut rng = thread_rng();
    set_test_license_business();

    // Create test location
    let location_1 = WireguardNetwork {
        id: NoId,
        acl_enabled: true,
        address: vec![IpNetwork::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0).unwrap()],
        ..Default::default()
    };
    let location_1 = location_1.save(&pool).await.unwrap();

    // Create another test location
    let location_2 = WireguardNetwork {
        id: NoId,
        acl_enabled: true,
        address: vec![IpNetwork::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0).unwrap()],
        ..Default::default()
    };
    let location_2 = location_2.save(&pool).await.unwrap();

    // Setup some test users and their devices
    create_test_users_and_devices(&mut rng, &pool, vec![&location_1, &location_2]).await;

    // create ACL rules
    let acl_rule_1 = AclRule {
        id: NoId,
        expires: None,
        enabled: true,
        allow_all_users: true,
        state: RuleState::Applied,
        addresses: vec!["192.168.1.0/24".parse().unwrap()],
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
        all_locations: true,
        allow_all_users: true,
        state: RuleState::Applied,
        addresses: vec!["192.168.2.0/24".parse().unwrap()],
        use_manual_destination_settings: true,
        ..Default::default()
    }
    .save(&pool)
    .await
    .unwrap();

    let _acl_rule_3 = AclRule {
        id: NoId,
        expires: None,
        enabled: true,
        all_locations: true,
        allow_all_users: true,
        state: RuleState::Applied,
        addresses: vec!["192.168.3.0/24".parse().unwrap()],
        use_manual_destination_settings: true,
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

    // all rules were assigned to this location
    assert_eq!(generated_firewall_rules.len(), 6);

    let generated_firewall_rules = try_get_location_firewall_config(&location_2, &mut conn)
        .await
        .unwrap()
        .unwrap()
        .rules;

    // rule with `all_networks` enabled was used for this location
    assert_eq!(generated_firewall_rules.len(), 4);
}

#[sqlx::test]
async fn test_acl_rules_all_locations_ipv6(_: PgPoolOptions, options: PgConnectOptions) {
    set_test_license_business();
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
    create_test_users_and_devices(&mut rng, &pool, vec![&location_1, &location_2]).await;

    // create ACL rules
    let acl_rule_1 = AclRule {
        id: NoId,
        expires: None,
        enabled: true,
        allow_all_users: true,
        state: RuleState::Applied,
        use_manual_destination_settings: true,
        addresses: vec!["fc00::0/112".parse().unwrap()],
        ..Default::default()
    }
    .save(&pool)
    .await
    .unwrap();

    let acl_rule_2 = AclRule {
        id: NoId,
        expires: None,
        enabled: true,
        allow_all_users: true,
        all_locations: true,
        state: RuleState::Applied,
        use_manual_destination_settings: true,
        addresses: vec!["fb00::0/112".parse().unwrap()],
        ..Default::default()
    }
    .save(&pool)
    .await
    .unwrap();

    let _acl_rule_3 = AclRule {
        id: NoId,
        expires: None,
        enabled: true,
        all_locations: true,
        allow_all_users: true,
        state: RuleState::Applied,
        use_manual_destination_settings: true,
        addresses: vec!["fa00::0/112".parse().unwrap()],
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
    assert_eq!(generated_firewall_rules.len(), 6);

    let generated_firewall_rules = try_get_location_firewall_config(&location_2, &mut conn)
        .await
        .unwrap()
        .unwrap()
        .rules;

    // rule with `all_networks` enabled was used for this location
    assert_eq!(generated_firewall_rules.len(), 4);
}

#[sqlx::test]
async fn test_acl_rules_all_locations_ipv4_and_ipv6(_: PgPoolOptions, options: PgConnectOptions) {
    set_test_license_business();
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
    create_test_users_and_devices(&mut rng, &pool, vec![&location_1, &location_2]).await;

    // create ACL rules
    let acl_rule_1 = AclRule {
        id: NoId,
        expires: None,
        enabled: true,
        allow_all_users: true,
        state: RuleState::Applied,
        use_manual_destination_settings: true,
        addresses: vec![
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
        all_locations: true,
        allow_all_users: true,
        state: RuleState::Applied,
        use_manual_destination_settings: true,
        addresses: vec![
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
        all_locations: true,
        allow_all_users: true,
        state: RuleState::Applied,
        use_manual_destination_settings: true,
        addresses: vec![
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
