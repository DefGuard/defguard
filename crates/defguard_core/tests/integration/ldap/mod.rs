//! Integration tests that require a running LDAP server.

use std::{collections::HashSet, env, str::FromStr};

use defguard_common::{
    config::{DefGuardConfig, SERVER_CONFIG},
    db::{
        models::{
            Settings, User,
            group::{Group, Permission},
            settings::{initialize_current_settings, set_settings},
        },
        setup_pool,
    },
    secret::SecretStringWrapper,
};
use defguard_core::{
    enterprise::ldap::LDAPConnection, events::LdapSyncEventType, grpc::GatewayCommand,
};
use ldap3::{
    LdapConnAsync,
    controls::{MakeCritical, ManageDsaIt},
};
use sqlx::{
    PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
};
use tokio::sync::{
    broadcast::{Receiver, Sender, channel},
    mpsc,
};

fn wg_test_channel() -> (Sender<GatewayCommand>, Receiver<GatewayCommand>) {
    channel(256)
}

async fn sync_ldap(
    ldap_conn: &mut LDAPConnection,
    pool: &PgPool,
    full: bool,
    wg_tx: &Sender<GatewayCommand>,
) {
    let (ldap_tx, _ldap_rx) = mpsc::unbounded_channel::<LdapSyncEventType>();
    ldap_conn.sync(pool, full, wg_tx, &ldap_tx).await.unwrap();
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
            .map(|s| s.split(',').map(str::to_owned).collect())
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
    settings.ldap_sync_groups = vec![sync_group.to_owned()];
    set_settings(Some(settings));
}

fn enable_secure_enrollment() {
    let mut settings = Settings::get_current_settings();
    settings.ldap_remote_enrollment_enabled = true;
    settings.ldap_remote_enrollment_send_invite = true;
    settings.smtp.server = Some("smtp.example.com".into());
    settings.smtp.port = Some(587);
    settings.smtp.sender = Some("noreply@test.defguard".into());
    settings.public_proxy_url = "http://proxy.example.com".into();
    set_settings(Some(settings));
}

async fn create_active_admin(pool: &PgPool, username: &str) {
    let admin_group = Group::new("secure_enroll_admins").save(pool).await.unwrap();
    admin_group
        .set_permission(pool, Permission::IsAdmin, true)
        .await
        .unwrap();
    let admin = User::new(
        username,
        Some("pass123"),
        "Admin",
        "User",
        &format!("{username}@test.defguard"),
        None,
    )
    .save(pool)
    .await
    .unwrap();
    admin.add_to_group(pool, &admin_group).await.unwrap();
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
    sync_ldap(&mut ldap_conn, &pool, true, &wg_tx).await;

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
    sync_ldap(&mut ldap_conn, &pool, true, &wg_tx).await;

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
    sync_ldap(&mut ldap_conn, &pool, true, &wg_tx).await;

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

    sync_ldap(&mut ldap_conn, &pool, true, &wg_tx).await;

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
    sync_ldap(&mut ldap_conn, &pool, true, &wg_tx).await;

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

    sync_ldap(&mut ldap_conn, &pool, true, &wg_tx).await;

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

    sync_ldap(&mut ldap_conn, &pool, true, &wg_tx).await;

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
        sync_ldap(&mut ldap_conn, &pool, true, &wg_tx).await;
        assert!(
            ldap_conn
                .get_user_by_username("status_idempotent_user")
                .await
                .is_err()
        );
    }

    let _ = ldap_conn.delete_group("status_idempotent_grp").await;
}

/// Snapshot of the Defguard-side state relevant to LDAP sync, used to assert idempotency against a
/// real directory. Each entry is (username, is_active, first_name, last_name, email, sorted groups).
async fn defguard_state(pool: &PgPool) -> Vec<(String, bool, String, String, String, Vec<String>)> {
    let mut users = User::all(pool).await.unwrap();
    users.sort_by(|a, b| a.username.cmp(&b.username));
    let mut snapshot = Vec::new();
    for user in users {
        let mut groups = user.member_of_names(pool).await.unwrap();
        groups.sort();
        snapshot.push((
            user.username.clone(),
            user.is_active,
            user.first_name.clone(),
            user.last_name.clone(),
            user.email.clone(),
            groups,
        ));
    }
    snapshot
}

