#[cfg(test)]
mod test {
    use std::{collections::HashSet, str::FromStr};

    use defguard_common::{
        config::{DefGuardConfig, SERVER_CONFIG},
        db::{
            models::{
                Device, DeviceType, Session, SessionState, Settings, User, WireguardNetwork,
                settings::initialize_current_settings, wireguard::ServiceLocationMode,
            },
            setup_pool,
        },
    };
    use ipnetwork::IpNetwork;
    use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
    use tokio::sync::{broadcast, mpsc};

    use super::super::{testprovider::FAILING_GROUP, *};
    use crate::{
        device_access::join_device_to_all_networks,
        enterprise::{
            db::models::{
                openid_provider::{DirectorySyncTarget, OpenIdProvider, OpenIdProviderKind},
                user_directory_identity::UserDirectoryIdentity,
            },
            license::{License, LicenseTier, SupportType, set_cached_license},
            limits::{get_counts, update_counts},
        },
        events::{DirectorySyncEvent, LdapSyncEventType},
        grpc::proto::enterprise::license::LicenseLimits,
    };

    /// Install a Business-tier licence with no limits.
    ///
    /// Tests needing specific limits build their own licence instead.
    fn set_business_license() {
        set_cached_license(Some(License::new(
            "test".to_owned(),
            false,
            None,
            None,
            None,
            LicenseTier::Business,
            SupportType::Basic,
            vec![],
        )));
    }

    async fn do_test_directory_sync(pool: &PgPool, gateway_tx: &broadcast::Sender<GatewayCommand>) {
        let (ldap_tx, _ldap_rx) = mpsc::unbounded_channel::<LdapSyncEventType>();
        let (dirsync_tx, _dirsync_rx) = dirsync_test_channel();
        do_directory_sync(pool, gateway_tx, &ldap_tx, &dirsync_tx)
            .await
            .unwrap();
    }

    fn ldap_test_channel() -> (
        mpsc::UnboundedSender<LdapSyncEventType>,
        mpsc::UnboundedReceiver<LdapSyncEventType>,
    ) {
        mpsc::unbounded_channel()
    }

    fn dirsync_test_channel() -> (
        mpsc::UnboundedSender<DirectorySyncEvent>,
        mpsc::UnboundedReceiver<DirectorySyncEvent>,
    ) {
        mpsc::unbounded_channel()
    }

    async fn get_test_network(pool: &PgPool) -> WireguardNetwork<Id> {
        WireguardNetwork::find_by_name(pool, "test")
            .await
            .unwrap()
            .unwrap()
            .pop()
            .unwrap()
    }

    async fn make_test_provider(
        pool: &PgPool,
        user_behavior: DirectorySyncUserBehavior,
        admin_behavior: DirectorySyncUserBehavior,
        target: DirectorySyncTarget,
        prefetch_users: bool,
    ) -> OpenIdProvider<Id> {
        // Directory sync is a business feature and its licence gate is compiled into test
        // builds, so without a licence every entry point below returns `Ok(())` without doing
        // any work. Seed one here; a test wanting the unlicensed path calls
        // `set_cached_license(None)` afterwards.
        set_business_license();

        Settings::initialize_runtime_defaults(pool).await.unwrap();
        initialize_current_settings(pool).await.unwrap();

        let current = OpenIdProvider::get_current(pool).await.unwrap();

        if let Some(provider) = current {
            provider.delete(pool).await.unwrap();
        }

        WireguardNetwork::new(
            "test".to_owned(),
            1234,
            "123.123.123.123".to_owned(),
            None,
            Vec::new(),
            true,
            false,
            false,
            false,
            false, // mfa_enabled
            ServiceLocationMode::Disabled,
        )
        .set_address([IpNetwork::from_str("10.10.10.1/24").unwrap()])
        .unwrap()
        .save(pool)
        .await
        .unwrap();

        OpenIdProvider::new(
            "Test".to_owned(),
            "base_url".to_owned(),
            OpenIdProviderKind::Google,
            "client_id".to_owned(),
            "client_secret".to_owned(),
            Some("display_name".to_owned()),
            Some("google_service_account_key".to_owned()),
            Some("google_service_account_email".to_owned()),
            Some("admin_email".to_owned()),
            true,
            60,
            user_behavior,
            admin_behavior,
            target,
            None,
            None,
            Vec::new(),
            None,
            prefetch_users,
            false,
            None,
        )
        .save(pool)
        .await
        .unwrap()
    }

    async fn make_test_user_and_device(name: &str, pool: &PgPool) -> User<Id> {
        let mut transaction = pool.begin().await.unwrap();

        let user = User::new(
            name,
            None,
            "lastname",
            "firstname",
            format!("{name}@email.com").as_str(),
            None,
        )
        .save(&mut *transaction)
        .await
        .unwrap();

        let dev = Device::new(
            format!("{name}-device"),
            format!("{name}-key"),
            user.id,
            DeviceType::User,
            None,
            true,
        )
        .save(&mut *transaction)
        .await
        .unwrap();

        join_device_to_all_networks(&mut transaction, &dev, &user)
            .await
            .unwrap();

        transaction.commit().await.unwrap();

        user
    }

    async fn get_test_user(pool: &PgPool, name: &str) -> Option<User<Id>> {
        User::find_by_username(pool, name).await.unwrap()
    }

    async fn make_admin(pool: &PgPool, user: &User<Id>) {
        let admin_group = Group::find_by_name(pool, "admin").await.unwrap().unwrap();
        user.add_to_group(pool, &admin_group).await.unwrap();
    }

    // Keep both users and admins
    #[sqlx::test]
    async fn test_users_state_keep_both(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;

        let config = DefGuardConfig::new_test_config();
        let _ = SERVER_CONFIG.set(config.clone());
        let (gateway_tx, mut gateway_rx) = broadcast::channel::<GatewayCommand>(16);
        make_test_provider(
            &pool,
            DirectorySyncUserBehavior::Keep,
            DirectorySyncUserBehavior::Keep,
            DirectorySyncTarget::All,
            false,
        )
        .await;
        let mut client = DirectorySyncClient::build(&pool).await.unwrap();
        client.prepare().await.unwrap();
        let user1 = make_test_user_and_device("user1", &pool).await;
        make_test_user_and_device("user2", &pool).await;
        make_test_user_and_device("testuser", &pool).await;
        make_admin(&pool, &user1).await;

        assert!(get_test_user(&pool, "user1").await.is_some());
        assert!(get_test_user(&pool, "user2").await.is_some());
        assert!(get_test_user(&pool, "testuser").await.is_some());

        let all_users = client.get_all_users().await.unwrap();
        let (ldap_tx, _ldap_rx) = ldap_test_channel();
        let (dirsync_tx, _dirsync_rx) = dirsync_test_channel();
        sync_all_users_state(&pool, &gateway_tx, &ldap_tx, &dirsync_tx, &all_users, None)
            .await
            .unwrap();

        assert!(get_test_user(&pool, "user1").await.is_some());
        assert!(get_test_user(&pool, "user2").await.is_some());
        assert!(get_test_user(&pool, "testuser").await.is_some());

        // No events
        assert!(gateway_rx.try_recv().is_err());
    }

