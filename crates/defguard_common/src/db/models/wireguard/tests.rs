use std::{net::Ipv6Addr, str::FromStr};

use matches::assert_matches;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

use super::*;
use crate::db::setup_pool;

#[test]
fn test_set_address() {
    // This is fine.
    let result = WireguardNetwork::default().set_address([
        IpNetwork::new(IpAddr::V4(Ipv4Addr::new(10, 10, 10, 10)), 10).unwrap(),
        IpNetwork::new(
            IpAddr::V6(Ipv6Addr::new(0x1010, 0, 0, 0, 0, 0, 0, 0x1010)),
            10,
        )
        .unwrap(),
    ]);
    assert!(result.is_ok());

    // This should return error.
    let result = WireguardNetwork::default().set_address([IpNetwork::new(
        IpAddr::V4(Ipv4Addr::new(10, 10, 10, 0)),
        24,
    )
    .unwrap()]);
    assert!(result.is_err());

    let result = WireguardNetwork::default().set_address([IpNetwork::new(
        IpAddr::V6(Ipv6Addr::new(0x1010, 0, 0, 0, 0, 0, 0, 0)),
        112,
    )
    .unwrap()]);
    assert!(result.is_err());
}

// FIXME(mwojcik): rewrite for new stats implementation
// #[sqlx::test]
// async fn test_connected_at_reconnection(_: PgPoolOptions, options: PgConnectOptions) {
//     let pool = setup_pool(options).await;
//     let mut location = WireguardNetwork::default();
//     location.try_set_address("10.1.1.1/29").unwrap();
//     let location = location.save(&pool).await.unwrap();

//     let user = User::new(
//         "testuser",
//         Some("hunter2"),
//         "Tester",
//         "Test",
//         "test@test.com",
//         None,
//     )
//     .save(&pool)
//     .await
//     .unwrap();
//     let device = Device::new(
//         String::new(),
//         String::new(),
//         user.id,
//         DeviceType::User,
//         None,
//         true,
//     )
//     .save(&pool)
//     .await
//     .unwrap();

//     // insert stats
//     let samples = 60; // 1 hour of samples
//     let now = Utc::now().naive_utc();
//     for i in 0..=samples {
//         // simulate connection 30 minutes ago
//         let handshake_minutes = i * if i < 31 { 1 } else { 10 };
//         WireguardPeerStats {
//             id: NoId,
//             device_id: device.id,
//             collected_at: now - TimeDelta::minutes(i),
//             network: location.id,
//             endpoint: Some("11.22.33.44".into()),
//             upload: (samples - i) * 10,
//             download: (samples - i) * 20,
//             latest_handshake: now - TimeDelta::minutes(handshake_minutes),
//             allowed_ips: Some("10.1.1.0/24".into()),
//         }
//         .save(&pool)
//         .await
//         .unwrap();
//     }

//     let connected_at = device
//         .last_connected_at(&pool, location.id)
//         .await
//         .unwrap()
//         .unwrap();
//     assert_eq!(
//         connected_at,
//         // PostgreSQL stores 6 sub-second digits while chrono stores 9.
//         (now - TimeDelta::minutes(30)).trunc_subsecs(6),
//     );
// }

// FIXME(mwojcik): rewrite for new stats implementation
// #[sqlx::test]
// async fn test_connected_at_always_connected(_: PgPoolOptions, options: PgConnectOptions) {
//     let pool = setup_pool(options).await;
//     let mut location = WireguardNetwork::default();
//     location.try_set_address("10.1.1.1/29").unwrap();
//     let location = location.save(&pool).await.unwrap();

//     let user = User::new(
//         "testuser",
//         Some("hunter2"),
//         "Tester",
//         "Test",
//         "test@test.com",
//         None,
//     )
//     .save(&pool)
//     .await
//     .unwrap();
//     let device = Device::new(
//         String::new(),
//         String::new(),
//         user.id,
//         DeviceType::User,
//         None,
//         true,
//     )
//     .save(&pool)
//     .await
//     .unwrap();

