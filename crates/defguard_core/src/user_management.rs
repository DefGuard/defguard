use std::collections::HashSet;

use defguard_common::db::Id;
use sqlx::PgConnection;
use tokio::sync::broadcast::Sender;

use crate::{
    db::{User, WireguardNetwork, models::device::DeviceInfo},
    enterprise::limits::update_counts,
    error::WebError,
    grpc::gateway::{events::GatewayEvent, send_multiple_wireguard_events, send_wireguard_event},
};

/// Deletes the user and cleans up his devices from gateways
pub async fn delete_user_and_cleanup_devices(
    user: User<Id>,
    conn: &mut PgConnection,
    wg_tx: &Sender<GatewayEvent>,
) -> Result<(), WebError> {
    let username = user.username.clone();
    debug!("Deleting user {username}, removing his devices from gateways and updating ldap...",);
    let devices = user.devices(&mut *conn).await?;
    let mut events = Vec::new();

    // get all locations affected by devices being deleted
    let mut affected_location_ids = HashSet::new();

    for device in devices {
        let device_info = DeviceInfo::from_device(&mut *conn, device).await?;
        for network_info in &device_info.network_info {
            affected_location_ids.insert(network_info.network_id);
        }
        events.push(GatewayEvent::DeviceDeleted(device_info));
    }

    user.delete(&mut *conn).await?;
    update_counts(&mut *conn).await?;

    // send firewall config updates to affected locations
    // if they have ACL enabled & enterprise features are active
    for location_id in affected_location_ids {
        if let Some(location) = WireguardNetwork::find_by_id(&mut *conn, location_id).await? {
            if let Some(firewall_config) = location.try_get_firewall_config(&mut *conn).await? {
                debug!(
                    "Sending firewall config update for location {location} affected by deleting user {username} devices"
                );
                events.push(GatewayEvent::FirewallConfigChanged(
                    location_id,
                    firewall_config,
                ));
            }
        }
    }

    send_multiple_wireguard_events(events, wg_tx);
    info!(
        "The user {} has been deleted and his devices removed from gateways.",
        &username
    );
    Ok(())
}

/// Disable user, log out all his sessions and update gateways state.
pub async fn disable_user(
    user: &mut User<Id>,
    conn: &mut PgConnection,
    wg_tx: &Sender<GatewayEvent>,
) -> Result<(), WebError> {
    user.is_active = false;
    user.save(&mut *conn).await?;
    user.logout_all_sessions(&mut *conn).await?;
    sync_allowed_user_devices(user, conn, wg_tx).await?;
    Ok(())
}

/// Update gateway state based on this user device access rights
pub async fn sync_allowed_user_devices(
    user: &User<Id>,
    conn: &mut PgConnection,
    wg_tx: &Sender<GatewayEvent>,
) -> Result<(), WebError> {
    debug!("Syncing allowed devices of user {}", user.username);
    let networks = WireguardNetwork::all(&mut *conn).await?;
    for network in networks {
        let gateway_events = network
            .sync_allowed_devices_for_user(&mut *conn, user, None)
            .await?;

        // check if any peers were updated
        if !gateway_events.is_empty() {
            // send peer update events
            send_multiple_wireguard_events(gateway_events, wg_tx);
        }

        // send firewall config update if ACLs & enterprise features are enabled
        if let Some(firewall_config) = network.try_get_firewall_config(&mut *conn).await? {
            send_wireguard_event(
                GatewayEvent::FirewallConfigChanged(network.id, firewall_config),
                wg_tx,
            );
        }
    }
    info!("Allowed devices of user {} synced", user.username);
    Ok(())
}
