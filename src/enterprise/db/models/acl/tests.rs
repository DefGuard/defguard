use std::ops::Bound;

use rand::{thread_rng, Rng};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

use super::*;
use crate::{db::setup_pool, handlers::wireguard::parse_address_list};

#[sqlx::test]
async fn test_alias(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let destination = parse_address_list("10.0.0.1, 10.1.0.0/16");
    let ports = vec![
        PgRange {
            start: Bound::Included(10),
            end: Bound::Excluded(21),
        },
        PgRange {
            start: Bound::Included(100),
            end: Bound::Excluded(201),
        },
    ];
    let alias = AclAlias::new(
        "alias",
        AliasState::Applied,
        AliasKind::Destination,
        destination.clone(),
        ports.clone(),
        vec![20, 30],
    )
    .save(&pool)
    .await
    .unwrap();

    assert_eq!(alias.id, 1);

    let retrieved = AclAlias::find_by_id(&pool, 1).await.unwrap().unwrap();

    assert_eq!(retrieved.id, 1);
    assert_eq!(retrieved.destination, destination);
    assert_eq!(retrieved.ports, ports);
}

#[sqlx::test]
async fn test_allow_conflicting_sources(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    // create the rule
    let rule = AclRule {
        id: NoId,
        parent_id: Default::default(),
        state: Default::default(),
        name: "rule".to_string(),
        enabled: true,
        allow_all_users: false,
        deny_all_users: false,
        allow_all_network_devices: false,
        deny_all_network_devices: false,
        all_networks: false,
        destination: Vec::new(),
        ports: Vec::new(),
        protocols: Vec::new(),
        expires: None,
    }
    .save(&pool)
    .await
    .unwrap();

    // user
    let user = User::new("user1", None, "", "", "u1@mail.com", None)
        .save(&pool)
        .await
        .unwrap();
    let _ = AclRuleUser {
        id: NoId,
        rule_id: rule.id,
        user_id: user.id,
        allow: true,
    }
    .save(&pool)
    .await
    .unwrap();
    let result = AclRuleUser {
        id: NoId,
        rule_id: rule.id,
        user_id: user.id,
        allow: false,
    }
    .save(&pool)
    .await;
    assert!(result.is_ok());

    // group
    let group = Group::new("group1").save(&pool).await.unwrap();
    let _ = AclRuleGroup {
        id: NoId,
        rule_id: rule.id,
        group_id: group.id,
        allow: true,
    }
    .save(&pool)
    .await
    .unwrap();
    let result = AclRuleGroup {
        id: NoId,
        rule_id: rule.id,
        group_id: group.id,
        allow: false,
    }
    .save(&pool)
    .await;
    assert!(result.is_ok());

    // device
    let device = Device::new(
        "device1".to_string(),
        String::new(),
        1,
        DeviceType::Network,
        None,
        true,
    )
    .save(&pool)
    .await
    .unwrap();
    let _ = AclRuleDevice {
        id: NoId,
        rule_id: rule.id,
        device_id: device.id,
        allow: true,
    }
    .save(&pool)
    .await
    .unwrap();
    let result = AclRuleDevice {
        id: NoId,
        rule_id: rule.id,
        device_id: device.id,
        allow: false,
    }
    .save(&pool)
    .await;
    assert!(result.is_ok());
}

