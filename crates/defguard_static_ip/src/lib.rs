use std::net::IpAddr;

use defguard_common::{
    db::{
        Id,
        models::{WireguardNetwork, device::WireguardNetworkDevice},
    },
    utils::{SplitIp, split_ip},
};
use serde::Serialize;
use sqlx::{PgConnection, PgPool, prelude::FromRow};
use tracing::debug;

use crate::error::StaticIpError;

pub mod error;

#[derive(Serialize)]
pub struct LocationDevices {
    pub location_id: i64,
    pub location_name: String,
    pub devices: Vec<DeviceIps>,
}

#[derive(Serialize)]
pub struct DeviceIps {
    pub device_id: i64,
    pub device_name: String,
    pub wireguard_ips: Vec<SplitIp>,
}

/// Flattened location entry used by the single-device IP endpoint.
/// Each entry represents one location the device belongs to,
/// without wrapping IPs in an inner device array.
#[derive(Serialize)]
pub struct DeviceLocationIp {
    pub location_id: i64,
    pub location_name: String,
    pub wireguard_ips: Vec<SplitIp>,
}

#[derive(FromRow)]
struct DeviceIpRow {
    location_id: i64,
    location_name: String,
    device_id: i64,
    device_name: String,
    wireguard_ips: Vec<IpAddr>,
}

pub async fn get_ips_for_user(
    username: &str,
    pool: &PgPool,
) -> Result<Vec<LocationDevices>, StaticIpError> {
    debug!("Fetching static IPs for user {username}");
    let rows = sqlx::query_as!(
        DeviceIpRow,
        "SELECT \
            wn.id AS location_id, \
            wn.name AS location_name, \
            d.id AS device_id, \
            d.name AS device_name, \
            wnd.wireguard_ips AS \"wireguard_ips: Vec<IpAddr>\" \
        FROM wireguard_network wn \
        JOIN wireguard_network_device wnd ON wnd.wireguard_network_id = wn.id \
        JOIN device d ON d.id = wnd.device_id \
        JOIN \"user\" u ON d.user_id = u.id \
        WHERE u.username = $1 \
        ORDER BY wn.name, d.name",
        username
    )
    .fetch_all(pool)
    .await?;

    debug!(
        "Found {} device-location assignments for user {username}",
        rows.len()
    );
    let mut locations: Vec<LocationDevices> = Vec::new();

    for row in rows {
        let network = WireguardNetwork::find_by_id(pool, row.location_id)
            .await?
            .ok_or(StaticIpError::NetworkNotFound(row.location_id))?;

        let wireguard_ips: Vec<SplitIp> = row
            .wireguard_ips
            .iter()
            .filter_map(|ip| {
                network
                    .get_containing_network(*ip)
                    .map(|net| split_ip(ip, &net))
            })
            .collect();

        let device = DeviceIps {
            device_id: row.device_id,
            device_name: row.device_name,
            wireguard_ips,
        };

        match locations.last_mut() {
            Some(loc) if loc.location_id == row.location_id => {
                loc.devices.push(device);
            }
            _ => {
                locations.push(LocationDevices {
                    location_id: row.location_id,
                    location_name: row.location_name,
                    devices: vec![device],
                });
            }
        }
    }

    debug!(
        "Returning IP data for {} location(s) for user {username}",
        locations.len()
    );
    Ok(locations)
}

pub async fn get_ips_for_device(
    username: &str,
    device_id: Id,
    pool: &PgPool,
) -> Result<Vec<DeviceLocationIp>, StaticIpError> {
    debug!("Fetching static IPs for device {device_id} of user {username}");
    let rows = sqlx::query_as!(
        DeviceIpRow,
        "SELECT \
            wn.id AS location_id, \
            wn.name AS location_name, \
            d.id AS device_id, \
            d.name AS device_name, \
            wnd.wireguard_ips AS \"wireguard_ips: Vec<IpAddr>\" \
        FROM wireguard_network wn \
        JOIN wireguard_network_device wnd ON wnd.wireguard_network_id = wn.id \
        JOIN device d ON d.id = wnd.device_id \
        JOIN \"user\" u ON d.user_id = u.id \
        WHERE u.username = $1 AND d.id = $2 \
        ORDER BY wn.name",
        username,
        device_id
    )
    .fetch_all(pool)
    .await?;

    debug!(
        "Found {} location(s) for device {device_id} of user {username}",
        rows.len()
    );
    let mut locations: Vec<DeviceLocationIp> = Vec::new();

    for row in rows {
        let network = WireguardNetwork::find_by_id(pool, row.location_id)
            .await?
            .ok_or(StaticIpError::NetworkNotFound(row.location_id))?;

        let wireguard_ips: Vec<SplitIp> = row
            .wireguard_ips
            .iter()
            .filter_map(|ip| {
                network
                    .get_containing_network(*ip)
                    .map(|net| split_ip(ip, &net))
            })
            .collect();

        locations.push(DeviceLocationIp {
            location_id: row.location_id,
            location_name: row.location_name,
            wireguard_ips,
        });
    }

    debug!(
        "Returning IP data for {} location(s) for device {device_id}",
        locations.len()
    );
    Ok(locations)
}