//     // insert stats
//     let samples = 60; // 1 hour of samples
//     let now = Utc::now().naive_utc();
//     for i in 0..=samples {
//         WireguardPeerStats {
//             id: NoId,
//             device_id: device.id,
//             collected_at: now - TimeDelta::minutes(i),
//             network: location.id,
//             endpoint: Some("11.22.33.44".into()),
//             upload: (samples - i) * 10,
//             download: (samples - i) * 20,
//             latest_handshake: now - TimeDelta::minutes(i), // handshake every minute
//             allowed_ips: Some("10.1.1.0/24".into()),
//         }
//         .save(&pool)
//         .await
//         .unwrap();
//     }

//     let connected_at = device
//         .last_connected_at(&pool, location.id)
//         .await
//         .unwrap()
//         .unwrap();
//     assert_eq!(
//         connected_at,
//         // PostgreSQL stores 6 sub-second digits while chrono stores 9.
//         (now - TimeDelta::minutes(samples)).trunc_subsecs(6),
//     );
// }

#[sqlx::test]
async fn test_get_allowed_devices_for_user(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let mut network = WireguardNetwork::default()
        .try_set_address("10.1.1.1/29")
        .unwrap();
    network.allow_all_groups = true;
    let network = network.save(&pool).await.unwrap();

    let user1 = User::new(
        "user1",
        Some("pass1"),
        "Test",
        "User1",
        "user1@test.com",
        None,
    )
    .save(&pool)
    .await
    .unwrap();

    let user2 = User::new(
        "user2",
        Some("pass2"),
        "Test",
        "User2",
        "user2@test.com",
        None,
    )
    .save(&pool)
    .await
    .unwrap();

    let device1 = Device::new(
        "device1".into(),
        "key1".into(),
        user1.id,
        DeviceType::User,
        None,
        true,
    )
    .save(&pool)
    .await
    .unwrap();

    let device2 = Device::new(
        "device2".into(),
        "key2".into(),
        user1.id,
        DeviceType::User,
        None,
        true,
    )
    .save(&pool)
    .await
    .unwrap();

    let device3 = Device::new(
        "device3".into(),
        "key3".into(),
        user2.id,
        DeviceType::User,
        None,
        true,
    )
    .save(&pool)
    .await
    .unwrap();

    let devices = network
        .get_allowed_devices_for_user(&mut pool.acquire().await.unwrap(), user1.id)
        .await
        .unwrap();
    assert_eq!(devices.len(), 2);
    assert!(devices.iter().any(|d| d.id == device1.id));
    assert!(devices.iter().any(|d| d.id == device2.id));

    let devices = network
        .get_allowed_devices_for_user(&mut pool.acquire().await.unwrap(), user2.id)
        .await
        .unwrap();
    assert_eq!(devices.len(), 1);
    assert!(devices.iter().any(|d| d.id == device3.id));

    let devices = network
        .get_allowed_devices_for_user(&mut pool.acquire().await.unwrap(), Id::from(999))
        .await
        .unwrap();
    assert!(devices.is_empty());
}

#[sqlx::test]
async fn test_get_allowed_devices_for_user_with_groups(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let network = WireguardNetwork::default()
        .try_set_address("10.1.1.1/29")
        .unwrap()
        .save(&pool)
        .await
        .unwrap();

    let user1 = User::new(
        "user1",
        Some("pass1"),
        "Test",
        "User1",
        "user1@test.com",
        None,
    )
    .save(&pool)
    .await
    .unwrap();

    let user2 = User::new(
        "user2",
        Some("pass2"),
        "Test",
        "User2",
        "user2@test.com",
        None,
    )
    .save(&pool)
    .await
    .unwrap();

    let group1 = Group::new("group1").save(&pool).await.unwrap();
    let group2 = Group::new("group2").save(&pool).await.unwrap();

    user1.add_to_group(&pool, &group1).await.unwrap();
    user2.add_to_group(&pool, &group2).await.unwrap();

    let device1 = Device::new(
        "device1".into(),
        "key1".into(),
        user1.id,
        DeviceType::User,
        None,
        true,
    )
    .save(&pool)
    .await
    .unwrap();

    Device::new(
        "device2".into(),
        "key2".into(),
        user2.id,
        DeviceType::User,
        None,
        true,
    )
    .save(&pool)
    .await
    .unwrap();

    let mut transaction = pool.begin().await.unwrap();

    network
        .set_allowed_groups(&mut transaction, &[group1.name])
        .await
        .unwrap();

    let devices = network
        .get_allowed_devices_for_user(&mut transaction, user1.id)
        .await
        .unwrap();
    assert_eq!(devices.len(), 1);
    assert_eq!(devices[0].id, device1.id);

    let devices = network
        .get_allowed_devices_for_user(&mut transaction, user2.id)
        .await
        .unwrap();
    assert!(devices.is_empty());
}

