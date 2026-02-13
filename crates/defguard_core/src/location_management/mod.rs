use std::{collections::HashMap, net::IpAddr};

use defguard_common::{
    csv::AsCsv,
    db::{
        Id,
        models::{
            Device, DeviceNetworkInfo, DeviceType, ModelError, WireguardNetwork,
            WireguardNetworkError,
            device::{DeviceInfo, WireguardNetworkDevice},
            user::User,
            wireguard::MappedDevice,
        },
    },
};
use sqlx::PgConnection;
use thiserror::Error;
use tokio::sync::broadcast::Sender;

use crate::{
    enterprise::firewall::{FirewallError, try_get_location_firewall_config},
    grpc::{GatewayEvent, send_multiple_wireguard_events},
    wg_config::ImportedDevice,
};

pub mod allowed_peers;

#[derive(Debug, Error)]
pub enum LocationManagementError {
    #[error(transparent)]
    FirewallError(#[from] FirewallError),
    #[error(transparent)]
    DbError(#[from] sqlx::Error),
    #[error(transparent)]
    WireguardNetworkError(#[from] WireguardNetworkError),
    #[error(transparent)]
    ModelError(#[from] ModelError),
}

// run sync_allowed_devices on all wireguard networks
pub(crate) async fn sync_all_networks(
    conn: &mut PgConnection,
    wireguard_tx: &Sender<GatewayEvent>,
) -> Result<(), LocationManagementError> {
    info!("Syncing allowed devices for all WireGuard locations");
    let locations = WireguardNetwork::all(&mut *conn).await?;
    for network in locations {
        // sync allowed devices for location
        let mut gateway_events = sync_location_allowed_devices(&network, &mut *conn, None).await?;

        // send firewall config update if ACLs are enabled for a given location
        if let Some(firewall_config) =
            try_get_location_firewall_config(&network, &mut *conn).await?
        {
            gateway_events.push(GatewayEvent::FirewallConfigChanged(
                network.id,
                firewall_config,
            ));
        }
        // check if any gateway events need to be sent
        if !gateway_events.is_empty() {
            send_multiple_wireguard_events(gateway_events, wireguard_tx);
        }
    }
    Ok(())
}

/// Refresh network IPs for all relevant devices
///
/// If the list of allowed devices has changed add/remove devices accordingly
///
/// If the network address has changed readdress existing devices
pub(crate) async fn sync_location_allowed_devices(
    location: &WireguardNetwork<Id>,
    conn: &mut PgConnection,
    reserved_ips: Option<&[IpAddr]>,
) -> Result<Vec<GatewayEvent>, LocationManagementError> {
    info!("Synchronizing IPs in network {location} for all allowed devices ");
    // list all allowed devices
    let mut allowed_devices = location.get_allowed_devices(&mut *conn).await?;

    // network devices are always allowed, make sure to take only network devices already assigned to that network
    let network_devices =
        Device::find_by_type_and_network(&mut *conn, DeviceType::Network, location.id).await?;
    allowed_devices.extend(network_devices);

    // convert to a map for easier processing
    let allowed_devices: HashMap<Id, Device<Id>> = allowed_devices
        .into_iter()
        .map(|dev| (dev.id, dev))
        .collect();

    // check if all devices can fit within network
    // include address, network, and broadcast in the calculation
    let count = allowed_devices.len() + 3;
    location.validate_network_size(count)?;

    // list all assigned IPs
    let assigned_ips = WireguardNetworkDevice::all_for_network(&mut *conn, location.id).await?;

    let events = process_device_access_changes(
        location,
        &mut *conn,
        allowed_devices,
        assigned_ips,
        reserved_ips,
    )
    .await?;

    Ok(events)
}

/// Refresh network IPs for all relevant devices of a given user
/// If the list of allowed devices has changed add/remove devices accordingly
/// If the network address has changed readdress existing devices
pub(crate) async fn sync_allowed_devices_for_user(
    location: &WireguardNetwork<Id>,
    transaction: &mut PgConnection,
    user: &User<Id>,
    reserved_ips: Option<&[IpAddr]>,
) -> Result<Vec<GatewayEvent>, WireguardNetworkError> {
    info!("Synchronizing IPs in network {location} for all allowed devices ");
    // list all allowed devices
    let allowed_devices = location
        .get_allowed_devices_for_user(&mut *transaction, user.id)
        .await?;

    // convert to a map for easier processing
    let allowed_devices: HashMap<Id, Device<Id>> = allowed_devices
        .into_iter()
        .map(|dev| (dev.id, dev))
        .collect();

    // check if all devices can fit within network
    // include address, network, and broadcast in the calculation
    let count = allowed_devices.len() + 3;
    location.validate_network_size(count)?;

    // list all assigned IPs
    let assigned_ips =
        WireguardNetworkDevice::all_for_network_and_user(&mut *transaction, location.id, user.id)
            .await?;

    let events = process_device_access_changes(
        location,
        &mut *transaction,
        allowed_devices,
        assigned_ips,
        reserved_ips,
    )
    .await?;

    Ok(events)
}

/// Works out which devices need to be added, removed, or readdressed based on the list
/// of currently configured devices and the list of devices which should be allowed.
pub async fn process_device_access_changes(
    location: &WireguardNetwork<Id>,
    transaction: &mut PgConnection,
    mut allowed_devices: HashMap<Id, Device<Id>>,
    currently_configured_devices: Vec<WireguardNetworkDevice>,
    reserved_ips: Option<&[IpAddr]>,
) -> Result<Vec<GatewayEvent>, WireguardNetworkError> {
    // Loop through current device configurations; remove no longer allowed, readdress
    // when necessary; remove processed entry from all devices list initial list should
    // now contain only devices to be added.
    let mut events: Vec<GatewayEvent> = Vec::new();
    for device_network_config in currently_configured_devices {
        // Device is allowed and an IP was already assigned
        if let Some(device) = allowed_devices.remove(&device_network_config.device_id) {
            // Network address has changed and IP addresses need to be updated
            if !location.contains_all(&device_network_config.wireguard_ips)
                || location.address.len() != device_network_config.wireguard_ips.len()
            {
                let wireguard_network_device = device
                    .assign_next_network_ip(
                        &mut *transaction,
                        location,
                        reserved_ips,
                        Some(&device_network_config.wireguard_ips),
                    )
                    .await?;
                events.push(GatewayEvent::DeviceModified(DeviceInfo {
                    device,
                    network_info: vec![DeviceNetworkInfo {
                        network_id: location.id,
                        device_wireguard_ips: wireguard_network_device.wireguard_ips,
                        preshared_key: wireguard_network_device.preshared_key,
                        is_authorized: wireguard_network_device.is_authorized,
                    }],
                }));
            }
        // Device is no longer allowed
        } else {
            debug!(
                "Device {} no longer allowed, removing network config for {location}",
                device_network_config.device_id
            );
            device_network_config.delete(&mut *transaction).await?;
            if let Some(device) =
                Device::find_by_id(&mut *transaction, device_network_config.device_id).await?
            {
                events.push(GatewayEvent::DeviceDeleted(DeviceInfo {
                    device,
                    network_info: vec![DeviceNetworkInfo {
                        network_id: location.id,
                        device_wireguard_ips: device_network_config.wireguard_ips,
                        preshared_key: device_network_config.preshared_key,
                        is_authorized: device_network_config.is_authorized,
                    }],
                }));
            } else {
                let msg = format!("Device {} does not exist", device_network_config.device_id);
                error!(msg);
                return Err(WireguardNetworkError::Unexpected(msg));
            }
        }
    }

    // Add configs for new allowed devices
    for device in allowed_devices.into_values() {
        let wireguard_network_device = device
            .assign_next_network_ip(&mut *transaction, location, reserved_ips, None)
            .await?;
        events.push(GatewayEvent::DeviceCreated(DeviceInfo {
            device,
            network_info: vec![DeviceNetworkInfo {
                network_id: location.id,
                device_wireguard_ips: wireguard_network_device.wireguard_ips,
                preshared_key: wireguard_network_device.preshared_key,
                is_authorized: wireguard_network_device.is_authorized,
            }],
        }));
    }

    Ok(events)
}

/// Check if devices found in an imported config file exist already,
/// if they do assign a specified IP.
/// Return a list of imported devices which need to be manually mapped to a user
/// and a list of WireGuard events to be sent out.
pub(crate) async fn handle_imported_devices(
    location: &WireguardNetwork<Id>,
    transaction: &mut PgConnection,
    imported_devices: Vec<ImportedDevice>,
) -> Result<(Vec<ImportedDevice>, Vec<GatewayEvent>), WireguardNetworkError> {
    let allowed_devices = location.get_allowed_devices(&mut *transaction).await?;
    // convert to a map for easier processing
    let allowed_devices: HashMap<Id, Device<Id>> = allowed_devices
        .into_iter()
        .map(|dev| (dev.id, dev))
        .collect();

    let mut devices_to_map = Vec::new();
    let mut assigned_device_ids = Vec::new();
    let mut events = Vec::new();
    for imported_device in imported_devices {
        // check if device with a given pubkey exists already
        match Device::find_by_pubkey(&mut *transaction, &imported_device.wireguard_pubkey).await? {
            Some(existing_device) => {
                // check if device is allowed in network
                match allowed_devices.get(&existing_device.id) {
                    Some(_) => {
                        info!(
                            "Device with pubkey {} exists already, assigning IPs {} for new network: {location}",
                            existing_device.wireguard_pubkey,
                            imported_device.wireguard_ips.as_csv()
                        );
                        let wireguard_network_device = WireguardNetworkDevice::new(
                            location.id,
                            existing_device.id,
                            imported_device.wireguard_ips,
                        );
                        wireguard_network_device.insert(&mut *transaction).await?;
                        // store ID of device with already generated config
                        assigned_device_ids.push(existing_device.id);
                        // send device to connected gateways
                        events.push(GatewayEvent::DeviceModified(DeviceInfo {
                            device: existing_device,
                            network_info: vec![DeviceNetworkInfo {
                                network_id: location.id,
                                device_wireguard_ips: wireguard_network_device.wireguard_ips,
                                preshared_key: wireguard_network_device.preshared_key,
                                is_authorized: wireguard_network_device.is_authorized,
                            }],
                        }));
                    }
                    None => {
                        warn!(
                            "Device with pubkey {} exists already, but is not allowed in network {location}. Skipping...",
                            existing_device.wireguard_pubkey
                        );
                    }
                }
            }
            None => devices_to_map.push(imported_device),
        }
    }

    Ok((devices_to_map, events))
}

/// Handle device -> user mapping in second step of network import wizard
pub(crate) async fn handle_mapped_devices(
    location: &WireguardNetwork<Id>,
    transaction: &mut PgConnection,
    mapped_devices: Vec<MappedDevice>,
) -> Result<Vec<GatewayEvent>, WireguardNetworkError> {
    info!("Mapping user devices for network {}", location);
    // get allowed groups for network
    let allowed_groups = location.get_allowed_groups(&mut *transaction).await?;

    let mut events = Vec::new();
    // use a helper hashmap to avoid repeated queries
    let mut user_groups = HashMap::new();
    for mapped_device in &mapped_devices {
        debug!("Mapping device {}", mapped_device.name);
        // validate device pubkey
        Device::validate_pubkey(&mapped_device.wireguard_pubkey).map_err(|_| {
            WireguardNetworkError::InvalidDevicePubkey(mapped_device.wireguard_pubkey.clone())
        })?;
        // save a new device
        let device = Device::new(
            mapped_device.name.clone(),
            mapped_device.wireguard_pubkey.clone(),
            mapped_device.user_id,
            DeviceType::User,
            None,
            true,
        )
        .save(&mut *transaction)
        .await?;
        debug!("Saved new device {device}");

        // get a list of groups user is assigned to
        let groups = match user_groups.get(&device.user_id) {
            // user info has already been fetched before
            Some(groups) => groups,
            // fetch user info
            None => match User::find_by_id(&mut *transaction, device.user_id).await? {
                Some(user) => {
                    let groups = user.member_of_names(&mut *transaction).await?;
                    user_groups.insert(device.user_id, groups);
                    // FIXME: ugly workaround to get around `groups` being dropped
                    user_groups.get(&device.user_id).unwrap()
                }
                None => return Err(WireguardNetworkError::from(ModelError::NotFound)),
            },
        };

        let mut network_info = Vec::new();
        match &allowed_groups {
            None => {
                let wireguard_network_device = WireguardNetworkDevice::new(
                    location.id,
                    device.id,
                    mapped_device.wireguard_ips.clone(),
                );
                wireguard_network_device.insert(&mut *transaction).await?;
                network_info.push(DeviceNetworkInfo {
                    network_id: location.id,
                    device_wireguard_ips: wireguard_network_device.wireguard_ips,
                    preshared_key: wireguard_network_device.preshared_key,
                    is_authorized: wireguard_network_device.is_authorized,
                });
            }
            Some(allowed) => {
                // check if user belongs to an allowed group
                if allowed.iter().any(|group| groups.contains(group)) {
                    // assign specified IP in imported network
                    let wireguard_network_device = WireguardNetworkDevice::new(
                        location.id,
                        device.id,
                        mapped_device.wireguard_ips.clone(),
                    );
                    wireguard_network_device.insert(&mut *transaction).await?;
                    network_info.push(DeviceNetworkInfo {
                        network_id: location.id,
                        device_wireguard_ips: wireguard_network_device.wireguard_ips,
                        preshared_key: wireguard_network_device.preshared_key,
                        is_authorized: wireguard_network_device.is_authorized,
                    });
                }
            }
        }

        // assign IPs in other networks
        let (mut all_network_info, _configs) =
            device.add_to_all_networks(&mut *transaction).await?;

        network_info.append(&mut all_network_info);

        // send device to connected gateways
        if !network_info.is_empty() {
            events.push(GatewayEvent::DeviceCreated(DeviceInfo {
                device,
                network_info,
            }));
        }
    }

    Ok(events)
}

#[cfg(test)]
mod test {
    use defguard_common::db::{models::group::Group, setup_pool};
    use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

    use super::*;

    #[sqlx::test]
    async fn test_sync_allowed_devices_for_user(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;
        let mut network = WireguardNetwork::default();
        network.try_set_address("10.1.1.1/29").unwrap();
        let network = network.save(&pool).await.unwrap();

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

        let mut transaction = pool.begin().await.unwrap();

        // user1 sync
        let events = sync_allowed_devices_for_user(&network, &mut transaction, &user1, None)
            .await
            .unwrap();

        assert_eq!(events.len(), 2);
        assert!(events.iter().any(|e| match e {
            GatewayEvent::DeviceCreated(info) => info.device.id == device1.id,
            _ => false,
        }));
        assert!(events.iter().any(|e| match e {
            GatewayEvent::DeviceCreated(info) => info.device.id == device2.id,
            _ => false,
        }));

        // user 2 sync
        let events = sync_allowed_devices_for_user(&network, &mut transaction, &user2, None)
            .await
            .unwrap();

        assert_eq!(events.len(), 1);
        match &events[0] {
            GatewayEvent::DeviceCreated(info) => {
                assert_eq!(info.device.id, device3.id);
            }
            _ => panic!("Expected DeviceCreated event"),
        }

        // Second sync should not generate any events
        let events = sync_allowed_devices_for_user(&network, &mut transaction, &user1, None)
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
        let mut network = WireguardNetwork::default();
        network.try_set_address("10.1.1.1/29").unwrap();
        let network = network.save(&pool).await.unwrap();

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
                vec![group1.name.clone(), group2.name.clone()],
            )
            .await
            .unwrap();

        let events = sync_allowed_devices_for_user(&network, &mut transaction, &user1, None)
            .await
            .unwrap();
        assert_eq!(events.len(), 0);

        user1.add_to_group(&pool, &group1).await.unwrap();
        user2.add_to_group(&pool, &group1).await.unwrap();
        user3.add_to_group(&pool, &group2).await.unwrap();

        let events = sync_allowed_devices_for_user(&network, &mut transaction, &user1, None)
            .await
            .unwrap();
        assert_eq!(events.len(), 1);
        match &events[0] {
            GatewayEvent::DeviceCreated(info) => {
                assert_eq!(info.device.id, device1.id);
            }
            _ => panic!("Expected DeviceCreated event"),
        }

        let events = sync_allowed_devices_for_user(&network, &mut transaction, &user2, None)
            .await
            .unwrap();
        assert_eq!(events.len(), 1);
        match &events[0] {
            GatewayEvent::DeviceCreated(info) => {
                assert_eq!(info.device.id, device2.id);
            }
            _ => panic!("Expected DeviceCreated event"),
        }

        let events = sync_allowed_devices_for_user(&network, &mut transaction, &user3, None)
            .await
            .unwrap();
        assert_eq!(events.len(), 1);
        match &events[0] {
            GatewayEvent::DeviceCreated(info) => {
                assert_eq!(info.device.id, device3.id);
            }
            _ => panic!("Expected DeviceCreated event"),
        }

        transaction.commit().await.unwrap();
    }
}
