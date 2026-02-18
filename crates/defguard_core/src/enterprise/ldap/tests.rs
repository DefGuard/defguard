use std::collections::HashMap;

use defguard_common::db::{models::settings::initialize_current_settings, setup_pool};
use ldap3::SearchEntry;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

use super::*;
use crate::enterprise::ldap::{
    model::{extract_rdn_value, get_users_without_ldap_path, user_from_searchentry},
    sync::{
        Authority, compute_group_sync_changes, compute_user_sync_changes,
        extract_intersecting_users,
    },
    test_client::{LdapEvent, group_to_test_attrs, user_to_test_attrs},
};

const PASSWORD: &str = "test_password";

fn make_test_user(
    username: &str,
    ldap_rdn: Option<String>,
    ldap_user_path: Option<String>,
) -> User {
    let mut user = User::new(
        username,
        Some(PASSWORD),
        "last name",
        "first name",
        format!("{username}@example.com").as_str(),
        None,
    );
    user.ldap_rdn = ldap_rdn;
    user.ldap_user_path = ldap_user_path;
    user
}

#[test]
fn test_get_rdn_attr() {
    // Default configuration should use 'cn' as the RDN attribute
    let config = LDAPConfig::default();
    assert_eq!(config.get_rdn_attr(), "cn");

    // Custom RDN attribute should be respected
    let config = LDAPConfig {
        ldap_user_rdn_attr: Some("uid".to_string()),
        ..LDAPConfig::default()
    };
    assert_eq!(config.get_rdn_attr(), "uid");

    // Empty string should fall back to default 'cn'
    let config = LDAPConfig {
        ldap_user_rdn_attr: Some(String::new()),
        ..LDAPConfig::default()
    };
    assert_eq!(config.get_rdn_attr(), "cn");

    // Whitespace-only string should also fall back to default 'cn'
    let config = LDAPConfig {
        ldap_user_rdn_attr: Some("   ".to_string()),
        ..LDAPConfig::default()
    };
    assert_eq!(config.get_rdn_attr(), "cn");

    // Leading/trailing whitespace should be trimmed from valid attributes
    let config = LDAPConfig {
        ldap_user_rdn_attr: Some("  uid  ".to_string()),
        ..LDAPConfig::default()
    };
    assert_eq!(config.get_rdn_attr(), "uid");
}

#[test]
fn test_user_dn() {
    let config = LDAPConfig::default();

    // Basic DN construction with default config
    let dn = config.user_dn("user1", "ou=users,dc=example,dc=com");
    assert_eq!(dn, "cn=user1,ou=users,dc=example,dc=com");

    // Using 'uid' instead of 'cn' for RDN construction
    let config = LDAPConfig {
        ldap_user_rdn_attr: Some("uid".to_string()),
        ..LDAPConfig::default()
    };
    let dn = config.user_dn("testuser2", "ou=people,dc=test,dc=org");
    assert_eq!(dn, "uid=testuser2,ou=people,dc=test,dc=org");
}

#[test]
fn test_user_dn_from_user() {
    let config = LDAPConfig::default();

    // User without stored LDAP data uses default search base
    let user = make_test_user("testuser", None, None);
    let dn = config.user_dn_from_user(&user);
    assert_eq!(dn, "cn=testuser,ou=users,dc=example,dc=com");

    // User with stored RDN and path uses the stored path instead of default
    let user = make_test_user(
        "testuser",
        Some("testuser".to_string()),
        Some("ou=admins,dc=example,dc=com".to_string()),
    );
    let dn = config.user_dn_from_user(&user);
    assert_eq!(dn, "cn=testuser,ou=admins,dc=example,dc=com");

    // RDN value takes precedence over username when available
    let user = make_test_user(
        "user3",
        Some("testuser3".to_string()),
        Some("ou=people,dc=example,dc=com".to_string()),
    );
    let dn = config.user_dn_from_user(&user);
    assert_eq!(dn, "cn=testuser3,ou=people,dc=example,dc=com");

    // Custom RDN attribute affects the final DN format
    let config = LDAPConfig {
        ldap_user_rdn_attr: Some("uid".to_string()),
        ..LDAPConfig::default()
    };
    let user = make_test_user("user4", Some("testuser4".to_string()), None);
    let dn = config.user_dn_from_user(&user);
    assert_eq!(dn, "uid=testuser4,ou=users,dc=example,dc=com");
}

#[test]
fn test_group_dn() {
    let config = LDAPConfig::default();

    // Groups use the default 'cn' attribute for naming
    let dn = config.group_dn("admins");
    assert_eq!(dn, "cn=admins,ou=groups,dc=example,dc=com");

    // Alternative naming attribute can be configured for groups
    let config = LDAPConfig {
        ldap_groupname_attr: "ou".to_string(),
        ..LDAPConfig::default()
    };
    let dn = config.group_dn("users");
    assert_eq!(dn, "ou=users,ou=groups,dc=example,dc=com");

    // Different search base location can be configured for groups
    let config = LDAPConfig {
        ldap_group_search_base: "ou=roles,dc=test,dc=org".to_string(),
        ..LDAPConfig::default()
    };
    let dn = config.group_dn("admin");
    assert_eq!(dn, "cn=admin,ou=roles,dc=test,dc=org");
}

#[test]
fn test_get_all_user_obj_classes() {
    // Base class plus one auxiliary class
    let config = LDAPConfig {
        ldap_user_auxiliary_obj_classes: vec!["simpleSecurityObject".to_string()],
        ..LDAPConfig::default()
    };
    let obj_classes = config.get_all_user_obj_classes();
    assert_eq!(obj_classes.len(), 2);
    assert!(obj_classes.contains(&"inetOrgPerson".to_string()));
    assert!(obj_classes.contains(&"simpleSecurityObject".to_string()));

    // Should always include the base object class even with no auxiliaries
    let config = LDAPConfig {
        ldap_user_auxiliary_obj_classes: vec![],
        ..LDAPConfig::default()
    };
    let obj_classes = config.get_all_user_obj_classes();
    assert_eq!(obj_classes.len(), 1);
    assert_eq!(obj_classes[0], "inetOrgPerson");

    // Single auxiliary class should be combined with base class
    let config = LDAPConfig {
        ldap_user_auxiliary_obj_classes: vec!["customUser".to_string()],
        ..LDAPConfig::default()
    };
    let obj_classes = config.get_all_user_obj_classes();
    assert_eq!(obj_classes.len(), 2);
    assert!(obj_classes.contains(&"inetOrgPerson".to_string()));
    assert!(obj_classes.contains(&"customUser".to_string()));

    // Multiple auxiliary classes
    let config = LDAPConfig {
        ldap_user_auxiliary_obj_classes: vec![
            "posixAccount".to_string(),
            "mailUser".to_string(),
            "customAttribute".to_string(),
        ],
        ..LDAPConfig::default()
    };
    let obj_classes = config.get_all_user_obj_classes();
    assert_eq!(obj_classes.len(), 4);
    assert!(obj_classes.contains(&"inetOrgPerson".to_string()));
    assert!(obj_classes.contains(&"posixAccount".to_string()));
    assert!(obj_classes.contains(&"mailUser".to_string()));
    assert!(obj_classes.contains(&"customAttribute".to_string()));
}

#[test]
fn test_using_username_as_rdn() {
    // Default config should use username as RDN since default is 'cn'
    let config = LDAPConfig::default();
    assert!(config.using_username_as_rdn());

    // Explicitly setting RDN to 'cn' should match username behavior
    let config = LDAPConfig {
        ldap_user_rdn_attr: Some("cn".to_string()),
        ..LDAPConfig::default()
    };
    assert!(config.using_username_as_rdn());

    // Using different RDN attribute means username != RDN value
    let config = LDAPConfig {
        ldap_user_rdn_attr: Some("uid".to_string()),
        ..LDAPConfig::default()
    };
    assert!(!config.using_username_as_rdn());

    // Empty RDN attribute falls back to 'cn', so username is used
    let config = LDAPConfig {
        ldap_user_rdn_attr: Some(String::new()),
        ..LDAPConfig::default()
    };
    assert!(config.using_username_as_rdn());

    // Active Directory scenario: username and RDN both use sAMAccountName
    let config = LDAPConfig {
        ldap_username_attr: "sAMAccountName".to_string(),
        ldap_user_rdn_attr: Some("sAMAccountName".to_string()),
        ..LDAPConfig::default()
    };
    assert!(config.using_username_as_rdn());

    // Mixed AD scenario: username from sAMAccountName but RDN uses CN
    let config = LDAPConfig {
        ldap_username_attr: "sAMAccountName".to_string(),
        ldap_user_rdn_attr: Some("cn".to_string()),
        ..LDAPConfig::default()
    };
    assert!(!config.using_username_as_rdn());
}

#[sqlx::test]
async fn test_update_users_state(_: PgPoolOptions, options: PgConnectOptions) {
    let mut ldap_conn = LDAPConnection::create().await.unwrap();
    let pool = setup_pool(options).await;
    let _ = initialize_current_settings(&pool).await;
    let config = ldap_conn.config.clone();

    // active user missing from LDAP, inactive user in LDAP, active user in LDAP
    let mut active_user_not_in_ldap =
        make_test_user("active_user", Some("active_user".to_string()), None)
            .save(&pool)
            .await
            .unwrap();

    let mut inactive_user_in_ldap =
        make_test_user("inactive_user", Some("inactive_user".to_string()), None)
            .save(&pool)
            .await
            .unwrap();
    inactive_user_in_ldap.is_active = false;

    let mut active_user_in_ldap =
        make_test_user("existing_user", Some("existing_user".to_string()), None)
            .save(&pool)
            .await
            .unwrap();
    active_user_in_ldap.is_active = true;

    // Populate LDAP with users that should be there
    ldap_conn
        .test_client_mut()
        .add_test_user(&inactive_user_in_ldap.clone().as_noid(), &config);
    ldap_conn
        .test_client_mut()
        .add_test_user(&active_user_in_ldap.clone().as_noid(), &config);

    // Verify initial LDAP state matches expectations
    assert!(ldap_conn.user_exists(&inactive_user_in_ldap).await.unwrap());
    assert!(ldap_conn.user_exists(&active_user_in_ldap).await.unwrap());
    assert!(
        !ldap_conn
            .user_exists(&active_user_not_in_ldap)
            .await
            .unwrap()
    );

    // Trigger state synchronization - should add missing active user and remove inactive user
    ldap_conn
        .update_users_state(
            vec![
                &mut active_user_not_in_ldap,
                &mut inactive_user_in_ldap,
                &mut active_user_in_ldap,
            ],
            &pool,
        )
        .await
        .unwrap();

    // missing user added, inactive user removed
    assert!(ldap_conn.test_client.events_match(
        &[
            LdapEvent::ObjectAdded {
                dn: ldap_conn.config.user_dn_from_user(&active_user_not_in_ldap),
                attrs: user_to_test_attrs(
                    &active_user_not_in_ldap,
                    Some(PASSWORD),
                    &ldap_conn.config
                ),
            },
            LdapEvent::ObjectDeleted {
                dn: ldap_conn.config.user_dn_from_user(&inactive_user_in_ldap),
            },
        ],
        false
    ));

    ldap_conn.test_client.clear_events();

    // Test group creation when user is added to a group
    let group = Group::new("test_group").save(&pool).await.unwrap();
    active_user_in_ldap
        .add_to_group(&pool, &group)
        .await
        .unwrap();

    ldap_conn
        .update_users_state(vec![&mut active_user_in_ldap], &pool)
        .await
        .unwrap();

    // Group should be created automatically when user sync runs
    assert!(ldap_conn.test_client.events_match(
        &[LdapEvent::ObjectAdded {
            dn: ldap_conn.config.group_dn(&group.name),
            attrs: group_to_test_attrs(
                &group,
                &ldap_conn.config,
                Some(&vec![&active_user_in_ldap])
            ),
        }],
        false
    ));

    ldap_conn.test_client.clear_events();

    // Setup for user deactivation scenario
    ldap_conn
        .test_client
        .add_test_group(&group.clone().as_noid(), &config);
    ldap_conn.test_client.add_test_membership(
        &group.clone().as_noid(),
        &active_user_in_ldap.clone().as_noid(),
        &config,
    );

    active_user_in_ldap.is_active = false;
    ldap_conn
        .update_users_state(vec![&mut active_user_in_ldap], &pool)
        .await
        .unwrap();

    // When last member is deactivated, both group and user should be deleted
    assert!(ldap_conn.test_client.events_match(
        &[
            LdapEvent::ObjectDeleted {
                dn: ldap_conn.config.group_dn(&group.name),
            },
            LdapEvent::ObjectDeleted {
                dn: ldap_conn.config.user_dn_from_user(&active_user_in_ldap),
            }
        ],
        true
    ));

    ldap_conn.test_client.clear_events();

    // Test partial group membership removal when other members remain
    let mut another_active_user_in_ldap = make_test_user(
        "another_active_user",
        Some("another_active_user".to_string()),
        None,
    )
    .save(&pool)
    .await
    .unwrap();

    ldap_conn
        .test_client_mut()
        .add_test_user(&another_active_user_in_ldap.clone().as_noid(), &config);
    ldap_conn.test_client.add_test_membership(
        &group.clone().as_noid(),
        &another_active_user_in_ldap.clone().as_noid(),
        &config,
    );

    another_active_user_in_ldap.is_active = false;
    active_user_in_ldap.is_active = false;

    ldap_conn
        .update_users_state(vec![&mut active_user_in_ldap], &pool)
        .await
        .unwrap();

    // Group should be modified to remove member, not deleted since other members exist
    assert!(ldap_conn.test_client.events_match(
        &[
            LdapEvent::ObjectModified {
                old_dn: ldap_conn.config.group_dn(&group.name),
                new_dn: ldap_conn.config.group_dn(&group.name),
                mods: vec![Mod::Delete(
                    ldap_conn.config.ldap_group_member_attr.clone(),
                    hashset![ldap_conn.config.user_dn_from_user(&active_user_in_ldap)],
                )],
            },
            LdapEvent::ObjectDeleted {
                dn: ldap_conn.config.user_dn_from_user(&active_user_in_ldap),
            },
        ],
        true,
    ));

    ldap_conn.test_client.clear_events();

    // Manually clean up LDAP state to simulate previous operations
    ldap_conn.test_client_mut().remove_test_membership(
        &group.clone().as_noid(),
        &active_user_in_ldap.clone().as_noid(),
        &config,
    );
    ldap_conn
        .test_client_mut()
        .remove_test_user(&active_user_in_ldap.clone().as_noid(), &config);

    ldap_conn
        .update_users_state(vec![&mut another_active_user_in_ldap], &pool)
        .await
        .unwrap();

    // Now removing the last member should delete both group and user
    assert!(
        ldap_conn.test_client.events_match(
            &[
                LdapEvent::ObjectDeleted {
                    dn: ldap_conn.config.group_dn(&group.name),
                },
                LdapEvent::ObjectDeleted {
                    dn: ldap_conn
                        .config
                        .user_dn_from_user(&another_active_user_in_ldap),
                },
            ],
            true,
        )
    );
}

