use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use chrono::{DateTime, NaiveDateTime};
use defguard_common::db::{NoId, models::WireguardNetwork, setup_pool};
use ipnetwork::IpNetwork;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

use crate::enterprise::{
    db::models::acl::{AclRule, AclRuleNetwork, RuleState},
    firewall::{tests::set_test_license_business, try_get_location_firewall_config},
};

#[sqlx::test]
async fn test_expired_acl_rules_ipv4(_: PgPoolOptions, options: PgConnectOptions) {
    set_test_license_business();
    let pool = setup_pool(options).await;
    // Create test location
    let location = WireguardNetwork {
        id: NoId,
        acl_enabled: true,
        ..Default::default()
    };
    let location = location.save(&pool).await.unwrap();

    // create expired ACL rules
    let mut acl_rule_1 = AclRule {
        id: NoId,
        expires: Some(DateTime::UNIX_EPOCH.naive_utc()),
        enabled: true,
        state: RuleState::Applied,
        ..Default::default()
    }
    .save(&pool)
    .await
    .unwrap();
    let mut acl_rule_2 = AclRule {
        id: NoId,
        expires: Some(DateTime::UNIX_EPOCH.naive_utc()),
        enabled: true,
        state: RuleState::Applied,
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

    // both rules were expired
    assert_eq!(generated_firewall_rules.len(), 0);

    // make both rules not expired
    acl_rule_1.expires = None;
    acl_rule_1.save(&pool).await.unwrap();

    acl_rule_2.expires = Some(NaiveDateTime::MAX);
    acl_rule_2.save(&pool).await.unwrap();

    let generated_firewall_rules = try_get_location_firewall_config(&location, &mut conn)
        .await
        .unwrap()
        .unwrap()
        .rules;
    assert_eq!(generated_firewall_rules.len(), 2);
}

#[sqlx::test]
async fn test_expired_acl_rules_ipv6(_: PgPoolOptions, options: PgConnectOptions) {
    set_test_license_business();
    let pool = setup_pool(options).await;
    // Create test location
    let location = WireguardNetwork {
        id: NoId,
        acl_enabled: true,
        address: vec![IpNetwork::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), 0).unwrap()],
        ..Default::default()
    };
    let location = location.save(&pool).await.unwrap();

    // create expired ACL rules
    let mut acl_rule_1 = AclRule {
        id: NoId,
        expires: Some(DateTime::UNIX_EPOCH.naive_utc()),
        enabled: true,
        state: RuleState::Applied,
        ..Default::default()
    }
    .save(&pool)
    .await
    .unwrap();
    let mut acl_rule_2 = AclRule {
        id: NoId,
        expires: Some(DateTime::UNIX_EPOCH.naive_utc()),
        enabled: true,
        state: RuleState::Applied,
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

    // both rules were expired
    assert_eq!(generated_firewall_rules.len(), 0);

    // make both rules not expired
    acl_rule_1.expires = None;
    acl_rule_1.save(&pool).await.unwrap();

    acl_rule_2.expires = Some(NaiveDateTime::MAX);
    acl_rule_2.save(&pool).await.unwrap();

    let generated_firewall_rules = try_get_location_firewall_config(&location, &mut conn)
        .await
        .unwrap()
        .unwrap()
        .rules;
    assert_eq!(generated_firewall_rules.len(), 2);
}

#[sqlx::test]
async fn test_expired_acl_rules_ipv4_and_ipv6(_: PgPoolOptions, options: PgConnectOptions) {
    set_test_license_business();
    let pool = setup_pool(options).await;
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

    // create expired ACL rules
    let mut acl_rule_1 = AclRule {
        id: NoId,
        expires: Some(DateTime::UNIX_EPOCH.naive_utc()),
        enabled: true,
        state: RuleState::Applied,
        ..Default::default()
    }
    .save(&pool)
    .await
    .unwrap();
    let mut acl_rule_2 = AclRule {
        id: NoId,
        expires: Some(DateTime::UNIX_EPOCH.naive_utc()),
        enabled: true,
        state: RuleState::Applied,
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

    // both rules were expired
    assert_eq!(generated_firewall_rules.len(), 0);

    // make both rules not expired
    acl_rule_1.expires = None;
    acl_rule_1.save(&pool).await.unwrap();

    acl_rule_2.expires = Some(NaiveDateTime::MAX);
    acl_rule_2.save(&pool).await.unwrap();

    let generated_firewall_rules = try_get_location_firewall_config(&location, &mut conn)
        .await
        .unwrap()
        .unwrap()
        .rules;
    assert_eq!(generated_firewall_rules.len(), 4);
}