#[sqlx::test]
async fn test_rule_relations(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    // create the rule
    let mut rule = AclRule {
        id: NoId,
        parent_id: Default::default(),
        state: Default::default(),
        name: "rule".to_string(),
        enabled: true,
        allow_all_users: false,
        deny_all_users: false,
        allow_all_network_devices: false,
        deny_all_network_devices: false,
        all_networks: false,
        destination: Vec::new(),
        ports: Vec::new(),
        protocols: Vec::new(),
        expires: None,
    }
    .save(&pool)
    .await
    .unwrap();

    // create 2 networks
    let network1 = WireguardNetwork::new(
        "network1".to_string(),
        Vec::new(),
        1000,
        "endpoint1".to_string(),
        None,
        Vec::new(),
        false,
        100,
        100,
        false,
        false,
    )
    .unwrap()
    .save(&pool)
    .await
    .unwrap();
    let _network2 = WireguardNetwork::new(
        "network2".to_string(),
        Vec::new(),
        2000,
        "endpoint2".to_string(),
        None,
        Vec::new(),
        false,
        200,
        200,
        false,
        false,
    )
    .unwrap()
    .save(&pool)
    .await
    .unwrap();

    // rule only applied to network1
    let _rn = AclRuleNetwork {
        id: NoId,
        rule_id: rule.id,
        network_id: network1.id,
    }
    .save(&pool)
    .await
    .unwrap();

    // create 2 users
    let mut user1 = User::new("user1", None, "", "", "u1@mail.com", None)
        .save(&pool)
        .await
        .unwrap();
    let user2 = User::new("user2", None, "", "", "u2@mail.com", None)
        .save(&pool)
        .await
        .unwrap();

    // user1 allowed
    let _ru1 = AclRuleUser {
        id: NoId,
        rule_id: rule.id,
        user_id: user1.id,
        allow: true,
    }
    .save(&pool)
    .await
    .unwrap();

    // user2 denied
    let mut ru2 = AclRuleUser {
        id: NoId,
        rule_id: rule.id,
        user_id: user2.id,
        allow: false,
    }
    .save(&pool)
    .await
    .unwrap();

    // create 2 grups
    let group1 = Group::new("group1").save(&pool).await.unwrap();
    let group2 = Group::new("group2").save(&pool).await.unwrap();

    // group1 allowed
    let _rg = AclRuleGroup {
        id: NoId,
        rule_id: rule.id,
        group_id: group1.id,
        allow: true,
    }
    .save(&pool)
    .await
    .unwrap();

    // group2 denied
    let _rg = AclRuleGroup {
        id: NoId,
        rule_id: rule.id,
        group_id: group2.id,
        allow: false,
    }
    .save(&pool)
    .await
    .unwrap();

    // create 2 devices
    let device1 = Device::new(
        "device1".to_string(),
        String::new(),
        1,
        DeviceType::Network,
        None,
        true,
    )
    .save(&pool)
    .await
    .unwrap();
    let device2 = Device::new(
        "device2".to_string(),
        String::new(),
        1,
        DeviceType::Network,
        None,
        true,
    )
    .save(&pool)
    .await
    .unwrap();

    // device1 allowed
    let _rd = AclRuleDevice {
        id: NoId,
        rule_id: rule.id,
        device_id: device1.id,
        allow: true,
    }
    .save(&pool)
    .await
    .unwrap();

    // device2 denied
    let _rd = AclRuleDevice {
        id: NoId,
        rule_id: rule.id,
        device_id: device2.id,
        allow: false,
    }
    .save(&pool)
    .await
    .unwrap();

    // create 2 aliases
    let alias1 = AclAlias::new(
        "alias1",
        AliasState::Applied,
        AliasKind::Destination,
        Vec::new(),
        Vec::new(),
        Vec::new(),
    )
    .save(&pool)
    .await
    .unwrap();
    let _alias2 = AclAlias::new(
        "alias2",
        AliasState::Applied,
        AliasKind::Destination,
        Vec::new(),
        Vec::new(),
        Vec::new(),
    )
    .save(&pool)
    .await
    .unwrap();

    // only alias1 applies to the rule
    let _ra = AclRuleAlias {
        id: NoId,
        rule_id: rule.id,
        alias_id: alias1.id,
    }
    .save(&pool)
    .await
    .unwrap();

    let mut conn = pool.acquire().await.unwrap();

    // convert to [`AclRuleInfo`] and verify results
    let info = rule.to_info(&mut conn).await.unwrap();

    assert_eq!(info.aliases.len(), 1);
    assert_eq!(info.aliases[0], alias1);

    assert_eq!(info.allowed_users.len(), 1);
    assert_eq!(info.allowed_users[0], user1);

    assert_eq!(info.denied_users.len(), 1);
    assert_eq!(info.denied_users[0], user2);

    assert_eq!(info.allowed_groups.len(), 1);
    assert_eq!(info.allowed_groups[0], group1);

    assert_eq!(info.denied_groups.len(), 1);
    assert_eq!(info.denied_groups[0], group2);

    assert_eq!(info.allowed_devices.len(), 1);
    assert_eq!(info.allowed_devices[0].id, device1.id); // db modifies datetime precision

    assert_eq!(info.denied_devices.len(), 1);
    assert_eq!(info.denied_devices[0].id, device2.id); // db modifies datetime precision

    assert_eq!(info.networks.len(), 1);
    assert_eq!(info.networks[0], network1);

    // test all_networks flag
    rule.all_networks = true;
    rule.save(&pool).await.unwrap();
    assert_eq!(rule.get_networks(&pool).await.unwrap().len(), 2);

    // test allowed/denied users
    let allowed_users = rule.get_users(&pool, true).await.unwrap();
    let denied_users = rule.get_users(&pool, false).await.unwrap();
    assert_eq!(allowed_users.len(), 1);
    assert_eq!(allowed_users[0], user1);
    assert_eq!(denied_users.len(), 1);
    assert_eq!(denied_users[0], user2);

    // test `allow_all_users` flag
    rule.allow_all_users = true;
    rule.deny_all_users = false;
    rule.save(&pool).await.unwrap();
    assert_eq!(rule.get_users(&pool, true).await.unwrap().len(), 1);
    assert_eq!(rule.get_users(&pool, false).await.unwrap().len(), 1);

    // test `deny_all_users` flag
    rule.allow_all_users = false;
    rule.deny_all_users = true;
    rule.save(&pool).await.unwrap();
    assert_eq!(rule.get_users(&pool, true).await.unwrap().len(), 1);
    assert_eq!(rule.get_users(&pool, false).await.unwrap().len(), 1);

    // test both flags
    rule.allow_all_users = true;
    rule.deny_all_users = true;
    rule.save(&pool).await.unwrap();
    assert_eq!(rule.get_users(&pool, true).await.unwrap().len(), 1);
    assert_eq!(rule.get_users(&pool, false).await.unwrap().len(), 1);

    // deactivate user1
    user1.is_active = false;
    user1.save(&pool).await.unwrap();

    // ensure only active users are allowed when `allow_all_users = true`
    rule.allow_all_users = true;
    rule.deny_all_users = false;
    rule.save(&pool).await.unwrap();

    let allowed_users = rule.get_users(&pool, true).await.unwrap();
    let denied_users = rule.get_users(&pool, false).await.unwrap();
    assert_eq!(allowed_users.len(), 0);
    assert_eq!(denied_users.len(), 1);

    // ensure only active users are allowed when `allow_all_users = false`
    rule.allow_all_users = false;
    rule.deny_all_users = false;
    rule.save(&pool).await.unwrap();
    ru2.allow = true; // allow user2
    ru2.save(&pool).await.unwrap();
    let allowed_users = rule.get_users(&pool, true).await.unwrap();
    let denied_users = rule.get_users(&pool, false).await.unwrap();
    assert_eq!(allowed_users.len(), 1);
    assert_eq!(allowed_users[0], user2);
    assert_eq!(denied_users.len(), 0);

    // ensure only active users are denied when `deny_all_users = true`
    rule.allow_all_users = false;
    rule.deny_all_users = true;
    rule.save(&pool).await.unwrap();

    let allowed_users = rule.get_users(&pool, true).await.unwrap();
    let denied_users = rule.get_users(&pool, false).await.unwrap();
    assert_eq!(allowed_users.len(), 1);
    assert_eq!(denied_users.len(), 0);

    // ensure only active users are denied when `deny_all_users = false`
    rule.allow_all_users = false;
    rule.deny_all_users = false;
    rule.save(&pool).await.unwrap();
    ru2.allow = false; // deny user2
    ru2.save(&pool).await.unwrap();
    let allowed_users = rule.get_users(&pool, true).await.unwrap();
    let denied_users = rule.get_users(&pool, false).await.unwrap();
    assert_eq!(allowed_users.len(), 0);
    assert_eq!(denied_users.len(), 1);
    assert_eq!(denied_users[0], user2);
}