#[tokio::test]
async fn test_get_user() {
    let mut ldap_conn = LDAPConnection::create().await.unwrap();

    ldap_conn.config = LDAPConfig {
        ldap_user_auxiliary_obj_classes: vec![UserObjectClass::InetOrgPerson.into()],
        ..ldap_conn.config
    };

    let config = ldap_conn.config.clone();

    let test_user = make_test_user("testuser", None, None);
    ldap_conn
        .test_client_mut()
        .add_test_user(&test_user, &config);
    let search_base = ldap_conn.config.ldap_user_search_base.clone();

    // Helper to verify user data integrity after LDAP retrieval
    let check = |result: User| {
        assert_eq!(result.username, test_user.username);
        assert_eq!(result.first_name, test_user.first_name);
        assert_eq!(result.last_name, test_user.last_name);
        assert_eq!(result.email, test_user.email);
        assert!(result.from_ldap);
        assert_eq!(
            result.ldap_rdn.as_ref(),
            Some(&test_user.ldap_rdn_value().to_string())
        );
        assert_eq!(result.ldap_user_path.as_ref(), Some(&search_base));
    };

    // By username
    let result = ldap_conn
        .get_user_by_username(&test_user.username)
        .await
        .unwrap();
    check(result);

    // By DN
    let result = ldap_conn.get_user_by_dn(&test_user).await.unwrap();
    check(result);

    // Non-existent user
    let non_existent_user = make_test_user("nonexistent", None, None);
    let result = ldap_conn
        .get_user_by_username(&non_existent_user.username)
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_user_in_ldap_sync_groups() {
    // Empty sync groups configuration = sync all users regardless of group membership
    {
        let mut ldap_conn = LDAPConnection::create().await.unwrap();
        ldap_conn.config.ldap_sync_groups = Vec::new();
        let test_user = make_test_user("user1", None, None);

        let result = ldap_conn
            .user_in_ldap_sync_groups(&test_user)
            .await
            .unwrap();
        assert!(result);
    }

    // User that doesn't exist in LDAP cannot be in any sync groups
    {
        let mut ldap_conn = LDAPConnection::create().await.unwrap();
        ldap_conn.config.ldap_sync_groups = vec!["group1".to_string(), "group2".to_string()];
        let test_user = make_test_user("nonexistent", None, None);

        let result = ldap_conn
            .user_in_ldap_sync_groups(&test_user)
            .await
            .unwrap();
        assert!(!result);
    }

    // User exists and is member of at least one configured sync group
    {
        let mut ldap_conn = LDAPConnection::create().await.unwrap();
        let config = ldap_conn.config.clone();
        ldap_conn.config.ldap_sync_groups = vec!["developers".to_string(), "admins".to_string()];

        let test_user = make_test_user("user2", None, None);
        ldap_conn
            .test_client_mut()
            .add_test_user(&test_user, &config);

        let developers_group = Group::new("developers");
        let other_group = Group::new("othergroup");

        ldap_conn
            .test_client_mut()
            .add_test_group(&developers_group, &config);
        ldap_conn
            .test_client_mut()
            .add_test_group(&other_group, &config);

        ldap_conn
            .test_client_mut()
            .add_test_membership(&developers_group, &test_user, &config);
        ldap_conn
            .test_client_mut()
            .add_test_membership(&other_group, &test_user, &config);

        let result = ldap_conn
            .user_in_ldap_sync_groups(&test_user)
            .await
            .unwrap();
        assert!(result);
    }

    // User exists but only belongs to groups not in sync configuration
    {
        let mut ldap_conn = LDAPConnection::create().await.unwrap();
        let config = ldap_conn.config.clone();
        ldap_conn.config.ldap_sync_groups = vec!["developers".to_string(), "admins".to_string()];

        let test_user = make_test_user("user3", None, None);
        ldap_conn
            .test_client_mut()
            .add_test_user(&test_user, &config);

        let marketing_group = Group::new("marketing");
        let sales_group = Group::new("sales");

        ldap_conn
            .test_client_mut()
            .add_test_group(&marketing_group, &config);
        ldap_conn
            .test_client_mut()
            .add_test_group(&sales_group, &config);

        ldap_conn
            .test_client_mut()
            .add_test_membership(&marketing_group, &test_user, &config);
        ldap_conn
            .test_client_mut()
            .add_test_membership(&sales_group, &test_user, &config);

        let result = ldap_conn
            .user_in_ldap_sync_groups(&test_user)
            .await
            .unwrap();
        assert!(!result);
    }

    // User belongs to multiple sync groups (should still return true)
    {
        let mut ldap_conn = LDAPConnection::create().await.unwrap();
        let config = ldap_conn.config.clone();
        ldap_conn.config.ldap_sync_groups = vec![
            "developers".to_string(),
            "admins".to_string(),
            "qa".to_string(),
        ];

        let test_user = make_test_user("user4", None, None);
        ldap_conn
            .test_client_mut()
            .add_test_user(&test_user, &config);

        let developers_group = Group::new("developers");
        let admins_group = Group::new("admins");
        let marketing_group = Group::new("marketing");

        ldap_conn
            .test_client_mut()
            .add_test_group(&developers_group, &config);
        ldap_conn
            .test_client_mut()
            .add_test_group(&admins_group, &config);
        ldap_conn
            .test_client_mut()
            .add_test_group(&marketing_group, &config);

        ldap_conn
            .test_client_mut()
            .add_test_membership(&developers_group, &test_user, &config);
        ldap_conn
            .test_client_mut()
            .add_test_membership(&admins_group, &test_user, &config);
        ldap_conn
            .test_client_mut()
            .add_test_membership(&marketing_group, &test_user, &config);

        let result = ldap_conn
            .user_in_ldap_sync_groups(&test_user)
            .await
            .unwrap();
        assert!(result);
    }

    // User exists in LDAP but has no group memberships at all
    {
        let mut ldap_conn = LDAPConnection::create().await.unwrap();
        let config = ldap_conn.config.clone();
        ldap_conn.config.ldap_sync_groups = vec!["developers".to_string(), "admins".to_string()];

        let test_user = make_test_user("user5", None, None);
        ldap_conn
            .test_client_mut()
            .add_test_user(&test_user, &config);

        let result = ldap_conn
            .user_in_ldap_sync_groups(&test_user)
            .await
            .unwrap();
        assert!(!result);
    }
}

#[test]
fn test_compute_user_sync_changes_empty_lists() {
    let mut ldap_users: Vec<User> = vec![];
    let mut defguard_users: Vec<User<Id>> = vec![];

    let changes = compute_user_sync_changes(
        &mut ldap_users,
        &mut defguard_users,
        Authority::LDAP,
        &LDAPConfig::default(),
    );

    // No users in either system, should result in no changes
    assert!(changes.delete_defguard.is_empty());
    assert!(changes.add_defguard.is_empty());
    assert!(changes.delete_ldap.is_empty());
    assert!(changes.add_ldap.is_empty());
}

#[test]
fn test_ldap_authority_add_to_defguard() {
    let ldap_user = User::new(
        "test_user",
        Some("test_password"),
        "last name",
        "first name",
        "email@email.com",
        None,
    );

    let mut ldap_users = vec![ldap_user];
    let mut defguard_users: Vec<User<Id>> = vec![];

    let changes = compute_user_sync_changes(
        &mut ldap_users,
        &mut defguard_users,
        Authority::LDAP,
        &LDAPConfig::default(),
    );

    // When LDAP is authoritative, users in LDAP but not Defguard should be added to Defguard
    assert!(changes.delete_defguard.is_empty());
    assert_eq!(changes.add_defguard.len(), 1);
    assert_eq!(changes.add_defguard[0].username, "test_user");
    assert!(changes.delete_ldap.is_empty());
    assert!(changes.add_ldap.is_empty());
}

#[sqlx::test]
fn test_ldap_authority_delete_from_defguard(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let defguard_user = User::new(
        "test_user",
        Some("test_password"),
        "last name",
        "first name",
        "email@email.com",
        None,
    )
    .save(&pool)
    .await
    .unwrap();

    let mut ldap_users: Vec<User> = vec![];
    let mut defguard_users = vec![defguard_user];

    let changes = compute_user_sync_changes(
        &mut ldap_users,
        &mut defguard_users,
        Authority::LDAP,
        &LDAPConfig::default(),
    );

    // When LDAP is authoritative, users in Defguard but not LDAP should be removed from Defguard
    assert_eq!(changes.delete_defguard.len(), 1);
    assert_eq!(changes.delete_defguard[0].username, "test_user");
    assert!(changes.add_defguard.is_empty());
    assert!(changes.delete_ldap.is_empty());
    assert!(changes.add_ldap.is_empty());
}

#[sqlx::test]
fn test_defguard_authority_add_to_ldap(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let defguard_user = User::new(
        "test_user",
        Some("test_password"),
        "last name",
        "first name",
        "email@email.com",
        None,
    )
    .save(&pool)
    .await
    .unwrap();

    let mut ldap_users: Vec<User> = vec![];
    let mut defguard_users = vec![defguard_user];

    let changes = compute_user_sync_changes(
        &mut ldap_users,
        &mut defguard_users,
        Authority::Defguard,
        &LDAPConfig::default(),
    );

    // When Defguard is authoritative, users in Defguard but not LDAP should be added to LDAP
    assert!(changes.delete_defguard.is_empty());
    assert!(changes.add_defguard.is_empty());
    assert!(changes.delete_ldap.is_empty());
    assert_eq!(changes.add_ldap.len(), 1);
    assert_eq!(changes.add_ldap[0].username, "test_user");
}

#[test]
fn test_defguard_authority_delete_from_ldap() {
    let ldap_user = User::new(
        "test_user",
        Some("test_password"),
        "last name",
        "first name",
        "email@email.com",
        None,
    );

    let mut ldap_users = vec![ldap_user];
    let mut defguard_users: Vec<User<Id>> = vec![];

    let changes = compute_user_sync_changes(
        &mut ldap_users,
        &mut defguard_users,
        Authority::Defguard,
        &LDAPConfig::default(),
    );

    // When Defguard is authoritative, users in LDAP but not Defguard should be removed from LDAP
    assert!(changes.delete_defguard.is_empty());
    assert!(changes.add_defguard.is_empty());
    assert_eq!(changes.delete_ldap.len(), 1);
    assert_eq!(changes.delete_ldap[0].username, "test_user");
    assert!(changes.add_ldap.is_empty());
}

#[sqlx::test]
fn test_matching_users_no_changes(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let ldap_user = User::new(
        "test_user",
        Some("test_password"),
        "last name",
        "first name",
        "email@email.com",
        None,
    );

    let defguard_user = User::new(
        "test_user",
        Some("test_password"),
        "last name",
        "first name",
        "email@email.com",
        None,
    )
    .save(&pool)
    .await
    .unwrap();

    let mut ldap_users = vec![ldap_user];
    let mut defguard_users = vec![defguard_user];

    // Test both authority directions with identical users
    let changes_ldap = compute_user_sync_changes(
        &mut ldap_users.clone(),
        &mut defguard_users.clone(),
        Authority::LDAP,
        &LDAPConfig::default(),
    );

    // Identical users should result in no sync changes regardless of authority
    assert!(changes_ldap.delete_defguard.is_empty());
    assert!(changes_ldap.add_defguard.is_empty());
    assert!(changes_ldap.delete_ldap.is_empty());
    assert!(changes_ldap.add_ldap.is_empty());

    let changes_defguard = compute_user_sync_changes(
        &mut ldap_users,
        &mut defguard_users,
        Authority::Defguard,
        &LDAPConfig::default(),
    );

    assert!(changes_defguard.delete_defguard.is_empty());
    assert!(changes_defguard.add_defguard.is_empty());
    assert!(changes_defguard.delete_ldap.is_empty());
    assert!(changes_defguard.add_ldap.is_empty());
}

#[test]
fn test_compute_group_sync_changes_empty_maps() {
    let defguard_memberships = HashMap::new();
    let ldap_memberships = HashMap::new();

    let changes = compute_group_sync_changes(
        defguard_memberships,
        ldap_memberships,
        Authority::LDAP,
        &LDAPConfig::default(),
    );

    // No groups in either system, should result in no changes
    assert!(changes.delete_defguard.is_empty());
    assert!(changes.add_defguard.is_empty());
    assert!(changes.delete_ldap.is_empty());
    assert!(changes.add_ldap.is_empty());
}

#[test]
fn test_ldap_authority_add_group_to_defguard() {
    let defguard_memberships = HashMap::new();
    let mut ldap_memberships = HashMap::new();
    let test_user = make_test_user("user1", None, None);
    ldap_memberships.insert(
        "test_group".to_string(),
        HashSet::from_iter(vec![&test_user]),
    );

    let changes = compute_group_sync_changes(
        defguard_memberships,
        ldap_memberships,
        Authority::LDAP,
        &LDAPConfig::default(),
    );

    // When LDAP is authoritative, groups in LDAP but not Defguard should be added to Defguard
    assert!(changes.delete_defguard.is_empty());
    assert_eq!(changes.add_defguard.len(), 1);
    assert!(changes.add_defguard.contains_key("test_group"));
    assert_eq!(changes.add_defguard["test_group"].len(), 1);
    assert!(changes.add_defguard["test_group"].contains(&test_user));
    assert!(changes.delete_ldap.is_empty());
    assert!(changes.add_ldap.is_empty());
}

#[sqlx::test]
fn test_ldap_authority_delete_group_from_defguard(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let mut defguard_memberships = HashMap::new();
    let test_user = make_test_user("user1", None, None)
        .save(&pool)
        .await
        .unwrap();
    defguard_memberships.insert(
        "test_group".to_string(),
        HashSet::from_iter(vec![test_user.clone()]),
    );
    let ldap_memberships = HashMap::new();

    let changes = compute_group_sync_changes(
        defguard_memberships,
        ldap_memberships,
        Authority::LDAP,
        &LDAPConfig::default(),
    );

    // When LDAP is authoritative, groups in Defguard but not LDAP should be removed from Defguard
    assert_eq!(changes.delete_defguard.len(), 1);
    assert!(changes.delete_defguard.contains_key("test_group"));
    assert_eq!(changes.delete_defguard["test_group"].len(), 1);
    assert!(changes.delete_defguard["test_group"].contains(&test_user));
    assert!(changes.add_defguard.is_empty());
    assert!(changes.delete_ldap.is_empty());
    assert!(changes.add_ldap.is_empty());
}

#[sqlx::test]
fn test_defguard_authority_add_group_to_ldap(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let mut defguard_memberships = HashMap::new();
    let test_user = make_test_user("user1", None, None)
        .save(&pool)
        .await
        .unwrap();
    defguard_memberships.insert(
        "test_group".to_string(),
        HashSet::from_iter(vec![test_user.clone()]),
    );
    let ldap_memberships = HashMap::new();

    let changes = compute_group_sync_changes(
        defguard_memberships,
        ldap_memberships,
        Authority::Defguard,
        &LDAPConfig::default(),
    );

    // When Defguard is authoritative, groups in Defguard but not LDAP should be added to LDAP
    assert!(changes.delete_defguard.is_empty());
    assert!(changes.add_defguard.is_empty());
    assert!(changes.delete_ldap.is_empty());
    assert_eq!(changes.add_ldap.len(), 1);
    assert!(changes.add_ldap.contains_key("test_group"));
    assert_eq!(changes.add_ldap["test_group"].len(), 1);
    assert!(changes.add_ldap["test_group"].contains(&test_user));
}

#[test]
fn test_defguard_authority_delete_group_from_ldap() {
    let defguard_memberships = HashMap::new();
    let mut ldap_memberships = HashMap::new();
    let test_user = make_test_user("user1", None, None);
    ldap_memberships.insert(
        "test_group".to_string(),
        HashSet::from_iter(vec![&test_user]),
    );

    let changes = compute_group_sync_changes(
        defguard_memberships,
        ldap_memberships,
        Authority::Defguard,
        &LDAPConfig::default(),
    );

    // When Defguard is authoritative, groups in LDAP but not Defguard should be removed from LDAP
    assert!(changes.delete_defguard.is_empty());
    assert!(changes.add_defguard.is_empty());
    assert_eq!(changes.delete_ldap.len(), 1);
    assert!(changes.delete_ldap.contains_key("test_group"));
    assert_eq!(changes.delete_ldap["test_group"].len(), 1);
    assert!(changes.delete_ldap["test_group"].contains(&test_user));
    assert!(changes.add_ldap.is_empty());
}

#[sqlx::test]
fn test_matching_groups_no_changes(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let mut defguard_memberships = HashMap::new();
    let test_user = make_test_user("user1", None, None);
    let test_user_id = test_user.clone().save(&pool).await.unwrap();
    defguard_memberships.insert(
        "test_group".to_string(),
        HashSet::from_iter(vec![test_user_id]),
    );
    let mut ldap_memberships = HashMap::new();
    ldap_memberships.insert(
        "test_group".to_string(),
        HashSet::from_iter(vec![&test_user]),
    );

    // Test both authority directions with identical group memberships
    let changes_ldap = compute_group_sync_changes(
        defguard_memberships.clone(),
        ldap_memberships.clone(),
        Authority::LDAP,
        &LDAPConfig::default(),
    );

    // Identical group memberships should result in no changes regardless of authority
    assert!(
        changes_ldap.delete_defguard.is_empty()
            || changes_ldap.delete_defguard["test_group"].is_empty()
    );
    assert!(
        changes_ldap.add_defguard.is_empty() || changes_ldap.add_defguard["test_group"].is_empty()
    );
    assert!(
        changes_ldap.delete_ldap.is_empty() || changes_ldap.delete_ldap["test_group"].is_empty()
    );
    assert!(changes_ldap.add_ldap.is_empty() || changes_ldap.add_ldap["test_group"].is_empty());

    let changes_defguard = compute_group_sync_changes(
        defguard_memberships,
        ldap_memberships,
        Authority::Defguard,
        &LDAPConfig::default(),
    );

    assert!(
        changes_defguard.delete_defguard.is_empty()
            || changes_defguard.delete_defguard["test_group"].is_empty()
    );
    assert!(
        changes_defguard.add_defguard.is_empty()
            || changes_defguard.add_defguard["test_group"].is_empty()
    );
    assert!(
        changes_defguard.delete_ldap.is_empty()
            || changes_defguard.delete_ldap["test_group"].is_empty()
    );
    assert!(
        changes_defguard.add_ldap.is_empty() || changes_defguard.add_ldap["test_group"].is_empty()
    );
}

#[sqlx::test]
fn test_ldap_authority_add_users_to_group(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let test_user = make_test_user("user1", None, None);
    let test_user_id = test_user.clone().save(&pool).await.unwrap();
    let test_user2 = make_test_user("user2", None, None);
    let mut defguard_memberships = HashMap::new();
    defguard_memberships.insert(
        "test_group".to_string(),
        HashSet::from_iter(vec![test_user_id]),
    );
    let mut ldap_memberships = HashMap::new();
    ldap_memberships.insert(
        "test_group".to_string(),
        HashSet::from_iter(vec![&test_user, &test_user2]),
    );

    let changes = compute_group_sync_changes(
        defguard_memberships,
        ldap_memberships,
        Authority::LDAP,
        &LDAPConfig::default(),
    );

    // LDAP has additional user that should be added to Defguard group
    assert!(changes.add_defguard.contains_key("test_group"));
    assert_eq!(changes.add_defguard["test_group"].len(), 1);
    assert!(changes.add_defguard["test_group"].contains(&test_user2));
}

#[sqlx::test]
fn test_ldap_authority_remove_users_from_group(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let mut defguard_memberships = HashMap::new();
    let user1 = make_test_user("user1", None, None)
        .save(&pool)
        .await
        .unwrap();
    let user2 = make_test_user("user2", None, None)
        .save(&pool)
        .await
        .unwrap();
    let user1_noid = user1.clone().as_noid();
    defguard_memberships.insert(
        "test_group".to_string(),
        HashSet::from_iter(vec![user1, user2.clone()]),
    );
    let mut ldap_memberships = HashMap::new();
    ldap_memberships.insert(
        "test_group".to_string(),
        HashSet::from_iter(vec![&user1_noid]),
    );

    let changes = compute_group_sync_changes(
        defguard_memberships,
        ldap_memberships,
        Authority::LDAP,
        &LDAPConfig::default(),
    );

    // Defguard has additional user that should be removed to match LDAP
    assert!(changes.delete_defguard.contains_key("test_group"));
    assert_eq!(changes.delete_defguard["test_group"].len(), 1);
    assert!(changes.delete_defguard["test_group"].contains(&user2));
}

#[sqlx::test]
fn test_multiple_groups_ldap_authority(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let user1 = make_test_user("user1", None, None)
        .save(&pool)
        .await
        .unwrap();
    let user2 = make_test_user("user2", None, None)
        .save(&pool)
        .await
        .unwrap();
    let user3 = make_test_user("user3", None, None)
        .save(&pool)
        .await
        .unwrap();
    let user4 = make_test_user("user4", None, None);
    let user5 = make_test_user("user5", None, None);
    let user6 = make_test_user("user6", None, None);
    let user1_noid = user1.clone().as_noid();
    let mut defguard_memberships = HashMap::new();
    defguard_memberships.insert(
        "group1".to_string(),
        HashSet::from_iter(vec![user1.clone(), user2.clone()]),
    );
    defguard_memberships.insert(
        "group2".to_string(),
        HashSet::from_iter(vec![user3.clone()]),
    );

    let mut ldap_memberships = HashMap::new();
    ldap_memberships.insert(
        "group1".to_string(),
        HashSet::from_iter(vec![&user1_noid, &user4]),
    );
    ldap_memberships.insert(
        "group3".to_string(),
        HashSet::from_iter(vec![&user5, &user6]),
    );

    let changes = compute_group_sync_changes(
        defguard_memberships,
        ldap_memberships,
        Authority::LDAP,
        &LDAPConfig::default(),
    );

    // Complex multi-group scenario with LDAP as authority:
    // group1: remove user2, add user4 (partial membership sync)
    assert!(changes.delete_defguard.contains_key("group1"));
    assert_eq!(changes.delete_defguard["group1"].len(), 1);
    assert!(changes.delete_defguard["group1"].contains(&user2));
    assert!(changes.add_defguard.contains_key("group1"));
    assert_eq!(changes.add_defguard["group1"].len(), 1);
    assert!(changes.add_defguard["group1"].contains(&user4));

    // group2: delete entirely from Defguard (not in LDAP)
    assert!(changes.delete_defguard.contains_key("group2"));
    assert_eq!(changes.delete_defguard["group2"].len(), 1);
    assert!(changes.delete_defguard["group2"].contains(&user3));

    // group3: add entirely to Defguard (new in LDAP)
    assert!(changes.add_defguard.contains_key("group3"));
    assert_eq!(changes.add_defguard["group3"].len(), 2);
    assert!(changes.add_defguard["group3"].contains(&user5));
    assert!(changes.add_defguard["group3"].contains(&user6));

    // LDAP should not be modified when it's the authority
    assert!(changes.delete_ldap.is_empty());
    assert!(changes.add_ldap.is_empty());
}

#[sqlx::test]
fn test_multiple_groups_defguard_authority(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let user1 = make_test_user("user1", None, None)
        .save(&pool)
        .await
        .unwrap();
    let user2 = make_test_user("user2", None, None)
        .save(&pool)
        .await
        .unwrap();
    let user5 = make_test_user("user5", None, None)
        .save(&pool)
        .await
        .unwrap();
    let user6 = make_test_user("user6", None, None)
        .save(&pool)
        .await
        .unwrap();
    let user1_noid = user1.clone().as_noid();
    let user4 = make_test_user("user4", None, None);
    let user3 = make_test_user("user3", None, None);
    let mut defguard_memberships = HashMap::new();
    defguard_memberships.insert(
        "group1".to_string(),
        HashSet::from_iter(vec![user1.clone(), user2.clone()]),
    );
    defguard_memberships.insert(
        "group3".to_string(),
        HashSet::from_iter(vec![user5.clone(), user6.clone()]),
    );

    let mut ldap_memberships = HashMap::new();
    ldap_memberships.insert(
        "group1".to_string(),
        HashSet::from_iter(vec![&user1_noid, &user4]),
    );
    ldap_memberships.insert("group2".to_string(), HashSet::from_iter(vec![&user3]));

    let changes = compute_group_sync_changes(
        defguard_memberships,
        ldap_memberships,
        Authority::Defguard,
        &LDAPConfig::default(),
    );

    assert!(changes.delete_defguard.is_empty());
    assert!(changes.add_defguard.is_empty());

    // group1: add user2, remove user4
    assert!(changes.add_ldap.contains_key("group1"));
    assert_eq!(changes.add_ldap["group1"].len(), 1);
    assert!(changes.add_ldap["group1"].contains(&user2));
    assert!(changes.delete_ldap.contains_key("group1"));
    assert_eq!(changes.delete_ldap["group1"].len(), 1);
    assert!(changes.delete_ldap["group1"].contains(&user4));

    // group2: should be deleted entirely
    assert!(changes.delete_ldap.contains_key("group2"));
    assert_eq!(changes.delete_ldap["group2"].len(), 1);
    assert!(changes.delete_ldap["group2"].contains(&user3));

    // group3: should be added entirely to LDAP
    assert!(changes.add_ldap.contains_key("group3"));
    assert_eq!(changes.add_ldap["group3"].len(), 2);
    assert!(changes.add_ldap["group3"].contains(&user5));
    assert!(changes.add_ldap["group3"].contains(&user6));
}

#[test]
fn test_empty_groups() {
    let mut defguard_memberships = HashMap::new();
    defguard_memberships.insert("empty_group1".to_string(), HashSet::new());

    let mut ldap_memberships = HashMap::new();
    ldap_memberships.insert("empty_group2".to_string(), HashSet::new());

    let changes = compute_group_sync_changes(
        defguard_memberships,
        ldap_memberships,
        Authority::LDAP,
        &LDAPConfig::default(),
    );

    // empty_group1 should be deleted from defguard (it's not in LDAP)
    assert!(changes.delete_defguard.contains_key("empty_group1"));
    assert_eq!(changes.delete_defguard["empty_group1"].len(), 0);
    assert!(changes.delete_defguard["empty_group1"].is_empty());

    // empty_group2 should be added to defguard (it's in LDAP)
    assert!(changes.add_defguard.contains_key("empty_group2"));
    assert_eq!(changes.add_defguard["empty_group2"].len(), 0);
    assert!(changes.add_defguard["empty_group2"].is_empty());
}

#[sqlx::test]
fn test_complex_group_memberships(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let user1 = make_test_user("user1", None, None)
        .save(&pool)
        .await
        .unwrap();
    let user2 = make_test_user("user2", None, None)
        .save(&pool)
        .await
        .unwrap();
    let user3 = make_test_user("user3", None, None)
        .save(&pool)
        .await
        .unwrap();
    let user4 = make_test_user("user4", None, None);
    let user5 = make_test_user("user5", None, None)
        .save(&pool)
        .await
        .unwrap();
    let user1_noid = user1.clone().as_noid();
    let user2_noid = user2.clone().as_noid();
    let user3_noid = user3.clone().as_noid();

    let mut defguard_memberships = HashMap::new();
    defguard_memberships.insert(
        "group1".to_string(),
        HashSet::from_iter(vec![user1.clone(), user2.clone()]),
    );
    defguard_memberships.insert(
        "group2".to_string(),
        HashSet::from_iter(vec![user1.clone(), user2.clone(), user3.clone()]),
    );
    defguard_memberships.insert(
        "group3".to_string(),
        HashSet::from_iter(vec![user1.clone(), user5.clone()]),
    );

    let mut ldap_memberships = HashMap::new();
    ldap_memberships.insert(
        "group1".to_string(),
        HashSet::from_iter(vec![&user1_noid, &user4]),
    );
    ldap_memberships.insert(
        "group2".to_string(),
        HashSet::from_iter(vec![&user1_noid, &user2_noid, &user4]),
    );
    ldap_memberships.insert(
        "group4".to_string(),
        HashSet::from_iter(vec![&user2_noid, &user3_noid]),
    );

    // Test with LDAP as authority
    let changes_ldap = compute_group_sync_changes(
        defguard_memberships.clone(),
        ldap_memberships.clone(),
        Authority::LDAP,
        &LDAPConfig::default(),
    );

    // group1: remove user2, add user4
    assert!(changes_ldap.delete_defguard.contains_key("group1"));
    assert_eq!(changes_ldap.delete_defguard["group1"].len(), 1);
    assert!(changes_ldap.delete_defguard["group1"].contains(&user2));
    assert!(changes_ldap.add_defguard.contains_key("group1"));
    assert_eq!(changes_ldap.add_defguard["group1"].len(), 1);
    assert!(changes_ldap.add_defguard["group1"].contains(&user4));

    // group2: remove user3, add user4
    assert!(changes_ldap.delete_defguard.contains_key("group2"));
    assert_eq!(changes_ldap.delete_defguard["group2"].len(), 1);
    assert!(changes_ldap.delete_defguard["group2"].contains(&user3));
    assert!(changes_ldap.add_defguard.contains_key("group2"));
    assert_eq!(changes_ldap.add_defguard["group2"].len(), 1);
    assert!(changes_ldap.add_defguard["group2"].contains(&user4));

    // group3: should be deleted entirely
    assert!(changes_ldap.delete_defguard.contains_key("group3"));
    assert_eq!(changes_ldap.delete_defguard["group3"].len(), 2);

    // group4: should be added entirely
    assert!(changes_ldap.add_defguard.contains_key("group4"));
    assert_eq!(changes_ldap.add_defguard["group4"].len(), 2);
    assert!(changes_ldap.add_defguard["group4"].contains(&user2_noid));
    assert!(changes_ldap.add_defguard["group4"].contains(&user3_noid));

    // Test with Defguard as authority
    let changes_defguard = compute_group_sync_changes(
        defguard_memberships,
        ldap_memberships,
        Authority::Defguard,
        &LDAPConfig::default(),
    );

    // group1: add user2, remove user4
    assert!(changes_defguard.add_ldap.contains_key("group1"));
    assert_eq!(changes_defguard.add_ldap["group1"].len(), 1);
    assert!(changes_defguard.add_ldap["group1"].contains(&user2));
    assert!(changes_defguard.delete_ldap.contains_key("group1"));
    assert_eq!(changes_defguard.delete_ldap["group1"].len(), 1);
    assert!(changes_defguard.delete_ldap["group1"].contains(&user4));

    // group2: add user3, remove user4
    assert!(changes_defguard.add_ldap.contains_key("group2"));
    assert_eq!(changes_defguard.add_ldap["group2"].len(), 1);
    assert!(changes_defguard.add_ldap["group2"].contains(&user3));
    assert!(changes_defguard.delete_ldap.contains_key("group2"));
    assert_eq!(changes_defguard.delete_ldap["group2"].len(), 1);
    assert!(changes_defguard.delete_ldap["group2"].contains(&user4));

    // group3: should be added entirely to ldap
    assert!(changes_defguard.add_ldap.contains_key("group3"));
    assert_eq!(changes_defguard.add_ldap["group3"].len(), 2);

    // group4: should be deleted entirely from ldap
    assert!(changes_defguard.delete_ldap.contains_key("group4"));
    assert_eq!(changes_defguard.delete_ldap["group4"].len(), 2);
    assert!(changes_defguard.delete_ldap["group4"].contains(&user2_noid));
    assert!(changes_defguard.delete_ldap["group4"].contains(&user3_noid));
}

#[test]
fn test_extract_intersecting_users_empty() {
    let mut defguard_users = Vec::<User<Id>>::new();
    let mut ldap_users = Vec::<User>::new();

    let result =
        extract_intersecting_users(&mut defguard_users, &mut ldap_users, &LDAPConfig::default());

    // Empty lists should result in no intersections and no remaining users
    assert!(result.is_empty());
    assert!(defguard_users.is_empty());
    assert!(ldap_users.is_empty());
}

#[sqlx::test]
fn test_extract_intersecting_users_with_matches(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let user1 = User::new(
        "user1",
        Some("password"),
        "Last1",
        "First1",
        "user1@example.com",
        None,
    )
    .save(&pool)
    .await
    .unwrap();

    let user2 = User::new(
        "user2",
        Some("password"),
        "Last2",
        "First2",
        "user2@example.com",
        None,
    )
    .save(&pool)
    .await
    .unwrap();

    let user3 = User::new(
        "user3",
        Some("password"),
        "Last3",
        "First3",
        "user3@example.com",
        None,
    )
    .save(&pool)
    .await
    .unwrap();

    // Create LDAP users with some matching and some unique usernames
    let ldap_user1 = User::new(
        "user1",
        Some("ldap_password"),
        "LdapLast1",
        "LdapFirst1",
        "ldap_user1@example.com",
        None,
    );

    let ldap_user2 = User::new(
        "user2",
        Some("ldap_password"),
        "LdapLast2",
        "LdapFirst2",
        "ldap_user2@example.com",
        None,
    );

    let ldap_user4 = User::new(
        "user4",
        Some("ldap_password"),
        "LdapLast4",
        "LdapFirst4",
        "ldap_user4@example.com",
        None,
    );

    let mut defguard_users = vec![user1, user2, user3];
    let mut ldap_users = vec![ldap_user1, ldap_user2, ldap_user4];

    let result =
        extract_intersecting_users(&mut defguard_users, &mut ldap_users, &LDAPConfig::default());

    // Should extract users with matching usernames, leaving unmatched users in original lists
    assert_eq!(result.len(), 2);

    // Verify the matched pairs are correctly identified by username
    let usernames: Vec<(&str, &str)> = result
        .iter()
        .map(|(ldap, defguard)| (ldap.username.as_str(), defguard.username.as_str()))
        .collect();

    assert!(usernames.contains(&("user1", "user1")));
    assert!(usernames.contains(&("user2", "user2")));

    // Verify unmatched users remain in their respective lists
    assert_eq!(defguard_users.len(), 1);
    assert_eq!(defguard_users[0].username, "user3");

    assert_eq!(ldap_users.len(), 1);
    assert_eq!(ldap_users[0].username, "user4");
}

#[sqlx::test]
fn test_extract_intersecting_users_no_matches(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let mut defguard_users = vec![
        User::new(
            "user1",
            Some("password"),
            "Last1",
            "First1",
            "user1@example.com",
            None,
        )
        .save(&pool)
        .await
        .unwrap(),
    ];

    let mut ldap_users = vec![User::new(
        "user2",
        Some("password"),
        "Last",
        "First",
        "email@example.com",
        None,
    )];

    let result =
        extract_intersecting_users(&mut defguard_users, &mut ldap_users, &LDAPConfig::default());

    // No matching usernames should result in no intersections and all users remaining
    assert!(result.is_empty());
    assert_eq!(defguard_users.len(), 1);
    assert_eq!(ldap_users.len(), 1);
}

#[sqlx::test]
async fn test_fix_missing_user_path(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let _ = initialize_current_settings(&pool).await;

    // User exists in both Defguard and LDAP with matching RDN
    {
        let mut ldap_conn = super::LDAPConnection::create().await.unwrap();
        let config = ldap_conn.config.clone();

        let mut user1 = User::new(
            "user1",
            Some("test_password"),
            "Last",
            "First",
            "user1@example.com",
            None,
        );
        user1.ldap_rdn = Some("user1".to_string());
        user1.ldap_user_path = None;
        user1.from_ldap = true;
        let user1 = user1.save(&pool).await.unwrap();

        let mut ldap_user = user1.clone().as_noid();
        ldap_user.ldap_user_path = Some("ou=users,dc=example,dc=com".to_string());
        ldap_conn
            .test_client_mut()
            .add_test_user(&ldap_user, &config);

        ldap_conn.fix_missing_user_path(&pool).await.unwrap();

        let updated_user = User::find_by_id(&pool, user1.id).await.unwrap().unwrap();
        assert_eq!(
            updated_user.ldap_user_path,
            Some("ou=users,dc=example,dc=com".to_string())
        );
    }

    // User with mismatched RDN should not be updated
    {
        let mut ldap_conn = super::LDAPConnection::create().await.unwrap();
        let config = ldap_conn.config.clone();

        let mut user2 = User::new(
            "user2",
            Some("test_password"),
            "Last",
            "First",
            "user2@example.com",
            None,
        );
        user2.ldap_rdn = Some("user2_defguard".to_string());
        user2.ldap_user_path = None;
        user2.from_ldap = true;
        let user2 = user2.save(&pool).await.unwrap();

        let mut ldap_user = User::new(
            "user2",
            Some("test_password"),
            "Last",
            "First",
            "user2@example.com",
            None,
        );
        ldap_user.ldap_rdn = Some("user2_ldap".to_string());
        ldap_user.ldap_user_path = Some("ou=users,dc=example,dc=com".to_string());
        ldap_conn
            .test_client_mut()
            .add_test_user(&ldap_user, &config);

        ldap_conn.fix_missing_user_path(&pool).await.unwrap();

        // path was NOT updated due to RDN mismatch
        let updated_user = User::find_by_id(&pool, user2.id).await.unwrap().unwrap();
        assert_eq!(updated_user.ldap_user_path, None);
    }

    // User not found in LDAP should remain unchanged
    {
        let mut ldap_conn = super::LDAPConnection::create().await.unwrap();

        let mut user3 = User::new(
            "user3",
            Some("test_password"),
            "Last",
            "First",
            "user3@example.com",
            None,
        );
        user3.ldap_rdn = Some("user3".to_string());
        user3.ldap_user_path = None; // Missing path
        user3.from_ldap = true;
        let user3 = user3.save(&pool).await.unwrap();

        ldap_conn.fix_missing_user_path(&pool).await.unwrap();

        let updated_user = User::find_by_id(&pool, user3.id).await.unwrap().unwrap();
        assert_eq!(updated_user.ldap_user_path, None);
    }

    // User that already has path should not be changed
    {
        let mut ldap_conn = super::LDAPConnection::create().await.unwrap();
        let config = ldap_conn.config.clone();

        let mut user4 = User::new(
            "user4",
            Some("test_password"),
            "Last",
            "First",
            "user4@example.com",
            None,
        );
        user4.ldap_rdn = Some("user4".to_string());
        user4.ldap_user_path = Some("ou=existing,dc=example,dc=com".to_string());
        user4.from_ldap = true;
        let user4 = user4.save(&pool).await.unwrap();

        let mut ldap_user = user4.clone().as_noid();
        ldap_user.ldap_user_path = Some("ou=different,dc=example,dc=com".to_string());
        ldap_conn
            .test_client_mut()
            .add_test_user(&ldap_user, &config);

        ldap_conn.fix_missing_user_path(&pool).await.unwrap();

        let updated_user = User::find_by_id(&pool, user4.id).await.unwrap().unwrap();
        assert_eq!(
            updated_user.ldap_user_path,
            Some("ou=existing,dc=example,dc=com".to_string())
        );
        assert_eq!(updated_user.id, user4.id);
    }
}

#[sqlx::test]
async fn test_sync_users_with_empty_paths_and_nested_ous(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let _ = initialize_current_settings(&pool).await;
    set_test_license_business();

    let mut ldap_conn = super::LDAPConnection::create().await.unwrap();
    let config = ldap_conn.config.clone();

    // user with empty path gets synced from nested OU
    {
        ldap_conn.test_client_mut().clear_events();
        let mut user1 = User::new(
            "user1",
            Some("test_password"),
            "Last",
            "First",
            "user1@example.com",
            None,
        );
        user1.ldap_rdn = Some("user1".to_string());
        user1.ldap_user_path = None; // Empty path initially
        user1.from_ldap = true;
        let user1 = user1.save(&pool).await.unwrap();
        let original_id = user1.id;

        let mut ldap_user = user1.clone().as_noid();
        ldap_user.ldap_user_path =
            Some("ou=developers,ou=engineering,ou=users,dc=example,dc=com".to_string());
        ldap_conn
            .test_client_mut()
            .add_test_user(&ldap_user, &config);

        ldap_conn.sync(&pool, false).await.unwrap();

        // verify that the user path was updated and ID remains the same
        let updated_user = User::find_by_id(&pool, original_id).await.unwrap().unwrap();
        assert_eq!(updated_user.id, original_id);
        assert_eq!(
            updated_user.ldap_user_path,
            Some("ou=developers,ou=engineering,ou=users,dc=example,dc=com".to_string())
        );
        assert_eq!(updated_user.username, "user1");
        assert!(ldap_conn.test_client.get_events().is_empty());
    }

    // user with empty path in deeply nested OU structure
    {
        ldap_conn.test_client_mut().clear_events();
        let mut user2 = User::new(
            "user2",
            Some("test_password"),
            "Last",
            "First",
            "user2@example.com",
            None,
        );
        user2.ldap_rdn = Some("user2".to_string());
        user2.ldap_user_path = None;
        user2.from_ldap = true;
        let user2 = user2.save(&pool).await.unwrap();
        let original_id = user2.id;

        let mut ldap_user = user2.clone().as_noid();
        ldap_user.ldap_user_path =
            Some("ou=qa,ou=testers,ou=internal,ou=company,ou=users,dc=example,dc=com".to_string());
        ldap_conn
            .test_client_mut()
            .add_test_user(&ldap_user, &config);

        ldap_conn.sync(&pool, false).await.unwrap();

        let updated_user = User::find_by_id(&pool, original_id).await.unwrap().unwrap();
        assert_eq!(updated_user.id, original_id);
        assert_eq!(
            updated_user.ldap_user_path,
            Some("ou=qa,ou=testers,ou=internal,ou=company,ou=users,dc=example,dc=com".to_string())
        );
        assert_eq!(updated_user.username, "user2");
        assert!(ldap_conn.test_client.get_events().is_empty());
    }

    // user exists with matching DN - should update path correctly
    {
        ldap_conn.test_client_mut().clear_events();
        let mut user3 = User::new(
            "user3",
            Some("test_password"),
            "Last",
            "First",
            "user3@example.com",
            None,
        );
        user3.ldap_rdn = Some("user3".to_string());
        user3.ldap_user_path = Some("ou=users,dc=example,dc=com".to_string());
        user3.from_ldap = true;
        let user3 = user3.save(&pool).await.unwrap();
        let original_id = user3.id;

        let mut ldap_user = user3.clone().as_noid();
        ldap_user.ldap_user_path = Some("ou=users,dc=example,dc=com".to_string());
        ldap_user.email = "updated3@example.com".to_string();
        ldap_conn
            .test_client_mut()
            .add_test_user(&ldap_user, &config);

        ldap_conn.sync(&pool, false).await.unwrap();

        // verify user still exists with same ID and path remains consistent
        let updated_user = User::find_by_id(&pool, original_id).await.unwrap().unwrap();
        assert_eq!(updated_user.id, original_id);
        assert_eq!(
            updated_user.ldap_user_path,
            Some("ou=users,dc=example,dc=com".to_string())
        );
        assert_eq!(updated_user.username, "user3");
        assert_eq!(updated_user.email, "updated3@example.com");
        assert!(ldap_conn.test_client.get_events().is_empty());
    }

    // multiple users with empty paths in different nested OUs
    {
        ldap_conn.test_client_mut().clear_events();
        let mut user4 = User::new(
            "user4",
            Some("test_password"),
            "Last",
            "First",
            "user4@example.com",
            None,
        );
        user4.ldap_rdn = Some("user4".to_string());
        user4.ldap_user_path = None; // Empty path
        user4.from_ldap = true;
        let user4 = user4.save(&pool).await.unwrap();
        let original_id4 = user4.id;

        let mut user5 = User::new(
            "user5",
            Some("test_password"),
            "Last",
            "First",
            "user5@example.com",
            None,
        );
        user5.ldap_rdn = Some("user5".to_string());
        user5.ldap_user_path = None; // Empty path
        user5.from_ldap = true;
        let user5 = user5.save(&pool).await.unwrap();
        let original_id5 = user5.id;

        let mut ldap_user4 = user4.clone().as_noid();
        ldap_user4.ldap_user_path = Some("ou=admins,ou=it,ou=users,dc=example,dc=com".to_string());
        ldap_conn
            .test_client_mut()
            .add_test_user(&ldap_user4, &config);

        let mut ldap_user5 = user5.clone().as_noid();
        ldap_user5.ldap_user_path =
            Some("ou=support,ou=helpdesk,ou=users,dc=example,dc=com".to_string());
        ldap_conn
            .test_client_mut()
            .add_test_user(&ldap_user5, &config);

        ldap_conn.sync(&pool, false).await.unwrap();

        let updated_user4 = User::find_by_id(&pool, original_id4)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(updated_user4.id, original_id4);
        assert_eq!(
            updated_user4.ldap_user_path,
            Some("ou=admins,ou=it,ou=users,dc=example,dc=com".to_string())
        );
        assert_eq!(updated_user4.username, "user4");

        let updated_user5 = User::find_by_id(&pool, original_id5)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(updated_user5.id, original_id5);
        assert_eq!(
            updated_user5.ldap_user_path,
            Some("ou=support,ou=helpdesk,ou=users,dc=example,dc=com".to_string())
        );
        assert_eq!(updated_user5.username, "user5");
        assert!(ldap_conn.test_client.get_events().is_empty());
    }

    // user with matching DN should get attributes synced
    {
        ldap_conn.test_client_mut().clear_events();
        let mut user6 = User::new(
            "user6",
            Some("test_password"),
            "Last",
            "First",
            "user6@example.com",
            None,
        );
        user6.ldap_rdn = Some("user6".to_string());
        user6.ldap_user_path =
            Some("ou=support,ou=helpdesk,ou=users,dc=example,dc=com".to_string());
        user6.from_ldap = true;
        let user6 = user6.save(&pool).await.unwrap();
        let original_id = user6.id;

        let mut ldap_user = user6.clone().as_noid();
        ldap_user.first_name = "UpdatedFirst".to_string(); // Updated attribute
        ldap_conn
            .test_client_mut()
            .add_test_user(&ldap_user, &config);

        ldap_conn.sync(&pool, false).await.unwrap();

        // verify user still exists with same ID and correct path
        let updated_user = User::find_by_id(&pool, original_id).await.unwrap().unwrap();
        assert_eq!(updated_user.id, original_id);
        assert_eq!(
            updated_user.ldap_user_path,
            Some("ou=support,ou=helpdesk,ou=users,dc=example,dc=com".to_string())
        );
        assert_eq!(updated_user.username, "user6");
        assert_eq!(updated_user.first_name, "UpdatedFirst");
        assert!(ldap_conn.test_client.get_events().is_empty());
    }

    // user in LDAP only should be added to Defguard
    {
        ldap_conn.test_client_mut().clear_events();
        let mut ldap_only_user = User::new(
            "ldap_only_user",
            Some("test_password"),
            "Last",
            "First",
            "ldap_only@example.com",
            None,
        );
        ldap_only_user.ldap_rdn = Some("ldap_only_user".to_string());
        ldap_only_user.ldap_user_path =
            Some("ou=dev-team,ou=project-alpha,ou=r&d,ou=users,dc=example,dc=com".to_string());
        ldap_conn
            .test_client_mut()
            .add_test_user(&ldap_only_user, &config);

        let users_before = User::all(&pool).await.unwrap();
        let count_before = users_before.len();

        ldap_conn.sync(&pool, false).await.unwrap();

        let users_after = User::all(&pool).await.unwrap();
        assert_eq!(users_after.len(), count_before + 1);

        let added_user = users_after
            .iter()
            .find(|u| u.username == "ldap_only_user")
            .expect("User should be added to Defguard");

        assert_eq!(
            added_user.ldap_user_path,
            Some("ou=dev-team,ou=project-alpha,ou=r&d,ou=users,dc=example,dc=com".to_string())
        );
        assert_eq!(added_user.username, "ldap_only_user");
        assert!(added_user.from_ldap);
        assert!(ldap_conn.test_client.get_events().is_empty());
    }
}

#[sqlx::test]
async fn test_sync_simple_nested_ou_changes(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let _ = initialize_current_settings(&pool).await;
    set_test_license_business();

    let mut ldap_conn = super::LDAPConnection::create().await.unwrap();
    let config = ldap_conn.config.clone();

    let group1 = Group::new("developers").save(&pool).await.unwrap();

    let mut user1 = make_test_user("user1", None, None);
    user1.ldap_user_path = Some("ou=engineering,ou=dept,dc=example,dc=com".to_string());
    user1.ldap_rdn = Some("user1".to_string());
    user1.from_ldap = true;
    let user1 = user1.save(&pool).await.unwrap();
    user1.add_to_group(&pool, &group1).await.unwrap();

    let mut ldap_user1 = user1.clone().as_noid();
    ldap_user1.first_name = "UpdatedFirst1".to_string();
    ldap_conn
        .test_client_mut()
        .add_test_user(&ldap_user1, &config);
    ldap_conn.test_client_mut().add_test_membership(
        &group1.clone().as_noid(),
        &ldap_user1,
        &config,
    );

    let mut ldap_only_user = make_test_user("user2", None, None);
    ldap_only_user.ldap_user_path =
        Some("ou=contractors,ou=external,ou=projects,ou=temp,dc=example,dc=com".to_string());
    ldap_conn
        .test_client_mut()
        .add_test_user(&ldap_only_user, &config);
    ldap_conn
        .test_client_mut()
        .add_test_group(&group1.clone().as_noid(), &config);
    ldap_conn.test_client_mut().add_test_membership(
        &group1.clone().as_noid(),
        &ldap_only_user,
        &config,
    );

    ldap_conn.sync(&pool, false).await.unwrap();

    // user1 should be updated
    let updated_user1 = User::find_by_id(&pool, user1.id).await.unwrap().unwrap();
    assert_eq!(updated_user1.first_name, "UpdatedFirst1");
    assert_eq!(updated_user1.id, user1.id);
    assert_eq!(
        updated_user1.ldap_user_path,
        Some("ou=engineering,ou=dept,dc=example,dc=com".to_string())
    );

    // user2 should be added
    let added_user = User::find_by_username(&pool, "user2")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        added_user.ldap_user_path,
        Some("ou=contractors,ou=external,ou=projects,ou=temp,dc=example,dc=com".to_string())
    );
    assert!(added_user.from_ldap);

    let user1_groups = updated_user1.member_of_names(&pool).await.unwrap();
    assert!(user1_groups.contains(&"developers".to_string()));

    let user2_groups = added_user.member_of_names(&pool).await.unwrap();
    assert!(user2_groups.contains(&"developers".to_string()));

    assert!(ldap_conn.test_client.get_events().is_empty());
}

