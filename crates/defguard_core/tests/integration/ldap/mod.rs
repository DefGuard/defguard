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
use defguard_core::{enterprise::ldap::LDAPConnection, grpc::GatewayEvent};
use sqlx::{
    PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
};
use tokio::sync::broadcast::{Receiver, Sender, channel};

fn wg_test_channel() -> (Sender<GatewayEvent>, Receiver<GatewayEvent>) {
    channel(256)
}

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
    settings.ldap_group_member_attr = env::var("LDAP_GROUP_MEMBER_ATTR").ok();
    settings.ldap_member_attr = env::var("LDAP_MEMBER_ATTR").ok();
    settings.ldap_use_starttls = env::var("LDAP_STARTTLS").is_ok();
    settings.ldap_tls_verify_cert = env::var("LDAP_TLS_VERIFY").is_ok();
    if env::var("LDAP_USES_AD").is_ok() {
        settings.ldap_user_rdn_attr = Some(String::from("cn"));
        settings.ldap_user_auxiliary_obj_classes = vec![String::from("user")];
    } else {
        settings.ldap_user_auxiliary_obj_classes = env::var("LDAP_USER_AUX_CLASSES")
            .map(|s| s.split(',').map(str::to_string).collect())
            .unwrap_or_default();
    }
    settings.ldap_enabled = true;
    set_settings(Some(settings));
}

fn enable_password_storage() {
    if env::var("LDAP_USES_AD").is_err() {
        let mut settings = Settings::get_current_settings();
        settings.ldap_user_auxiliary_obj_classes = vec![String::from("simpleSecurityObject")];
        set_settings(Some(settings));
    }
}

async fn set_sync_settings(pool: &PgPool, sync_group: &str, authoritative: bool) {
    set_ldap_settings(pool).await;
    let mut settings = Settings::get_current_settings();
    settings.ldap_sync_enabled = true;
    settings.ldap_is_authoritative = authoritative;
    settings.ldap_sync_groups = vec![sync_group.to_string()];
    set_settings(Some(settings));
}

fn user_dn<I>(conn: &LDAPConnection, user: &User<I>) -> String {
    format!(
        "{}={},{}",
        conn.config.get_rdn_attr(),
        user.ldap_rdn.as_ref().unwrap(),
        user.ldap_user_path.as_ref().unwrap()
    )
}

#[ignore = "requires LDAP server"]
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
    ldap_conn.config.ldap_uses_ad = env::var("LDAP_USES_AD").is_ok();
    // Try to remove user first, in case the previous test run failed.
    let _ = ldap_conn.delete_user(&user).await;

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
        ldap_conn.config.get_rdn_attr(),
        user.ldap_rdn.as_ref().unwrap(),
        user.ldap_user_path.as_ref().unwrap()
    );
    // Get groups the user belongs to.
    let groups = ldap_conn.get_user_groups(&dn).await.unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0], group.name);

    // Cleanup
    ldap_conn.delete_group(&group.name).await.unwrap();
    ldap_conn.delete_user(&user).await.unwrap();
}

#[ignore = "requires LDAP server"]
#[sqlx::test]
async fn test_special_characters(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    set_ldap_settings(&pool).await;

    let mut ldap_conn = LDAPConnection::create().await.unwrap();
    ldap_conn.config.ldap_uses_ad = env::var("LDAP_USES_AD").is_ok();

    let password = "pass123";
    let mut user = User::new(
        "ハリー・ポッター",
        Some(password),
        "ハリー",
        "ポッター",
        "hari.potta@hogwards.jp",
        None,
    )
    .save(&pool)
    .await
    .unwrap();

    // Try to remove user first, in case the previous test run failed.
    let _ = ldap_conn.delete_user(&user).await;
    // Add user to LDAP.
    ldap_conn
        .add_user(&mut user, Some(password), &pool)
        .await
        .unwrap();

    const TEST_GROUP: &str = "Wizards🪄,+\"\\<>=#🧙‍♂️";
    // Add group to LDAP. This is redundant as `add_user_to_group` does the same.
    ldap_conn
        .add_group_with_members(TEST_GROUP, &[&user])
        .await
        .unwrap();

    // Cleanup
    ldap_conn.delete_group(TEST_GROUP).await.unwrap();
    ldap_conn.delete_user(&user).await.unwrap();
}

