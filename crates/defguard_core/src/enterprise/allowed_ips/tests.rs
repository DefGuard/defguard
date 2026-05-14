use defguard_common::db::{
    Id, NoId,
    models::{WireguardNetwork, group::Group, user::User},
    setup_pool,
};
use ipnetwork::IpNetwork;
use sqlx::{
    PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
    query,
};
use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    sync::LazyLock,
};

use crate::enterprise::{
    allowed_ips::get_allowed_ips_from_acl_rules,
    db::models::acl::{
        AclRule, AclRuleDestinationRange, AclRuleGroup, AclRuleNetwork, AclRuleUser, RuleState,
    },
    license::{License, LicenseTier, SupportType, set_cached_license},
};

static IPV4_DEFAULT_ROUTE: LazyLock<IpNetwork> =
    LazyLock::new(|| IpNetwork::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0).unwrap());
static IPV6_DEFAULT_ROUTE: LazyLock<IpNetwork> =
    LazyLock::new(|| IpNetwork::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), 0).unwrap());

fn set_test_license_business() {
    let license = License {
        customer_id: "0c4dcb5400544d47ad8617fcdf2704cb".into(),
        limits: None,
        subscription: false,
        support_type: SupportType::Basic,
        tier: LicenseTier::Business,
        valid_until: None,
        version_date_limit: None,
    };
    set_cached_license(Some(license));
}

/// Creates a location with `acl_enabled = true` and the given CIDR address.
async fn create_acl_location(pool: &PgPool, address: &str) -> WireguardNetwork<Id> {
    let mut location = WireguardNetwork::default()
        .try_set_address(address)
        .unwrap();
    location.acl_enabled = true;
    location.save(pool).await.unwrap()
}

/// Saves a bare `AclRule` and wires up locations, allowed/denied users and
/// groups, and destination address ranges.
async fn create_acl_rule(
    pool: &PgPool,
    rule: AclRule,
    location_ids: &[Id],
    allowed_user_ids: &[Id],
    denied_user_ids: &[Id],
    allowed_group_ids: &[Id],
    denied_group_ids: &[Id],
    destination_ranges: &[(IpNetwork, IpNetwork)],
) -> Id {
    let mut conn = pool.acquire().await.unwrap();
    let rule = rule.save(&mut *conn).await.unwrap();
    let rule_id = rule.id;

    for &id in location_ids {
        AclRuleNetwork::new(rule_id, id)
            .save(&mut *conn)
            .await
            .unwrap();
    }
    for &id in allowed_user_ids {
        AclRuleUser::new(rule_id, id, true)
            .save(&mut *conn)
            .await
            .unwrap();
    }
    for &id in denied_user_ids {
        AclRuleUser::new(rule_id, id, false)
            .save(&mut *conn)
            .await
            .unwrap();
    }
    for &id in allowed_group_ids {
        AclRuleGroup::new(rule_id, id, true)
            .save(&mut *conn)
            .await
            .unwrap();
    }
    for &id in denied_group_ids {
        AclRuleGroup::new(rule_id, id, false)
            .save(&mut *conn)
            .await
            .unwrap();
    }
    for (start, end) in destination_ranges {
        AclRuleDestinationRange {
            id: NoId,
            rule_id,
            start: start.network(),
            end: end.broadcast(),
        }
        .save(&mut *conn)
        .await
        .unwrap();
    }

    rule_id
}

/// Returns a minimal applied `AclRule` with the given destination addresses.
fn applied_rule_with_addresses(name: &str, addresses: Vec<IpNetwork>) -> AclRule {
    AclRule {
        name: name.into(),
        state: RuleState::Applied,
        enabled: true,
        addresses,
        any_address: false,
        any_port: true,
        any_protocol: true,
        use_manual_destination_settings: true,
        ..Default::default()
    }
}

async fn add_user_to_group(pool: &PgPool, user_id: Id, group_id: Id) {
    query!(
        "INSERT INTO group_user (user_id, group_id) VALUES ($1, $2)",
        user_id,
        group_id
    )
    .execute(pool)
    .await
    .unwrap();
}