#[sqlx::test]
async fn test_sync_incremental_with_nested_ou_conflicts(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let _ = initialize_current_settings(&pool).await;
    set_test_license_business();

    let mut ldap_conn = super::LDAPConnection::create().await.unwrap();
    let config = ldap_conn.config.clone();

    let mut user1 = make_test_user("user1", None, None);
    user1.ldap_user_path = Some("ou=dept,dc=example,dc=com".to_string());
    user1.ldap_rdn = Some("user1".to_string());
    user1.from_ldap = true;
    let user1 = user1.save(&pool).await.unwrap();

    let mut user2 = make_test_user("user2", None, None);
    user2.ldap_user_path = None;
    user2.ldap_rdn = Some("user2".to_string());
    user2.from_ldap = true;
    let user2 = user2.save(&pool).await.unwrap();

    let mut user3 = make_test_user("user3", None, None);
    user3.ldap_user_path = Some("ou=wrong,ou=path,dc=example,dc=com".to_string());
    user3.ldap_rdn = Some("different_rdn".to_string());
    user3.from_ldap = true;
    let user3 = user3.save(&pool).await.unwrap();

    let mut ldap_user1 = user1.clone().as_noid();
    ldap_user1.ldap_user_path = Some("ou=dept,dc=example,dc=com".to_string());
    ldap_user1.email = "updated1@example.com".to_string();
    ldap_conn
        .test_client_mut()
        .add_test_user(&ldap_user1, &config);

    let mut ldap_user2 = user2.clone().as_noid();
    ldap_user2.ldap_user_path =
        Some("ou=found,ou=department,ou=division,dc=example,dc=com".to_string());
    ldap_user2.first_name = "FoundFirst".to_string();
    ldap_conn
        .test_client_mut()
        .add_test_user(&ldap_user2, &config);

    let mut ldap_user3 = user3.clone().as_noid();
    ldap_user3.ldap_rdn = Some("different_rdn".to_string());
    ldap_user3.ldap_user_path = Some("ou=correct,ou=path,dc=example,dc=com".to_string());
    ldap_user3.last_name = "UpdatedLast".to_string();
    ldap_conn
        .test_client_mut()
        .add_test_user(&ldap_user3, &config);

    ldap_conn.sync(&pool, false).await.unwrap();

    // user1: should get updated attributes and path from intersecting users
    let updated_user1 = User::find_by_id(&pool, user1.id).await.unwrap().unwrap();
    assert_eq!(updated_user1.email, "updated1@example.com");
    assert_eq!(
        updated_user1.ldap_user_path,
        Some("ou=dept,dc=example,dc=com".to_string())
    );

    let updated_user2 = User::find_by_id(&pool, user2.id).await.unwrap().unwrap();
    assert_eq!(updated_user2.first_name, "FoundFirst");
    assert_eq!(
        updated_user2.ldap_user_path,
        Some("ou=found,ou=department,ou=division,dc=example,dc=com".to_string())
    );

    // user3 should be re-created as it has a different path
    let deleted_user3 = User::find_by_id(&pool, user3.id).await.unwrap();
    assert!(deleted_user3.is_none());

    let created_user3 = User::find_by_username(&pool, "different_rdn")
        .await
        .unwrap()
        .unwrap();

    assert_ne!(created_user3.id, user3.id);
    assert_eq!(created_user3.first_name, "first name");
    assert_eq!(created_user3.last_name, "UpdatedLast");
    assert_eq!(
        created_user3.ldap_user_path,
        Some("ou=correct,ou=path,dc=example,dc=com".to_string())
    );
    assert!(ldap_conn.test_client.get_events().is_empty());
}