#[ignore = "requires LDAP server"]
#[sqlx::test]
async fn test_get_user(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    set_ldap_settings(&pool).await;

    let mut user = User::new(
        "readback",
        Some("pass123"),
        "LastName",
        "FirstName",
        "readback@test.defguard",
        None,
    )
    .save(&pool)
    .await
    .unwrap();

    let mut ldap_conn = LDAPConnection::create().await.unwrap();
    ldap_conn.config.ldap_uses_ad = env::var("LDAP_USES_AD").is_ok();
    let _ = ldap_conn.delete_user(&user).await;
    ldap_conn
        .add_user(&mut user, Some("pass123"), &pool)
        .await
        .unwrap();

    // The attributes written to LDAP must map back to the same Defguard fields.
    let by_username = ldap_conn.get_user_by_username("readback").await.unwrap();
    assert_eq!(by_username.username, "readback");
    assert_eq!(by_username.first_name, "FirstName");
    assert_eq!(by_username.last_name, "LastName");
    assert_eq!(by_username.email, "readback@test.defguard");
    assert!(by_username.from_ldap);

    let by_dn = ldap_conn.get_user_by_dn(&user).await.unwrap();
    assert_eq!(by_dn.username, "readback");
    assert_eq!(by_dn.email, "readback@test.defguard");

    // A user that was never added must not be found.
    let missing = ldap_conn.get_user_by_username("does-not-exist").await;
    assert!(missing.is_err());

    ldap_conn.delete_user(&user).await.unwrap();
}

#[ignore = "requires LDAP server"]
#[sqlx::test]
async fn test_authentication(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    set_ldap_settings(&pool).await;
    enable_password_storage();

    let password = "Sup3rSecret!";
    let mut user = User::new(
        "authuser",
        Some(password),
        "Auth",
        "User",
        "authuser@test.defguard",
        None,
    )
    .save(&pool)
    .await
    .unwrap();

    let mut ldap_conn = LDAPConnection::create().await.unwrap();
    ldap_conn.config.ldap_uses_ad = env::var("LDAP_USES_AD").is_ok();
    let _ = ldap_conn.delete_user(&user).await;
    ldap_conn
        .add_user(&mut user, Some(password), &pool)
        .await
        .unwrap();

    // Correct password authenticates and returns the user.
    let authed = ldap_conn
        .get_user_by_credentials("authuser", password)
        .await
        .unwrap();
    assert_eq!(authed.username, "authuser");

    // Wrong password is rejected.
    let rejected = ldap_conn
        .get_user_by_credentials("authuser", "wrong-password")
        .await;
    assert!(rejected.is_err());

    // After changing the password, only the new one works.
    let new_password = "Even-M0re-Secret!";
    ldap_conn.set_password(&user, new_password).await.unwrap();
    assert!(
        ldap_conn
            .get_user_by_credentials("authuser", new_password)
            .await
            .is_ok()
    );
    assert!(
        ldap_conn
            .get_user_by_credentials("authuser", password)
            .await
            .is_err()
    );

    ldap_conn.delete_user(&user).await.unwrap();
}

