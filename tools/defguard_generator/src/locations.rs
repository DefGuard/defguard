use std::net::{IpAddr, Ipv4Addr};

use anyhow::Result;
use defguard_common::db::{
    Id,
    models::{
        Device, WireguardNetwork, device::WireguardNetworkDevice, wireguard::ServiceLocationMode,
    },
};
use ipnetwork::IpNetwork;
use sqlx::PgPool;
use tracing::info;

const BASE_PORT: i32 = 51820;

/// Tops up the database to `count` locations. Each new location gets its own `10.<n>.0.0/16`
/// subnet, so there is room for plenty of devices.
pub(crate) async fn prepare_locations(
    pool: &PgPool,
    count: usize,
) -> Result<Vec<WireguardNetwork<Id>>> {
    let mut locations = WireguardNetwork::all(pool).await?;
    if locations.len() >= count {
        info!(
            "Found {} existing locations in the database, skipping",
            locations.len()
        );
        locations.truncate(count);
        return Ok(locations);
    }

    let taken_subnets: Vec<IpNetwork> = locations
        .iter()
        .flat_map(|location| location.address().to_vec())
        .collect();
    let mut octet: u8 = 100;
    info!("Creating {} locations", count - locations.len());

    while locations.len() < count {
        let second_octet = octet;
        octet = octet
            .checked_add(1)
            .expect("Ran out of 10.x.0.0/16 subnets");
        let subnet = IpNetwork::new(IpAddr::V4(Ipv4Addr::new(10, second_octet, 0, 0)), 16)?;
        if taken_subnets
            .iter()
            .any(|taken| taken.contains(subnet.ip()) || subnet.contains(taken.ip()))
        {
            continue;
        }
        let index = locations.len() + 1;
        let location = WireguardNetwork::new(
            format!("Generated Location {index}"),
            BASE_PORT + i32::try_from(index)?,
            format!("vpn{index}.example.com"),
            None,
            vec![subnet],
            true,
            false,
            false,
            false,
            false,
            ServiceLocationMode::Disabled,
        )
        .set_address([IpNetwork::new(
            IpAddr::V4(Ipv4Addr::new(10, second_octet, 0, 1)),
            16,
        )?])?
        .save(pool)
        .await?;
        locations.push(location);
    }

    Ok(locations)
}

/// Assigns each device to each location it isn't assigned to yet.
pub(crate) async fn assign_devices_to_locations(
    pool: &PgPool,
    devices: &[Device<Id>],
    locations: &[WireguardNetwork<Id>],
) -> Result<()> {
    for location in locations {
        let mut transaction = pool.begin().await?;
        let mut used_ips = location.all_used_ip_addresses(&mut transaction).await?;
        for device in devices {
            if WireguardNetworkDevice::find(&mut *transaction, device.id, location.id)
                .await?
                .is_some()
            {
                continue;
            }
            let network_device = device
                .assign_next_network_ip(&mut transaction, location, &used_ips, None, None)
                .await?;
            used_ips.extend(network_device.wireguard_ips);
        }
        transaction.commit().await?;
    }

    Ok(())
}
