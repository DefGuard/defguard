use std::{collections::HashMap, net::IpAddr};

use defguard_common::{
    csv::AsCsv,
    db::{
        Id,
        models::{
            Device, DeviceError, DeviceType, ModelError, WireguardNetwork, WireguardNetworkError,
            device::{DeviceInfo, WireguardNetworkDevice},
            user::User,
            vpn_client_session::VpnClientSession,
            wireguard::MappedDevice,
        },
    },
    gateway_event::send_multiple_gateway_commands,
};
use sqlx::PgConnection;
use thiserror::Error;
use tokio::sync::broadcast::Sender;

use crate::{
    device_access::join_device_to_all_networks,
    enterprise::firewall::{FirewallError, try_get_location_firewall_config},
    grpc::GatewayCommand,
    wg_config::ImportedDevice,
};

pub mod allowed_peers;
#[cfg(test)]
mod tests;

pub(crate) struct LocationManager {
    gateway_commands: Vec<GatewayCommand>,
}

impl LocationManager {
    // Run `sync_allowed_devices` on all WireGuard networks.
    pub(crate) async fn sync_all_networks(
        conn: &mut PgConnection,
    ) -> Result<Self, LocationManagementError> {
        info!("Syncing allowed devices for all WireGuard locations");
        let locations = WireguardNetwork::all(&mut *conn).await?;
        let mut gateway_commands = Vec::new();
        for network in locations {
            sync_location_allowed_devices(&network, &mut *conn, None, &mut gateway_commands)
                .await?;

            // Send firewall config update, if ACLs are enabled for a given location.
            if let Some(firewall_config) =
                try_get_location_firewall_config(&network, &mut *conn).await?
            {
                gateway_commands.push(GatewayCommand::FirewallConfigChanged(
                    network.id,
                    firewall_config,
                ));
            }
        }

        Ok(Self { gateway_commands })
    }

    /// Send all commands to Gateway. Use this method *after* database transaction is committed.
    pub(crate) fn send(self, gateway_tx: &Sender<GatewayCommand>) {
        send_multiple_gateway_commands(self.gateway_commands, gateway_tx);
    }
}

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

/// Refresh network IPs for all relevant devices.
///
/// If the list of allowed devices has changed add/remove devices accordingly.
///
/// If the network address has changed, re-address existing devices.
pub(crate) async fn sync_location_allowed_devices(
    location: &WireguardNetwork<Id>,
    conn: &mut PgConnection,
    reserved_ips: Option<&[IpAddr]>,
    events: &mut Vec<GatewayCommand>,
) -> Result<(), LocationManagementError> {
    info!("Synchronizing IPs in network {location} for all allowed devices ");
    // list all allowed devices
    let mut allowed_devices = location.get_allowed_devices(&mut *conn).await?;

    // Network devices are always allowed, make sure to take only network devices already assigned
    // to that network.
    let network_devices =
        Device::find_by_type_and_network(&mut *conn, DeviceType::Network, location.id).await?;
    allowed_devices.extend(network_devices);

    // Convert to a map for easier processing.
    let allowed_devices = allowed_devices
        .into_iter()
        .map(|dev| (dev.id, dev))
        .collect::<HashMap<_, _>>();

    // Check if all devices can fit within network.
    let count = allowed_devices.len();
    location.validate_network_size(count)?;

    // list all assigned IPs
    let assigned_ips = WireguardNetworkDevice::all_for_network(&mut *conn, location.id).await?;

    process_device_access_changes(
        location,
        &mut *conn,
        allowed_devices,
        assigned_ips,
        reserved_ips,
        events,
    )
    .await?;

    Ok(())
}