#[ignore = "requires LDAP server"]
#[sqlx::test]
async fn test_is_username_available(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    set_ldap_settings(&pool).await;

    let mut user = User::new(
        "availuser",
        Some("pass123"),
        "Avail",
        "User",
        "availuser@test.defguard",
        None,
    )
    .save(&pool)
    .await
    .unwrap();

    let mut ldap_conn = LDAPConnection::create().await.unwrap();
    ldap_conn.config.ldap_uses_ad = env::var("LDAP_USES_AD").is_ok();
    let _ = ldap_conn.delete_user(&user).await;

    assert!(ldap_conn.is_username_available("availuser").await.unwrap());

    ldap_conn
        .add_user(&mut user, Some("pass123"), &pool)
        .await
        .unwrap();
    assert!(!ldap_conn.is_username_available("availuser").await.unwrap());

    ldap_conn.delete_user(&user).await.unwrap();
    assert!(ldap_conn.is_username_available("availuser").await.unwrap());
}

#[ignore = "requires LDAP server"]
#[sqlx::test]
async fn test_modify_user(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    set_ldap_settings(&pool).await;

    let mut user = User::new(
        "modifyme",
        Some("pass123"),
        "OldLast",
        "OldFirst",
        "old@test.defguard",
        None,
    )
    .save(&pool)
    .await
    .unwrap();

    let mut ldap_conn = LDAPConnection::create().await.unwrap();
    ldap_conn.config.ldap_uses_ad = env::var("LDAP_USES_AD").is_ok();
    let _ = ldap_conn.delete_user(&user).await;
    ldap_conn
        .add_user(&mut user, Some("pass123"), &pool)
        .await
        .unwrap();

    // Change attributes (no rename) and confirm they propagate to LDAP.
    user.first_name = "NewFirst".into();
    user.last_name = "NewLast".into();
    user.email = "new@test.defguard".into();
    user.save(&pool).await.unwrap();
    ldap_conn.modify_user("modifyme", &user).await.unwrap();

    let fetched = ldap_conn.get_user_by_username("modifyme").await.unwrap();
    assert_eq!(fetched.first_name, "NewFirst");
    assert_eq!(fetched.last_name, "NewLast");
    assert_eq!(fetched.email, "new@test.defguard");

    // Rename: the username is the RDN here, so this moves the entry.
    let old_username = user.username.clone();
    user.username = "renamed".into();
    user.ldap_rdn = Some("renamed".into());
    user.save(&pool).await.unwrap();
    ldap_conn.modify_user(&old_username, &user).await.unwrap();

    assert!(ldap_conn.get_user_by_username("renamed").await.is_ok());
    assert!(ldap_conn.get_user_by_username("modifyme").await.is_err());

    ldap_conn.delete_user(&user).await.unwrap();
}

#[ignore = "requires LDAP server"]
#[sqlx::test]
async fn test_modify_group(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    set_ldap_settings(&pool).await;

    let mut user = User::new(
        "groupmember",
        Some("pass123"),
        "Group",
        "Member",
        "groupmember@test.defguard",
        None,
    )
    .save(&pool)
    .await
    .unwrap();

    let mut ldap_conn = LDAPConnection::create().await.unwrap();
    ldap_conn.config.ldap_uses_ad = env::var("LDAP_USES_AD").is_ok();
    let _ = ldap_conn.delete_user(&user).await;
    let _ = ldap_conn.delete_group("oldgroup").await;
    let _ = ldap_conn.delete_group("newgroup").await;
    ldap_conn
        .add_user(&mut user, Some("pass123"), &pool)
        .await
        .unwrap();

    ldap_conn
        .add_group_with_members("oldgroup", &[&user])
        .await
        .unwrap();

    // Rename the group; membership must be preserved under the new name.
    let renamed = Group::new("newgroup").save(&pool).await.unwrap();
    ldap_conn.modify_group("oldgroup", &renamed).await.unwrap();

    let dn = user_dn(&ldap_conn, &user);
    let groups = ldap_conn.get_user_groups(&dn).await.unwrap();
    assert_eq!(groups, vec![String::from("newgroup")]);

    ldap_conn.delete_group("newgroup").await.unwrap();
    ldap_conn.delete_user(&user).await.unwrap();
}