#[sqlx::test]
async fn test_explicit_user_allow(_: PgPoolOptions, options: PgConnectOptions) {
    set_test_license_business();
    let pool = setup_pool(options).await;

    let location = create_acl_location(&pool, "10.0.0.1/24").await;
    let user = User::new(
        "alice",
        Some("password"),
        "Alice",
        "Test",
        "alice@example.com",
        None,
    );
    let user = user.save(&pool).await.unwrap();
    let other_user = User::new(
        "bob",
        Some("password"),
        "Bob",
        "Test",
        "bob@example.com",
        None,
    );
    let other_user = other_user.save(&pool).await.unwrap();

    let destination = "192.168.1.0/24".parse().unwrap();
    let rule = applied_rule_with_addresses("allow-alice", vec![destination]);
    create_acl_rule(
        &pool,
        rule,
        &[location.id],
        &[user.id], // only alice is explicitly allowed
        &[],
        &[],
        &[],
        &[],
    )
    .await;

    let mut conn = pool.acquire().await.unwrap();

    // Alice should receive the destination
    let result = get_allowed_ips_from_acl_rules(&mut conn, &location, &user)
        .await
        .unwrap();
    assert_eq!(result, vec![destination]);

    // Bob is not in the allowed list - should receive nothing
    let result = get_allowed_ips_from_acl_rules(&mut conn, &location, &other_user)
        .await
        .unwrap();
    assert!(result.is_empty());
}

#[sqlx::test]
async fn test_group_membership_allow(_: PgPoolOptions, options: PgConnectOptions) {
    set_test_license_business();
    let pool = setup_pool(options).await;

    let location = create_acl_location(&pool, "10.0.0.1/24").await;

    let user = User::new(
        "alice",
        Some("password"),
        "Alice",
        "Test",
        "alice@example.com",
        None,
    );
    let user = user.save(&pool).await.unwrap();
    let non_member = User::new(
        "bob",
        Some("password"),
        "Bob",
        "Test",
        "bob@example.com",
        None,
    );
    let non_member = non_member.save(&pool).await.unwrap();

    let group = Group {
        name: "eng".into(),
        ..Default::default()
    };
    let group = group.save(&pool).await.unwrap();
    add_user_to_group(&pool, user.id, group.id).await;

    let destination = "10.1.0.0/16".parse().unwrap();
    let rule = applied_rule_with_addresses("allow-eng-group", vec![destination]);
    create_acl_rule(
        &pool,
        rule,
        &[location.id],
        &[],
        &[],
        &[group.id], // group is in the allowed set
        &[],
        &[],
    )
    .await;

    let mut conn = pool.acquire().await.unwrap();

    // Group member should receive the destination
    let result = get_allowed_ips_from_acl_rules(&mut conn, &location, &user)
        .await
        .unwrap();
    assert_eq!(result, vec![destination]);

    // Non-member should receive nothing
    let result = get_allowed_ips_from_acl_rules(&mut conn, &location, &non_member)
        .await
        .unwrap();
    assert!(result.is_empty());
}

#[sqlx::test]
async fn test_allow_all_users(_: PgPoolOptions, options: PgConnectOptions) {
    set_test_license_business();
    let pool = setup_pool(options).await;

    let location = create_acl_location(&pool, "10.0.0.1/24").await;

    let user_1 = User::new("alice", Some("pw"), "Alice", "T", "a@example.com", None);
    let user_1 = user_1.save(&pool).await.unwrap();
    let user_2 = User::new("bob", Some("pw"), "Bob", "T", "b@example.com", None);
    let user_2 = user_2.save(&pool).await.unwrap();

    let destination = "172.16.0.0/12".parse().unwrap();
    let rule = AclRule {
        name: "allow-everyone".into(),
        state: RuleState::Applied,
        enabled: true,
        allow_all_users: true,
        addresses: vec![destination],
        any_address: false,
        any_port: true,
        any_protocol: true,
        use_manual_destination_settings: true,
        ..Default::default()
    };
    create_acl_rule(&pool, rule, &[location.id], &[], &[], &[], &[], &[]).await;

    let mut conn = pool.acquire().await.unwrap();

    // Every user should receive the destination regardless of explicit membership
    for user in [&user_1, &user_2] {
        let result = get_allowed_ips_from_acl_rules(&mut conn, &location, user)
            .await
            .unwrap();
        assert_eq!(
            result,
            vec![destination],
            "user {} should be allowed",
            user.id
        );
    }
}

