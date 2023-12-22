//! This module implements a functionality of disconnecting inactive peers
//! in MFA-protected locations.
//! If a device does not disconnect explicitly and just becomes inactive
//! it should be removed from gateway configuration and marked as "not allowed",
//! which enforces an authentication requirement to connect again.

use crate::db::{DbPool, Device, GatewayEvent, WireguardNetwork, WireguardPeerStats};
use chrono::{DateTime, Duration as ChronoDuration, NaiveDateTime, Utc};
use humantime::format_duration;
use sqlx::{query, query_as, query_scalar, Error as SqlxError, PgExecutor};
use std::time::Duration;
use tokio::sync::broadcast::Sender;
use tokio::time::sleep;
use crate::db::models::device::DeviceInfo;

// How long to sleep between loop iterations
const DISCONNECT_LOOP_SLEEP_SECONDS: u64 = 180; // 3 minutes

/// Run periodic disconnect task
///
/// Run with a specified frequency and disconnect all inactive peers in MFA-protected locations.
pub async fn run_periodic_peer_disconnect(
    pool: DbPool,
    wireguard_tx: Sender<GatewayEvent>,
) -> Result<(), SqlxError> {
    info!("Starting periodic disconnect of inactive devices in MFA-protected locations");
    loop {
        // start transaction
        let transaction = pool.begin().await?;

        // get all MFA-protected locations
        let locations = query_as!(
            WireguardNetwork,
            "SELECT \
                id as \"id?\", name, address, port, pubkey, prvkey, endpoint, dns, allowed_ips, \
                connected_at, mfa_enabled, keepalive_interval, peer_disconnect_threshold \
            FROM wireguard_network WHERE mfa_enabled = true",
        )
        .fetch_all(&transaction)
        .await?;

        // loop over all locations
        for location in locations {
            debug!("Fetching inactive devices for location {location}");
            let devices = query_as!(
            Device,
            "WITH stats AS ( \
                    SELECT DISTINCT ON (network) network, endpoint, latest_handshake \
                    FROM wireguard_peer_stats \
                    WHERE device_id = $2 \
                    ORDER BY network, collected_at DESC \
                ) \
            SELECT d.id as \"id?\", d.name, d.wireguard_pubkey, d.user_id, d.created, d.preshared_key \
            FROM device d \
            JOIN wireguard_network_device wnd ON wnd.device_id = d.id \
            JOIN
            WHERE wnd.wireguard_network_id = $1 AND is_authorized = true",
            location.id
        )
                .fetch_all(&mut *transaction)
                .await?;

            for device in devices {
                // get network config for device

                // change `is_allowed` value for each device

                // send `peer_delete` message to gateway
                let event = GatewayEvent::DeviceDeleted();
                if let Err(err) = wireguard_tx.send(event) {
                    error!("Error sending wireguard event {err}");
                }
            }
        }

        // commit transaction
        transaction.commit().await?;

        // wait till next iteration
        debug!("Sleeping until next iteration");
        sleep(Duration::from_secs(DISCONNECT_LOOP_SLEEP_SECONDS)).await;
    }
}