/// Pulling from a real directory must reach a fixed point: a second incremental (LDAP-authority)
/// sync must not change any Defguard-side state. A failure here means the pull path never converges
/// against the actual attribute/DN encoding the server returns (e.g. a comparison that always
/// reports a difference), which in production causes perpetual write churn.
#[ignore = "requires LDAP server"]
#[sqlx::test]
async fn test_sync_ldap_authority_is_idempotent(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (wg_tx, _wg_rx) = wg_test_channel();
    set_sync_settings(&pool, "idem_pull_grp", true).await;

    let mut user = User::new(
        "idem_pull_user",
        Some("pass123"),
        "Pull",
        "Idem",
        "idem_pull@test.defguard",
        None,
    )
    .save(&pool)
    .await
    .unwrap();

    let mut ldap_conn = LDAPConnection::create().await.unwrap();
    ldap_conn.config.ldap_uses_ad = env::var("LDAP_USES_AD").is_ok();
    let _ = ldap_conn.delete_user(&user).await;
    let _ = ldap_conn.delete_group("idem_pull_grp").await;
    ldap_conn
        .add_user(&mut user, Some("pass123"), &pool)
        .await
        .unwrap();
    ldap_conn
        .add_user_to_group(&user, "idem_pull_grp")
        .await
        .unwrap();
    // Remove from Defguard so the user exists only in LDAP, within the synced group's scope.
    user.clone().delete(&pool).await.unwrap();

    // First sync pulls the LDAP-only user into Defguard.
    sync_ldap(&mut ldap_conn, &pool, true, &wg_tx).await;
    let after_first = defguard_state(&pool).await;
    assert!(
        after_first.iter().any(|u| u.0 == "idem_pull_user"),
        "first sync should have pulled the LDAP user into Defguard"
    );

    // Second sync must be a no-op: Defguard-side state stays byte-for-byte identical.
    sync_ldap(&mut ldap_conn, &pool, false, &wg_tx).await;
    let after_second = defguard_state(&pool).await;
    assert_eq!(
        after_first, after_second,
        "second LDAP-authority sync was not idempotent"
    );

    let pulled = User::find_by_username(&pool, "idem_pull_user")
        .await
        .unwrap()
        .unwrap();
    let _ = ldap_conn.delete_user(&pulled).await;
    let _ = ldap_conn.delete_group("idem_pull_grp").await;
}