    // Delete users, keep admins
    #[sqlx::test]
    async fn test_users_state_delete_users(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;

        let config = DefGuardConfig::new_test_config();
        let _ = SERVER_CONFIG.set(config.clone());
        let (gateway_tx, mut gateway_rx) = broadcast::channel::<GatewayCommand>(16);
        make_test_provider(
            &pool,
            DirectorySyncUserBehavior::Delete,
            DirectorySyncUserBehavior::Keep,
            DirectorySyncTarget::All,
            false,
        )
        .await;
        let mut client = DirectorySyncClient::build(&pool).await.unwrap();
        client.prepare().await.unwrap();

        let user1 = make_test_user_and_device("user1", &pool).await;
        let user2 = make_test_user_and_device("user2", &pool).await;
        make_test_user_and_device("testuser", &pool).await;
        make_admin(&pool, &user1).await;

        assert!(get_test_user(&pool, "user1").await.is_some());
        assert!(get_test_user(&pool, "user2").await.is_some());
        assert!(get_test_user(&pool, "testuser").await.is_some());

        let all_users = client.get_all_users().await.unwrap();
        let (ldap_tx, _ldap_rx) = ldap_test_channel();
        let (dirsync_tx, _dirsync_rx) = dirsync_test_channel();
        sync_all_users_state(&pool, &gateway_tx, &ldap_tx, &dirsync_tx, &all_users, None)
            .await
            .unwrap();

        assert!(get_test_user(&pool, "user1").await.is_some());
        assert!(get_test_user(&pool, "user2").await.is_none());
        assert!(get_test_user(&pool, "testuser").await.is_some());

        let event = gateway_rx.try_recv();
        if let Ok(GatewayCommand::DeviceDeleted(dev)) = event {
            assert_eq!(dev.device.user_id, user2.id);
        } else {
            panic!("Expected a DeviceDeleted event");
        }
    }
    #[sqlx::test]
    async fn test_users_state_delete_admins(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;

        let config = DefGuardConfig::new_test_config();
        let _ = SERVER_CONFIG.set(config.clone());
        let (gateway_tx, mut gateway_rx) = broadcast::channel::<GatewayCommand>(16);
        User::init_admin_user(&pool, "pass123").await.unwrap();

        let _ = make_test_provider(
            &pool,
            DirectorySyncUserBehavior::Keep,
            DirectorySyncUserBehavior::Delete,
            DirectorySyncTarget::All,
            false,
        )
        .await;
        let mut client = DirectorySyncClient::build(&pool).await.unwrap();
        client.prepare().await.unwrap();

        let user1 = make_test_user_and_device("user1", &pool).await;
        make_test_user_and_device("user2", &pool).await;
        let user3 = make_test_user_and_device("user3", &pool).await;
        make_test_user_and_device("testuser", &pool).await;
        make_admin(&pool, &user1).await;
        make_admin(&pool, &user3).await;

        assert!(get_test_user(&pool, "user1").await.is_some());
        assert!(get_test_user(&pool, "user2").await.is_some());
        assert!(get_test_user(&pool, "testuser").await.is_some());
        let all_users = client.get_all_users().await.unwrap();
        let (ldap_tx, _ldap_rx) = ldap_test_channel();
        let (dirsync_tx, _dirsync_rx) = dirsync_test_channel();
        sync_all_users_state(&pool, &gateway_tx, &ldap_tx, &dirsync_tx, &all_users, None)
            .await
            .unwrap();

        assert!(
            get_test_user(&pool, "user1").await.is_none()
                || get_test_user(&pool, "user3").await.is_none()
        );
        assert!(
            get_test_user(&pool, "user1").await.is_some()
                || get_test_user(&pool, "user3").await.is_some()
        );
        assert!(get_test_user(&pool, "user2").await.is_some());
        assert!(get_test_user(&pool, "testuser").await.is_some());

        // Check that we received a device deleted event for whichever admin was removed
        let event = gateway_rx.try_recv();
        if let Ok(GatewayCommand::DeviceDeleted(dev)) = event {
            assert!(dev.device.user_id == user1.id || dev.device.user_id == user3.id);
        } else {
            panic!("Expected a DeviceDeleted event");
        }
    }

    #[sqlx::test]
    async fn test_users_state_delete_both(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;

        let config = DefGuardConfig::new_test_config();
        let _ = SERVER_CONFIG.set(config.clone());
        let (gateway_tx, mut gateway_rx) = broadcast::channel::<GatewayCommand>(16);
        make_test_provider(
            &pool,
            DirectorySyncUserBehavior::Delete,
            DirectorySyncUserBehavior::Delete,
            DirectorySyncTarget::All,
            false,
        )
        .await;
        User::init_admin_user(&pool, "pass123").await.unwrap();
        let mut client = DirectorySyncClient::build(&pool).await.unwrap();
        client.prepare().await.unwrap();

        let user1 = make_test_user_and_device("user1", &pool).await;
        let user2 = make_test_user_and_device("user2", &pool).await;
        let user3 = make_test_user_and_device("user3", &pool).await;
        make_test_user_and_device("testuser", &pool).await;
        make_admin(&pool, &user1).await;
        make_admin(&pool, &user3).await;

        assert!(get_test_user(&pool, "user1").await.is_some());
        assert!(get_test_user(&pool, "user2").await.is_some());
        assert!(get_test_user(&pool, "testuser").await.is_some());
        let all_users = client.get_all_users().await.unwrap();
        let (ldap_tx, _ldap_rx) = ldap_test_channel();
        let (dirsync_tx, _dirsync_rx) = dirsync_test_channel();
        sync_all_users_state(&pool, &gateway_tx, &ldap_tx, &dirsync_tx, &all_users, None)
            .await
            .unwrap();

        assert!(
            get_test_user(&pool, "user1").await.is_none()
                || get_test_user(&pool, "user3").await.is_none()
        );
        assert!(
            get_test_user(&pool, "user1").await.is_some()
                || get_test_user(&pool, "user3").await.is_some()
        );
        assert!(get_test_user(&pool, "user2").await.is_none());
        assert!(get_test_user(&pool, "testuser").await.is_some());

        // Check for device deletion events
        let event1 = gateway_rx.try_recv();
        if let Ok(GatewayCommand::DeviceDeleted(dev)) = event1 {
            assert!(
                dev.device.user_id == user1.id
                    || dev.device.user_id == user2.id
                    || dev.device.user_id == user3.id
            );
        } else {
            panic!("Expected a DeviceDeleted event");
        }

        let event2 = gateway_rx.try_recv();
        if let Ok(GatewayCommand::DeviceDeleted(dev)) = event2 {
            assert!(
                dev.device.user_id == user1.id
                    || dev.device.user_id == user2.id
                    || dev.device.user_id == user3.id
            );
        } else {
            panic!("Expected a DeviceDeleted event");
        }
    }