#[sqlx::test]
async fn test_all_allowed_users(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let mut rng = thread_rng();

    // Create test users
    let user_1: User<NoId> = rng.gen();
    let user_1 = user_1.save(&pool).await.unwrap();
    let user_2: User<NoId> = rng.gen();
    let user_2 = user_2.save(&pool).await.unwrap();
    let user_3: User<NoId> = rng.gen();
    let user_3 = user_3.save(&pool).await.unwrap();
    // inactive user
    let mut user_4: User<NoId> = rng.gen();
    user_4.is_active = false;
    let user_4 = user_4.save(&pool).await.unwrap();

    // Create test groups
    let group_1 = Group {
        id: NoId,
        name: "group_1".into(),
        ..Default::default()
    }
    .save(&pool)
    .await
    .unwrap();
    let group_2 = Group {
        id: NoId,
        name: "group_2".into(),
        ..Default::default()
    }
    .save(&pool)
    .await
    .unwrap();

    // Assign users to groups:
    // Group 1: users 1,2
    // Group 2: user 3,4
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

    // Create ACL rule
    let rule = AclRule {
        id: NoId,
        name: "test_rule".to_string(),
        allow_all_users: false,
        deny_all_users: false,
        allow_all_network_devices: false,
        deny_all_network_devices: false,
        all_networks: false,
        destination: Vec::new(),
        ports: Vec::new(),
        protocols: Vec::new(),
        expires: None,
        enabled: true,
        parent_id: None,
        state: RuleState::Applied,
    }
    .save(&pool)
    .await
    .unwrap();

    // Allow user_1 explicitly and group_2
    AclRuleUser {
        id: NoId,
        rule_id: rule.id,
        user_id: user_1.id,
        allow: true,
    }
    .save(&pool)
    .await
    .unwrap();

    AclRuleGroup {
        id: NoId,
        rule_id: rule.id,
        group_id: group_2.id,
        allow: true,
    }
    .save(&pool)
    .await
    .unwrap();

    // Get rule info
    let mut conn = pool.acquire().await.unwrap();
    let rule_info = rule.to_info(&mut conn).await.unwrap();
    assert_eq!(rule_info.allowed_users.len(), 1);
    assert_eq!(rule_info.allowed_groups.len(), 1);

    // Get all allowed users
    let allowed_users = rule_info.get_all_allowed_users(&pool).await.unwrap();

    // Should contain user1 (explicit) and user3 (from group2), but not inactive user_4
    assert_eq!(allowed_users.len(), 2);
    assert!(allowed_users.iter().any(|u| u.id == user_1.id));
    assert!(allowed_users.iter().any(|u| u.id == user_3.id));
    assert!(!allowed_users.iter().any(|u| u.id == user_4.id));
}