/// Pushing to a real directory must reach a fixed point: a second Defguard-authority sync must not
/// change Defguard state nor rewrite the LDAP entry. This catches a push path that never converges
/// against the server's actual attribute encoding (perpetual modify churn).
#[ignore = "requires LDAP server"]
#[sqlx::test]
async fn test_sync_defguard_authority_is_idempotent(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (wg_tx, _wg_rx) = wg_test_channel();
    set_sync_settings(&pool, "idem_push_grp", false).await;

    let group = Group::new("idem_push_grp").save(&pool).await.unwrap();
    let user = User::new(
        "idem_push_user",
        Some("pass123"),
        "Push",
        "Idem",
        "idem_push@test.defguard",
        None,
    )
    .save(&pool)
    .await
    .unwrap();
    user.add_to_group(&pool, &group).await.unwrap();

    let mut ldap_conn = LDAPConnection::create().await.unwrap();
    ldap_conn.config.ldap_uses_ad = env::var("LDAP_USES_AD").is_ok();
    let _ = ldap_conn.delete_user(&user).await;
    let _ = ldap_conn.delete_group("idem_push_grp").await;

    // First sync pushes the Defguard-only user and membership into LDAP.
    sync_ldap(&mut ldap_conn, &pool, true, &wg_tx).await;
    let ldap_user_first = ldap_conn
        .get_user_by_username("idem_push_user")
        .await
        .unwrap();
    let ldap_groups_first = ldap_conn
        .get_user_groups(&user_dn(&ldap_conn, &ldap_user_first))
        .await
        .unwrap();
    let defguard_first = defguard_state(&pool).await;

    // Second sync must not churn either side.
    sync_ldap(&mut ldap_conn, &pool, true, &wg_tx).await;
    let ldap_user_second = ldap_conn
        .get_user_by_username("idem_push_user")
        .await
        .unwrap();
    let ldap_groups_second = ldap_conn
        .get_user_groups(&user_dn(&ldap_conn, &ldap_user_second))
        .await
        .unwrap();
    let defguard_second = defguard_state(&pool).await;

    assert_eq!(
        defguard_first, defguard_second,
        "second Defguard-authority sync mutated Defguard state"
    );
    assert_eq!(ldap_user_first.first_name, ldap_user_second.first_name);
    assert_eq!(ldap_user_first.last_name, ldap_user_second.last_name);
    assert_eq!(ldap_user_first.email, ldap_user_second.email);
    assert_eq!(
        ldap_groups_first, ldap_groups_second,
        "second Defguard-authority sync changed LDAP group membership"
    );

    let _ = ldap_conn.delete_user(&ldap_user_second).await;
    let _ = ldap_conn.delete_group("idem_push_grp").await;
}

/// A user invited through "allow secure enrollment" (`ldap_remote_enrollment_enabled`) is
/// enrollment-pending after being pulled in, yet must stay within LDAP sync scope.
///
/// Before the fix, `is_enrolled()` resolves to `false` on `enrollment_pending`, so such a
/// user was dropped from sync entirely: subsequent directory changes (here, an attribute edit in
/// LDAP) never reached Defguard. The whole scenario:
///   1. Secure enrollment + invite enabled, LDAP authority, one sync group.
///   2. The user lives only in LDAP and belongs to the sync group.
///   3. First sync pulls the user into Defguard, marks them `enrollment_pending`, and mirrors the
///      group membership.
///   4. The user's surname is changed in LDAP.
///   5. A second sync must propagate that change to Defguard - which only happens if the
///      still-pending user is treated as in scope (and the user must not be deleted).
#[ignore = "requires LDAP server"]
#[sqlx::test]
async fn test_sync_keeps_enrollment_pending_ldap_user_in_scope(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let (wg_tx, _wg_rx) = wg_test_channel();
    set_sync_settings(&pool, "secure_enroll_grp", true).await;
    enable_secure_enrollment();
    create_active_admin(&pool, "secure_enroll_admin").await;

    let mut user = User::new(
        "secure_enroll_user",
        Some("pass123"),
        "Secure",
        "Enroll",
        "secure_enroll@test.defguard",
        None,
    )
    .save(&pool)
    .await
    .unwrap();

    let mut ldap_conn = LDAPConnection::create().await.unwrap();
    ldap_conn.config.ldap_uses_ad = env::var("LDAP_USES_AD").is_ok();
    ldap_conn.config.ldap_sync_groups = vec![String::from("secure_enroll_grp")];
    let _ = ldap_conn.delete_user(&user).await;
    let _ = ldap_conn.delete_group("secure_enroll_grp").await;

    // Provision the user in LDAP and in the sync group, then drop the Defguard copy so the first
    // sync pulls them in fresh (as an LDAP-origin user).
    ldap_conn
        .add_user(&mut user, Some("pass123"), &pool)
        .await
        .unwrap();
    ldap_conn
        .add_user_to_group(&user, "secure_enroll_grp")
        .await
        .unwrap();
    user.clone().delete(&pool).await.unwrap();

    // First sync: pull the LDAP-only user into Defguard. Secure enrollment + invite are on and an
    // active admin exists, so the user is marked enrollment-pending.
    sync_ldap(&mut ldap_conn, &pool, true, &wg_tx).await;

    let pulled = User::find_by_username(&pool, "secure_enroll_user")
        .await
        .unwrap()
        .expect("LDAP user should have been pulled into Defguard");
    assert!(pulled.from_ldap);
    assert!(
        pulled.enrollment_pending,
        "secure enrollment invite should mark the pulled user as enrollment-pending"
    );
    assert!(
        !pulled.is_enrolled(),
        "an enrollment-pending user is not yet enrolled"
    );
    assert!(
        pulled
            .member_of_names(&pool)
            .await
            .unwrap()
            .contains(&String::from("secure_enroll_grp")),
        "first sync should mirror the LDAP group membership"
    );

    // Change the user's surname in LDAP, keeping them in the sync group.
    let mut ldap_side = pulled.clone();
    ldap_side.last_name = String::from("Renamed");
    ldap_conn
        .modify_user(&pulled.username, &ldap_side)
        .await
        .unwrap();

    // Second sync: the attribute change must reach Defguard. This only happens when the
    // still-pending user remains in sync scope - the regression this guards against.
    sync_ldap(&mut ldap_conn, &pool, false, &wg_tx).await;

    let after = User::find_by_username(&pool, "secure_enroll_user")
        .await
        .unwrap()
        .expect("pending LDAP user must not be deleted while still present in LDAP");
    assert_eq!(
        after.last_name, "Renamed",
        "attribute change for an enrollment-pending LDAP user must propagate to Defguard"
    );
    assert!(
        after
            .member_of_names(&pool)
            .await
            .unwrap()
            .contains(&String::from("secure_enroll_grp")),
        "the pending user should still belong to the sync group"
    );
    assert!(
        after.enrollment_pending,
        "the user should still be enrollment-pending after sync"
    );

    let _ = ldap_conn.delete_user(&after).await;
    let _ = ldap_conn.delete_group("secure_enroll_grp").await;
}