#[sqlx::test]
async fn test_deny_overrides_allow(_: PgPoolOptions, options: PgConnectOptions) {
    set_test_license_business();
    let pool = setup_pool(options).await;

    let location = create_acl_location(&pool, "10.0.0.1/24").await;
    let user = User::new("alice", Some("pw"), "Alice", "T", "a@example.com", None);
    let user = user.save(&pool).await.unwrap();

    let destination = "192.168.1.0/24".parse().unwrap();
    let rule = applied_rule_with_addresses("allow-then-deny", vec![destination]);

    // User appears in both allowed and denied - deny overrides allow.
    create_acl_rule(
        &pool,
        rule,
        &[location.id],
        &[user.id], // explicitly allowed
        &[user.id], // explicitly denied
        &[],
        &[],
        &[],
    )
    .await;

    let mut conn = pool.acquire().await.unwrap();
    let result = get_allowed_ips_from_acl_rules(&mut conn, &location, &user)
        .await
        .unwrap();
    assert!(result.is_empty(), "deny should override allow");
}

#[sqlx::test]
async fn test_deny_all_users(_: PgPoolOptions, options: PgConnectOptions) {
    set_test_license_business();
    let pool = setup_pool(options).await;

    let location = create_acl_location(&pool, "10.0.0.1/24").await;
    let user = User::new("alice", Some("pw"), "Alice", "T", "a@example.com", None);
    let user = user.save(&pool).await.unwrap();

    let destination = "192.168.1.0/24".parse().unwrap();
    // deny_all_users = true combined with allow_all_users = true - deny overrides allow.
    let rule = AclRule {
        name: "deny-everyone".into(),
        state: RuleState::Applied,
        enabled: true,
        allow_all_users: true,
        deny_all_users: true,
        addresses: vec![destination],
        any_address: false,
        any_port: true,
        any_protocol: true,
        use_manual_destination_settings: true,
        ..Default::default()
    };
    create_acl_rule(&pool, rule, &[location.id], &[], &[], &[], &[], &[]).await;

    let mut conn = pool.acquire().await.unwrap();
    let result = get_allowed_ips_from_acl_rules(&mut conn, &location, &user)
        .await
        .unwrap();
    assert!(
        result.is_empty(),
        "deny_all_users should prevent any access"
    );
}

#[sqlx::test]
async fn test_any_address_returns_all_traffic(_: PgPoolOptions, options: PgConnectOptions) {
    set_test_license_business();
    let pool = setup_pool(options).await;

    // Dual-stack location so we can assert both networks are returned.
    let mut location = WireguardNetwork::default()
        .set_address([
            "10.0.0.1/24".parse().unwrap(),
            "fd00::1/64".parse().unwrap(),
        ])
        .unwrap();
    location.acl_enabled = true;
    let location = location.save(&pool).await.unwrap();

    let user = User::new("alice", Some("pw"), "Alice", "T", "a@example.com", None);
    let user = user.save(&pool).await.unwrap();

    // Rule with any_address - should short-circuit to all-traffic networks.
    let any_address_rule = AclRule {
        name: "any-address".into(),
        state: RuleState::Applied,
        enabled: true,
        allow_all_users: true,
        any_address: true,
        any_port: true,
        any_protocol: true,
        use_manual_destination_settings: true,
        ..Default::default()
    };
    create_acl_rule(
        &pool,
        any_address_rule,
        &[location.id],
        &[],
        &[],
        &[],
        &[],
        &[],
    )
    .await;

    // A second rule that would add extra addresses - should never be reached.
    let extra_rule = AclRule {
        allow_all_users: true,
        ..applied_rule_with_addresses("extra", vec!["192.168.99.0/24".parse().unwrap()])
    };
    create_acl_rule(&pool, extra_rule, &[location.id], &[], &[], &[], &[], &[]).await;

    let mut conn = pool.acquire().await.unwrap();
    let result = get_allowed_ips_from_acl_rules(&mut conn, &location, &user)
        .await
        .unwrap();

    // Expect exactly the default routes for both IP versions present on the location.
    let expected = vec![*IPV4_DEFAULT_ROUTE, *IPV6_DEFAULT_ROUTE];
    assert_eq!(result, expected);
}

