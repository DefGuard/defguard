//! This module implements a functionality of disconnecting inactive peers
//! in MFA-protected locations.
//! If a device does not disconnect explicitly and just becomes inactive
//! it should be removed from gateway configuration and marked as "not allowed",
//! which enforces an authentication requirement to connect again.

use std::{
    net::{IpAddr, Ipv4Addr},
    str::FromStr,
    time::Duration,
};

use chrono::NaiveDateTime;
use sqlx::{Error as SqlxError, PgPool, query_as};
use thiserror::Error;
use tokio::{
    sync::{
        broadcast::{self, Sender},
        mpsc::{self, UnboundedSender},
    },
    time::sleep,
};

use crate::{
    db::{
        Device, GatewayEvent, Id, WireguardNetwork,
        models::{
            device::{DeviceInfo, DeviceNetworkInfo, DeviceType, WireguardNetworkDevice},
            error::ModelError,
            wireguard::WireguardNetworkError,
        },
    },
    events::{InternalEvent, InternalEventContext},
};

// How long to sleep between loop iterations
const DISCONNECT_LOOP_SLEEP: Duration = Duration::from_secs(60); // 1 minute

#[derive(Debug, Error)]
pub enum PeerDisconnectError {
    #[error(transparent)]
    DbError(#[from] SqlxError),
    #[error(transparent)]
    ModelError(#[from] ModelError),
    #[error(transparent)]
    WireguardError(#[from] WireguardNetworkError),
    #[error("Failed to send gateway event: {0}")]
    GatewayEventError(#[from] broadcast::error::SendError<GatewayEvent>),
    #[error("Failed to send internal event: {0}")]
    InternalEventError(#[from] mpsc::error::SendError<InternalEvent>),
}

#[derive(Debug)]
struct DeviceWithEndpoint {
    pub id: Id,
    pub name: String,
    pub wireguard_pubkey: String,
    pub user_id: Id,
    pub created: NaiveDateTime,
    pub device_type: DeviceType,
    pub description: Option<String>,
    pub configured: bool,
    pub endpoint: Option<String>,
}

impl From<DeviceWithEndpoint> for Device<Id> {
    fn from(device: DeviceWithEndpoint) -> Self {
        Self {
            id: device.id,
            name: device.name,
            wireguard_pubkey: device.wireguard_pubkey,
            user_id: device.user_id,
            created: device.created,
            device_type: device.device_type,
            description: device.description,
            configured: device.configured,
        }
    }
}

/// Run periodic disconnect task
///
/// Run with a specified frequency and disconnect all inactive peers in MFA-protected locations.
#[instrument(skip_all)]
pub async fn run_periodic_peer_disconnect(
    pool: PgPool,
    wireguard_tx: Sender<GatewayEvent>,
    internal_event_tx: UnboundedSender<InternalEvent>,
) -> Result<(), PeerDisconnectError> {
    info!("Starting periodic disconnect of inactive devices in MFA-protected locations");
    loop {
        debug!("Starting periodic inactive device disconnect");

        // get all MFA-protected locations
        let locations = query_as!(
            WireguardNetwork::<Id>,
            "SELECT \
                id, name, address, port, pubkey, prvkey, endpoint, dns, allowed_ips, \
                connected_at, mfa_enabled, keepalive_interval, peer_disconnect_threshold, \
                acl_enabled, acl_default_allow \
            FROM wireguard_network WHERE mfa_enabled = true",
        )
        .fetch_all(&pool)
        .await?;

        // loop over all locations
        for location in locations {
            debug!("Fetching inactive devices for location {location}");
            let devices = query_as!(
                DeviceWithEndpoint,
                "WITH stats AS ( \
                SELECT DISTINCT ON (device_id) device_id, endpoint, latest_handshake \
                FROM wireguard_peer_stats \
                WHERE network = $1 \
                ORDER BY device_id, collected_at DESC \
            ) \
            SELECT d.id, d.name, d.wireguard_pubkey, d.user_id, d.created, d.description,
            d.device_type \"device_type: DeviceType\", configured, stats.endpoint \
            FROM device d \
            JOIN wireguard_network_device wnd ON wnd.device_id = d.id \
            LEFT JOIN stats on d.id = stats.device_id \
            WHERE wnd.wireguard_network_id = $1 AND wnd.is_authorized = true \
            AND d.configured = true \
            AND (NOW() - wnd.authorized_at) > $2 * interval '1 second' \
            AND (NOW() - stats.latest_handshake) > $2 * interval '1 second'",
                location.id,
                f64::from(location.peer_disconnect_threshold)
            )
            .fetch_all(&pool)
            .await?;

            for device_with_endpoint in devices {
                debug!("Processing inactive device {device_with_endpoint:?}");
                let endpoint = device_with_endpoint.endpoint.clone();
                let device: Device<Id> = device_with_endpoint.into();

                // start transaction
                let mut transaction = pool.begin().await?;

                // get network config for device
                if let Some(mut device_network_config) =
                    WireguardNetworkDevice::find(&mut *transaction, device.id, location.id).await?
                {
                    info!(
                        "Marking device {device} as not authorized to connect to location {location}"
                    );
                    // change `is_authorized` value for device
                    device_network_config.is_authorized = false;
                    // clear `preshared_key` value
                    device_network_config.preshared_key = None;
                    device_network_config.update(&mut *transaction).await?;

                    debug!("Sending `peer_delete` message to gateway");
                    let device_info = DeviceInfo {
                        device: device.clone(),
                        network_info: vec![DeviceNetworkInfo {
                            network_id: location.id,
                            device_wireguard_ips: device_network_config.wireguard_ips,
                            preshared_key: device_network_config.preshared_key,
                            is_authorized: device_network_config.is_authorized,
                        }],
                    };
                    let event = GatewayEvent::DeviceDeleted(device_info);
                    wireguard_tx.send(event).map_err(|err| {
                        error!("Error sending WireGuard event: {err}");
                        PeerDisconnectError::GatewayEventError(err)
                    })?;
                    let user = device.get_owner(&mut *transaction).await?;
                    let ip = endpoint
                        .as_ref()
                        .and_then(|endpoint| endpoint.split_once(':'))
                        .and_then(|(ip, _)| IpAddr::from_str(ip).ok())
                        // endpoint is a `text` column in the db so we have to
                        // handle potential parsing issues here
                        .unwrap_or_else(|| IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)));
                    let event = InternalEvent::DesktopClientMfaDisconnected {
                        context: InternalEventContext::new(user.id, user.username, ip, device),
                        location: location.clone(),
                    };
                    internal_event_tx.send(event).map_err(|err| {
                        error!("Error sending internal event: {err}");
                        PeerDisconnectError::InternalEventError(err)
                    })?;
                } else {
                    error!(
                        "Network config for device {device} in location {location} not found. Skipping device..."
                    );
                    continue;
                }

                // commit transaction
                transaction.commit().await?;
            }
        }

        // wait till next iteration
        debug!("Sleeping until next iteration");
        sleep(DISCONNECT_LOOP_SLEEP).await;
    }
}