#[ignore = "requires LDAP server"]
#[sqlx::test]
async fn test_group_membership_with_multiple_members(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    set_ldap_settings(&pool).await;

    let mut user1 = User::new(
        "multi1",
        Some("pass123"),
        "Multi",
        "One",
        "multi1@test.defguard",
        None,
    )
    .save(&pool)
    .await
    .unwrap();
    let mut user2 = User::new(
        "multi2",
        Some("pass123"),
        "Multi",
        "Two",
        "multi2@test.defguard",
        None,
    )
    .save(&pool)
    .await
    .unwrap();

    let mut ldap_conn = LDAPConnection::create().await.unwrap();
    ldap_conn.config.ldap_uses_ad = env::var("LDAP_USES_AD").is_ok();
    let _ = ldap_conn.delete_user(&user1).await;
    let _ = ldap_conn.delete_user(&user2).await;
    let _ = ldap_conn.delete_group("multigroup").await;
    ldap_conn
        .add_user(&mut user1, Some("pass123"), &pool)
        .await
        .unwrap();
    ldap_conn
        .add_user(&mut user2, Some("pass123"), &pool)
        .await
        .unwrap();

    ldap_conn
        .add_group_with_members("multigroup", &[&user1, &user2])
        .await
        .unwrap();

    let dn1 = user_dn(&ldap_conn, &user1);
    let dn2 = user_dn(&ldap_conn, &user2);
    assert_eq!(
        ldap_conn.get_user_groups(&dn1).await.unwrap(),
        vec![String::from("multigroup")]
    );
    assert_eq!(
        ldap_conn.get_user_groups(&dn2).await.unwrap(),
        vec![String::from("multigroup")]
    );

    // Removing one member of a multi-member group keeps the group and the other member.
    ldap_conn
        .remove_user_from_group(&user1, "multigroup")
        .await
        .unwrap();
    assert!(ldap_conn.get_user_groups(&dn1).await.unwrap().is_empty());
    assert_eq!(
        ldap_conn.get_user_groups(&dn2).await.unwrap(),
        vec![String::from("multigroup")]
    );

    // Removing the last member deletes the group entirely.
    ldap_conn
        .remove_user_from_group(&user2, "multigroup")
        .await
        .unwrap();
    assert!(ldap_conn.get_user_groups(&dn2).await.unwrap().is_empty());

    ldap_conn.delete_user(&user1).await.unwrap();
    ldap_conn.delete_user(&user2).await.unwrap();
}

#[ignore = "requires LDAP server"]
#[sqlx::test]
async fn test_sync_defguard_authority_pushes_to_ldap(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (wg_tx, _wg_rx) = wg_test_channel();
    set_sync_settings(&pool, "syncpush_grp", false).await;

    let group = Group::new("syncpush_grp").save(&pool).await.unwrap();
    let user = User::new(
        "syncpush_user",
        Some("pass123"),
        "Push",
        "User",
        "syncpush@test.defguard",
        None,
    )
    .save(&pool)
    .await
    .unwrap();
    user.add_to_group(&pool, &group).await.unwrap();

    let mut ldap_conn = LDAPConnection::create().await.unwrap();
    ldap_conn.config.ldap_uses_ad = env::var("LDAP_USES_AD").is_ok();
    let _ = ldap_conn.delete_user(&user).await;
    let _ = ldap_conn.delete_group("syncpush_grp").await;

    // A full sync with Defguard authority must push the Defguard-only user and their group
    // membership into LDAP.
    ldap_conn.sync(&pool, true, &wg_tx).await.unwrap();

    let ldap_user = ldap_conn
        .get_user_by_username("syncpush_user")
        .await
        .unwrap();
    assert_eq!(ldap_user.email, "syncpush@test.defguard");
    let groups = ldap_conn
        .get_user_groups(&user_dn(&ldap_conn, &ldap_user))
        .await
        .unwrap();
    assert_eq!(groups, vec![String::from("syncpush_grp")]);

    let _ = ldap_conn.delete_user(&ldap_user).await;
    let _ = ldap_conn.delete_group("syncpush_grp").await;
}