#[sqlx::test]
async fn test_any_address_respects_location_ip_version(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    set_test_license_business();
    let pool = setup_pool(options).await;

    // IPv4-only location - should only return 0.0.0.0/0, not ::/0.
    let location = create_acl_location(&pool, "10.0.0.1/24").await;

    let user = User::new("alice", Some("pw"), "Alice", "T", "a@example.com", None);
    let user = user.save(&pool).await.unwrap();

    let rule = AclRule {
        name: "any-address-ipv4-only".into(),
        state: RuleState::Applied,
        enabled: true,
        allow_all_users: true,
        any_address: true,
        any_port: true,
        any_protocol: true,
        use_manual_destination_settings: true,
        ..Default::default()
    };
    create_acl_rule(&pool, rule, &[location.id], &[], &[], &[], &[], &[]).await;

    let mut conn = pool.acquire().await.unwrap();
    let result = get_allowed_ips_from_acl_rules(&mut conn, &location, &user)
        .await
        .unwrap();

    assert_eq!(result, vec![*IPV4_DEFAULT_ROUTE]);
    assert!(
        !result.contains(&*IPV6_DEFAULT_ROUTE),
        "IPv4-only location should not include ::/0"
    );
}

#[sqlx::test]
async fn test_multiple_rules_destinations_merged(_: PgPoolOptions, options: PgConnectOptions) {
    set_test_license_business();
    let pool = setup_pool(options).await;

    let location = create_acl_location(&pool, "10.0.0.1/24").await;
    let user = User::new("alice", Some("pw"), "Alice", "T", "a@example.com", None);
    let user = user.save(&pool).await.unwrap();

    // Rule 1: gives access to 192.168.1.0/24
    let rule1 = AclRule {
        allow_all_users: true,
        ..applied_rule_with_addresses("rule-1", vec!["192.168.1.0/24".parse().unwrap()])
    };
    create_acl_rule(&pool, rule1, &[location.id], &[], &[], &[], &[], &[]).await;

    // Rule 2: gives access to 10.10.0.0/16 - distinct, no overlap with rule 1
    let rule2 = AclRule {
        allow_all_users: true,
        ..applied_rule_with_addresses("rule-2", vec!["10.10.0.0/16".parse().unwrap()])
    };
    create_acl_rule(&pool, rule2, &[location.id], &[], &[], &[], &[], &[]).await;

    // Rule 3: overlaps with rule 1 (192.168.1.128/25 is a subset of 192.168.1.0/24)
    // After merging it should not expand the result.
    let rule3 = AclRule {
        allow_all_users: true,
        ..applied_rule_with_addresses("rule-3", vec!["192.168.1.128/25".parse().unwrap()])
    };
    create_acl_rule(&pool, rule3, &[location.id], &[], &[], &[], &[], &[]).await;

    let mut conn = pool.acquire().await.unwrap();
    let result = get_allowed_ips_from_acl_rules(&mut conn, &location, &user)
        .await
        .unwrap();

    // merge_ranges sorts by range start, so 10.10.0.0/16 comes before 192.168.1.0/24.
    // The overlapping 192.168.1.128/25 should be absorbed into 192.168.1.0/24.
    let expected = vec![
        "10.10.0.0/16".parse().unwrap(),
        "192.168.1.0/24".parse().unwrap(),
    ];
    assert_eq!(result, expected);
}

#[sqlx::test]
async fn test_non_matching_rule_excluded(_: PgPoolOptions, options: PgConnectOptions) {
    set_test_license_business();
    let pool = setup_pool(options).await;

    let location = create_acl_location(&pool, "10.0.0.1/24").await;
    let user = User::new("alice", Some("pw"), "Alice", "T", "a@example.com", None);
    let user = user.save(&pool).await.unwrap();
    let other_user = User::new("bob", Some("pw"), "Bob", "T", "b@example.com", None);
    let other_user = other_user.save(&pool).await.unwrap();

    // Rule only allows other_user - alice should get nothing.
    let rule = AclRule {
        ..applied_rule_with_addresses("other-user-only", vec!["172.16.0.0/12".parse().unwrap()])
    };
    create_acl_rule(
        &pool,
        rule,
        &[location.id],
        &[other_user.id],
        &[],
        &[],
        &[],
        &[],
    )
    .await;

    let mut conn = pool.acquire().await.unwrap();

    // other_user matches and gets the destination
    let result = get_allowed_ips_from_acl_rules(&mut conn, &location, &other_user)
        .await
        .unwrap();
    assert_eq!(result, vec!["172.16.0.0/12".parse().unwrap()]);

    // alice does not match and gets nothing
    let result = get_allowed_ips_from_acl_rules(&mut conn, &location, &user)
        .await
        .unwrap();
    assert!(result.is_empty());
}