#[sqlx::test]
async fn test_sync_defguard_authority_with_complex_nested_ous(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let _ = initialize_current_settings(&pool).await;

    let mut settings = Settings::get_current_settings();
    settings.ldap_is_authoritative = false;
    update_current_settings(&pool, settings).await.unwrap();

    let mut ldap_conn = super::LDAPConnection::create().await.unwrap();
    let config = ldap_conn.config.clone();

    let group1 = Group::new("backend-devs").save(&pool).await.unwrap();
    let group2 = Group::new("frontend-devs").save(&pool).await.unwrap();

    let mut user1 = make_test_user("user1", None, None);
    user1.ldap_user_path =
        Some("ou=backend,ou=engineering,ou=product,ou=company,dc=example,dc=com".to_string());
    user1.ldap_rdn = Some("user1".to_string());
    user1.from_ldap = true;
    let user1 = user1.save(&pool).await.unwrap();
    user1.add_to_group(&pool, &group1).await.unwrap();

    let mut user2 = make_test_user("user2", None, None);
    user2.ldap_user_path =
        Some("ou=frontend,ou=ui-ux,ou=design,ou=creative,dc=example,dc=com".to_string());
    user2.ldap_rdn = Some("user2".to_string());
    user2.from_ldap = true;
    let user2 = user2.save(&pool).await.unwrap();
    user2.add_to_group(&pool, &group2).await.unwrap();

    let mut defguard_only_user = make_test_user("user3", None, None);
    defguard_only_user.ldap_user_path =
        Some("ou=devops,ou=infrastructure,ou=operations,dc=example,dc=com".to_string());
    defguard_only_user.from_ldap = false; // Not from LDAP initially
    let defguard_only_user = defguard_only_user.save(&pool).await.unwrap();
    defguard_only_user
        .add_to_group(&pool, &group1)
        .await
        .unwrap();
    defguard_only_user
        .add_to_group(&pool, &group2)
        .await
        .unwrap();

    let mut ldap_user1 = user1.clone().as_noid();
    ldap_user1.email = "old1@example.com".to_string();
    ldap_conn
        .test_client_mut()
        .add_test_user(&ldap_user1, &config);
    ldap_conn.test_client_mut().add_test_membership(
        &group2.clone().as_noid(),
        &ldap_user1,
        &config,
    );

    let mut ldap_user2 = user2.clone().as_noid();
    ldap_user2.first_name = "OldFirst".to_string();
    ldap_conn
        .test_client_mut()
        .add_test_user(&ldap_user2, &config);

    let mut ldap_only_user = make_test_user("user4", None, None);
    ldap_only_user.ldap_user_path =
        Some("ou=temp,ou=contractors,ou=external,dc=example,dc=com".to_string());
    ldap_conn
        .test_client_mut()
        .add_test_user(&ldap_only_user, &config);
    ldap_conn.test_client_mut().add_test_membership(
        &group1.clone().as_noid(),
        &ldap_only_user,
        &config,
    );

    let initial_ldap_users = ldap_conn.get_all_users().await.unwrap();
    let initial_count = initial_ldap_users.len();

    ldap_conn.sync(&pool, true).await.unwrap();

    // intersecting users still exist in Defguard with same IDs
    let updated_user1 = User::find_by_id(&pool, user1.id).await.unwrap().unwrap();
    assert_eq!(updated_user1.id, user1.id);
    assert_eq!(
        updated_user1.ldap_user_path,
        Some("ou=backend,ou=engineering,ou=product,ou=company,dc=example,dc=com".to_string())
    );
    assert_ne!(updated_user1.email, "old1@example.com"); // Should not be overridden by LDAP with Defguard authority

    let updated_user2 = User::find_by_id(&pool, user2.id).await.unwrap().unwrap();
    assert_eq!(updated_user2.id, user2.id);
    assert_eq!(
        updated_user2.ldap_user_path,
        Some("ou=frontend,ou=ui-ux,ou=design,ou=creative,dc=example,dc=com".to_string())
    );
    assert_ne!(updated_user2.first_name, "OldFirst"); // Should not be overridden by LDAP with Defguard authority

    let updated_defguard_only = User::find_by_id(&pool, defguard_only_user.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(updated_defguard_only.id, defguard_only_user.id);
    assert_eq!(
        updated_defguard_only.ldap_user_path,
        Some("ou=devops,ou=infrastructure,ou=operations,dc=example,dc=com".to_string())
    );

    // LDAP-only user was deleted from Defguard (with Defguard authority)
    let removed_user = User::find_by_username(&pool, "user4").await.unwrap();
    assert!(
        removed_user.is_none(),
        "LDAP-only user should have been deleted from Defguard with Defguard authority"
    );

    let final_ldap_users = ldap_conn.get_all_users().await.unwrap();
    assert_eq!(final_ldap_users.len(), initial_count);

    // group memberships were pushed to LDAP from Defguard
    let user1_groups = updated_user1.member_of_names(&pool).await.unwrap();
    assert!(user1_groups.contains(&"backend-devs".to_string()));

    let user2_groups = updated_user2.member_of_names(&pool).await.unwrap();
    assert!(user2_groups.contains(&"frontend-devs".to_string()));

    let user3_groups = updated_defguard_only.member_of_names(&pool).await.unwrap();
    assert!(user3_groups.contains(&"backend-devs".to_string()));
    assert!(user3_groups.contains(&"frontend-devs".to_string()));
    assert!(!ldap_conn.test_client.get_events().is_empty());
}

#[sqlx::test]
async fn test_sync_with_ou_path_edge_cases(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let _ = initialize_current_settings(&pool).await;
    let mut ldap_conn = super::LDAPConnection::create().await.unwrap();
    let config = ldap_conn.config.clone();

    // user with missing path and special characters in OU names
    let mut user2 = make_test_user("user2", None, None);
    user2.ldap_user_path = None;
    user2.ldap_rdn = Some("user2".to_string());
    user2.from_ldap = true;
    let user2 = user2.save(&pool).await.unwrap();

    let mut ldap_user2 = user2.clone().as_noid();
    ldap_user2.ldap_user_path =
        Some("ou=r&d,ou=research-development,ou=company-name,dc=example,dc=com".to_string());
    ldap_conn
        .test_client_mut()
        .add_test_user(&ldap_user2, &config);

    // user with missing path and minimal OU structure
    let mut user3 = make_test_user("user3", None, None);
    user3.ldap_user_path = None;
    user3.ldap_rdn = Some("user3".to_string());
    user3.from_ldap = true;
    let user3 = user3.save(&pool).await.unwrap();

    let mut ldap_user3 = user3.clone().as_noid();
    ldap_user3.ldap_user_path = Some("ou=users,dc=example,dc=com".to_string());
    ldap_conn
        .test_client_mut()
        .add_test_user(&ldap_user3, &config);

    // user with different DN structure - will be deleted and recreated
    let mut user4 = make_test_user("user4", None, None);
    user4.ldap_user_path = Some("ou=old-structure,dc=example,dc=com".to_string());
    user4.ldap_rdn = Some("user4".to_string());
    user4.from_ldap = true;
    let user4 = user4.save(&pool).await.unwrap();

    let mut ldap_user4 = make_test_user("user4", None, None);
    ldap_user4.ldap_user_path =
        Some("ou=new-structure,ou=reorganized,dc=example,dc=com".to_string());
    ldap_user4.ldap_rdn = Some("user4".to_string());
    ldap_user4.email = "updated4@example.com".to_string();
    ldap_conn
        .test_client_mut()
        .add_test_user(&ldap_user4, &config);

    ldap_conn.sync(&pool, false).await.unwrap();

    let updated_user2 = User::find_by_id(&pool, user2.id).await.unwrap().unwrap();
    assert_eq!(updated_user2.id, user2.id); // Same user
    assert_eq!(
        updated_user2.ldap_user_path,
        Some("ou=r&d,ou=research-development,ou=company-name,dc=example,dc=com".to_string())
    );

    let updated_user3 = User::find_by_id(&pool, user3.id).await.unwrap().unwrap();
    assert_eq!(updated_user3.id, user3.id); // Same user
    assert_eq!(
        updated_user3.ldap_user_path,
        Some("ou=users,dc=example,dc=com".to_string())
    );

    // The old user should be deleted and a new one created
    let old_user4_deleted = User::find_by_id(&pool, user4.id).await.unwrap();
    assert!(
        old_user4_deleted.is_none(),
        "Old user4 should be deleted when DN differs"
    );

    let new_user4 = User::find_by_username(&pool, "user4")
        .await
        .unwrap()
        .unwrap();
    assert_ne!(new_user4.id, user4.id, "New user4 should have different ID");
    assert_eq!(new_user4.email, "updated4@example.com");
    assert_eq!(
        new_user4.ldap_user_path,
        Some("ou=new-structure,ou=reorganized,dc=example,dc=com".to_string())
    );
    assert!(ldap_conn.test_client.get_events().is_empty());
}

#[sqlx::test]
async fn test_sync_group_membership_with_intersecting_users(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let _ = initialize_current_settings(&pool).await;
    set_test_license_business();

    let mut ldap_conn = super::LDAPConnection::create().await.unwrap();
    let config = ldap_conn.config.clone();

    let group1 = Group::new("engineering").save(&pool).await.unwrap();
    let group2 = Group::new("management").save(&pool).await.unwrap();

    let mut user1 = make_test_user("user1", None, None);
    user1.ldap_user_path = Some("ou=backend,ou=engineering,dc=example,dc=com".to_string());
    user1.ldap_rdn = Some("user1".to_string());
    user1.from_ldap = true;
    let user1 = user1.save(&pool).await.unwrap();
    user1.add_to_group(&pool, &group1).await.unwrap();

    let mut user2 = make_test_user("user2", None, None);
    user2.ldap_user_path = Some("ou=frontend,ou=engineering,dc=example,dc=com".to_string());
    user2.ldap_rdn = Some("user2".to_string());
    user2.from_ldap = true;
    let user2 = user2.save(&pool).await.unwrap();
    user2.add_to_group(&pool, &group1).await.unwrap();

    let ldap_user1 = user1.clone().as_noid();
    ldap_conn
        .test_client_mut()
        .add_test_user(&ldap_user1, &config);
    ldap_conn.test_client_mut().add_test_membership(
        &group1.clone().as_noid(),
        &ldap_user1,
        &config,
    );
    ldap_conn.test_client_mut().add_test_membership(
        &group2.clone().as_noid(),
        &ldap_user1,
        &config,
    );

    let ldap_user2 = user2.clone().as_noid();
    ldap_conn
        .test_client_mut()
        .add_test_user(&ldap_user2, &config);
    ldap_conn.test_client_mut().add_test_membership(
        &group2.clone().as_noid(),
        &ldap_user2,
        &config,
    );

    ldap_conn.sync(&pool, false).await.unwrap();

    let updated_user1 = User::find_by_id(&pool, user1.id).await.unwrap().unwrap();
    assert_eq!(updated_user1.id, user1.id);
    assert_eq!(
        updated_user1.ldap_user_path,
        Some("ou=backend,ou=engineering,dc=example,dc=com".to_string())
    );

    let updated_user2 = User::find_by_id(&pool, user2.id).await.unwrap().unwrap();
    assert_eq!(updated_user2.id, user2.id);
    assert_eq!(
        updated_user2.ldap_user_path,
        Some("ou=frontend,ou=engineering,dc=example,dc=com".to_string())
    );

    let user1_groups = updated_user1.member_of_names(&pool).await.unwrap();
    assert!(user1_groups.contains(&"engineering".to_string()));
    assert!(user1_groups.contains(&"management".to_string())); // Added from LDAP

    let user2_groups = updated_user2.member_of_names(&pool).await.unwrap();
    assert!(user2_groups.contains(&"management".to_string()));
    assert!(!user2_groups.contains(&"engineering".to_string())); // Removed from LDAP
    assert!(ldap_conn.test_client.get_events().is_empty());
}

#[sqlx::test]
async fn test_get_empty_user_path(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let _ = initialize_current_settings(&pool).await;
    let user = make_test_user("testuser", None, None);
    let user = user.save(&pool).await.unwrap();

    let mut users = get_users_without_ldap_path(&pool).await.unwrap();
    let user_found = users.pop().unwrap();
    assert_eq!(user_found.username, user.username);
}

#[test]
fn test_extract_dn_value() {
    assert_eq!(
        extract_rdn_value("cn=testuser,dc=example,dc=com"),
        Some("testuser".to_string())
    );
    assert_eq!(
        extract_rdn_value("cn=Test User,dc=example,dc=com"),
        Some("Test User".to_string())
    );
    assert_eq!(
        extract_rdn_value("cn=user.name+123,dc=example,dc=com"),
        Some("user.name+123".to_string())
    );
    assert_eq!(extract_rdn_value("invalid-dn"), None);
    assert_eq!(extract_rdn_value("cn=onlyvalue"), None);
    assert_eq!(
        extract_rdn_value("cn=,dc=example,dc=com"),
        Some(String::new())
    );
    assert_eq!(extract_rdn_value(""), None);
}

#[test]
fn test_from_searchentry() {
    // all attributes
    {
        let mut attrs = HashMap::new();
        attrs.insert("sn".to_string(), vec!["lastname1".to_string()]);
        attrs.insert("givenName".to_string(), vec!["firstname1".to_string()]);
        attrs.insert("mail".to_string(), vec!["user1@example.com".to_string()]);
        attrs.insert("mobile".to_string(), vec!["1234567890".to_string()]);

        let entry = SearchEntry {
            dn: "cn=user1,dc=example,dc=com".to_string(),
            attrs,
            bin_attrs: HashMap::new(),
        };

        let user = user_from_searchentry(&entry, "user1", Some("password123")).unwrap();

        assert_eq!(user.username, "user1");
        assert_eq!(user.last_name, "lastname1");
        assert_eq!(user.first_name, "firstname1");
        assert_eq!(user.email, "user1@example.com");
        assert_eq!(user.phone, Some("1234567890".to_string()));
        assert!(user.from_ldap);
    }

    // without mobile
    {
        let mut attrs = HashMap::new();
        attrs.insert("sn".to_string(), vec!["lastname1".to_string()]);
        attrs.insert("givenName".to_string(), vec!["firstname1".to_string()]);
        attrs.insert("mail".to_string(), vec!["user1@example.com".to_string()]);

        let entry = SearchEntry {
            dn: "cn=user1,dc=example,dc=com".to_string(),
            attrs,
            bin_attrs: HashMap::new(),
        };

        let user = user_from_searchentry(&entry, "user1", None).unwrap();

        assert_eq!(user.username, "user1");
        assert_eq!(user.last_name, "lastname1");
        assert_eq!(user.first_name, "firstname1");
        assert_eq!(user.email, "user1@example.com");
        assert_eq!(user.phone, None);
        assert!(user.from_ldap);
    }

    // missing givenName attribute
    {
        let mut attrs = HashMap::new();
        attrs.insert("sn".to_string(), vec!["lastname1".to_string()]);
        attrs.insert("mail".to_string(), vec!["user1@example.com".to_string()]);

        let entry = SearchEntry {
            dn: "cn=user1,dc=example,dc=com".to_string(),
            attrs,
            bin_attrs: HashMap::new(),
        };

        let result = user_from_searchentry(&entry, "user1", None);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            LdapError::MissingAttribute(attr) if attr == "givenName"
        ));
    }

    // missing sn attribute
    {
        let mut attrs = HashMap::new();
        attrs.insert("givenName".to_string(), vec!["firstname1".to_string()]);
        attrs.insert("mail".to_string(), vec!["user1@example.com".to_string()]);

        let entry = SearchEntry {
            dn: "cn=user1,dc=example,dc=com".to_string(),
            attrs,
            bin_attrs: HashMap::new(),
        };

        let result = user_from_searchentry(&entry, "user1", None);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            LdapError::MissingAttribute(attr) if attr == "sn"
        ));
    }

    // missing mail attribute
    {
        let mut attrs = HashMap::new();
        attrs.insert("sn".to_string(), vec!["lastname1".to_string()]);
        attrs.insert("givenName".to_string(), vec!["firstname1".to_string()]);

        let entry = SearchEntry {
            dn: "cn=user1,dc=example,dc=com".to_string(),
            attrs,
            bin_attrs: HashMap::new(),
        };

        let result = user_from_searchentry(&entry, "user1", None);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            LdapError::MissingAttribute(attr) if attr == "mail"
        ));
    }

    // empty attribute values
    {
        let mut attrs = HashMap::new();
        attrs.insert("sn".to_string(), vec![]);
        attrs.insert("givenName".to_string(), vec!["firstname1".to_string()]);
        attrs.insert("mail".to_string(), vec!["user1@example.com".to_string()]);

        let entry = SearchEntry {
            dn: "cn=user1,dc=example,dc=com".to_string(),
            attrs,
            bin_attrs: HashMap::new(),
        };

        let result = user_from_searchentry(&entry, "user1", None);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            LdapError::MissingAttribute(attr) if attr == "sn"
        ));
    }

    // invalid DN
    {
        let mut attrs = HashMap::new();
        attrs.insert("sn".to_string(), vec!["lastname1".to_string()]);
        attrs.insert("givenName".to_string(), vec!["firstname1".to_string()]);
        attrs.insert("mail".to_string(), vec!["user1@example.com".to_string()]);

        let entry = SearchEntry {
            dn: "cn=user1".to_string(), // No comma, invalid DN
            attrs,
            bin_attrs: HashMap::new(),
        };

        let result = user_from_searchentry(&entry, "user1", None);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            LdapError::InvalidDN(dn) if dn == "cn=user1"
        ));

        let mut attrs = HashMap::new();
        attrs.insert("sn".to_string(), vec!["lastname1".to_string()]);
        attrs.insert("givenName".to_string(), vec!["firstname1".to_string()]);
        attrs.insert("mail".to_string(), vec!["user1@example.com".to_string()]);

        let entry = SearchEntry {
            dn: "user1,dc=example,dc=com".to_string(), // No equals sign in RDN
            attrs,
            bin_attrs: HashMap::new(),
        };

        let result = user_from_searchentry(&entry, "user1", None);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            LdapError::InvalidDN(dn) if dn == "user1,dc=example,dc=com"
        ));
    }

    // invalid username
    {
        let mut attrs = HashMap::new();
        attrs.insert("sn".to_string(), vec!["lastname1".to_string()]);
        attrs.insert("givenName".to_string(), vec!["firstname1".to_string()]);
        attrs.insert("mail".to_string(), vec!["user1@example.com".to_string()]);

        let entry = SearchEntry {
            dn: "cn=user1,dc=example,dc=com".to_string(),
            attrs,
            bin_attrs: HashMap::new(),
        };

        // Test with invalid username (contains special characters)
        let result = user_from_searchentry(&entry, "user@#$%", None);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            LdapError::InvalidUsername(username) if username == "user@#$%"
        ));
    }

    // complex DN
    {
        let mut attrs = HashMap::new();
        attrs.insert("sn".to_string(), vec!["lastname1".to_string()]);
        attrs.insert("givenName".to_string(), vec!["firstname1".to_string()]);
        attrs.insert("mail".to_string(), vec!["user1@example.com".to_string()]);
        attrs.insert("mobile".to_string(), vec!["1234567890".to_string()]);

        let entry = SearchEntry {
            dn: "uid=user1,ou=People,ou=Department,dc=example,dc=com".to_string(),
            attrs,
            bin_attrs: HashMap::new(),
        };

        let user = user_from_searchentry(&entry, "user1", Some("password123")).unwrap();

        assert_eq!(user.username, "user1");
        assert_eq!(user.last_name, "lastname1");
        assert_eq!(user.first_name, "firstname1");
        assert_eq!(user.email, "user1@example.com");
        assert_eq!(user.phone, Some("1234567890".to_string()));
        assert!(user.from_ldap);
        assert_eq!(user.ldap_rdn, Some("user1".to_string()));
        assert_eq!(
            user.ldap_user_path,
            Some("ou=People,ou=Department,dc=example,dc=com".to_string())
        );
    }

    // with password
    {
        let mut attrs = HashMap::new();
        attrs.insert("sn".to_string(), vec!["lastname1".to_string()]);
        attrs.insert("givenName".to_string(), vec!["firstname1".to_string()]);
        attrs.insert("mail".to_string(), vec!["user1@example.com".to_string()]);

        let entry = SearchEntry {
            dn: "cn=user1,dc=example,dc=com".to_string(),
            attrs,
            bin_attrs: HashMap::new(),
        };

        let user = user_from_searchentry(&entry, "user1", Some("mypassword")).unwrap();

        assert_eq!(user.username, "user1");
        assert!(user.password_hash.is_some());
        assert!(user.from_ldap);
    }

    // multiple attribute values
    {
        let mut attrs = HashMap::new();
        attrs.insert(
            "sn".to_string(),
            vec!["lastname1".to_string(), "lastname2".to_string()],
        );
        attrs.insert(
            "givenName".to_string(),
            vec!["firstname1".to_string(), "firstname2".to_string()],
        );
        attrs.insert(
            "mail".to_string(),
            vec![
                "user1@example.com".to_string(),
                "user1@other.com".to_string(),
            ],
        );
        attrs.insert(
            "mobile".to_string(),
            vec!["1234567890".to_string(), "0987654321".to_string()],
        );

        let entry = SearchEntry {
            dn: "cn=user1,dc=example,dc=com".to_string(),
            attrs,
            bin_attrs: HashMap::new(),
        };

        let user = user_from_searchentry(&entry, "user1", None).unwrap();

        // Should use the first value when multiple values are present
        assert_eq!(user.last_name, "lastname1");
        assert_eq!(user.first_name, "firstname1");
        assert_eq!(user.email, "user1@example.com");
        assert_eq!(user.phone, Some("1234567890".to_string()));
        assert!(user.from_ldap);
    }

    // fields properly set
    {
        let mut attrs = HashMap::new();
        attrs.insert("sn".to_string(), vec!["lastname1".to_string()]);
        attrs.insert("givenName".to_string(), vec!["firstname1".to_string()]);
        attrs.insert("mail".to_string(), vec!["user1@example.com".to_string()]);

        let entry = SearchEntry {
            dn: "cn=testuser,ou=users,dc=example,dc=com".to_string(),
            attrs,
            bin_attrs: HashMap::new(),
        };

        let user = user_from_searchentry(&entry, "testuser", None).unwrap();

        // Verify LDAP-specific fields are properly set
        assert!(user.from_ldap);
        assert_eq!(user.ldap_rdn, Some("testuser".to_string()));
        assert_eq!(
            user.ldap_user_path,
            Some("ou=users,dc=example,dc=com".to_string())
        );
    }
}