pub async fn assign_static_ips(
    device_id: Id,
    ips: Vec<IpAddr>,
    location: Id,
    transaction: &mut PgConnection,
) -> Result<(), StaticIpError> {
    debug!("Assigning static IPs {ips:?} to device {device_id} in location {location}");
    let network = WireguardNetwork::find_by_id(&mut *transaction, location)
        .await?
        .ok_or(StaticIpError::NetworkNotFound(location))?;

    let mut network_device = WireguardNetworkDevice::find(&mut *transaction, device_id, location)
        .await?
        .ok_or(StaticIpError::DeviceNotInNetwork(device_id, location))?;

    network
        .can_assign_ips(transaction, &ips, Some(device_id))
        .await?;

    network_device.wireguard_ips = ips;
    network_device.update(&mut *transaction).await?;

    debug!("Static IPs successfully assigned to device {device_id} in location {location}");
    Ok(())
}

pub async fn validate_ip(
    device_id: Id,
    ip: IpAddr,
    location: Id,
    transaction: &mut PgConnection,
) -> Result<(), StaticIpError> {
    debug!("Validating IP {ip} for device {device_id} in location {location}");
    let network = WireguardNetwork::find_by_id(&mut *transaction, location)
        .await?
        .ok_or(StaticIpError::NetworkNotFound(location))?;

    let result = network
        .can_assign_ips(transaction, &[ip], Some(device_id))
        .await
        .map_err(StaticIpError::InvalidIpAssignment);

    if result.is_ok() {
        debug!("IP {ip} is valid for device {device_id} in location {location}");
    } else {
        debug!(
            "IP {ip} is NOT valid for device {device_id} in location {location}: {:?}",
            result
        );
    }

    result
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use defguard_common::db::{
        models::{Device, DeviceType, User, WireguardNetwork, device::WireguardNetworkDevice},
        setup_pool,
    };
    use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

    use super::*;

    #[sqlx::test]
    async fn test_get_ips_for_user_groups_by_location(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;

        // Create a test user
        let user = User::new(
            "testuser",
            Some("Test123!"),
            "User",
            "Test",
            "test@example.com",
            None,
        )
        .save(&pool)
        .await
        .expect("Failed to create user");

        // Create test locations
        let mut location_a = WireguardNetwork {
            name: "Location A".into(),
            ..Default::default()
        };
        location_a.try_set_address("10.0.1.1/24").unwrap();
        let location_a = location_a
            .save(&pool)
            .await
            .expect("Failed to create Location A");

        let mut location_b = WireguardNetwork {
            name: "Location B".into(),
            ..Default::default()
        };
        location_b.try_set_address("10.0.2.1/24").unwrap();
        let location_b = location_b
            .save(&pool)
            .await
            .expect("Failed to create Location B");

        // Create test devices for the user
        let device1 = Device::new(
            "Device 1".into(),
            "pubkey1".into(),
            user.id,
            DeviceType::User,
            None,
            true,
        )
        .save(&pool)
        .await
        .expect("Failed to create device 1");

        let device2 = Device::new(
            "Device 2".into(),
            "pubkey2".into(),
            user.id,
            DeviceType::User,
            None,
            true,
        )
        .save(&pool)
        .await
        .expect("Failed to create device 2");

        let device3 = Device::new(
            "Device 3".into(),
            "pubkey3".into(),
            user.id,
            DeviceType::User,
            None,
            true,
        )
        .save(&pool)
        .await
        .expect("Failed to create device 3");

        let device4 = Device::new(
            "Device 4".into(),
            "pubkey4".into(),
            user.id,
            DeviceType::User,
            None,
            true,
        )
        .save(&pool)
        .await
        .expect("Failed to create device 4");

        // Create network-device mappings with IPs
        WireguardNetworkDevice::new(
            location_a.id,
            device1.id,
            vec![
                IpAddr::from_str("10.0.1.2").unwrap(),
                IpAddr::from_str("10.0.1.3").unwrap(),
            ],
        )
        .insert(&pool)
        .await
        .expect("Failed to assign device 1 to location A");

        WireguardNetworkDevice::new(
            location_a.id,
            device2.id,
            vec![IpAddr::from_str("10.0.1.4").unwrap()],
        )
        .insert(&pool)
        .await
        .expect("Failed to assign device 2 to location A");

        WireguardNetworkDevice::new(
            location_b.id,
            device3.id,
            vec![IpAddr::from_str("10.0.2.2").unwrap()],
        )
        .insert(&pool)
        .await
        .expect("Failed to assign device 3 to location B");

        WireguardNetworkDevice::new(
            location_b.id,
            device4.id,
            vec![
                IpAddr::from_str("10.0.2.3").unwrap(),
                IpAddr::from_str("10.0.2.4").unwrap(),
            ],
        )
        .insert(&pool)
        .await
        .expect("Failed to assign device 4 to location B");

        // Call the function
        let result = get_ips_for_user("testuser", &pool).await;
        assert!(result.is_ok());

        let locations = result.unwrap();
        assert_eq!(locations.len(), 2);

        let net_a = location_a.address[0];
        let net_b = location_b.address[0];

        // Verify Location A
        assert_eq!(locations[0].location_name, "Location A");
        assert_eq!(locations[0].devices.len(), 2);
        assert_eq!(locations[0].devices[0].device_name, "Device 1");
        assert_eq!(
            locations[0].devices[0].wireguard_ips,
            vec![
                split_ip(&IpAddr::from_str("10.0.1.2").unwrap(), &net_a),
                split_ip(&IpAddr::from_str("10.0.1.3").unwrap(), &net_a),
            ]
        );
        assert_eq!(locations[0].devices[1].device_name, "Device 2");
        assert_eq!(
            locations[0].devices[1].wireguard_ips,
            vec![split_ip(&IpAddr::from_str("10.0.1.4").unwrap(), &net_a)]
        );

        // Verify Location B
        assert_eq!(locations[1].location_name, "Location B");
        assert_eq!(locations[1].devices.len(), 2);
        assert_eq!(locations[1].devices[0].device_name, "Device 3");
        assert_eq!(locations[1].devices[1].device_name, "Device 4");
        assert_eq!(
            locations[1].devices[0].wireguard_ips,
            vec![split_ip(&IpAddr::from_str("10.0.2.2").unwrap(), &net_b)]
        );
        assert_eq!(
            locations[1].devices[1].wireguard_ips,
            vec![
                split_ip(&IpAddr::from_str("10.0.2.3").unwrap(), &net_b),
                split_ip(&IpAddr::from_str("10.0.2.4").unwrap(), &net_b),
            ]
        );
    }

    #[sqlx::test]
    async fn test_assign_static_ips_success(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;

        let user = User::new(
            "assignuser",
            Some("Test123!"),
            "User",
            "Test",
            "assign@example.com",
            None,
        )
        .save(&pool)
        .await
        .expect("Failed to create user");

        let mut network = WireguardNetwork {
            name: "Assign Network".into(),
            ..Default::default()
        };
        network.try_set_address("10.0.0.1/24").unwrap();
        let network = network.save(&pool).await.expect("Failed to create network");

        let device = Device::new(
            "Assign Device".into(),
            "assignpubkey1".into(),
            user.id,
            DeviceType::User,
            None,
            true,
        )
        .save(&pool)
        .await
        .expect("Failed to create device");

        WireguardNetworkDevice::new(
            network.id,
            device.id,
            vec![IpAddr::from_str("10.0.0.2").unwrap()],
        )
        .insert(&pool)
        .await
        .expect("Failed to assign device to network");

        let new_ips = vec![IpAddr::from_str("10.0.0.10").unwrap()];
        let mut conn = pool.acquire().await.expect("Failed to acquire connection");
        assign_static_ips(device.id, new_ips.clone(), network.id, &mut conn)
            .await
            .expect("assign_static_ips should succeed");

        let updated = WireguardNetworkDevice::find(&pool, device.id, network.id)
            .await
            .unwrap()
            .expect("Network device entry should exist");
        assert_eq!(updated.wireguard_ips, new_ips);
    }

    #[sqlx::test]
    async fn test_assign_static_ips_network_not_found(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;

        let mut conn = pool.acquire().await.expect("Failed to acquire connection");
        let result = assign_static_ips(
            1,
            vec![IpAddr::from_str("10.0.0.2").unwrap()],
            9999,
            &mut conn,
        )
        .await;

        assert!(matches!(result, Err(StaticIpError::NetworkNotFound(9999))));
    }

    #[sqlx::test]
    async fn test_assign_static_ips_device_not_in_network(
        _: PgPoolOptions,
        options: PgConnectOptions,
    ) {
        let pool = setup_pool(options).await;

        let user = User::new(
            "nonetworkuser",
            Some("Test123!"),
            "User",
            "Test",
            "nonet@example.com",
            None,
        )
        .save(&pool)
        .await
        .expect("Failed to create user");

        let mut network = WireguardNetwork {
            name: "NoDevice Network".into(),
            ..Default::default()
        };
        network.try_set_address("10.0.0.1/24").unwrap();
        let network = network.save(&pool).await.expect("Failed to create network");

        let device = Device::new(
            "Unassigned Device".into(),
            "unassignedpubkey".into(),
            user.id,
            DeviceType::User,
            None,
            true,
        )
        .save(&pool)
        .await
        .expect("Failed to create device");

        let mut conn = pool.acquire().await.expect("Failed to acquire connection");
        let result = assign_static_ips(
            device.id,
            vec![IpAddr::from_str("10.0.0.2").unwrap()],
            network.id,
            &mut conn,
        )
        .await;

        assert!(matches!(
            result,
            Err(StaticIpError::DeviceNotInNetwork(_, _))
        ));
    }

    #[sqlx::test]
    async fn test_assign_static_ips_ip_out_of_range(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;

        let user = User::new(
            "rangeuser",
            Some("Test123!"),
            "User",
            "Test",
            "range@example.com",
            None,
        )
        .save(&pool)
        .await
        .expect("Failed to create user");

        let mut network = WireguardNetwork {
            name: "Range Network".into(),
            ..Default::default()
        };
        network.try_set_address("10.0.0.1/24").unwrap();
        let network = network.save(&pool).await.expect("Failed to create network");

        let device = Device::new(
            "Range Device".into(),
            "rangepubkey".into(),
            user.id,
            DeviceType::User,
            None,
            true,
        )
        .save(&pool)
        .await
        .expect("Failed to create device");

        WireguardNetworkDevice::new(
            network.id,
            device.id,
            vec![IpAddr::from_str("10.0.0.2").unwrap()],
        )
        .insert(&pool)
        .await
        .expect("Failed to assign device to network");

        // IP is outside the 10.0.0.0/24 range
        let mut conn = pool.acquire().await.expect("Failed to acquire connection");
        let result = assign_static_ips(
            device.id,
            vec![IpAddr::from_str("192.168.1.5").unwrap()],
            network.id,
            &mut conn,
        )
        .await;

        assert!(matches!(result, Err(StaticIpError::InvalidIpAssignment(_))));
    }

    #[sqlx::test]
    async fn test_assign_static_ips_ip_already_used(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;

        let user = User::new(
            "conflictuser",
            Some("Test123!"),
            "User",
            "Test",
            "conflict@example.com",
            None,
        )
        .save(&pool)
        .await
        .expect("Failed to create user");

        let mut network = WireguardNetwork {
            name: "Conflict Network".into(),
            ..Default::default()
        };
        network.try_set_address("10.0.0.1/24").unwrap();
        let network = network.save(&pool).await.expect("Failed to create network");

        let device1 = Device::new(
            "Conflict Device 1".into(),
            "conflictpubkey1".into(),
            user.id,
            DeviceType::User,
            None,
            true,
        )
        .save(&pool)
        .await
        .expect("Failed to create device 1");

        let device2 = Device::new(
            "Conflict Device 2".into(),
            "conflictpubkey2".into(),
            user.id,
            DeviceType::User,
            None,
            true,
        )
        .save(&pool)
        .await
        .expect("Failed to create device 2");

        WireguardNetworkDevice::new(
            network.id,
            device1.id,
            vec![IpAddr::from_str("10.0.0.3").unwrap()],
        )
        .insert(&pool)
        .await
        .expect("Failed to assign device 1 to network");

        WireguardNetworkDevice::new(
            network.id,
            device2.id,
            vec![IpAddr::from_str("10.0.0.4").unwrap()],
        )
        .insert(&pool)
        .await
        .expect("Failed to assign device 2 to network");

        // Try to steal 10.0.0.3 which belongs to device1
        let mut conn = pool.acquire().await.expect("Failed to acquire connection");
        let result = assign_static_ips(
            device2.id,
            vec![IpAddr::from_str("10.0.0.3").unwrap()],
            network.id,
            &mut conn,
        )
        .await;

        assert!(matches!(result, Err(StaticIpError::InvalidIpAssignment(_))));
    }
}
