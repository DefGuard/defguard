//! Device access management — join devices to networks and build WireGuard configs.
//!
//! This module owns the process of assigning a device to a VPN location and
//! generating its WireGuard configuration.

use defguard_common::{
    db::{
        Id,
        models::{
            Device, DeviceConfig, DeviceError, WireguardNetwork,
            device::{DeviceNetworkInfo, WireguardNetworkDevice},
            user::User,
            wireguard::WireguardNetworkError,
        },
    },
    device_config_gen::create_wireguard_config,
};
use sqlx::PgConnection;
use tracing::warn;

use crate::enterprise::allowed_ips::get_effective_allowed_ips;

/// Build a `DeviceConfig` for a device already assigned to a network.
///
/// Computes effective AllowedIPs (manual + ACL-derived when the location
/// toggle is enabled and an enterprise license is active) and generates the
/// WireGuard config string.
pub async fn build_device_config(
    conn: &mut PgConnection,
    network: &WireguardNetwork<Id>,
    wireguard_network_device: &WireguardNetworkDevice,
    user: &User<Id>,
) -> Result<DeviceConfig, DeviceError> {
    let effective_ips = get_effective_allowed_ips(conn, network, user).await;

    let config = create_wireguard_config(network, wireguard_network_device, &effective_ips);
    let has_postures = network
        .has_postures(&mut *conn)
        .await
        .map_err(|e| DeviceError::Unexpected(e.to_string()))?;

    Ok(DeviceConfig {
        network_id: network.id,
        network_name: network.name.clone(),
        config,
        endpoint: format!("{}:{}", network.endpoint, network.port),
        address: wireguard_network_device.wireguard_ips.clone(),
        allowed_ips: effective_ips,
        pubkey: network.pubkey.clone(),
        dns: network.dns.clone(),
        keepalive_interval: network.keepalive_interval,
        location_mfa_mode: network.location_mfa_mode.clone(),
        service_location_mode: network.service_location_mode.clone(),
        posture_check_required: has_postures,
    })
}

/// Assign IPs to a device in a network and generate its config.
pub async fn join_device_to_network(
    conn: &mut PgConnection,
    device: &Device<Id>,
    network: &WireguardNetwork<Id>,
    user: &User<Id>,
    ips: &[std::net::IpAddr],
) -> Result<(DeviceNetworkInfo, DeviceConfig), DeviceError> {
    let wireguard_network_device = device.assign_network_ips(&mut *conn, network, ips).await?;

    let device_network_info = wireguard_network_device
        .to_device_network_info_runtime(&mut *conn, network)
        .await?;

    let device_config = build_device_config(conn, network, &wireguard_network_device, user).await?;

    Ok((device_network_info, device_config))
}

/// Add a device to every network the user is allowed to join, generating
/// ACL-aware configs for each.
pub async fn join_device_to_all_networks(
    conn: &mut PgConnection,
    device: &Device<Id>,
    user: &User<Id>,
) -> Result<(Vec<DeviceNetworkInfo>, Vec<DeviceConfig>), DeviceError> {
    let networks = WireguardNetwork::all(&mut *conn).await?;

    let mut configs = Vec::new();
    let mut network_info = Vec::new();

    for network in networks {
        // Skip networks where the device's pubkey conflicts with the network pubkey.
        if network.pubkey == device.wireguard_pubkey {
            return Err(DeviceError::PubkeyConflict(device.wireguard_pubkey.clone()));
        }

        // Skip networks the device is already registered in.
        if WireguardNetworkDevice::find(&mut *conn, device.id, network.id)
            .await?
            .is_some()
        {
            continue;
        }

        let wireguard_network_device = match network
            .add_device_to_network(&mut *conn, device, None)
            .await
        {
            Ok(d) => d,
            Err(WireguardNetworkError::DeviceNotAllowed(_)) => {
                warn!(
                    "Device {device} not allowed in network {network}, skipping config \
                    generation for this network"
                );
                continue;
            }
            Err(WireguardNetworkError::DeviceError(DeviceError::NetworkFull(_))) => {
                return Err(DeviceError::NetworkFull(network.name.clone()));
            }
            Err(err) => return Err(DeviceError::Unexpected(err.to_string())),
        };

        let device_network_info = wireguard_network_device
            .to_device_network_info_runtime(&mut *conn, &network)
            .await?;
        network_info.push(device_network_info);

        let device_config =
            build_device_config(conn, &network, &wireguard_network_device, user).await?;
        configs.push(device_config);
    }

    Ok((network_info, configs))
}