#[test]
fn test_as_ldap_attrs() {
    let user = User::new(
        "testuser".to_string(),
        Some("password123"),
        "Smith".to_string(),
        "John".to_string(),
        "john.smith@example.com".to_string(),
        Some("5551234".to_string()),
    );

    // Basic test with InetOrgPerson
    let attrs = user_as_ldap_attrs(
        &user,
        "{SSHA}hashedpw",
        "NT_HASH",
        hashset![UserObjectClass::InetOrgPerson.into()],
        false,
        "uid",
        "cn",
    );

    assert!(attrs.contains(&("cn", hashset!["testuser"])));
    assert!(attrs.contains(&("sn", hashset!["Smith"])));
    assert!(attrs.contains(&("givenName", hashset!["John"])));
    assert!(attrs.contains(&("mail", hashset!["john.smith@example.com"])));
    assert!(attrs.contains(&("mobile", hashset!["5551234"])));
    assert!(attrs.contains(&("objectClass", hashset!["inetOrgPerson"])));

    // Test with ActiveDirectory
    let attrs = user_as_ldap_attrs(
        &user,
        "{SSHA}hashedpw",
        "NT_HASH",
        hashset![UserObjectClass::User.into()],
        true,
        "uid",
        "cn",
    );

    assert!(attrs.contains(&("sAMAccountName", hashset!["testuser"])));

    // Test with SimpleSecurityObject and SambaSamAccount
    let attrs = user_as_ldap_attrs(
        &user,
        "{SSHA}hashedpw",
        "NT_HASH",
        hashset![
            UserObjectClass::SimpleSecurityObject.into(),
            UserObjectClass::SambaSamAccount.into()
        ],
        false,
        "uid",
        "uid",
    );

    assert!(attrs.contains(&("userPassword", hashset!["{SSHA}hashedpw"])));
    assert!(attrs.contains(&("sambaSID", hashset!["0"])));
    assert!(attrs.contains(&("sambaNTPassword", hashset!["NT_HASH"])));

    // Test with custom RDN attribute
    let attrs = user_as_ldap_attrs(
        &user,
        "{SSHA}hashedpw",
        "NT_HASH",
        hashset![UserObjectClass::User.into()],
        false,
        "uid",
        "customRDN",
    );

    assert!(attrs.contains(&("customRDN", hashset![user.ldap_rdn_value()])));
    assert!(attrs.contains(&("uid", hashset!["testuser"])));

    // Test with empty phone
    let user_no_phone = User::new(
        "testuser".to_string(),
        Some("password123"),
        "Smith".to_string(),
        "John".to_string(),
        "john.smith@example.com".to_string(),
        Some(String::new()),
    );

    let attrs = user_as_ldap_attrs(
        &user_no_phone,
        "{SSHA}hashedpw",
        "NT_HASH",
        hashset![UserObjectClass::InetOrgPerson.into()],
        false,
        "uid",
        "cn",
    );

    assert!(
        !attrs
            .iter()
            .any(|(key, _)| key.eq_ignore_ascii_case("mobile"))
    );
}