/// The converse of `test_sync_keeps_enrollment_pending_ldap_user_in_scope`: the
/// enrollment-pending allowance is scoped to LDAP-origin users only. A Defguard-native user
/// (`from_ldap = false`) with a pending enrollment is not yet a real account and must NOT be
/// pushed into LDAP, even under Defguard authority. Clearing the pending flag then lets the same
/// user through, proving it is the flag that holds them back.
#[ignore = "requires LDAP server"]
#[sqlx::test]
async fn test_sync_does_not_push_pending_defguard_user_to_ldap(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let (wg_tx, _wg_rx) = wg_test_channel();
    // Defguard authority: a full sync pushes in-scope Defguard-only users into LDAP.
    set_sync_settings(&pool, "no_push_grp", false).await;

    let group = Group::new("no_push_grp").save(&pool).await.unwrap();
    // The user has a password, so the only thing keeping them out of sync is the pending flag.
    let mut user = User::new(
        "no_push_user",
        Some("pass123"),
        "NoPush",
        "User",
        "no_push@test.defguard",
        None,
    )
    .save(&pool)
    .await
    .unwrap();
    user.enrollment_pending = true;
    user.save(&pool).await.unwrap();
    user.add_to_group(&pool, &group).await.unwrap();

    let mut ldap_conn = LDAPConnection::create().await.unwrap();
    ldap_conn.config.ldap_uses_ad = env::var("LDAP_USES_AD").is_ok();
    let _ = ldap_conn.delete_user(&user).await;
    let _ = ldap_conn.delete_group("no_push_grp").await;

    // While pending, the Defguard-native user must not be created in LDAP.
    sync_ldap(&mut ldap_conn, &pool, true, &wg_tx).await;
    assert!(
        ldap_conn
            .get_user_by_username("no_push_user")
            .await
            .is_err(),
        "an enrollment-pending Defguard-native user must not be pushed to LDAP"
    );

    // Clearing the pending flag makes the user enrolled (they have a password), so the next sync
    // pushes them - confirming the pending flag was the sole reason they were held back.
    user.enrollment_pending = false;
    user.save(&pool).await.unwrap();

    sync_ldap(&mut ldap_conn, &pool, true, &wg_tx).await;
    let pushed = ldap_conn
        .get_user_by_username("no_push_user")
        .await
        .expect("once no longer pending, the Defguard user must be pushed to LDAP");
    assert_eq!(pushed.email, "no_push@test.defguard");

    let _ = ldap_conn.delete_user(&pushed).await;
    let _ = ldap_conn.delete_group("no_push_grp").await;
}

