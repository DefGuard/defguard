use std::collections::HashSet;

use defguard_common::db::{
    Id,
    models::{User, WireguardNetwork, device::DeviceInfo},
};
use sqlx::PgConnection;
use thiserror::Error;
use tokio::sync::broadcast::Sender;

use crate::{
    enterprise::{
        firewall::{FirewallError, try_get_location_firewall_config},
        limits::update_counts,
    },
    grpc::{GatewayCommand, send_gateway_command, send_multiple_gateway_commands},
    location_management::sync_allowed_devices_for_user,
};

/// Errors arising from user management operations.
#[derive(Debug, Error)]
pub enum UserManagementError {
    #[error("Database error: {0}")]
    Db(#[from] sqlx::Error),
    #[error("Model error: {0}")]
    Model(#[from] defguard_common::db::models::ModelError),
    #[error("WireGuard network error: {0}")]
    Network(#[from] defguard_common::db::models::WireguardNetworkError),
    #[error("Firewall error: {0}")]
    Firewall(#[from] FirewallError),
}

/// Deletes the user and cleans up his devices from gateways
pub async fn delete_user_and_cleanup_devices(
    user: User<Id>,
    conn: &mut PgConnection,
    gateway_tx: &Sender<GatewayCommand>,
) -> Result<(), UserManagementError> {
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
        events.push(GatewayCommand::DeviceDeleted(device_info));
    }

    user.delete(&mut *conn).await?;
    update_counts(&mut *conn).await?;

    // send firewall config updates to affected locations
    // if they have ACL enabled & enterprise features are active
    for location_id in affected_location_ids {
        if let Some(location) = WireguardNetwork::find_by_id(&mut *conn, location_id).await?
            && let Some(firewall_config) =
                try_get_location_firewall_config(&location, &mut *conn).await?
        {
            debug!(
                "Sending firewall config update for location {location} affected by deleting user {username} devices"
            );
            events.push(GatewayCommand::FirewallConfigChanged(
                location_id,
                firewall_config,
            ));
        }
    }

    send_multiple_gateway_commands(events, gateway_tx);
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
    gateway_tx: &Sender<GatewayCommand>,
) -> Result<(), UserManagementError> {
    user.is_active = false;
    user.save(&mut *conn).await?;
    update_counts(&mut *conn).await?;
    user.logout_all_sessions(&mut *conn).await?;
    sync_allowed_user_devices(user, conn, gateway_tx).await?;
    Ok(())
}

/// Update gateway state based on this user device access rights
pub async fn sync_allowed_user_devices(
    user: &User<Id>,
    conn: &mut PgConnection,
    gateway_tx: &Sender<GatewayCommand>,
) -> Result<(), UserManagementError> {
    debug!("Syncing allowed devices of user {}", user.username);
    let locations = WireguardNetwork::all(&mut *conn).await?;
    for location in locations {
        let gateway_events =
            sync_allowed_devices_for_user(&location, &mut *conn, user, None).await?;

        // check if any peers were updated
        if !gateway_events.is_empty() {
            // send peer update events
            send_multiple_gateway_commands(gateway_events, gateway_tx);
        }

        // send firewall config update if ACLs & enterprise features are enabled
        if let Some(firewall_config) =
            try_get_location_firewall_config(&location, &mut *conn).await?
        {
            send_gateway_command(
                GatewayCommand::FirewallConfigChanged(location.id, firewall_config),
                gateway_tx,
            );
        }
    }
    info!("Allowed devices of user {} synced", user.username);
    Ok(())
}