#[test]
fn test_as_ldap_mod_inetorgperson() {
    let user = User::new(
        "testuser".to_string(),
        Some("password123"),
        "Smith".to_string(),
        "John".to_string(),
        "john.smith@example.com".to_string(),
        Some("5551234".to_string()),
    );

    let config = LDAPConfig {
        ldap_user_rdn_attr: Some("cn".to_string()),
        ldap_username_attr: "uid".to_string(),
        ..Default::default()
    };

    let mods = user_as_ldap_mod(&user, &config);
    assert!(mods.contains(&Mod::Replace("sn", hashset!["Smith"])));
    assert!(mods.contains(&Mod::Replace("givenName", hashset!["John"])));
    assert!(mods.contains(&Mod::Replace("mail", hashset!["john.smith@example.com"])));
    assert!(mods.contains(&Mod::Replace("mobile", hashset!["5551234"])));
}

#[test]
fn test_as_ldap_mod_with_empty_phone() {
    let user = User::new(
        "testuser".to_string(),
        Some("password123"),
        "Smith".to_string(),
        "John".to_string(),
        "john.smith@example.com".to_string(),
        Some(String::new()),
    );

    let config = LDAPConfig {
        ldap_user_rdn_attr: Some("cn".to_string()),
        ldap_username_attr: "uid".to_string(),
        ..Default::default()
    };

    let mods = user_as_ldap_mod(&user, &config);

    assert!(mods.contains(&Mod::Replace("sn", hashset!["Smith"])));
    assert!(mods.contains(&Mod::Replace("givenName", hashset!["John"])));
    assert!(mods.contains(&Mod::Replace("mail", hashset!["john.smith@example.com"])));
    assert!(mods.contains(&Mod::Replace("mobile", HashSet::new())));
}

