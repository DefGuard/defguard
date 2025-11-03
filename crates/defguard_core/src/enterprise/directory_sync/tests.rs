#[cfg(test)]
mod test {
    use std::str::FromStr;

    use defguard_common::{
        config::{DefGuardConfig, SERVER_CONFIG},
        db::{
            models::{Settings, settings::initialize_current_settings},
            setup_pool,
        },
    };
    use ipnetwork::IpNetwork;
    use secrecy::ExposeSecret;
    use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
    use tokio::sync::broadcast;

    use super::super::*;
    use crate::{
        db::{
            Device, Session, SessionState, WireguardNetwork,
            models::{
                device::DeviceType,
                wireguard::{LocationMfaMode, ServiceLocationMode},
            },
        },
        enterprise::db::models::openid_provider::DirectorySyncTarget,
    };

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
        Settings::init_defaults(pool).await.unwrap();
        initialize_current_settings(pool).await.unwrap();

        let current = OpenIdProvider::get_current(pool).await.unwrap();

        if let Some(provider) = current {
            provider.delete(pool).await.unwrap();
        }

        WireguardNetwork::new(
            "test".to_string(),
            vec![IpNetwork::from_str("10.10.10.1/24").unwrap()],
            1234,
            "123.123.123.123".to_string(),
            None,
            vec![],
            32,
            32,
            false,
            false,
            LocationMfaMode::Disabled,
            ServiceLocationMode::Disabled,
        )
        .save(pool)
        .await
        .unwrap();