#[ignore = "requires LDAP server"]
#[sqlx::test]
async fn test_sync_ldap_authority_pulls_from_ldap(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (wg_tx, _wg_rx) = wg_test_channel();
    set_sync_settings(&pool, "syncpull_grp", true).await;

    // The group exists in Defguard but is empty; the member lives only in LDAP.
    Group::new("syncpull_grp").save(&pool).await.unwrap();
    let mut user = User::new(
        "syncpull_user",
        Some("pass123"),
        "Pull",
        "User",
        "syncpull@test.defguard",
        None,
    )
    .save(&pool)
    .await
    .unwrap();

    let mut ldap_conn = LDAPConnection::create().await.unwrap();
    ldap_conn.config.ldap_uses_ad = env::var("LDAP_USES_AD").is_ok();
    let _ = ldap_conn.delete_user(&user).await;
    let _ = ldap_conn.delete_group("syncpull_grp").await;
    ldap_conn
        .add_user(&mut user, Some("pass123"), &pool)
        .await
        .unwrap();
    ldap_conn
        .add_user_to_group(&user, "syncpull_grp")
        .await
        .unwrap();
    // Remove from Defguard so the user exists only in LDAP and falls in the sync group's scope.
    user.clone().delete(&pool).await.unwrap();

    // A full sync with LDAP authority must create the LDAP-only user in Defguard and add them to
    // the synced group.
    ldap_conn.sync(&pool, true, &wg_tx).await.unwrap();

    let pulled = User::find_by_username(&pool, "syncpull_user")
        .await
        .unwrap()
        .unwrap();
    assert!(pulled.from_ldap);
    assert_eq!(pulled.email, "syncpull@test.defguard");
    let groups = pulled.member_of_names(&pool).await.unwrap();
    assert!(groups.contains(&String::from("syncpull_grp")));

    let _ = ldap_conn.delete_user(&user).await;
    let _ = ldap_conn.delete_group("syncpull_grp").await;
}

#[ignore = "requires LDAP server"]
#[sqlx::test]
async fn test_sync_ldap_authority_deletes_missing_from_ldap(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let (wg_tx, _wg_rx) = wg_test_channel();
    set_sync_settings(&pool, "syncdel_grp", true).await;

    let group = Group::new("syncdel_grp").save(&pool).await.unwrap();
    let user = User::new(
        "syncdel_user",
        Some("pass123"),
        "Del",
        "User",
        "syncdel@test.defguard",
        None,
    )
    .save(&pool)
    .await
    .unwrap();
    user.add_to_group(&pool, &group).await.unwrap();

    let mut ldap_conn = LDAPConnection::create().await.unwrap();
    ldap_conn.config.ldap_uses_ad = env::var("LDAP_USES_AD").is_ok();
    // The user must be absent from LDAP for this scenario.
    let _ = ldap_conn.delete_user(&user).await;

    // LDAP is authoritative and the user is absent from LDAP, so a full sync removes them from
    // Defguard.
    ldap_conn.sync(&pool, true, &wg_tx).await.unwrap();

    assert!(
        User::find_by_username(&pool, "syncdel_user")
            .await
            .unwrap()
            .is_none()
    );
}

/// Flips the account-status setting. Independent from `set_sync_settings` so each test can
/// configure both sync scoping and the new toggle without re-deriving the rest of the LDAP config.
fn set_sync_account_status(enabled: bool) {
    let mut settings = Settings::get_current_settings();
    settings.ldap_sync_account_status = enabled;
    set_settings(Some(settings));
}