#[test]
fn test_as_ldap_mod_with_active_directory() {
    let user = User::new(
        "testuser".to_string(),
        Some("password123"),
        "Smith".to_string(),
        "John".to_string(),
        "john.smith@example.com".to_string(),
        Some("5551234".to_string()),
    );

    let config = LDAPConfig {
        ldap_user_obj_class: "user".to_string(),
        ldap_user_rdn_attr: Some("cn".to_string()),
        ldap_username_attr: "sAMAccountName".to_string(),
        ldap_uses_ad: true,
        ..Default::default()
    };

    let mods = user_as_ldap_mod(&user, &config);

    assert!(mods.contains(&Mod::Replace("sn", hashset!["Smith"])));
    assert!(mods.contains(&Mod::Replace("givenName", hashset!["John"])));
    assert!(mods.contains(&Mod::Replace("mail", hashset!["john.smith@example.com"])));
    assert!(mods.contains(&Mod::Replace("sAMAccountName", hashset!["testuser"])));
}

#[test]
fn test_as_ldap_mod_with_custom_rdn() {
    let user = User::new(
        "testuser".to_string(),
        Some("password123"),
        "Smith".to_string(),
        "John".to_string(),
        "john.smith@example.com".to_string(),
        Some("5551234".to_string()),
    );

    let config = LDAPConfig {
        ldap_user_rdn_attr: Some("customRDN".to_string()),
        ldap_username_attr: "uid".to_string(),
        ldap_uses_ad: true,
        ..Default::default()
    };

    let mods = user_as_ldap_mod(&user, &config);

    assert!(mods.contains(&Mod::Replace("sn", hashset!["Smith"])));
    assert!(mods.contains(&Mod::Replace("givenName", hashset!["John"])));
    assert!(mods.contains(&Mod::Replace("mail", hashset!["john.smith@example.com"])));
    assert!(mods.contains(&Mod::Replace("cn", hashset!["testuser"])));
    assert!(mods.contains(&Mod::Replace("sAMAccountName", hashset!["testuser"])));
}