    #[sqlx::test]
    async fn test_users_state_disable_users(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;

        let config = DefGuardConfig::new_test_config();
        let _ = SERVER_CONFIG.set(config.clone());
        let (gateway_tx, mut gateway_rx) = broadcast::channel::<GatewayCommand>(16);
        make_test_provider(
            &pool,
            DirectorySyncUserBehavior::Disable,
            DirectorySyncUserBehavior::Keep,
            DirectorySyncTarget::All,
            false,
        )
        .await;
        let mut client = DirectorySyncClient::build(&pool).await.unwrap();
        client.prepare().await.unwrap();

        let user1 = make_test_user_and_device("user1", &pool).await;
        make_test_user_and_device("user2", &pool).await;
        make_test_user_and_device("testuser", &pool).await;
        make_test_user_and_device("testuserdisabled", &pool).await;
        make_admin(&pool, &user1).await;

        let user1 = get_test_user(&pool, "user1").await.unwrap();
        let user2 = get_test_user(&pool, "user2").await.unwrap();
        let testuser = get_test_user(&pool, "testuser").await.unwrap();
        let testuserdisabled = get_test_user(&pool, "testuserdisabled").await.unwrap();
        let disabled_user_session = Session::new(
            testuserdisabled.id,
            SessionState::PasswordVerified,
            "127.0.0.1".into(),
            None,
        );
        disabled_user_session.save(&pool).await.unwrap();
        assert!(
            Session::find_by_id(&pool, &disabled_user_session.id)
                .await
                .unwrap()
                .is_some()
        );

        assert!(user1.is_active);
        assert!(user2.is_active);
        assert!(testuser.is_active);
        assert!(testuserdisabled.is_active);

        let all_users = client.get_all_users().await.unwrap();
        let (ldap_tx, _ldap_rx) = ldap_test_channel();
        let (dirsync_tx, _dirsync_rx) = dirsync_test_channel();
        sync_all_users_state(&pool, &gateway_tx, &ldap_tx, &dirsync_tx, &all_users, None)
            .await
            .unwrap();

        // Check for device disconnection events
        let event1 = gateway_rx.try_recv();
        if let Ok(GatewayCommand::DeviceDeleted(dev)) = event1 {
            assert!(dev.device.user_id == user2.id || dev.device.user_id == testuserdisabled.id);
        } else {
            panic!("Expected a DeviceDisconnected event");
        }

        let event2 = gateway_rx.try_recv();
        if let Ok(GatewayCommand::DeviceDeleted(dev)) = event2 {
            assert!(dev.device.user_id == user2.id || dev.device.user_id == testuserdisabled.id);
        } else {
            panic!("Expected a DeviceDisconnected event");
        }

        let user1 = get_test_user(&pool, "user1").await.unwrap();
        let user2 = get_test_user(&pool, "user2").await.unwrap();
        let testuser = get_test_user(&pool, "testuser").await.unwrap();
        let testuserdisabled = get_test_user(&pool, "testuserdisabled").await.unwrap();

        assert!(
            Session::find_by_id(&pool, &disabled_user_session.id)
                .await
                .unwrap()
                .is_none()
        );
        assert!(user1.is_active);
        assert!(!user2.is_active);
        assert!(testuser.is_active);
        assert!(!testuserdisabled.is_active);
    }
    #[sqlx::test]
    async fn test_users_state_disable_admins(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;

        let config = DefGuardConfig::new_test_config();
        let _ = SERVER_CONFIG.set(config.clone());
        let (gateway_tx, mut gateway_rx) = broadcast::channel::<GatewayCommand>(16); // Added mut gateway_rx
        make_test_provider(
            &pool,
            DirectorySyncUserBehavior::Keep,
            DirectorySyncUserBehavior::Disable,
            DirectorySyncTarget::All,
            false,
        )
        .await;
        let mut client = DirectorySyncClient::build(&pool).await.unwrap();
        client.prepare().await.unwrap();

        let user1 = make_test_user_and_device("user1", &pool).await;
        make_test_user_and_device("user2", &pool).await;
        let user3 = make_test_user_and_device("user3", &pool).await;
        make_test_user_and_device("testuser", &pool).await;
        make_test_user_and_device("testuserdisabled", &pool).await;
        make_admin(&pool, &user1).await;
        make_admin(&pool, &user3).await;

        let user1 = get_test_user(&pool, "user1").await.unwrap();
        let user2 = get_test_user(&pool, "user2").await.unwrap();
        let testuser = get_test_user(&pool, "testuser").await.unwrap();
        let testuserdisabled = get_test_user(&pool, "testuserdisabled").await.unwrap();

        assert!(user1.is_active);
        assert!(user2.is_active);
        assert!(user3.is_active);
        assert!(testuser.is_active);
        assert!(testuserdisabled.is_active);

        let all_users = client.get_all_users().await.unwrap();
        let (ldap_tx, _ldap_rx) = ldap_test_channel();
        let (dirsync_tx, _dirsync_rx) = dirsync_test_channel();
        sync_all_users_state(&pool, &gateway_tx, &ldap_tx, &dirsync_tx, &all_users, None)
            .await
            .unwrap();

        // Check for device disconnection events
        let event1 = gateway_rx.try_recv();
        if let Ok(GatewayCommand::DeviceDeleted(dev)) = event1 {
            assert!(
                dev.device.user_id == user1.id
                    || dev.device.user_id == user3.id
                    || dev.device.user_id == testuserdisabled.id
            );
        } else {
            panic!("Expected a DeviceDisconnected event");
        }

        let event2 = gateway_rx.try_recv();
        if let Ok(GatewayCommand::DeviceDeleted(dev)) = event2 {
            assert!(
                dev.device.user_id == user1.id
                    || dev.device.user_id == user3.id
                    || dev.device.user_id == testuserdisabled.id
            );
        } else {
            panic!("Expected a DeviceDisconnected event");
        }

        let user1 = get_test_user(&pool, "user1").await.unwrap();
        let user2 = get_test_user(&pool, "user2").await.unwrap();
        let user3 = get_test_user(&pool, "user3").await.unwrap();
        let testuser = get_test_user(&pool, "testuser").await.unwrap();
        let testuserdisabled = get_test_user(&pool, "testuserdisabled").await.unwrap();

        assert!(!user1.is_active || !user3.is_active);
        assert!(user1.is_active || user3.is_active);
        assert!(user2.is_active);
        assert!(testuser.is_active);
        assert!(!testuserdisabled.is_active);
    }