#[sqlx::test]
async fn test_can_assign_ips(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let network = WireguardNetwork::new(
        "network".to_string(),
        50051,
        String::new(),
        None,
        [IpNetwork::from_str("10.1.1.0/24").unwrap()],
        false,
        false,
        false,
        LocationMfaMode::Disabled,
        ServiceLocationMode::Disabled,
    )
    .set_address([IpNetwork::from_str("10.1.1.1/24").unwrap()])
    .unwrap()
    .save(&pool)
    .await
    .unwrap();

    // assign free address
    let addrs = vec![IpAddr::from_str("10.1.1.2").unwrap()];
    assert_matches!(
        network
            .can_assign_ips(&mut pool.acquire().await.unwrap(), &addrs, None)
            .await,
        Ok(())
    );

    // assign multiple free addresses
    let addrs = vec![
        IpAddr::from_str("10.1.1.2").unwrap(),
        IpAddr::from_str("10.1.1.3").unwrap(),
    ];
    assert_matches!(
        network
            .can_assign_ips(&mut pool.acquire().await.unwrap(), &addrs, None)
            .await,
        Ok(())
    );

    // try to assign address from another network
    let addrs = vec![IpAddr::from_str("10.2.1.2").unwrap()];
    assert_matches!(
        network
            .can_assign_ips(&mut pool.acquire().await.unwrap(), &addrs, None)
            .await,
        Err(NetworkAddressError::NoContainingNetwork(..))
    );

    // try to assign already assigned address
    let user = User::new(
        "hpotter",
        Some("pass123"),
        "Potter",
        "Harry",
        "h.potter@hogwart.edu.uk",
        None,
    )
    .save(&pool)
    .await
    .unwrap();

    let device = Device::new(
        "device".to_string(),
        String::new(),
        user.id,
        DeviceType::User,
        None,
        true,
    )
    .save(&pool)
    .await
    .unwrap();
    WireguardNetworkDevice::new(
        network.id,
        device.id,
        vec![IpAddr::from_str("10.1.1.2").unwrap()],
    )
    .insert(&pool)
    .await
    .unwrap();
    let addrs = vec![IpAddr::from_str("10.1.1.2").unwrap()];
    assert_matches!(
        network
            .can_assign_ips(&mut pool.acquire().await.unwrap(), &addrs, None)
            .await,
        Err(NetworkAddressError::AddressAlreadyAssigned(..))
    );

    // assign with exception for the device
    let addrs = vec![IpAddr::from_str("10.1.1.2").unwrap()];
    assert_matches!(
        network
            .can_assign_ips(&mut pool.acquire().await.unwrap(), &addrs, Some(device.id))
            .await,
        Ok(())
    );

    // try to assign gateway address
    let addrs = vec![IpAddr::from_str("10.1.1.1").unwrap()];
    assert_matches!(
        network
            .can_assign_ips(&mut pool.acquire().await.unwrap(), &addrs, None)
            .await,
        Err(NetworkAddressError::ReservedForGateway(..))
    );

    // try to assign network address
    let addrs = vec![IpAddr::from_str("10.1.1.0").unwrap()];
    assert_matches!(
        network
            .can_assign_ips(&mut pool.acquire().await.unwrap(), &addrs, None)
            .await,
        Err(NetworkAddressError::IsNetworkAddress(..))
    );

    // try to assign broadcast address
    let addrs = vec![IpAddr::from_str("10.1.1.255").unwrap()];
    assert_matches!(
        network
            .can_assign_ips(&mut pool.acquire().await.unwrap(), &addrs, None)
            .await,
        Err(NetworkAddressError::IsBroadcastAddress(..))
    );
}

