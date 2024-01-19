//! This module implements a functionality of disconnecting inactive peers
//! in MFA-protected locations.
//! If a device does not disconnect explicitly and just becomes inactive
//! it should be removed from gateway configuration and marked as "not allowed",
//! which enforces an authentication requirement to connect again.

use crate::db::{
    models::{
        device::{DeviceInfo, DeviceNetworkInfo, WireguardNetworkDevice},
        error::ModelError,
        wireguard::WireguardNetworkError,
    },
    DbPool, Device, GatewayEvent, WireguardNetwork,
};
use sqlx::{query_as, Error as SqlxError};
use std::time::Duration;
use thiserror::Error;
use tokio::{sync::broadcast::Sender, time::sleep};

// How long to sleep between loop iterations
const DISCONNECT_LOOP_SLEEP_SECONDS: u64 = 180; // 3 minutes

#[derive(Debug, Error)]
pub enum PeerDisconnectError {
    #[error(transparent)]
    DbError(#[from] SqlxError),
    #[error(transparent)]
    ModelError(#[from] ModelError),
    #[error(transparent)]
    WireguardError(#[from] WireguardNetworkError),
    #[error("Failed to send gateway event: {0}")]
    EventError(String),
}

/// Run periodic disconnect task
///
/// Run with a specified frequency and disconnect all inactive peers in MFA-protected locations.
pub async fn run_periodic_peer_disconnect(
    pool: DbPool,
    wireguard_tx: Sender<GatewayEvent>,
) -> Result<(), PeerDisconnectError> {
    info!("Starting periodic disconnect of inactive devices in MFA-protected locations");
    loop {
        debug!("Starting periodic inactive device disconnect");

        // get all MFA-protected locations
        let locations = query_as!(
            WireguardNetwork,
            "SELECT \
                id as \"id?\", name, address, port, pubkey, prvkey, endpoint, dns, allowed_ips, \
                connected_at, mfa_enabled, keepalive_interval, peer_disconnect_threshold \
            FROM wireguard_network WHERE mfa_enabled = true",
        )
        .fetch_all(&pool)
        .await?;

        // loop over all locations
        for location in locations {
            debug!("Fetching inactive devices for location {location}");
            let location_id = location.get_id()?;
            let devices = query_as!(
            Device,
            "WITH stats AS ( \
                    SELECT DISTINCT ON (device_id) device_id, endpoint, latest_handshake \
                    FROM wireguard_peer_stats \
                    WHERE network = $1 \
                    ORDER BY device_id, collected_at DESC \
                ) \
            SELECT d.id as \"id?\", d.name, d.wireguard_pubkey, d.user_id, d.created \
            FROM device d \
            JOIN wireguard_network_device wnd ON wnd.device_id = d.id \
            LEFT JOIN stats on d.id = stats.device_id \
            WHERE wnd.wireguard_network_id = $1 AND wnd.is_authorized = true AND \
            (stats.latest_handshake IS NULL OR (NOW() - stats.latest_handshake) > $2 * interval '1 second')",
            location_id,
                location.peer_disconnect_threshold as f64
        )
                .fetch_all(&pool)
                .await?;

            for device in devices {
                debug!("Processing inactive device {device}");
                let device_id = device.get_id()?;

                // start transaction
                let mut transaction = pool.begin().await?;

                // get network config for device
                if let Some(mut device_network_config) =
                    WireguardNetworkDevice::find(&mut *transaction, device_id, location_id).await?
                {
                    info!("Marking device {device} as not authorized to connect to location {location}");
                    // change `is_authorized` value for device
                    device_network_config.is_authorized = false;
                    // clear `preshared_key` value
                    device_network_config.preshared_key = None;
                    device_network_config.update(&mut *transaction).await?;

                    debug!("Sending `peer_delete` message to gateway");
                    let device_info = DeviceInfo {
                        device,
                        network_info: vec![DeviceNetworkInfo {
                            network_id: location_id,
                            device_wireguard_ip: device_network_config.wireguard_ip,
                            preshared_key: device_network_config.preshared_key,
                            is_authorized: device_network_config.is_authorized,
                        }],
                    };
                    let event = GatewayEvent::DeviceDeleted(device_info);
                    wireguard_tx.send(event).map_err(|err| {
                        error!("Error sending WireGuard event: {err}");
                        PeerDisconnectError::EventError(err.to_string())
                    })?;
                } else {
                    error!("Network config for device {device} in location {location} not found. Skipping device...");
                    continue;
                }

                // commit transaction
                transaction.commit().await?;
            }
        }

        // wait till next iteration
        debug!("Sleeping until next iteration");
        sleep(Duration::from_secs(DISCONNECT_LOOP_SLEEP_SECONDS)).await;
    }
}