    #[sqlx::test]
    async fn test_users_groups(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;

        let config = DefGuardConfig::new_test_config();
        let _ = SERVER_CONFIG.set(config.clone());
        let (gateway_tx, _) = broadcast::channel::<GatewayCommand>(16);
        make_test_provider(
            &pool,
            DirectorySyncUserBehavior::Delete,
            DirectorySyncUserBehavior::Delete,
            DirectorySyncTarget::All,
            false,
        )
        .await;
        let mut client = DirectorySyncClient::build(&pool).await.unwrap();
        client.prepare().await.unwrap();

        make_test_user_and_device("testuser", &pool).await;
        make_test_user_and_device("testuser2", &pool).await;
        make_test_user_and_device("testuserdisabled", &pool).await;
        let all_users = client.get_all_users().await.unwrap();
        let (ldap_tx, _ldap_rx) = ldap_test_channel();
        let (dirsync_tx, _dirsync_rx) = dirsync_test_channel();
        sync_all_users_groups(
            &client,
            &pool,
            &gateway_tx,
            &ldap_tx,
            &dirsync_tx,
            "Test",
            Some(&all_users),
        )
        .await
        .unwrap();

        let mut groups = Group::all(&pool).await.unwrap();

        let testuser = get_test_user(&pool, "testuser").await.unwrap();
        let testuser2 = get_test_user(&pool, "testuser2").await.unwrap();
        let testuserdisabled = get_test_user(&pool, "testuserdisabled").await.unwrap();

        let testuser_groups = testuser.member_of(&pool).await.unwrap();
        let testuser2_groups = testuser2.member_of(&pool).await.unwrap();
        let testuserdisabled_groups = testuserdisabled.member_of(&pool).await.unwrap();

        assert_eq!(testuser_groups.len(), 3);
        assert_eq!(testuser2_groups.len(), 3);
        assert_eq!(testuserdisabled_groups.len(), 3);
        groups.sort_by(|a, b| a.name.cmp(&b.name));

        let group_present =
            |groups: &Vec<Group<Id>>, name: &str| groups.iter().any(|g| g.name == name);

        assert!(group_present(&testuser_groups, "group1"));
        assert!(group_present(&testuser_groups, "group2"));
        assert!(group_present(&testuser_groups, "group3"));

        assert!(group_present(&testuser2_groups, "group1"));
        assert!(group_present(&testuser2_groups, "group2"));
        assert!(group_present(&testuser2_groups, "group3"));

        assert!(group_present(&testuserdisabled_groups, "group1"));
        assert!(group_present(&testuserdisabled_groups, "group2"));
        assert!(group_present(&testuserdisabled_groups, "group3"));
    }

    #[sqlx::test]
    async fn test_sync_user_groups(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;

        let config = DefGuardConfig::new_test_config();
        let _ = SERVER_CONFIG.set(config.clone());
        let (gateway_tx, _) = broadcast::channel::<GatewayCommand>(16);
        make_test_provider(
            &pool,
            DirectorySyncUserBehavior::Delete,
            DirectorySyncUserBehavior::Delete,
            DirectorySyncTarget::All,
            false,
        )
        .await;
        let mut client = DirectorySyncClient::build(&pool).await.unwrap();
        client.prepare().await.unwrap();
        let user = make_test_user_and_device("testuser", &pool).await;
        let (ldap_tx, _ldap_rx) = mpsc::unbounded_channel::<LdapSyncEventType>();
        let (dirsync_tx, _dirsync_rx) = dirsync_test_channel();
        let user_groups = user.member_of(&pool).await.unwrap();
        assert_eq!(user_groups.len(), 0);
        sync_user_groups_if_configured(&user, &pool, &gateway_tx, &ldap_tx, &dirsync_tx)
            .await
            .unwrap();
        let user_groups = user.member_of(&pool).await.unwrap();
        assert_eq!(user_groups.len(), 1);
        let group = Group::find_by_name(&pool, "group1").await.unwrap().unwrap();
        assert_eq!(user_groups[0].id, group.id);
    }

    // Logging in through OIDC used to sync the user's groups regardless of the configured sync
    // target, overwriting locally managed group assignments when the target was set to users only.
    #[sqlx::test]
    async fn test_sync_user_groups_target_users(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;

        let config = DefGuardConfig::new_test_config();
        let _ = SERVER_CONFIG.set(config.clone());
        let (gateway_tx, _) = broadcast::channel::<GatewayCommand>(16);
        make_test_provider(
            &pool,
            DirectorySyncUserBehavior::Delete,
            DirectorySyncUserBehavior::Delete,
            DirectorySyncTarget::Users,
            false,
        )
        .await;
        let user = make_test_user_and_device("testuser", &pool).await;
        let local_group = Group::new("localgroup").save(&pool).await.unwrap();
        user.add_to_group(&pool, &local_group).await.unwrap();
        let (ldap_tx, _ldap_rx) = mpsc::unbounded_channel::<LdapSyncEventType>();
        let (dirsync_tx, _dirsync_rx) = dirsync_test_channel();

        sync_user_groups_if_configured(&user, &pool, &gateway_tx, &ldap_tx, &dirsync_tx)
            .await
            .unwrap();

        let user_groups = user.member_of(&pool).await.unwrap();
        assert_eq!(user_groups.len(), 1);
        assert_eq!(user_groups[0].id, local_group.id);
    }

    #[sqlx::test]
    async fn test_sync_target_users(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;

        let config = DefGuardConfig::new_test_config();
        let _ = SERVER_CONFIG.set(config.clone());
        let (gateway_tx, _) = broadcast::channel::<GatewayCommand>(16);
        make_test_provider(
            &pool,
            DirectorySyncUserBehavior::Delete,
            DirectorySyncUserBehavior::Delete,
            DirectorySyncTarget::Users,
            false,
        )
        .await;
        let mut client = DirectorySyncClient::build(&pool).await.unwrap();
        client.prepare().await.unwrap();
        let user = make_test_user_and_device("testuser", &pool).await;
        let user_groups = user.member_of(&pool).await.unwrap();
        assert_eq!(user_groups.len(), 0);
        do_test_directory_sync(&pool, &gateway_tx).await;
        let user_groups = user.member_of(&pool).await.unwrap();
        assert_eq!(user_groups.len(), 0);
    }