#[sqlx::test]
async fn test_can_assign_ips_multiple_addresses(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let network = WireguardNetwork::new(
        "network".to_string(),
        50051,
        String::new(),
        None,
        [IpNetwork::from_str("10.1.1.0/24").unwrap()],
        false,
        false,
        false,
        LocationMfaMode::Disabled,
        ServiceLocationMode::Disabled,
    )
    .set_address([
        IpNetwork::from_str("10.1.1.1/24").unwrap(),
        IpNetwork::from_str("fc00::1/112").unwrap(),
    ])
    .unwrap()
    .save(&pool)
    .await
    .unwrap();

    // assign free addresses
    let addrs = vec![
        IpAddr::from_str("10.1.1.2").unwrap(),
        IpAddr::from_str("fc00::2").unwrap(),
    ];
    assert_matches!(
        network
            .can_assign_ips(&mut pool.acquire().await.unwrap(), &addrs, None)
            .await,
        Ok(())
    );

    // assign multiple free addresses
    let addrs = vec![
        IpAddr::from_str("10.1.1.2").unwrap(),
        IpAddr::from_str("10.1.1.3").unwrap(),
        IpAddr::from_str("fc00::2").unwrap(),
        IpAddr::from_str("fc00::3").unwrap(),
    ];
    assert_matches!(
        network
            .can_assign_ips(&mut pool.acquire().await.unwrap(), &addrs, None)
            .await,
        Ok(())
    );

    // try to assign address from another network
    let addrs = vec![IpAddr::from_str("fa::2").unwrap()];
    assert_matches!(
        network
            .can_assign_ips(&mut pool.acquire().await.unwrap(), &addrs, None)
            .await,
        Err(NetworkAddressError::NoContainingNetwork(..))
    );

    // try to assign already assigned address
    let user = User::new(
        "hpotter",
        Some("pass123"),
        "Potter",
        "Harry",
        "h.potter@hogwart.edu.uk",
        None,
    )
    .save(&pool)
    .await
    .unwrap();

    let device = Device::new(
        "device".to_string(),
        String::new(),
        user.id,
        DeviceType::User,
        None,
        true,
    )
    .save(&pool)
    .await
    .unwrap();
    WireguardNetworkDevice::new(
        network.id,
        device.id,
        vec![
            IpAddr::from_str("10.1.1.2").unwrap(),
            IpAddr::from_str("fc00::2").unwrap(),
        ],
    )
    .insert(&pool)
    .await
    .unwrap();
    let addrs = vec![IpAddr::from_str("fc00::2").unwrap()];
    assert_matches!(
        network
            .can_assign_ips(&mut pool.acquire().await.unwrap(), &addrs, None)
            .await,
        Err(NetworkAddressError::AddressAlreadyAssigned(..))
    );

    // assign with exception for the device
    let addrs = vec![IpAddr::from_str("fc00::2").unwrap()];
    assert_matches!(
        network
            .can_assign_ips(&mut pool.acquire().await.unwrap(), &addrs, Some(device.id))
            .await,
        Ok(())
    );

    // try to assign gateway address
    let addrs = vec![IpAddr::from_str("fc00::1").unwrap()];
    assert_matches!(
        network
            .can_assign_ips(&mut pool.acquire().await.unwrap(), &addrs, None)
            .await,
        Err(NetworkAddressError::ReservedForGateway(..))
    );

    // try to assign network address
    let addrs = vec![IpAddr::from_str("fc00::0").unwrap()];
    assert_matches!(
        network
            .can_assign_ips(&mut pool.acquire().await.unwrap(), &addrs, None)
            .await,
        Err(NetworkAddressError::IsNetworkAddress(..))
    );

    // try to assign broadcast address
    let addrs = vec![IpAddr::from_str("fc00::ffff").unwrap()];
    assert_matches!(
        network
            .can_assign_ips(&mut pool.acquire().await.unwrap(), &addrs, None)
            .await,
        Err(NetworkAddressError::IsBroadcastAddress(..))
    );
}
