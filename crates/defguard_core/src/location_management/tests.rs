use std::net::{IpAddr, Ipv4Addr};

use defguard_common::db::{
    models::{Device, DeviceType, User, WireguardNetwork, device::WireguardNetworkDevice},
    setup_pool,
};
use ipnetwork::IpNetwork;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

use crate::location_management::sync_location_allowed_devices;

#[sqlx::test]
fn test_network_readdress(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let user = User::new("tester", None, "Tester", "Test", "test@test.pl", None)
        .save(&pool)
        .await
        .unwrap();

    let mut network = WireguardNetwork::default();
    // 192.168.42.44: network
    // 192.168.42.45: device
    // 192.168.42.46: gateway
    // 192.168.42.47: broadcast
    network.address =
        vec![IpNetwork::new(IpAddr::V4(Ipv4Addr::new(192, 168, 42, 46)), 30).unwrap()];
    let mut network = network.save(&pool).await.unwrap();

    let mut conn = pool.begin().await.unwrap();

    // Only one device will fit.
    let device = Device::new(
        "device".to_string(),
        "fF9K0tgatZTEJRvzpNUswr0h8HqCIi+v39B45+QZZzE=".to_string(),
        user.id,
        DeviceType::User,
        None,
        true,
    )
    .save(&pool)
    .await
    .unwrap();
    let (_, _) = device.add_to_all_networks(&mut conn).await.unwrap();

    let devices = Device::all(&mut *conn).await.unwrap();
    assert_eq!(1, devices.len(), "{devices:#?}");
    let network_devices = WireguardNetworkDevice::all_for_network(&mut *conn, network.id)
        .await
        .unwrap();
    assert_eq!(1, network_devices.len(), "{network_devices:#?}");

    // Re-address the network **without** changing its addresses.
    let _ = sync_location_allowed_devices(&network, &mut conn, None)
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
    network.address =
        vec![IpNetwork::new(IpAddr::V4(Ipv4Addr::new(192, 168, 42, 77)), 30).unwrap()];
    network.save(&pool).await.unwrap();

    // Re-address the network.
    let _ = sync_location_allowed_devices(&network, &mut conn, None)
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