    #[sqlx::test]
    async fn test_sync_target_all(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;

        let config = DefGuardConfig::new_test_config();
        let _ = SERVER_CONFIG.set(config.clone());
        let (gateway_tx, mut gateway_rx) = broadcast::channel::<GatewayCommand>(16);
        make_test_provider(
            &pool,
            DirectorySyncUserBehavior::Delete,
            DirectorySyncUserBehavior::Delete,
            DirectorySyncTarget::All,
            false,
        )
        .await;
        let network = get_test_network(&pool).await;
        let mut transaction = pool.begin().await.unwrap();
        let group = Group::new("group1").save(&mut *transaction).await.unwrap();
        network
            .set_allowed_groups(&mut transaction, &[group.name])
            .await
            .unwrap();
        transaction.commit().await.unwrap();
        let mut client = DirectorySyncClient::build(&pool).await.unwrap();
        client.prepare().await.unwrap();
        let user = make_test_user_and_device("testuser", &pool).await;
        let user2_pre_sync = make_test_user_and_device("user2", &pool).await;
        let user_groups = user.member_of(&pool).await.unwrap();
        assert_eq!(user_groups.len(), 0);
        do_test_directory_sync(&pool, &gateway_tx).await;
        let user_groups = user.member_of(&pool).await.unwrap();
        assert_eq!(user_groups.len(), 3);
        let user2 = get_test_user(&pool, "user2").await;
        assert!(user2.is_none());
        let mut transaction = pool.begin().await.unwrap();
        sync_allowed_user_devices(&user, &mut transaction, &gateway_tx)
            .await
            .unwrap();
        transaction.commit().await.unwrap();
        let event = gateway_rx.try_recv();
        if let Ok(GatewayCommand::DeviceDeleted(dev)) = event {
            assert_eq!(dev.device.user_id, user2_pre_sync.id);
        } else {
            panic!("Expected DeviceDeleted event");
        }
        let event = gateway_rx.try_recv();
        if let Ok(GatewayCommand::DeviceCreated(dev)) = event {
            panic!("Unexpected DeviceCreated event: {dev:?}");
        }
    }

    #[sqlx::test]
    async fn test_sync_target_groups(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;

        let config = DefGuardConfig::new_test_config();
        let _ = SERVER_CONFIG.set(config.clone());
        let (gateway_tx, _) = broadcast::channel::<GatewayCommand>(16);
        make_test_provider(
            &pool,
            DirectorySyncUserBehavior::Delete,
            DirectorySyncUserBehavior::Delete,
            DirectorySyncTarget::Groups,
            false,
        )
        .await;
        let mut client = DirectorySyncClient::build(&pool).await.unwrap();
        client.prepare().await.unwrap();
        let user = make_test_user_and_device("testuser", &pool).await;
        make_test_user_and_device("user2", &pool).await;
        let user_groups = user.member_of(&pool).await.unwrap();
        assert_eq!(user_groups.len(), 0);
        do_test_directory_sync(&pool, &gateway_tx).await;
        let user_groups = user.member_of(&pool).await.unwrap();
        assert_eq!(user_groups.len(), 3);
        let user2 = get_test_user(&pool, "user2").await;
        assert!(user2.is_some());
    }

    #[sqlx::test]
    async fn test_sync_unassign_last_admin_group(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;

        let config = DefGuardConfig::new_test_config();
        let _ = SERVER_CONFIG.set(config.clone());
        let (gateway_tx, _) = broadcast::channel::<GatewayCommand>(16);
        make_test_provider(
            &pool,
            DirectorySyncUserBehavior::Delete,
            DirectorySyncUserBehavior::Delete,
            DirectorySyncTarget::All,
            false,
        )
        .await;
        let mut client = DirectorySyncClient::build(&pool).await.unwrap();
        client.prepare().await.unwrap();

        // Make one admin and check if he's deleted
        let user = make_test_user_and_device("testuser", &pool).await;
        let admin_grp = Group::find_by_name(&pool, "admin").await.unwrap().unwrap();
        user.add_to_group(&pool, &admin_grp).await.unwrap();
        let user_groups = user.member_of(&pool).await.unwrap();
        assert_eq!(user_groups.len(), 1);
        assert!(user.is_admin(&pool).await.unwrap());

        do_test_directory_sync(&pool, &gateway_tx).await;

        // He should still be an admin as it's the last one
        assert!(user.is_admin(&pool).await.unwrap());

        // Make another admin and check if one of them is deleted
        let user2 = make_test_user_and_device("testuser2", &pool).await;
        user2.add_to_group(&pool, &admin_grp).await.unwrap();

        do_test_directory_sync(&pool, &gateway_tx).await;

        let admins = User::find_admins(&pool).await.unwrap();
        // There should be only one admin left
        assert_eq!(admins.len(), 1);

        let defguard_user = make_test_user_and_device("defguard", &pool).await;
        make_admin(&pool, &defguard_user).await;

        do_test_directory_sync(&pool, &gateway_tx).await;
    }

    #[sqlx::test]
    async fn test_sync_delete_last_admin_user(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;

        let config = DefGuardConfig::new_test_config();
        let _ = SERVER_CONFIG.set(config.clone());
        let (gateway_tx, _) = broadcast::channel::<GatewayCommand>(16);
        make_test_provider(
            &pool,
            DirectorySyncUserBehavior::Delete,
            DirectorySyncUserBehavior::Delete,
            DirectorySyncTarget::All,
            false,
        )
        .await;
        let mut client = DirectorySyncClient::build(&pool).await.unwrap();
        client.prepare().await.unwrap();

        // a user that's not in the directory
        let defguard_user = make_test_user_and_device("defguard", &pool).await;
        make_admin(&pool, &defguard_user).await;
        assert!(defguard_user.is_admin(&pool).await.unwrap());

        do_test_directory_sync(&pool, &gateway_tx).await;

        // The user should still be an admin
        assert!(defguard_user.is_admin(&pool).await.unwrap());

        // remove his admin status
        let admin_grp = Group::find_by_name(&pool, "admin").await.unwrap().unwrap();
        defguard_user
            .remove_from_group(&pool, &admin_grp)
            .await
            .unwrap();

        do_test_directory_sync(&pool, &gateway_tx).await;
        let user = User::find_by_username(&pool, "defguard").await.unwrap();
        assert!(user.is_none());
    }

    #[sqlx::test]
    async fn test_users_no_prefetch(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;

        let config = DefGuardConfig::new_test_config();
        let _ = SERVER_CONFIG.set(config.clone());
        let (gateway_tx, mut gateway_rx) = broadcast::channel::<GatewayCommand>(16);

        // disable prefetching users
        make_test_provider(
            &pool,
            DirectorySyncUserBehavior::Keep,
            DirectorySyncUserBehavior::Keep,
            DirectorySyncTarget::All,
            false,
        )
        .await;
        let mut client = DirectorySyncClient::build(&pool).await.unwrap();
        client.prepare().await.unwrap();

        // no users in Defguard before sync
        let defguard_users = User::all(&pool).await.unwrap();
        assert!(defguard_users.is_empty());

        do_test_directory_sync(&pool, &gateway_tx).await;

        // no users in Defguard after sync
        let defguard_users = User::all(&pool).await.unwrap();
        assert!(defguard_users.is_empty());

        // No events
        assert!(gateway_rx.try_recv().is_err());
    }