/// Strips the leftmost RDN, yielding the parent DN. For "ou=groups,dc=example,dc=org" this
/// returns "dc=example,dc=org".
fn parent_dn(dn: &str) -> String {
    dn.split_once(',')
        .expect("DN should have a parent component")
        .1
        .to_owned()
}

/// Regression for sync silently dropping every user when the group search base is set above the
/// sync group's own container.
///
/// `LDAPConfig::group_dn` builds the sync group's DN by literal concatenation with the group
/// search base, so before the fix a base of the domain root produced a `memberOf` filter DN that
/// no user actually carries (the group really lives one level deeper, e.g. under `ou=groups`), and
/// every user fell out of sync scope. The fix resolves the group's real DN by search, so a group
/// nested anywhere beneath a broad base is still matched.
#[ignore = "requires LDAP server"]
#[sqlx::test]
async fn test_sync_pulls_user_when_sync_group_nested_below_search_base(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let (wg_tx, _wg_rx) = wg_test_channel();
    set_sync_settings(&pool, "nested_sync_grp", true).await;

    let mut user = User::new(
        "nested_sync_user",
        Some("pass123"),
        "Nested",
        "User",
        "nested_sync@test.defguard",
        None,
    )
    .save(&pool)
    .await
    .unwrap();

    let mut ldap_conn = LDAPConnection::create().await.unwrap();
    ldap_conn.config.ldap_uses_ad = env::var("LDAP_USES_AD").is_ok();
    let nested_base = ldap_conn.config.ldap_group_search_base.clone();
    let _ = ldap_conn.delete_user(&user).await;
    let _ = ldap_conn.delete_group("nested_sync_grp").await;

    // Create the user and the group at the normal (nested) base, e.g.
    // cn=nested_sync_grp,ou=groups,dc=example,dc=org, then drop the Defguard copy so the user
    // exists only in LDAP and must be pulled back in by sync.
    ldap_conn
        .add_user(&mut user, Some("pass123"), &pool)
        .await
        .unwrap();
    ldap_conn
        .add_user_to_group(&user, "nested_sync_grp")
        .await
        .unwrap();
    user.clone().delete(&pool).await.unwrap();

    // Broaden the group search base to its parent (the domain root in this fixture). The sync
    // group now lives below the base rather than directly under it.
    ldap_conn.config.ldap_group_search_base = parent_dn(&nested_base);

    sync_ldap(&mut ldap_conn, &pool, true, &wg_tx).await;

    let pulled = User::find_by_username(&pool, "nested_sync_user")
        .await
        .unwrap()
        .expect("a user whose sync group is nested below the search base must still be pulled");
    assert!(pulled.from_ldap);
    assert!(
        pulled
            .member_of_names(&pool)
            .await
            .unwrap()
            .contains(&String::from("nested_sync_grp")),
        "the pulled user must be mirrored into the synced group"
    );

    ldap_conn.config.ldap_group_search_base = nested_base;
    let _ = ldap_conn.delete_user(&user).await;
    let _ = ldap_conn.delete_group("nested_sync_grp").await;
}

