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
async fn test_unapplied_acl_rules_ipv4(_: PgPoolOptions, options: PgConnectOptions) {
    set_test_license_business();
    let pool = setup_pool(options).await;
    let mut rng = thread_rng();

    // Create test location
    let location = WireguardNetwork {
        id: NoId,
        acl_enabled: true,
        address: vec![IpNetwork::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0).unwrap()],
        ..Default::default()
    };
    let location = location.save(&pool).await.unwrap();

    // Setup some test users and their devices
    create_test_users_and_devices(&mut rng, &pool, vec![&location]).await;

    // create unapplied ACL rules
    let mut acl_rule_1 = AclRule {
        id: NoId,
        expires: None,
        enabled: true,
        allow_all_users: true,
        state: RuleState::New,
        use_manual_destination_settings: true,
        ..Default::default()
    }
    .save(&pool)
    .await
    .unwrap();
    let mut acl_rule_2 = AclRule {
        id: NoId,
        expires: None,
        enabled: true,
        allow_all_users: true,
        state: RuleState::Modified,
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

    // both rules were not applied
    assert_eq!(generated_firewall_rules.len(), 0);

    // make both rules applied
    acl_rule_1.state = RuleState::Applied;
    acl_rule_1.save(&pool).await.unwrap();

    acl_rule_2.state = RuleState::Applied;
    acl_rule_2.save(&pool).await.unwrap();

    let generated_firewall_rules = try_get_location_firewall_config(&location, &mut conn)
        .await
        .unwrap()
        .unwrap()
        .rules;
    assert_eq!(generated_firewall_rules.len(), 4);
}

#[sqlx::test]
async fn test_unapplied_acl_rules_ipv6(_: PgPoolOptions, options: PgConnectOptions) {
    set_test_license_business();
    let pool = setup_pool(options).await;
    let mut rng = thread_rng();

    // Create test location
    let location = WireguardNetwork {
        id: NoId,
        acl_enabled: true,
        address: vec![IpNetwork::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), 0).unwrap()],
        ..Default::default()
    };
    let location = location.save(&pool).await.unwrap();

    // Setup some test users and their devices
    create_test_users_and_devices(&mut rng, &pool, vec![&location]).await;

    // create unapplied ACL rules
    let mut acl_rule_1 = AclRule {
        id: NoId,
        expires: None,
        enabled: true,
        allow_all_users: true,
        state: RuleState::New,
        use_manual_destination_settings: true,
        ..Default::default()
    }
    .save(&pool)
    .await
    .unwrap();
    let mut acl_rule_2 = AclRule {
        id: NoId,
        expires: None,
        enabled: true,
        allow_all_users: true,
        state: RuleState::Modified,
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

    // both rules were not applied
    assert_eq!(generated_firewall_rules.len(), 0);

    // make both rules applied
    acl_rule_1.state = RuleState::Applied;
    acl_rule_1.save(&pool).await.unwrap();

    acl_rule_2.state = RuleState::Applied;
    acl_rule_2.save(&pool).await.unwrap();

    let generated_firewall_rules = try_get_location_firewall_config(&location, &mut conn)
        .await
        .unwrap()
        .unwrap()
        .rules;
    assert_eq!(generated_firewall_rules.len(), 4);
}

#[sqlx::test]
async fn test_unapplied_acl_rules_ipv4_and_ipv6(_: PgPoolOptions, options: PgConnectOptions) {
    set_test_license_business();
    let pool = setup_pool(options).await;
    let mut rng = thread_rng();

    // Create test location
    let location = WireguardNetwork {
        id: NoId,
        acl_enabled: true,
        address: vec![
            IpNetwork::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0).unwrap(),
            IpNetwork::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), 0).unwrap(),
        ],
        ..Default::default()
    };
    let location = location.save(&pool).await.unwrap();

    // Setup some test users and their devices
    create_test_users_and_devices(&mut rng, &pool, vec![&location]).await;

    // create unapplied ACL rules
    let mut acl_rule_1 = AclRule {
        id: NoId,
        expires: None,
        enabled: true,
        allow_all_users: true,
        state: RuleState::New,
        use_manual_destination_settings: true,
        ..Default::default()
    }
    .save(&pool)
    .await
    .unwrap();
    let mut acl_rule_2 = AclRule {
        id: NoId,
        expires: None,
        enabled: true,
        allow_all_users: true,
        state: RuleState::Modified,
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

    // both rules were not applied
    assert_eq!(generated_firewall_rules.len(), 0);

    // make both rules applied
    acl_rule_1.state = RuleState::Applied;
    acl_rule_1.save(&pool).await.unwrap();

    acl_rule_2.state = RuleState::Applied;
    acl_rule_2.save(&pool).await.unwrap();

    let generated_firewall_rules = try_get_location_firewall_config(&location, &mut conn)
        .await
        .unwrap()
        .unwrap()
        .rules;
    assert_eq!(generated_firewall_rules.len(), 8);
}