    #[sqlx::test]
    async fn test_users_prefetch(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;

        let config = DefGuardConfig::new_test_config();
        let _ = SERVER_CONFIG.set(config.clone());
        let (gateway_tx, mut gateway_rx) = broadcast::channel::<GatewayCommand>(16);

        // enable prefetching users
        make_test_provider(
            &pool,
            DirectorySyncUserBehavior::Keep,
            DirectorySyncUserBehavior::Keep,
            DirectorySyncTarget::All,
            true,
        )
        .await;
        let mut client = DirectorySyncClient::build(&pool).await.unwrap();
        client.prepare().await.unwrap();

        // no users in Defguard before sync
        let defguard_users = User::all(&pool).await.unwrap();
        assert!(defguard_users.is_empty());

        do_test_directory_sync(&pool, &gateway_tx).await;

        // all active directory users were synced
        let defguard_users = User::all(&pool).await.unwrap();
        assert_eq!(defguard_users.len(), 3);

        // No events
        assert!(gateway_rx.try_recv().is_err());
    }

    #[sqlx::test]
    async fn test_users_prefetch_group_filter(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;

        let config = DefGuardConfig::new_test_config();
        let _ = SERVER_CONFIG.set(config.clone());
        let (gateway_tx, mut gateway_rx) = broadcast::channel::<GatewayCommand>(16);

        // enable prefetching users, import only members of group1
        let mut provider = make_test_provider(
            &pool,
            DirectorySyncUserBehavior::Keep,
            DirectorySyncUserBehavior::Keep,
            DirectorySyncTarget::All,
            true,
        )
        .await;
        provider.directory_sync_user_groups = Some(vec!["group1".to_owned()]);
        provider.save(&pool).await.unwrap();

        // no users in Defguard before sync
        let defguard_users = User::all(&pool).await.unwrap();
        assert!(defguard_users.is_empty());

        let (ldap_tx, _ldap_rx) = ldap_test_channel();
        let (dirsync_tx, _dirsync_rx) = dirsync_test_channel();
        do_directory_sync(&pool, &gateway_tx, &ldap_tx, &dirsync_tx)
            .await
            .unwrap();

        // all directory users are members of group1, so all of them were imported
        let defguard_users = User::all(&pool).await.unwrap();
        assert_eq!(defguard_users.len(), 3);

        // No events
        assert!(gateway_rx.try_recv().is_err());
    }

    #[sqlx::test]
    async fn test_users_prefetch_group_filter_no_match(
        _: PgPoolOptions,
        options: PgConnectOptions,
    ) {
        let pool = setup_pool(options).await;

        let config = DefGuardConfig::new_test_config();
        let _ = SERVER_CONFIG.set(config.clone());
        let (gateway_tx, mut gateway_rx) = broadcast::channel::<GatewayCommand>(16);

        // enable prefetching users, import only members of a group that doesn't exist
        // in the directory
        let mut provider = make_test_provider(
            &pool,
            DirectorySyncUserBehavior::Keep,
            DirectorySyncUserBehavior::Keep,
            DirectorySyncTarget::Users,
            true,
        )
        .await;
        provider.directory_sync_user_groups = Some(vec!["nonexistent-group".to_owned()]);
        provider.save(&pool).await.unwrap();

        let (ldap_tx, _ldap_rx) = ldap_test_channel();
        let (dirsync_tx, _dirsync_rx) = dirsync_test_channel();
        do_directory_sync(&pool, &gateway_tx, &ldap_tx, &dirsync_tx)
            .await
            .unwrap();

        // no users were imported
        let defguard_users = User::all(&pool).await.unwrap();
        assert!(defguard_users.is_empty());

        // No events
        assert!(gateway_rx.try_recv().is_err());
    }

    #[sqlx::test]
    async fn test_user_sync_filter_keeps_users_when_member_query_fails(
        _: PgPoolOptions,
        options: PgConnectOptions,
    ) {
        let pool = setup_pool(options).await;

        let config = DefGuardConfig::new_test_config();
        let _ = SERVER_CONFIG.set(config.clone());
        let (gateway_tx, _gateway_rx) = broadcast::channel::<GatewayCommand>(16);

        // limit the sync to a group whose member query always fails
        let mut provider = make_test_provider(
            &pool,
            DirectorySyncUserBehavior::Delete,
            DirectorySyncUserBehavior::Delete,
            DirectorySyncTarget::Users,
            true,
        )
        .await;
        provider.directory_sync_user_groups = Some(vec![FAILING_GROUP.to_owned()]);
        provider.save(&pool).await.unwrap();

        make_test_user_and_device("FirstName", &pool).await;
        make_test_user_and_device("LastName", &pool).await;
        let users_before = User::all(&pool).await.unwrap().len();
        assert_eq!(users_before, 2);

        let (ldap_tx, _ldap_rx) = ldap_test_channel();
        let (dirsync_tx, _dirsync_rx) = dirsync_test_channel();
        let result = do_directory_sync(&pool, &gateway_tx, &ldap_tx, &dirsync_tx).await;

        // the sync aborts instead of acting on an incomplete list of allowed users
        assert_eq!(User::all(&pool).await.unwrap().len(), users_before);
        assert!(get_test_user(&pool, "FirstName").await.is_some());
        assert!(get_test_user(&pool, "LastName").await.is_some());
        assert!(result.is_err());
    }

    #[sqlx::test]
    async fn test_users_prefetch_allowed_emails(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;

        let config = DefGuardConfig::new_test_config();
        let _ = SERVER_CONFIG.set(config.clone());
        let (gateway_tx, mut gateway_rx) = broadcast::channel::<GatewayCommand>(16);

        // enable prefetching users
        make_test_provider(
            &pool,
            DirectorySyncUserBehavior::Keep,
            DirectorySyncUserBehavior::Keep,
            DirectorySyncTarget::All,
            true,
        )
        .await;
        let mut client = DirectorySyncClient::build(&pool).await.unwrap();
        client.prepare().await.unwrap();

        // no users in Defguard before sync
        let defguard_users = User::all(&pool).await.unwrap();
        assert!(defguard_users.is_empty());

        // only allow one of the directory users to be imported
        let allowed_emails = HashSet::from(["testuser@email.com".to_owned()]);
        let all_users = client.get_all_users().await.unwrap();
        let (ldap_tx, _ldap_rx) = ldap_test_channel();
        let (dirsync_tx, _dirsync_rx) = dirsync_test_channel();
        sync_all_users_state(
            &pool,
            &gateway_tx,
            &ldap_tx,
            &dirsync_tx,
            &all_users,
            Some(allowed_emails),
        )
        .await
        .unwrap();

        // only the allowed user was imported
        let defguard_users = User::all(&pool).await.unwrap();
        assert_eq!(defguard_users.len(), 1);
        assert_eq!(defguard_users[0].email, "testuser@email.com");

        // No events
        assert!(gateway_rx.try_recv().is_err());
    }