        OpenIdProvider::new(
            "Test".to_string(),
            "base_url".to_string(),
            "client_id".to_string(),
            "client_secret".to_string(),
            Some("display_name".to_string()),
            Some("google_service_account_key".to_string()),
            Some("google_service_account_email".to_string()),
            Some("admin_email".to_string()),
            true,
            60,
            user_behavior,
            admin_behavior,
            target,
            None,
            None,
            vec![],
            None,
            prefetch_users,
        )
        .save(pool)
        .await
        .unwrap()
    }

    async fn make_test_user_and_device(name: &str, pool: &PgPool) -> User<Id> {
        let user = User::new(
            name,
            None,
            "lastname",
            "firstname",
            format!("{name}@email.com").as_str(),
            None,
        )
        .save(pool)
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
        .save(pool)
        .await
        .unwrap();

        let mut transaction = pool.begin().await.unwrap();
        dev.add_to_all_networks(&mut transaction).await.unwrap();
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
        let (wg_tx, mut wg_rx) = broadcast::channel::<GatewayEvent>(16);
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
        sync_all_users_state(&pool, &wg_tx, &all_users)
            .await
            .unwrap();

        assert!(get_test_user(&pool, "user1").await.is_some());
        assert!(get_test_user(&pool, "user2").await.is_some());
        assert!(get_test_user(&pool, "testuser").await.is_some());

        // No events
        assert!(wg_rx.try_recv().is_err());
    }

    // Delete users, keep admins
    #[sqlx::test]
    async fn test_users_state_delete_users(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;

        let config = DefGuardConfig::new_test_config();
        let _ = SERVER_CONFIG.set(config.clone());
        let (wg_tx, mut wg_rx) = broadcast::channel::<GatewayEvent>(16);
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
        sync_all_users_state(&pool, &wg_tx, &all_users)
            .await
            .unwrap();

        assert!(get_test_user(&pool, "user1").await.is_some());
        assert!(get_test_user(&pool, "user2").await.is_none());
        assert!(get_test_user(&pool, "testuser").await.is_some());

        let event = wg_rx.try_recv();
        if let Ok(GatewayEvent::DeviceDeleted(dev)) = event {
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
        let (wg_tx, mut wg_rx) = broadcast::channel::<GatewayEvent>(16);
        User::init_admin_user(&pool, config.default_admin_password.expose_secret())
            .await
            .unwrap();

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
        sync_all_users_state(&pool, &wg_tx, &all_users)
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
        let event = wg_rx.try_recv();
        if let Ok(GatewayEvent::DeviceDeleted(dev)) = event {
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
        let (wg_tx, mut wg_rx) = broadcast::channel::<GatewayEvent>(16);
        make_test_provider(
            &pool,
            DirectorySyncUserBehavior::Delete,
            DirectorySyncUserBehavior::Delete,
            DirectorySyncTarget::All,
            false,
        )
        .await;
        User::init_admin_user(&pool, config.default_admin_password.expose_secret())
            .await
            .unwrap();
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
        sync_all_users_state(&pool, &wg_tx, &all_users)
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
        let event1 = wg_rx.try_recv();
        if let Ok(GatewayEvent::DeviceDeleted(dev)) = event1 {
            assert!(
                dev.device.user_id == user1.id
                    || dev.device.user_id == user2.id
                    || dev.device.user_id == user3.id
            );
        } else {
            panic!("Expected a DeviceDeleted event");
        }

        let event2 = wg_rx.try_recv();
        if let Ok(GatewayEvent::DeviceDeleted(dev)) = event2 {
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
        let (wg_tx, mut wg_rx) = broadcast::channel::<GatewayEvent>(16);
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
        sync_all_users_state(&pool, &wg_tx, &all_users)
            .await
            .unwrap();

        // Check for device disconnection events
        let event1 = wg_rx.try_recv();
        if let Ok(GatewayEvent::DeviceDeleted(dev)) = event1 {
            assert!(dev.device.user_id == user2.id || dev.device.user_id == testuserdisabled.id);
        } else {
            panic!("Expected a DeviceDisconnected event");
        }

        let event2 = wg_rx.try_recv();
        if let Ok(GatewayEvent::DeviceDeleted(dev)) = event2 {
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
        let (wg_tx, mut wg_rx) = broadcast::channel::<GatewayEvent>(16); // Added mut wg_rx
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
        sync_all_users_state(&pool, &wg_tx, &all_users)
            .await
            .unwrap();

        // Check for device disconnection events
        let event1 = wg_rx.try_recv();
        if let Ok(GatewayEvent::DeviceDeleted(dev)) = event1 {
            assert!(
                dev.device.user_id == user1.id
                    || dev.device.user_id == user3.id
                    || dev.device.user_id == testuserdisabled.id
            );
        } else {
            panic!("Expected a DeviceDisconnected event");
        }

        let event2 = wg_rx.try_recv();
        if let Ok(GatewayEvent::DeviceDeleted(dev)) = event2 {
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
        let (wg_tx, _) = broadcast::channel::<GatewayEvent>(16);
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
        sync_all_users_groups(&client, &pool, &wg_tx, Some(&all_users))
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
        let (wg_tx, _) = broadcast::channel::<GatewayEvent>(16);
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
        let user_groups = user.member_of(&pool).await.unwrap();
        assert_eq!(user_groups.len(), 0);
        sync_user_groups_if_configured(&user, &pool, &wg_tx)
            .await
            .unwrap();
        let user_groups = user.member_of(&pool).await.unwrap();
        assert_eq!(user_groups.len(), 1);
        let group = Group::find_by_name(&pool, "group1").await.unwrap().unwrap();
        assert_eq!(user_groups[0].id, group.id);
    }

    #[sqlx::test]
    async fn test_sync_target_users(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;

        let config = DefGuardConfig::new_test_config();
        let _ = SERVER_CONFIG.set(config.clone());
        let (wg_tx, _) = broadcast::channel::<GatewayEvent>(16);
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
        do_directory_sync(&pool, &wg_tx).await.unwrap();
        let user_groups = user.member_of(&pool).await.unwrap();
        assert_eq!(user_groups.len(), 0);
    }

    #[sqlx::test]
    async fn test_sync_target_all(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;

        let config = DefGuardConfig::new_test_config();
        let _ = SERVER_CONFIG.set(config.clone());
        let (wg_tx, mut wg_rx) = broadcast::channel::<GatewayEvent>(16);
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
        let group = Group::new("group1".to_string())
            .save(&mut *transaction)
            .await
            .unwrap();
        network
            .set_allowed_groups(&mut transaction, vec![group.name])
            .await
            .unwrap();
        transaction.commit().await.unwrap();
        let mut client = DirectorySyncClient::build(&pool).await.unwrap();
        client.prepare().await.unwrap();
        let user = make_test_user_and_device("testuser", &pool).await;
        let user2_pre_sync = make_test_user_and_device("user2", &pool).await;
        let user_groups = user.member_of(&pool).await.unwrap();
        assert_eq!(user_groups.len(), 0);
        do_directory_sync(&pool, &wg_tx).await.unwrap();
        let user_groups = user.member_of(&pool).await.unwrap();
        assert_eq!(user_groups.len(), 3);
        let user2 = get_test_user(&pool, "user2").await;
        assert!(user2.is_none());
        let mut transaction = pool.begin().await.unwrap();
        user.sync_allowed_devices(&mut transaction, &wg_tx)
            .await
            .unwrap();
        transaction.commit().await.unwrap();
        let event = wg_rx.try_recv();
        if let Ok(GatewayEvent::DeviceDeleted(dev)) = event {
            assert_eq!(dev.device.user_id, user2_pre_sync.id);
        } else {
            panic!("Expected a DeviceDeleted event");
        }
        let event = wg_rx.try_recv();
        if let Ok(GatewayEvent::DeviceCreated(dev)) = event {
            assert_eq!(dev.device.user_id, user.id);
        } else {
            panic!("Expected a DeviceDeleted event");
        }
    }

    #[sqlx::test]
    async fn test_sync_target_groups(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;

        let config = DefGuardConfig::new_test_config();
        let _ = SERVER_CONFIG.set(config.clone());
        let (wg_tx, _) = broadcast::channel::<GatewayEvent>(16);
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
        do_directory_sync(&pool, &wg_tx).await.unwrap();
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
        let (wg_tx, _) = broadcast::channel::<GatewayEvent>(16);
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

        do_directory_sync(&pool, &wg_tx).await.unwrap();

        // He should still be an admin as it's the last one
        assert!(user.is_admin(&pool).await.unwrap());

        // Make another admin and check if one of them is deleted
        let user2 = make_test_user_and_device("testuser2", &pool).await;
        user2.add_to_group(&pool, &admin_grp).await.unwrap();

        do_directory_sync(&pool, &wg_tx).await.unwrap();

        let admins = User::find_admins(&pool).await.unwrap();
        // There should be only one admin left
        assert_eq!(admins.len(), 1);

        let defguard_user = make_test_user_and_device("defguard", &pool).await;
        make_admin(&pool, &defguard_user).await;

        do_directory_sync(&pool, &wg_tx).await.unwrap();
    }

    #[sqlx::test]
    async fn test_sync_delete_last_admin_user(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;

        let config = DefGuardConfig::new_test_config();
        let _ = SERVER_CONFIG.set(config.clone());
        let (wg_tx, _) = broadcast::channel::<GatewayEvent>(16);
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

        do_directory_sync(&pool, &wg_tx).await.unwrap();

        // The user should still be an admin
        assert!(defguard_user.is_admin(&pool).await.unwrap());

        // remove his admin status
        let admin_grp = Group::find_by_name(&pool, "admin").await.unwrap().unwrap();
        defguard_user
            .remove_from_group(&pool, &admin_grp)
            .await
            .unwrap();

        do_directory_sync(&pool, &wg_tx).await.unwrap();
        let user = User::find_by_username(&pool, "defguard").await.unwrap();
        assert!(user.is_none());
    }
}