#[sqlx::test]
async fn test_all_denied_users(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let mut rng = thread_rng();

    // Create test users
    let user_1: User<NoId> = rng.gen();
    let user_1 = user_1.save(&pool).await.unwrap();
    let user_2: User<NoId> = rng.gen();
    let user_2 = user_2.save(&pool).await.unwrap();
    let user_3: User<NoId> = rng.gen();
    let user_3 = user_3.save(&pool).await.unwrap();
    // inactive user
    let mut user_4: User<NoId> = rng.gen();
    user_4.is_active = false;
    let user_4 = user_4.save(&pool).await.unwrap();

    // Create test groups
    let group_1 = Group {
        id: NoId,
        name: "group_1".into(),
        ..Default::default()
    }
    .save(&pool)
    .await
    .unwrap();
    let group_2 = Group {
        id: NoId,
        name: "group_2".into(),
        ..Default::default()
    }
    .save(&pool)
    .await
    .unwrap();

    // Assign users to groups:
    // Group 1: users 2,3,4
    // Group 2: user 1
    let group_assignments = vec![
        (&group_1, vec![&user_2, &user_3, &user_4]),
        (&group_2, vec![&user_1]),
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

    // Create ACL rule
    let rule = AclRule {
        id: NoId,
        name: "test_rule".to_string(),
        allow_all_users: false,
        deny_all_users: false,
        allow_all_network_devices: false,
        deny_all_network_devices: false,
        all_networks: false,
        destination: Vec::new(),
        ports: Vec::new(),
        protocols: Vec::new(),
        expires: None,
        enabled: true,
        parent_id: None,
        state: RuleState::Applied,
    }
    .save(&pool)
    .await
    .unwrap();

    // Deny user_1, user_3 explicitly and group_1
    AclRuleUser {
        id: NoId,
        rule_id: rule.id,
        user_id: user_1.id,
        allow: false,
    }
    .save(&pool)
    .await
    .unwrap();
    AclRuleUser {
        id: NoId,
        rule_id: rule.id,
        user_id: user_3.id,
        allow: false,
    }
    .save(&pool)
    .await
    .unwrap();

    AclRuleGroup {
        id: NoId,
        rule_id: rule.id,
        group_id: group_1.id,
        allow: false,
    }
    .save(&pool)
    .await
    .unwrap();

    // Get rule info
    let mut conn = pool.acquire().await.unwrap();
    let rule_info = rule.to_info(&mut conn).await.unwrap();
    assert_eq!(rule_info.denied_users.len(), 2);
    assert_eq!(rule_info.denied_groups.len(), 1);

    // Get all denied users
    let denied_users = rule_info.get_all_denied_users(&pool).await.unwrap();

    // Should contain user_1 (explicit), user_2 and user_3 (from group_1), but not inactive user_4
    assert_eq!(denied_users.len(), 3);
    assert!(denied_users.iter().any(|u| u.id == user_1.id));
    assert!(denied_users.iter().any(|u| u.id == user_2.id));
    assert!(denied_users.iter().any(|u| u.id == user_3.id));
    assert!(!denied_users.iter().any(|u| u.id == user_4.id));
}