    // Regression test for a bug where changing a user's email address in the directory (e.g.
    // Entra ID) caused directory sync to try to create a new Defguard user for them.
    #[sqlx::test]
    async fn test_users_prefetch_email_changed_in_directory(
        _: PgPoolOptions,
        options: PgConnectOptions,
    ) {
        let pool = setup_pool(options).await;

        let config = DefGuardConfig::new_test_config();
        let _ = SERVER_CONFIG.set(config.clone());
        let (gateway_tx, _gateway_rx) = broadcast::channel::<GatewayCommand>(16);

        // enable prefetching users
        make_test_provider(
            &pool,
            DirectorySyncUserBehavior::Keep,
            DirectorySyncUserBehavior::Keep,
            DirectorySyncTarget::All,
            true,
        )
        .await;

        let directory_user = DirectoryUser {
            email: "alice@email.com".into(),
            active: true,
            id: Some("entra-alice-id".into()),
            user_details: Some(DirectoryUserDetails {
                last_name: "Doe".into(),
                first_name: "Alice".into(),
                phone_number: None,
            }),
        };

        let (ldap_tx, _ldap_rx) = ldap_test_channel();
        let (dirsync_tx, _dirsync_rx) = dirsync_test_channel();
        sync_all_users_state(
            &pool,
            &gateway_tx,
            &ldap_tx,
            &dirsync_tx,
            &[directory_user],
            None,
        )
        .await
        .unwrap();

        // user was imported with a username derived from their original email
        let defguard_users = User::all(&pool).await.unwrap();
        assert_eq!(defguard_users.len(), 1);
        assert_eq!(defguard_users[0].username, "alice");
        assert_eq!(defguard_users[0].email, "alice@email.com");
        let original_user_id = defguard_users[0].id;

        // the same directory user (matched by directory id) changes their email in the
        // directory, e.g. via a name change in Entra ID
        let directory_user_new_email = DirectoryUser {
            email: "alice@newteam.com".into(),
            active: true,
            id: Some("entra-alice-id".into()),
            user_details: Some(DirectoryUserDetails {
                last_name: "Doe".into(),
                first_name: "Alice".into(),
                phone_number: None,
            }),
        };

        let (ldap_tx, _ldap_rx) = ldap_test_channel();
        let (dirsync_tx, _dirsync_rx) = dirsync_test_channel();
        sync_all_users_state(
            &pool,
            &gateway_tx,
            &ldap_tx,
            &dirsync_tx,
            &[directory_user_new_email],
            None,
        )
        .await
        .unwrap();

        // the existing user was updated in place instead of a duplicate being created
        let defguard_users = User::all(&pool).await.unwrap();
        assert_eq!(defguard_users.len(), 1);
        assert_eq!(defguard_users[0].username, "alice");
        assert_eq!(defguard_users[0].email, "alice@newteam.com");
        assert_eq!(defguard_users[0].id, original_user_id);
    }

    // Regression test for the case where a user was never created through directory sync's
    // prefetch (e.g. they were invited manually, or existed before prefetch was enabled) and so
    // has no directory identity mapping stored. Directory sync should backfill it the first time
    // it matches such a user by email, so that a later email change in the directory can still be
    // matched by directory ID.
    #[sqlx::test]
    async fn test_users_prefetch_backfills_directory_identity_for_preexisting_user(
        _: PgPoolOptions,
        options: PgConnectOptions,
    ) {
        let pool = setup_pool(options).await;

        let config = DefGuardConfig::new_test_config();
        let _ = SERVER_CONFIG.set(config.clone());
        let (gateway_tx, _gateway_rx) = broadcast::channel::<GatewayCommand>(16);

        // enable prefetching users
        make_test_provider(
            &pool,
            DirectorySyncUserBehavior::Keep,
            DirectorySyncUserBehavior::Keep,
            DirectorySyncTarget::All,
            true,
        )
        .await;

        // user already exists in Defguard, created some other way (e.g. manual invite)
        let _user = User::new("alice", None, "Doe", "Alice", "alice@email.com", None)
            .save(&pool)
            .await
            .unwrap();

        let directory_user = DirectoryUser {
            email: "alice@email.com".into(),
            active: true,
            id: Some("entra-alice-id".into()),
            user_details: Some(DirectoryUserDetails {
                last_name: "Doe".into(),
                first_name: "Alice".into(),
                phone_number: None,
            }),
        };

        let (ldap_tx, _ldap_rx) = ldap_test_channel();
        let (dirsync_tx, _dirsync_rx) = dirsync_test_channel();
        sync_all_users_state(
            &pool,
            &gateway_tx,
            &ldap_tx,
            &dirsync_tx,
            &[directory_user],
            None,
        )
        .await
        .unwrap();

        // no duplicate was created, and the directory ID was backfilled into the mapping table
        let defguard_users = User::all(&pool).await.unwrap();
        assert_eq!(defguard_users.len(), 1);
        let provider = OpenIdProvider::get_current(&pool).await.unwrap().unwrap();
        let identity = UserDirectoryIdentity::find_by_user_and_provider(
            &pool,
            defguard_users[0].id,
            provider.id,
        )
        .await
        .unwrap();
        assert_eq!(
            identity.map(|i| i.external_id),
            Some("entra-alice-id".to_string())
        );

        // the user changes their email in the directory
        let directory_user_new_email = DirectoryUser {
            email: "alice@newteam.com".into(),
            active: true,
            id: Some("entra-alice-id".into()),
            user_details: Some(DirectoryUserDetails {
                last_name: "Doe".into(),
                first_name: "Alice".into(),
                phone_number: None,
            }),
        };

        let (ldap_tx, _ldap_rx) = ldap_test_channel();
        let (dirsync_tx, _dirsync_rx) = dirsync_test_channel();
        sync_all_users_state(
            &pool,
            &gateway_tx,
            &ldap_tx,
            &dirsync_tx,
            &[directory_user_new_email],
            None,
        )
        .await
        .unwrap();

        // the existing user was updated in place instead of a duplicate being created
        let defguard_users = User::all(&pool).await.unwrap();
        assert_eq!(defguard_users.len(), 1);
        assert_eq!(defguard_users[0].username, "alice");
        assert_eq!(defguard_users[0].email, "alice@newteam.com");
        assert_eq!(defguard_users[0].id, _user.id);
    }