#[test]
fn test_extract_dn_path_various_cases() {
    assert_eq!(
        extract_dn_path("cn=testuser,dc=example,dc=com"),
        Some("dc=example,dc=com".to_string())
    );
    assert_eq!(
        extract_dn_path("uid=abc,ou=users,dc=example,dc=org"),
        Some("ou=users,dc=example,dc=org".to_string())
    );
    assert_eq!(
        extract_dn_path("cn=Test User,dc=example,dc=com"),
        Some("dc=example,dc=com".to_string())
    );
    assert_eq!(
        extract_dn_path("cn=user.name+123,ou=group,dc=example,dc=com"),
        Some("ou=group,dc=example,dc=com".to_string())
    );

    assert_eq!(extract_dn_path("invalid-dn"), None);
    assert_eq!(extract_dn_path("value"), None);

    assert_eq!(extract_dn_path(""), None);

    assert_eq!(extract_dn_path("cn=abc,"), Some(String::new()));

    assert_eq!(
        extract_dn_path("uid=cde,ou=users,ou=staff,dc=example,dc=org"),
        Some("ou=users,ou=staff,dc=example,dc=org".to_string())
    );

    assert_eq!(extract_dn_path("cn=abc"), None);

    assert_eq!(extract_dn_path("cn=abc"), None);

    assert_eq!(
        extract_dn_path("cn=,dc=example,dc=com"),
        Some("dc=example,dc=com".to_string())
    );

    assert_eq!(
        extract_dn_path("cn=abc=cde,dc=example,dc=com"),
        Some("dc=example,dc=com".to_string())
    );

    assert_eq!(
        extract_dn_path(" cn=abc ,dc=example,dc=com "),
        Some("dc=example,dc=com ".to_string())
    );
}

#[sqlx::test]
async fn test_ldap_sync_allowed_with_empty_sync_groups(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let _ = initialize_current_settings(&pool).await;
    set_test_license_business();

    let mut user = make_test_user("testuser", None, None);
    user.is_active = true;
    user.password_hash = Some("hash".to_string());
    let user = user.save(&pool).await.unwrap();

    let result = ldap_sync_allowed_for_user(&user, &pool).await.unwrap();
    assert!(result);
}

#[sqlx::test]
async fn test_ldap_sync_allowed_with_inactive_user(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let _ = initialize_current_settings(&pool).await;

    let mut user = make_test_user("testuser", None, None);
    user.is_active = false;
    user.password_hash = Some("hash".to_string());
    let user = user.save(&pool).await.unwrap();

    let result = ldap_sync_allowed_for_user(&user, &pool).await.unwrap();
    assert!(!result);
}

#[sqlx::test]
async fn test_ldap_sync_allowed_with_unenrolled_user(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let _ = initialize_current_settings(&pool).await;

    let mut user = make_test_user("testuser", None, None);
    user.is_active = true;
    user.password_hash = None;
    user.openid_sub = None;
    user.from_ldap = false;
    let user = user.save(&pool).await.unwrap();

    let result = ldap_sync_allowed_for_user(&user, &pool).await.unwrap();
    assert!(!result);
}

#[sqlx::test]
async fn test_ldap_sync_allowed_with_sync_groups_user_in_group(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let _ = initialize_current_settings(&pool).await;

    let mut user = make_test_user("testuser", None, None);
    user.is_active = true;
    user.password_hash = Some("hash".to_string());
    let user = user.save(&pool).await.unwrap();

    let group = Group::new("ldap_sync_group").save(&pool).await.unwrap();
    user.add_to_group(&pool, &group).await.unwrap();

    let mut settings = Settings::get_current_settings();
    settings.ldap_sync_groups = vec!["ldap_sync_group".to_string()];
    update_current_settings(&pool, settings).await.unwrap();

    let result = ldap_sync_allowed_for_user(&user, &pool).await.unwrap();
    assert!(result);
}

#[sqlx::test]
async fn test_ldap_sync_allowed_with_sync_groups_user_not_in_group(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let _ = initialize_current_settings(&pool).await;

    let mut user = make_test_user("testuser", None, None);
    user.is_active = true;
    user.password_hash = Some("hash".to_string());
    let user = user.save(&pool).await.unwrap();

    let _group = Group::new("ldap_sync_group").save(&pool).await.unwrap();
    let other_group = Group::new("other_group").save(&pool).await.unwrap();
    user.add_to_group(&pool, &other_group).await.unwrap();

    let mut settings = Settings::get_current_settings();
    settings.ldap_sync_groups = vec!["ldap_sync_group".to_string()];
    update_current_settings(&pool, settings).await.unwrap();

    let result = ldap_sync_allowed_for_user(&user, &pool).await.unwrap();
    assert!(!result);
}

#[sqlx::test]
async fn test_ldap_sync_allowed_with_multiple_sync_groups(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let _ = initialize_current_settings(&pool).await;

    let mut user = make_test_user("testuser", None, None);
    user.is_active = true;
    user.password_hash = Some("hash".to_string());
    let user = user.save(&pool).await.unwrap();

    let _group1 = Group::new("group1").save(&pool).await.unwrap();
    let group2 = Group::new("group2").save(&pool).await.unwrap();
    let _group3 = Group::new("group3").save(&pool).await.unwrap();

    user.add_to_group(&pool, &group2).await.unwrap();

    let mut settings = Settings::get_current_settings();
    settings.ldap_sync_groups = vec![
        "group1".to_string(),
        "group2".to_string(),
        "group3".to_string(),
    ];
    update_current_settings(&pool, settings).await.unwrap();

    let result = ldap_sync_allowed_for_user(&user, &pool).await.unwrap();
    assert!(result);
}

#[sqlx::test]
async fn test_ldap_sync_allowed_enrolled_via_openid(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let _ = initialize_current_settings(&pool).await;
    set_test_license_business();

    let mut user = make_test_user("testuser", None, None);
    user.is_active = true;
    user.password_hash = None;
    user.openid_sub = Some("openid_sub".to_string());
    user.from_ldap = false;
    let user = user.save(&pool).await.unwrap();

    let result = ldap_sync_allowed_for_user(&user, &pool).await.unwrap();
    assert!(result);
}

#[sqlx::test]
async fn test_ldap_sync_allowed_enrolled_via_ldap(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let _ = initialize_current_settings(&pool).await;

    let mut user = make_test_user("testuser", None, None);
    user.is_active = true;
    user.password_hash = None;
    user.openid_sub = None;
    user.from_ldap = true;
    let user = user.save(&pool).await.unwrap();

    let result = ldap_sync_allowed_for_user(&user, &pool).await.unwrap();
    assert!(result);
}

#[sqlx::test]
async fn test_ldap_sync_allowed_all_conditions_false(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let _ = initialize_current_settings(&pool).await;

    let mut user = make_test_user("testuser", None, None);
    user.is_active = false;
    user.password_hash = None;
    user.openid_sub = None;
    user.from_ldap = false;
    let user = user.save(&pool).await.unwrap();

    let _group = Group::new("ldap_sync_group").save(&pool).await.unwrap();

    let mut settings = Settings::get_current_settings();
    settings.ldap_sync_groups = vec!["ldap_sync_group".to_string()];
    update_current_settings(&pool, settings).await.unwrap();

    let result = ldap_sync_allowed_for_user(&user, &pool).await.unwrap();
    assert!(!result);
}