/// The real multi-OU AD layout: sync groups live in *different* subtrees that share only the
/// domain root (e.g. one OU per site), so the only group search base covering them all is that
/// root. Each group must still be resolved to its own DN and its members pulled. This is the
/// generalization of the single-nested-group case to several groups in separate branches.
#[ignore = "requires LDAP server"]
#[sqlx::test]
async fn test_sync_resolves_sync_groups_in_separate_subtrees_from_shared_root(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let (wg_tx, _wg_rx) = wg_test_channel();
    // set_sync_settings only takes one group; the full set is applied to the connection below.
    set_sync_settings(&pool, "multitree_a_grp", true).await;

    let mut user_a = User::new(
        "multitree_a_user",
        Some("pass123"),
        "Multi",
        "TreeA",
        "multitree_a@test.defguard",
        None,
    )
    .save(&pool)
    .await
    .unwrap();
    let mut user_b = User::new(
        "multitree_b_user",
        Some("pass123"),
        "Multi",
        "TreeB",
        "multitree_b@test.defguard",
        None,
    )
    .save(&pool)
    .await
    .unwrap();

    let mut ldap_conn = LDAPConnection::create().await.unwrap();
    ldap_conn.config.ldap_uses_ad = env::var("LDAP_USES_AD").is_ok();
    // Two distinct subtrees that share only the domain root: ou=groups and ou=users.
    let groups_base = ldap_conn.config.ldap_group_search_base.clone();
    let users_base = ldap_conn.config.ldap_user_search_base.clone();
    let root_base = parent_dn(&groups_base);

    let _ = ldap_conn.delete_user(&user_a).await;
    let _ = ldap_conn.delete_user(&user_b).await;
    ldap_conn
        .add_user(&mut user_a, Some("pass123"), &pool)
        .await
        .unwrap();
    ldap_conn
        .add_user(&mut user_b, Some("pass123"), &pool)
        .await
        .unwrap();

    // Group A lives under ou=groups; the write helpers build the group DN from the configured
    // group search base, so point it at each target subtree in turn.
    ldap_conn.config.ldap_group_search_base = groups_base.clone();
    let _ = ldap_conn.delete_group("multitree_a_grp").await;
    ldap_conn
        .add_user_to_group(&user_a, "multitree_a_grp")
        .await
        .unwrap();

    // Group B lives under ou=users, a different branch of the same root.
    ldap_conn.config.ldap_group_search_base = users_base.clone();
    let _ = ldap_conn.delete_group("multitree_b_grp").await;
    ldap_conn
        .add_user_to_group(&user_b, "multitree_b_grp")
        .await
        .unwrap();

    // Remove the Defguard copies so both users exist only in LDAP and must be pulled by sync.
    user_a.clone().delete(&pool).await.unwrap();
    user_b.clone().delete(&pool).await.unwrap();

    // Search both groups from the shared root, with both as sync groups.
    ldap_conn.config.ldap_group_search_base = root_base;
    ldap_conn.config.ldap_sync_groups = vec![
        String::from("multitree_a_grp"),
        String::from("multitree_b_grp"),
    ];

    sync_ldap(&mut ldap_conn, &pool, true, &wg_tx).await;

    let pulled_a = User::find_by_username(&pool, "multitree_a_user")
        .await
        .unwrap()
        .expect("user in a sync group under one subtree of the shared root must be pulled");
    assert!(pulled_a.from_ldap);
    assert!(
        pulled_a
            .member_of_names(&pool)
            .await
            .unwrap()
            .contains(&String::from("multitree_a_grp"))
    );

    let pulled_b = User::find_by_username(&pool, "multitree_b_user")
        .await
        .unwrap()
        .expect("user in a sync group under a different subtree of the shared root must be pulled");
    assert!(pulled_b.from_ldap);
    assert!(
        pulled_b
            .member_of_names(&pool)
            .await
            .unwrap()
            .contains(&String::from("multitree_b_grp"))
    );

    let _ = ldap_conn.delete_user(&user_a).await;
    let _ = ldap_conn.delete_user(&user_b).await;
    ldap_conn.config.ldap_group_search_base = groups_base;
    let _ = ldap_conn.delete_group("multitree_a_grp").await;
    ldap_conn.config.ldap_group_search_base = users_base;
    let _ = ldap_conn.delete_group("multitree_b_grp").await;
}