    // Regression test: if a directory user's new email already belongs to a different Defguard
    // user, updating in place would hit the email UNIQUE constraint. Directory sync should skip
    // that user instead of aborting the whole sync.
    #[sqlx::test]
    async fn test_users_prefetch_email_changed_to_existing_user_email(
        _: PgPoolOptions,
        options: PgConnectOptions,
    ) {
        let pool = setup_pool(options).await;

        let config = DefGuardConfig::new_test_config();
        let _ = SERVER_CONFIG.set(config.clone());
        let (gateway_tx, _gateway_rx) = broadcast::channel::<GatewayCommand>(16);

        make_test_provider(
            &pool,
            DirectorySyncUserBehavior::Keep,
            DirectorySyncUserBehavior::Keep,
            DirectorySyncTarget::All,
            true,
        )
        .await;

        let directory_user = DirectoryUser {
            email: "alice@email.com".into(),
            active: true,
            id: Some("entra-alice-id".into()),
            user_details: Some(DirectoryUserDetails {
                last_name: "Doe".into(),
                first_name: "Alice".into(),
                phone_number: None,
            }),
        };

        let (ldap_tx, _ldap_rx) = ldap_test_channel();
        let (dirsync_tx, _dirsync_rx) = dirsync_test_channel();
        sync_all_users_state(
            &pool,
            &gateway_tx,
            &ldap_tx,
            &dirsync_tx,
            &[directory_user],
            None,
        )
        .await
        .unwrap();

        let defguard_users = User::all(&pool).await.unwrap();
        assert_eq!(defguard_users.len(), 1);
        let alice_id = defguard_users[0].id;

        // a different Defguard user already occupies the email alice is about to change to,
        // e.g. someone who was invited manually
        let _bob = User::new("bob", None, "Smith", "Bob", "bob@newteam.com", None)
            .save(&pool)
            .await
            .unwrap();

        // alice changes her email (matched by directory id) to bob's email
        let directory_user_new_email = DirectoryUser {
            email: "bob@newteam.com".into(),
            active: true,
            id: Some("entra-alice-id".into()),
            user_details: Some(DirectoryUserDetails {
                last_name: "Doe".into(),
                first_name: "Alice".into(),
                phone_number: None,
            }),
        };

        let (ldap_tx, _ldap_rx) = ldap_test_channel();
        let (dirsync_tx, _dirsync_rx) = dirsync_test_channel();
        sync_all_users_state(
            &pool,
            &gateway_tx,
            &ldap_tx,
            &dirsync_tx,
            &[directory_user_new_email],
            None,
        )
        .await
        .unwrap();

        // sync succeeded as a whole; alice was skipped and left unchanged instead of the
        // email update failing on the unique constraint
        let defguard_users = User::all(&pool).await.unwrap();
        assert_eq!(defguard_users.len(), 2);
        let alice = defguard_users.iter().find(|u| u.id == alice_id).unwrap();
        assert_eq!(alice.email, "alice@email.com");
        let bob = defguard_users.iter().find(|u| u.username == "bob").unwrap();
        assert_eq!(bob.email, "bob@newteam.com");
    }

    #[sqlx::test]
    async fn test_user_in_directory_groups(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;

        let config = DefGuardConfig::new_test_config();
        let _ = SERVER_CONFIG.set(config.clone());
        make_test_provider(
            &pool,
            DirectorySyncUserBehavior::Keep,
            DirectorySyncUserBehavior::Keep,
            DirectorySyncTarget::All,
            false,
        )
        .await;

        // the test provider returns group1 as the only group of any user
        assert!(
            user_in_directory_groups(&pool, "testuser@email.com", &["group1".to_owned()])
                .await
                .unwrap()
        );
        assert!(
            !user_in_directory_groups(&pool, "testuser@email.com", &["group2".to_owned()])
                .await
                .unwrap()
        );
    }

    #[sqlx::test]
    async fn test_users_prefetch_respects_license_user_limit(
        _: PgPoolOptions,
        options: PgConnectOptions,
    ) {
        let pool = setup_pool(options).await;

        let config = DefGuardConfig::new_test_config();
        let _ = SERVER_CONFIG.set(config);
        let (gateway_tx, mut gateway_rx) = broadcast::channel::<GatewayCommand>(16);

        // enable prefetching users
        make_test_provider(
            &pool,
            DirectorySyncUserBehavior::Keep,
            DirectorySyncUserBehavior::Keep,
            DirectorySyncTarget::All,
            true,
        )
        .await;

        let user_limit = 1;
        let license = License::new(
            "test".to_owned(),
            false,
            None,
            Some(LicenseLimits {
                users: user_limit,
                devices: 100,
                locations: 100,
                network_devices: Some(100),
            }),
            None,
            LicenseTier::Business,
            SupportType::Basic,
            vec![],
        );
        set_cached_license(Some(license));
        update_counts(&pool).await.unwrap();

        do_test_directory_sync(&pool, &gateway_tx).await;
        update_counts(&pool).await.unwrap();

        let user_count = get_counts().user();
        assert!(user_count <= user_limit);

        let defguard_users = User::all(&pool).await.unwrap();
        assert_eq!(defguard_users.len(), user_limit as usize);

        // No events
        assert!(gateway_rx.try_recv().is_err());
    }

    // directory_sync_user_groups must be honored for every provider and
    // regardless of the prefetch setting.
    #[sqlx::test]
    async fn test_users_group_filter_applies_without_prefetch(
        _: PgPoolOptions,
        options: PgConnectOptions,
    ) {
        let pool = setup_pool(options).await;

        let config = DefGuardConfig::new_test_config();
        let _ = SERVER_CONFIG.set(config.clone());
        let (gateway_tx, _gateway_rx) = broadcast::channel::<GatewayCommand>(16);

        // prefetch disabled, restrict sync to a group that has no members in the directory
        let mut provider = make_test_provider(
            &pool,
            DirectorySyncUserBehavior::Disable,
            DirectorySyncUserBehavior::Keep,
            DirectorySyncTarget::Users,
            false,
        )
        .await;
        provider.directory_sync_user_groups = Some(vec!["nonexistent-group".to_owned()]);
        provider.save(&pool).await.unwrap();

        // users already present in Defguard, matching directory users which are normally active
        make_test_user_and_device("testuser", &pool).await;
        make_test_user_and_device("testuser2", &pool).await;

        let testuser = get_test_user(&pool, "testuser").await.unwrap();
        let testuser2 = get_test_user(&pool, "testuser2").await.unwrap();
        assert!(testuser.is_active);
        assert!(testuser2.is_active);

        do_test_directory_sync(&pool, &gateway_tx).await;

        // both users were excluded from the group filter, so they are treated as no longer
        // present in the directory and get disabled, even though prefetch was never enabled
        let testuser = get_test_user(&pool, "testuser").await.unwrap();
        let testuser2 = get_test_user(&pool, "testuser2").await.unwrap();
        assert!(!testuser.is_active);
        assert!(!testuser2.is_active);
    }
}
