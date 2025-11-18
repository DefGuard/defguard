use std::{collections::HashMap, net::IpAddr};

use defguard_common::db::{Id, models::ModelError};
use sqlx::PgConnection;
use thiserror::Error;
use tokio::sync::broadcast::Sender;

use crate::{
    db::{
        Device, WireguardNetwork,
        models::{
            device::{DeviceType, WireguardNetworkDevice},
            wireguard::WireguardNetworkError,
        },
    },
    enterprise::firewall::{FirewallError, try_get_location_firewall_config},
    grpc::gateway::{events::GatewayEvent, send_multiple_wireguard_events},
};

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

    let events = location
        .process_device_access_changes(&mut *conn, allowed_devices, assigned_ips, reserved_ips)
        .await?;

    Ok(events)
}