/// Refresh network IPs for all relevant devices of a given user.
/// If the list of allowed devices has changed add/remove devices accordingly.
/// If the network address has changed readdress existing devices.
pub(crate) async fn sync_allowed_devices_for_user(
    location: &WireguardNetwork<Id>,
    conn: &mut PgConnection,
    user: &User<Id>,
    reserved_ips: Option<&[IpAddr]>,
    events: &mut Vec<GatewayCommand>,
) -> Result<(), WireguardNetworkError> {
    info!("Synchronizing IPs in network {location} for all allowed devices ");
    // list all allowed devices
    let allowed_devices = location
        .get_allowed_devices_for_user(&mut *conn, user.id)
        .await?;

    // Convert to a map for easier processing.
    let allowed_devices = allowed_devices
        .into_iter()
        .map(|dev| (dev.id, dev))
        .collect::<HashMap<_, _>>();

    // Check if all devices can fit within network.
    let count = allowed_devices.len();
    location.validate_network_size(count)?;

    // list all assigned IPs
    let assigned_ips =
        WireguardNetworkDevice::all_for_network_and_user(&mut *conn, location.id, user.id).await?;

    process_device_access_changes(
        location,
        &mut *conn,
        allowed_devices,
        assigned_ips,
        reserved_ips,
        events,
    )
    .await?;

    Ok(())
}

/// Works out which devices need to be added, removed, or readdressed based on the list
/// of currently configured devices and the list of devices which should be allowed.
pub async fn process_device_access_changes(
    location: &WireguardNetwork<Id>,
    conn: &mut PgConnection,
    mut allowed_devices: HashMap<Id, Device<Id>>,
    currently_configured_devices: Vec<WireguardNetworkDevice>,
    reserved_ips: Option<&[IpAddr]>,
    events: &mut Vec<GatewayCommand>,
) -> Result<(), WireguardNetworkError> {
    // Loop through current device configurations; remove no longer allowed, readdress
    // when necessary; remove processed entry from all devices list initial list should
    // now contain only devices to be added.
    let mut used_ips = location.all_used_ip_addresses(&mut *conn).await?;
    let active_sessions = if location.mfa_enabled {
        VpnClientSession::get_all_active_for_location(&mut *conn, location.id)
            .await?
            .into_iter()
            .map(|session| (session.device_id, session))
            .collect::<HashMap<_, _>>()
    } else {
        HashMap::new()
    };
    for device_network_config in currently_configured_devices {
        // Device is allowed and an IP address was already assigned.
        if let Some(device) = allowed_devices.remove(&device_network_config.device_id) {
            // Network address has changed and IP addresses need to be updated.
            if !location.contains_all(&device_network_config.wireguard_ips)
                || location.address().len() != device_network_config.wireguard_ips.len()
            {
                let wireguard_network_device = device
                    .assign_next_network_ip(
                        &mut *conn,
                        location,
                        &used_ips,
                        reserved_ips,
                        Some(&device_network_config.wireguard_ips),
                    )
                    .await?;
                used_ips.extend(wireguard_network_device.wireguard_ips.iter().copied());
                let network_info = wireguard_network_device
                    .to_device_network_info(location, active_sessions.get(&device.id));
                events.push(GatewayCommand::DeviceModified(DeviceInfo {
                    device,
                    network_info: vec![network_info],
                }));
            }
        // Device is no longer allowed.
        } else {
            debug!(
                "Device {} no longer allowed, removing network config for {location}",
                device_network_config.device_id
            );
            device_network_config.delete(&mut *conn).await?;
            // Remove freed IPs so they can be reused by later assignments
            used_ips.retain(|ip| !device_network_config.wireguard_ips.contains(ip));
            if let Some(device) =
                Device::find_by_id(&mut *conn, device_network_config.device_id).await?
            {
                let network_info = device_network_config.to_device_network_info(location, None);
                events.push(GatewayCommand::DeviceDeleted(DeviceInfo {
                    device,
                    network_info: vec![network_info],
                }));
            } else {
                let msg = format!("Device {} does not exist", device_network_config.device_id);
                error!(msg);
                return Err(WireguardNetworkError::Unexpected(msg));
            }
        }
    }
    // Add configs for new allowed devices.
    for device in allowed_devices.into_values() {
        let wireguard_network_device = device
            .assign_next_network_ip(&mut *conn, location, &used_ips, reserved_ips, None)
            .await?;
        used_ips.extend(wireguard_network_device.wireguard_ips.iter().copied());
        let network_info = wireguard_network_device
            .to_device_network_info_runtime(&mut *conn, location)
            .await?;
        events.push(GatewayCommand::DeviceCreated(DeviceInfo {
            device,
            network_info: vec![network_info],
        }));
    }

    Ok(())
}