/// Two LDAP groups can share the same name (cn) in different subtrees under the shared root (e.g.
/// a per-site "DgProvisioning" group in each OU). Resolving by name then yields several DNs; the
/// filter must OR them all so members of every same-named group are pulled. Defguard has a single
/// group of that name, so all such members land in it.
#[ignore = "requires LDAP server"]
#[sqlx::test]
async fn test_sync_resolves_duplicate_named_groups_in_different_subtrees(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let (wg_tx, _wg_rx) = wg_test_channel();
    set_sync_settings(&pool, "dup_grp", true).await;

    let mut user_a = User::new(
        "dup_a_user",
        Some("pass123"),
        "Dup",
        "MemberA",
        "dup_a@test.defguard",
        None,
    )
    .save(&pool)
    .await
    .unwrap();
    let mut user_b = User::new(
        "dup_b_user",
        Some("pass123"),
        "Dup",
        "MemberB",
        "dup_b@test.defguard",
        None,
    )
    .save(&pool)
    .await
    .unwrap();

    let mut ldap_conn = LDAPConnection::create().await.unwrap();
    ldap_conn.config.ldap_uses_ad = env::var("LDAP_USES_AD").is_ok();
    let groups_base = ldap_conn.config.ldap_group_search_base.clone();
    let users_base = ldap_conn.config.ldap_user_search_base.clone();
    let root_base = parent_dn(&groups_base);

    let _ = ldap_conn.delete_user(&user_a).await;
    let _ = ldap_conn.delete_user(&user_b).await;
    ldap_conn
        .add_user(&mut user_a, Some("pass123"), &pool)
        .await
        .unwrap();
    ldap_conn
        .add_user(&mut user_b, Some("pass123"), &pool)
        .await
        .unwrap();

    // First "dup_grp" lives under ou=groups, with user A. group_exists only searches the current
    // base, so creating the second one under ou=users yields a distinct entry of the same cn.
    ldap_conn.config.ldap_group_search_base = groups_base.clone();
    let _ = ldap_conn.delete_group("dup_grp").await;
    ldap_conn
        .add_user_to_group(&user_a, "dup_grp")
        .await
        .unwrap();

    // Second "dup_grp" lives under ou=users, with user B.
    ldap_conn.config.ldap_group_search_base = users_base.clone();
    let _ = ldap_conn.delete_group("dup_grp").await;
    ldap_conn
        .add_user_to_group(&user_b, "dup_grp")
        .await
        .unwrap();

    user_a.clone().delete(&pool).await.unwrap();
    user_b.clone().delete(&pool).await.unwrap();

    // Search from the shared root: "dup_grp" resolves to both DNs and both members are pulled.
    ldap_conn.config.ldap_group_search_base = root_base;

    sync_ldap(&mut ldap_conn, &pool, true, &wg_tx).await;

    for username in ["dup_a_user", "dup_b_user"] {
        let pulled = User::find_by_username(&pool, username)
            .await
            .unwrap()
            .unwrap_or_else(|| {
                panic!("{username} belongs to a same-named sync group and must be pulled")
            });
        assert!(pulled.from_ldap);
        assert!(
            pulled
                .member_of_names(&pool)
                .await
                .unwrap()
                .contains(&String::from("dup_grp")),
            "{username} must be mirrored into the single Defguard group"
        );
    }

    let _ = ldap_conn.delete_user(&user_a).await;
    let _ = ldap_conn.delete_user(&user_b).await;
    ldap_conn.config.ldap_group_search_base = groups_base;
    let _ = ldap_conn.delete_group("dup_grp").await;
    ldap_conn.config.ldap_group_search_base = users_base;
    let _ = ldap_conn.delete_group("dup_grp").await;
}