#[ignore = "requires LDAP server"]
#[sqlx::test]
async fn test_sync_skips_disabled_defguard_user_not_in_ldap(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let (wg_tx, _wg_rx) = wg_test_channel();
    set_sync_settings(&pool, "status_skip_grp", false).await;
    set_sync_account_status(true);

    let group = Group::new("status_skip_grp").save(&pool).await.unwrap();
    let mut user = User::new(
        "status_skip_user",
        Some("pass123"),
        "Skip",
        "User",
        "status_skip@test.defguard",
        None,
    )
    .save(&pool)
    .await
    .unwrap();
    user.is_active = false;
    user.save(&pool).await.unwrap();
    user.add_to_group(&pool, &group).await.unwrap();

    let mut ldap_conn = LDAPConnection::create().await.unwrap();
    ldap_conn.config.ldap_uses_ad = env::var("LDAP_USES_AD").is_ok();
    let _ = ldap_conn.delete_user(&user).await;
    let _ = ldap_conn.delete_group("status_skip_grp").await;

    ldap_conn.sync(&pool, true, &wg_tx).await.unwrap();

    // A disabled Defguard user that doesn't exist in LDAP must not be created there: otherwise we'd
    // be provisioning an enabled AD account for someone Defguard says is disabled.
    assert!(
        ldap_conn
            .get_user_by_username("status_skip_user")
            .await
            .is_err()
    );

    let _ = ldap_conn.delete_group("status_skip_grp").await;
}

#[ignore = "requires LDAP server"]
#[sqlx::test]
async fn test_sync_creates_active_user_when_status_sync_on(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let (wg_tx, _wg_rx) = wg_test_channel();
    set_sync_settings(&pool, "status_create_grp", false).await;
    set_sync_account_status(true);

    let group = Group::new("status_create_grp").save(&pool).await.unwrap();
    let user = User::new(
        "status_create_user",
        Some("pass123"),
        "Create",
        "User",
        "status_create@test.defguard",
        None,
    )
    .save(&pool)
    .await
    .unwrap();
    user.add_to_group(&pool, &group).await.unwrap();

    let mut ldap_conn = LDAPConnection::create().await.unwrap();
    ldap_conn.config.ldap_uses_ad = env::var("LDAP_USES_AD").is_ok();
    let _ = ldap_conn.delete_user(&user).await;
    let _ = ldap_conn.delete_group("status_create_grp").await;

    // Control case for `test_sync_skips_disabled_defguard_user_not_in_ldap`: an active user with
    // the same setting on must still be created in LDAP.
    ldap_conn.sync(&pool, true, &wg_tx).await.unwrap();

    let created = ldap_conn
        .get_user_by_username("status_create_user")
        .await
        .unwrap();
    assert_eq!(created.email, "status_create@test.defguard");

    let _ = ldap_conn.delete_user(&created).await;
    let _ = ldap_conn.delete_group("status_create_grp").await;
}

#[ignore = "requires LDAP server"]
#[sqlx::test]
async fn test_sync_status_setting_off_deletes_disabled_user_from_ldap(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let (wg_tx, _wg_rx) = wg_test_channel();
    set_sync_settings(&pool, "status_delete_grp", false).await;
    set_sync_account_status(false);

    let group = Group::new("status_delete_grp").save(&pool).await.unwrap();
    let mut user = User::new(
        "status_delete_user",
        Some("pass123"),
        "Delete",
        "User",
        "status_delete@test.defguard",
        None,
    )
    .save(&pool)
    .await
    .unwrap();
    user.add_to_group(&pool, &group).await.unwrap();

    let mut ldap_conn = LDAPConnection::create().await.unwrap();
    ldap_conn.config.ldap_uses_ad = env::var("LDAP_USES_AD").is_ok();
    let _ = ldap_conn.delete_user(&user).await;
    let _ = ldap_conn.delete_group("status_delete_grp").await;
    ldap_conn
        .add_user(&mut user, Some("pass123"), &pool)
        .await
        .unwrap();
    ldap_conn
        .add_user_to_group(&user, "status_delete_grp")
        .await
        .unwrap();

    // Disable in Defguard, then run sync: with the status-sync setting off, legacy behavior must
    // still apply and the user must be removed from LDAP.
    user.is_active = false;
    user.save(&pool).await.unwrap();

    ldap_conn.sync(&pool, true, &wg_tx).await.unwrap();

    assert!(
        ldap_conn
            .get_user_by_username("status_delete_user")
            .await
            .is_err()
    );

    let _ = ldap_conn.delete_group("status_delete_grp").await;
}