/// Check if devices found in an imported config file exist already,
/// if they do assign a specified IP.
/// Return a list of imported devices which need to be manually mapped to a user
/// and a list of gateway commands to be sent out.
pub(crate) async fn handle_imported_devices(
    location: &WireguardNetwork<Id>,
    conn: &mut PgConnection,
    imported_devices: Vec<ImportedDevice>,
) -> Result<(Vec<ImportedDevice>, Vec<GatewayCommand>), WireguardNetworkError> {
    let allowed_devices = location.get_allowed_devices(&mut *conn).await?;
    // convert to a map for easier processing
    let allowed_devices = allowed_devices
        .into_iter()
        .map(|dev| (dev.id, dev))
        .collect::<HashMap<_, _>>();

    let mut devices_to_map = Vec::new();
    let mut assigned_device_ids = Vec::new();
    let mut events = Vec::new();
    for imported_device in imported_devices {
        // check if device with a given pubkey exists already
        match Device::find_by_pubkey(&mut *conn, &imported_device.wireguard_pubkey).await? {
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
                        wireguard_network_device.insert(&mut *conn).await?;
                        // store ID of device with already generated config
                        assigned_device_ids.push(existing_device.id);
                        let network_info = wireguard_network_device
                            .to_device_network_info_runtime(&mut *conn, location)
                            .await?;
                        // send device to connected gateways
                        events.push(GatewayCommand::DeviceModified(DeviceInfo {
                            device: existing_device,
                            network_info: vec![network_info],
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
    conn: &mut PgConnection,
    mapped_devices: &[MappedDevice],
) -> Result<Vec<GatewayCommand>, WireguardNetworkError> {
    info!("Mapping user devices for network {location}");
    // get allowed groups for network
    let allowed_groups = location.get_allowed_groups(&mut *conn).await?;

    let mut events = Vec::new();
    // use a helper hashmap to avoid repeated queries
    let mut user_groups = HashMap::new();
    for mapped_device in mapped_devices {
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
        .save(&mut *conn)
        .await?;
        debug!("Saved new device {device}");

        // get a list of groups user is assigned to
        let groups = match user_groups.get(&device.user_id) {
            // user info has already been fetched before
            Some(groups) => groups,
            // fetch user info
            None => match User::find_by_id(&mut *conn, device.user_id).await? {
                Some(user) => {
                    let groups = user.member_of_names(&mut *conn).await?;
                    user_groups.insert(device.user_id, groups);
                    // FIXME: ugly workaround to get around `groups` being dropped
                    user_groups.get(&device.user_id).unwrap()
                }
                None => return Err(WireguardNetworkError::from(ModelError::NotFound)),
            },
        };

        let mut network_info = Vec::new();
        if location.allow_all_groups || allowed_groups.iter().any(|group| groups.contains(group)) {
            let wireguard_network_device = WireguardNetworkDevice::new(
                location.id,
                device.id,
                mapped_device.wireguard_ips.clone(),
            );
            wireguard_network_device.insert(&mut *conn).await?;
            network_info.push(
                wireguard_network_device
                    .to_device_network_info_runtime(&mut *conn, location)
                    .await?,
            );
        }

        // Assign IP addresses in other networks.
        let user = User::find_by_id(&mut *conn, device.user_id)
            .await?
            .ok_or_else(|| {
                WireguardNetworkError::DeviceError(DeviceError::Unexpected(format!(
                    "User {} not found",
                    device.user_id
                )))
            })?;
        let (mut all_network_info, _configs) =
            join_device_to_all_networks(&mut *conn, &device, &user).await?;

        network_info.append(&mut all_network_info);

        // send device to connected gateways
        if !network_info.is_empty() {
            events.push(GatewayCommand::DeviceCreated(DeviceInfo {
                device,
                network_info,
            }));
        }
    }

    Ok(events)
}