/// Creates a smart referral entry (RFC 3296) directly in the directory and returns its DN. A
/// subtree search that crosses it makes the server emit a `SearchResultReference` PDU. The
/// `ManageDsaIt` control is required so the server lets us write the referral object itself
/// instead of returning it as a referral.
async fn put_referral_entry(cn: &str, base: &str, ref_url: &str) -> String {
    let url = env::var("LDAP_URL").unwrap();
    let bind = env::var("LDAP_BIND_USERNAME").unwrap();
    let pass = env::var("LDAP_BIND_PASSWORD").unwrap();
    let (conn, mut ldap) = LdapConnAsync::new(&url).await.unwrap();
    ldap3::drive!(conn);
    ldap.simple_bind(&bind, &pass)
        .await
        .unwrap()
        .success()
        .unwrap();

    let dn = format!("cn={cn},{base}");
    // Best-effort cleanup in case a previous run left it behind.
    let _ = ldap.with_controls(ManageDsaIt.critical()).delete(&dn).await;
    ldap.with_controls(ManageDsaIt.critical())
        .add(
            &dn,
            vec![
                (
                    "objectClass",
                    HashSet::from(["referral", "extensibleObject"]),
                ),
                ("cn", HashSet::from([cn])),
                ("ref", HashSet::from([ref_url])),
            ],
        )
        .await
        .unwrap()
        .success()
        .unwrap();
    let _ = ldap.unbind().await;
    dn
}

/// Deletes a referral entry created by [`put_referral_entry`]. `ManageDsaIt` is needed so the
/// delete targets the referral object itself rather than being chased.
async fn remove_referral_entry(dn: &str) {
    let url = env::var("LDAP_URL").unwrap();
    let bind = env::var("LDAP_BIND_USERNAME").unwrap();
    let pass = env::var("LDAP_BIND_PASSWORD").unwrap();
    let (conn, mut ldap) = LdapConnAsync::new(&url).await.unwrap();
    ldap3::drive!(conn);
    ldap.simple_bind(&bind, &pass)
        .await
        .unwrap()
        .success()
        .unwrap();
    let _ = ldap.with_controls(ManageDsaIt.critical()).delete(dn).await;
    let _ = ldap.unbind().await;
}

/// A subtree search that crosses an LDAP referral makes the server return a `SearchResultReference`
/// PDU. ldap3 0.12.1's `SearchEntry::construct` panics on those. The fix
/// filters referral/intermediate PDUs out, so a full sync must complete normally and still pull the
/// real user that lives under the same search base.
#[ignore = "requires LDAP server"]
#[sqlx::test]
async fn test_sync_skips_referral_pdus_without_aborting(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let (wg_tx, _wg_rx) = wg_test_channel();
    set_sync_settings(&pool, "referral_grp", true).await;

    let mut user = User::new(
        "referral_user",
        Some("pass123"),
        "Referral",
        "User",
        "referral@test.defguard",
        None,
    )
    .save(&pool)
    .await
    .unwrap();

    let mut ldap_conn = LDAPConnection::create().await.unwrap();
    ldap_conn.config.ldap_uses_ad = env::var("LDAP_USES_AD").is_ok();
    let user_base = ldap_conn.config.ldap_user_search_base.clone();
    let _ = ldap_conn.delete_user(&user).await;
    let _ = ldap_conn.delete_group("referral_grp").await;
    ldap_conn
        .add_user(&mut user, Some("pass123"), &pool)
        .await
        .unwrap();
    ldap_conn
        .add_user_to_group(&user, "referral_grp")
        .await
        .unwrap();
    user.clone().delete(&pool).await.unwrap();

    // Place a referral inside the user search base so list_users' subtree scan for the sync
    // group's members crosses it and the server returns a SearchResultReference PDU.
    let referral_dn = put_referral_entry(
        "referral_probe",
        &user_base,
        "ldap://referred.invalid/dc=referred,dc=invalid",
    )
    .await;

    // Before the fix this call aborts the process; after it, the sync completes and the real user
    // behind the referral is still pulled.
    sync_ldap(&mut ldap_conn, &pool, true, &wg_tx).await;

    let pulled = User::find_by_username(&pool, "referral_user")
        .await
        .unwrap()
        .expect("sync must finish and pull real users despite referral PDUs");
    assert!(pulled.from_ldap);

    remove_referral_entry(&referral_dn).await;
    let _ = ldap_conn.delete_user(&user).await;
    let _ = ldap_conn.delete_group("referral_grp").await;
}