#[ignore = "requires LDAP server"]
#[sqlx::test]
async fn test_sync_status_setting_requires_ad_flag(_: PgPoolOptions, options: PgConnectOptions) {
    // When the operator turned the setting on but the directory isn't AD, the AND gate
    // (`ldap_uses_ad && ldap_sync_account_status`) must keep the disabled user on the legacy delete
    // path: writing `userAccountControl` to a non-AD server would fail at the schema level.
    if env::var("LDAP_USES_AD").is_ok() {
        return;
    }

    let pool = setup_pool(options).await;
    let (wg_tx, _wg_rx) = wg_test_channel();
    set_sync_settings(&pool, "status_gate_grp", false).await;
    set_sync_account_status(true);

    let group = Group::new("status_gate_grp").save(&pool).await.unwrap();
    let mut user = User::new(
        "status_gate_user",
        Some("pass123"),
        "Gate",
        "User",
        "status_gate@test.defguard",
        None,
    )
    .save(&pool)
    .await
    .unwrap();
    user.add_to_group(&pool, &group).await.unwrap();

    let mut ldap_conn = LDAPConnection::create().await.unwrap();
    ldap_conn.config.ldap_uses_ad = false;
    let _ = ldap_conn.delete_user(&user).await;
    let _ = ldap_conn.delete_group("status_gate_grp").await;
    ldap_conn
        .add_user(&mut user, Some("pass123"), &pool)
        .await
        .unwrap();
    ldap_conn
        .add_user_to_group(&user, "status_gate_grp")
        .await
        .unwrap();

    user.is_active = false;
    user.save(&pool).await.unwrap();

    ldap_conn.sync(&pool, true, &wg_tx).await.unwrap();

    assert!(
        ldap_conn
            .get_user_by_username("status_gate_user")
            .await
            .is_err()
    );

    let _ = ldap_conn.delete_group("status_gate_grp").await;
}

#[ignore = "requires LDAP server"]
#[sqlx::test]
async fn test_sync_does_not_recreate_disabled_user_after_legacy_delete(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let (wg_tx, _wg_rx) = wg_test_channel();
    set_sync_settings(&pool, "status_idempotent_grp", false).await;
    set_sync_account_status(true);

    let group = Group::new("status_idempotent_grp")
        .save(&pool)
        .await
        .unwrap();
    let mut user = User::new(
        "status_idempotent_user",
        Some("pass123"),
        "Idem",
        "User",
        "status_idem@test.defguard",
        None,
    )
    .save(&pool)
    .await
    .unwrap();
    user.is_active = false;
    user.save(&pool).await.unwrap();
    user.add_to_group(&pool, &group).await.unwrap();

    let mut ldap_conn = LDAPConnection::create().await.unwrap();
    ldap_conn.config.ldap_uses_ad = env::var("LDAP_USES_AD").is_ok();
    let _ = ldap_conn.delete_user(&user).await;
    let _ = ldap_conn.delete_group("status_idempotent_grp").await;

    // Two consecutive syncs must be idempotent: neither run may resurrect the disabled user in
    // LDAP.
    for _ in 0..2 {
        ldap_conn.sync(&pool, true, &wg_tx).await.unwrap();
        assert!(
            ldap_conn
                .get_user_by_username("status_idempotent_user")
                .await
                .is_err()
        );
    }

    let _ = ldap_conn.delete_group("status_idempotent_grp").await;
}
