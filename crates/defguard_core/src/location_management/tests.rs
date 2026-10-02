use std::net::{IpAddr, Ipv4Addr};

use defguard_common::db::{
    models::{
        Device, DeviceType, User, WireguardNetwork, device::WireguardNetworkDevice, group::Group,
    },
    setup_pool,
};
use ipnetwork::IpNetwork;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

use super::*;
use crate::{
    device_access::join_device_to_all_networks, location_management::sync_location_allowed_devices,
};

#[sqlx::test]
fn test_network_readdress(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let user = User::new("tester", None, "Tester", "Test", "test@test.pl", None)
        .save(&pool)
        .await
        .unwrap();

    // 192.168.42.44: network
    // 192.168.42.45: device
    // 192.168.42.46: gateway
    // 192.168.42.47: broadcast
    let mut network = WireguardNetwork::default()
        .set_address([IpNetwork::new(IpAddr::V4(Ipv4Addr::new(192, 168, 42, 46)), 30).unwrap()])
        .unwrap();
    network.allow_all_groups = true;
    let network = network.save(&pool).await.unwrap();

    let mut conn = pool.begin().await.unwrap();

    // Only one device will fit.
    let device = Device::new(
        "device".to_owned(),
        "fF9K0tgatZTEJRvzpNUswr0h8HqCIi+v39B45+QZZzE=".to_owned(),
        user.id,
        DeviceType::User,
        None,
        true,
    )
    .save(&pool)
    .await
    .unwrap();
    let (_, _) = join_device_to_all_networks(&mut conn, &device, &user)
        .await
        .unwrap();

    let devices = Device::all(&mut *conn).await.unwrap();
    assert_eq!(1, devices.len(), "{devices:#?}");
    let network_devices = WireguardNetworkDevice::all_for_network(&mut *conn, network.id)
        .await
        .unwrap();
    assert_eq!(1, network_devices.len(), "{network_devices:#?}");

    // Re-address the network **without** changing its addresses.
    let mut events = Vec::new();
    sync_location_allowed_devices(&network, &mut conn, None, &mut events)
        .await
        .unwrap();
    let network_device = WireguardNetworkDevice::find(&mut *conn, device.id, network.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(1, network_device.wireguard_ips.len());
    assert_eq!(
        IpAddr::V4(Ipv4Addr::new(192, 168, 42, 45)),
        network_device.wireguard_ips[0]
    );

    // 192.168.42.76: network
    // 192.168.42.77: gateway
    // 192.168.42.78: device
    // 192.168.42.79: broadcast
    let network = network
        .set_address([IpNetwork::new(IpAddr::V4(Ipv4Addr::new(192, 168, 42, 77)), 30).unwrap()])
        .unwrap();
    network.save(&pool).await.unwrap();

    // Re-address the network.
    let mut events = Vec::new();
    sync_location_allowed_devices(&network, &mut conn, None, &mut events)
        .await
        .unwrap();
    let network_device = WireguardNetworkDevice::find(&mut *conn, device.id, network.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(1, network_device.wireguard_ips.len());
    assert_eq!(
        IpAddr::V4(Ipv4Addr::new(192, 168, 42, 78)),
        network_device.wireguard_ips[0]
    );
}

#[sqlx::test]
async fn test_sync_allowed_devices_for_user(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let network = WireguardNetwork::default()
        .try_set_address("10.1.1.1/29")
        .unwrap()
        .save(&pool)
        .await
        .unwrap();

    let user1 = User::new(
        "testuser1",
        Some("pass1"),
        "Tester1",
        "Test1",
        "test1@test.com",
        None,
    )
    .save(&pool)
    .await
    .unwrap();

    let user2 = User::new(
        "testuser2",
        Some("pass2"),
        "Tester2",
        "Test2",
        "test2@test.com",
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

    let group = Group::new("group").save(&pool).await.unwrap();
    user1.add_to_group(&pool, &group).await.unwrap();
    user2.add_to_group(&pool, &group).await.unwrap();

    let mut transaction = pool.begin().await.unwrap();
    network
        .set_allowed_groups(&mut transaction, std::slice::from_ref(&group.name))
        .await
        .unwrap();

    // user1 sync
    let mut events = Vec::new();
    sync_allowed_devices_for_user(&network, &mut transaction, &user1, None, &mut events)
        .await
        .unwrap();

    assert_eq!(events.len(), 2);
    assert!(events.iter().any(|e| match e {
        GatewayCommand::DeviceCreated(info) => info.device.id == device1.id,
        _ => false,
    }));
    assert!(events.iter().any(|e| match e {
        GatewayCommand::DeviceCreated(info) => info.device.id == device2.id,
        _ => false,
    }));

    // user 2 sync
    let mut events = Vec::new();
    sync_allowed_devices_for_user(&network, &mut transaction, &user2, None, &mut events)
        .await
        .unwrap();

    assert_eq!(events.len(), 1);
    match &events[0] {
        GatewayCommand::DeviceCreated(info) => {
            assert_eq!(info.device.id, device3.id);
        }
        _ => panic!("Expected DeviceCreated event"),
    }

    // Second sync should not generate any events
    let mut events = Vec::new();
    sync_allowed_devices_for_user(&network, &mut transaction, &user1, None, &mut events)
        .await
        .unwrap();
    assert_eq!(events.len(), 0);

    transaction.commit().await.unwrap();
}

#[sqlx::test]
async fn test_sync_allowed_devices_for_user_with_groups(
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
        "testuser1",
        Some("pass1"),
        "Tester1",
        "Test1",
        "test1@test.com",
        None,
    )
    .save(&pool)
    .await
    .unwrap();

    let user2 = User::new(
        "testuser2",
        Some("pass2"),
        "Tester2",
        "Test2",
        "test2@test.com",
        None,
    )
    .save(&pool)
    .await
    .unwrap();

    let user3 = User::new(
        "testuser3",
        Some("pass3"),
        "Tester3",
        "Test3",
        "test3@test.com",
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
        user2.id,
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
        user3.id,
        DeviceType::User,
        None,
        true,
    )
    .save(&pool)
    .await
    .unwrap();

    let group1 = Group::new("group1").save(&pool).await.unwrap();
    let group2 = Group::new("group2").save(&pool).await.unwrap();

    let mut transaction = pool.begin().await.unwrap();

    network
        .set_allowed_groups(
            &mut transaction,
            &[group1.name.clone(), group2.name.clone()],
        )
        .await
        .unwrap();

    let mut events = Vec::new();
    sync_allowed_devices_for_user(&network, &mut transaction, &user1, None, &mut events)
        .await
        .unwrap();
    assert_eq!(events.len(), 0);

    user1.add_to_group(&pool, &group1).await.unwrap();
    user2.add_to_group(&pool, &group1).await.unwrap();
    user3.add_to_group(&pool, &group2).await.unwrap();

    let mut events = Vec::new();
    sync_allowed_devices_for_user(&network, &mut transaction, &user1, None, &mut events)
        .await
        .unwrap();
    assert_eq!(events.len(), 1);
    match &events[0] {
        GatewayCommand::DeviceCreated(info) => {
            assert_eq!(info.device.id, device1.id);
        }
        _ => panic!("Expected DeviceCreated event"),
    }

    let mut events = Vec::new();
    sync_allowed_devices_for_user(&network, &mut transaction, &user2, None, &mut events)
        .await
        .unwrap();
    assert_eq!(events.len(), 1);
    match &events[0] {
        GatewayCommand::DeviceCreated(info) => {
            assert_eq!(info.device.id, device2.id);
        }
        _ => panic!("Expected DeviceCreated event"),
    }

    let mut events = Vec::new();
    sync_allowed_devices_for_user(&network, &mut transaction, &user3, None, &mut events)
        .await
        .unwrap();
    assert_eq!(events.len(), 1);
    match &events[0] {
        GatewayCommand::DeviceCreated(info) => {
            assert_eq!(info.device.id, device3.id);
        }
        _ => panic!("Expected DeviceCreated event"),
    }

    transaction.commit().await.unwrap();
}
