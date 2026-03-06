//! Integration tests that require a running LDAP server.

use std::{env, str::FromStr};

use defguard_common::{
    config::{DefGuardConfig, SERVER_CONFIG},
    db::{
        models::{
            Settings, User,
            group::Group,
            settings::{initialize_current_settings, set_settings},
        },
        setup_pool,
    },
    secret::SecretStringWrapper,
};
use defguard_core::enterprise::ldap::LDAPConnection;
use sqlx::{
    PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
};

/// Set LDAP settings from environment variables.
async fn set_ldap_settings(pool: &PgPool) {
    let config = DefGuardConfig::new_test_config();
    let _ = SERVER_CONFIG.set(config);
    initialize_current_settings(pool).await.unwrap();

    let mut settings = Settings::get_current_settings();
    settings.ldap_url = env::var("LDAP_URL").ok();
    settings.ldap_bind_username = env::var("LDAP_BIND_USERNAME").ok();
    settings.ldap_bind_password = env::var("LDAP_BIND_PASSWORD")
        .map(|pass| SecretStringWrapper::from_str(&pass).unwrap())
        .ok();
    settings.ldap_group_search_base = env::var("LDAP_GROUP_SEARCH_BASE").ok();
    settings.ldap_user_search_base = env::var("LDAP_USER_SEARCH_BASE").ok();
    settings.ldap_user_obj_class = env::var("LDAP_USER_CLASS").ok();
    settings.ldap_group_obj_class = env::var("LDAP_GROUP_CLASS").ok();
    settings.ldap_username_attr = env::var("LDAP_USERNAME_ATTR").ok();
    settings.ldap_groupname_attr = env::var("LDAP_GROUPNAME_ATTR").ok();
    settings.ldap_group_member_attr = env::var("LDAP_GROU_MEMBER_ATTR").ok();
    settings.ldap_member_attr = env::var("LDAP_MEMBER_ATTR").ok();
    settings.ldap_use_starttls = env::var("LDAP_STARTTLS").is_ok();
    settings.ldap_tls_verify_cert = env::var("LDAP_TLS_VERIFY").is_ok();
    settings.ldap_enabled = true;
    set_settings(Some(settings));
}

#[ignore = "Requires LDAP server"]
#[sqlx::test]
async fn test_ldap(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    set_ldap_settings(&pool).await;

    let password = "pass123";
    let mut user = User::new(
        "user1",
        Some(password),
        "Test",
        "One",
        "user1@test.defguard",
        None,
    )
    .save(&pool)
    .await
    .unwrap();
    let group = Group::new("testers").save(&pool).await.unwrap();

    let mut ldap_conn = LDAPConnection::create().await.unwrap();
    ldap_conn.config.ldap_sync_groups = vec![String::from("testers")];
    // Try to remove user first, in case the previous test run failed.
    let _ = ldap_conn.delete_user(&mut user).await;

    // Add user to LDAP.
    ldap_conn
        .add_user(&mut user, Some(password), &pool)
        .await
        .unwrap();

    let groups = ldap_conn
        .get_user_groups(user.ldap_rdn.as_ref().unwrap())
        .await
        .unwrap();
    assert_eq!(groups.len(), 0);

    // Add group to LDAP. This is redundant as `add_user_to_group` does the same.
    ldap_conn
        .add_group_with_members(&group.name, &[&user])
        .await
        .unwrap();
    // Add user to group; `add_group_with_members` doesn't do it.
    ldap_conn
        .add_user_to_group(&user, &group.name)
        .await
        .unwrap();

    // Build user DN.
    let dn = format!(
        "{}={},{}",
        ldap_conn.config.ldap_username_attr,
        user.ldap_rdn.as_ref().unwrap(),
        user.ldap_user_path.as_ref().unwrap()
    );
    // Get groups the user belongs to.
    let groups = ldap_conn.get_user_groups(&dn).await.unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0], group.name);

    // Cleanup
    ldap_conn.delete_group(&group.name).await.unwrap();
    ldap_conn.delete_user(&mut user).await.unwrap();
}
