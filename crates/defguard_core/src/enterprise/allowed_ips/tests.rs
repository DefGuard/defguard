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

use crate::enterprise::{
    allowed_ips::get_allowed_ips_from_acl_rules,
    db::models::acl::{
        AclRule, AclRuleDestinationRange, AclRuleGroup, AclRuleNetwork, AclRuleUser, RuleState,
    },
    license::{License, LicenseTier, SupportType, set_cached_license},
};

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
    let user: User<NoId> = User::new(
        "alice",
        Some("password"),
        "Alice",
        "Test",
        "alice@example.com",
        None,
    );
    let user = user.save(&pool).await.unwrap();
    let other_user: User<NoId> = User::new(
        "bob",
        Some("password"),
        "Bob",
        "Test",
        "bob@example.com",
        None,
    );
    let other_user = other_user.save(&pool).await.unwrap();

    let destination: IpNetwork = "192.168.1.0/24".parse().unwrap();
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

    let user: User<NoId> = User::new(
        "alice",
        Some("password"),
        "Alice",
        "Test",
        "alice@example.com",
        None,
    );
    let user = user.save(&pool).await.unwrap();
    let non_member: User<NoId> = User::new(
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

    let destination: IpNetwork = "10.1.0.0/16".parse().unwrap();
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

    let user_1: User<NoId> = User::new("alice", Some("pw"), "Alice", "T", "a@example.com", None);
    let user_1 = user_1.save(&pool).await.unwrap();
    let user_2: User<NoId> = User::new("bob", Some("pw"), "Bob", "T", "b@example.com", None);
    let user_2 = user_2.save(&pool).await.unwrap();

    let destination: IpNetwork = "172.16.0.0/12".parse().unwrap();
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
    let user: User<NoId> = User::new("alice", Some("pw"), "Alice", "T", "a@example.com", None);
    let user = user.save(&pool).await.unwrap();

    let destination: IpNetwork = "192.168.1.0/24".parse().unwrap();
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
    let user: User<NoId> = User::new("alice", Some("pw"), "Alice", "T", "a@example.com", None);
    let user = user.save(&pool).await.unwrap();

    let destination: IpNetwork = "192.168.1.0/24".parse().unwrap();
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
