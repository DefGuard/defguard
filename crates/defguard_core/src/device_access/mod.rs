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
            mfa_flow::MfaFlow,
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
        .map_err(|err| DeviceError::Unexpected(err.to_string()))?;

    // `None` when the location's flow configuration has no legacy equivalent. Carried through as
    // absent rather than coerced to `Disabled`, which would advertise an MFA-enabled location as
    // unprotected. Gating such locations for legacy clients is tracked separately (#3042).
    let location_mfa_mode = MfaFlow::derive_legacy_mode(&mut *conn, network.id)
        .await
        .map_err(|err| DeviceError::Unexpected(err.to_string()))?;

    // Resolve the location's MFA flow for this user, carrying the ordered steps (methods only;
    // per-method `configured` flags are computed separately). Empty when MFA is disabled or no
    // flow resolves for the user.
    let steps = if network.mfa_enabled {
        MfaFlow::resolve_for_user(conn, network.id, user.id)
            .await
            .map_err(|err| DeviceError::Unexpected(err.to_string()))?
            .map(|(_, steps)| steps)
            .unwrap_or_default()
    } else {
        Vec::new()
    };

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
        mfa_enabled: network.mfa_enabled,
        location_mfa_mode,
        service_location_mode: network.service_location_mode.clone(),
        posture_check_required: has_postures,
        steps,
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

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr};

    use defguard_common::db::{
        Id,
        models::{
            Device, DeviceType, User, WireguardNetwork,
            device::WireguardNetworkDevice,
            group::Group,
            mfa_flow::{LocationMfaFlowAssignment, MfaFlow},
            vpn_client_session::VpnClientMfaMethod,
        },
        setup_pool,
    };
    use sqlx::{
        PgPool,
        postgres::{PgConnectOptions, PgPoolOptions},
    };

    use super::build_device_config;

    async fn create_user(pool: &PgPool, username: &str) -> User<Id> {
        User::new(
            username.to_owned(),
            None,
            "Test".to_owned(),
            "User".to_owned(),
            format!("{username}@test.example"),
            None,
        )
        .save(pool)
        .await
        .expect("failed to create user")
    }

    async fn create_device(pool: &PgPool, user_id: Id) -> Device<Id> {
        Device::new(
            "device-access-test".to_owned(),
            format!("device-access-pubkey-{user_id}"),
            user_id,
            DeviceType::User,
            None,
            true,
        )
        .save(pool)
        .await
        .expect("failed to create device")
    }

    #[sqlx::test]
    async fn test_build_device_config_resolves_flow_steps(
        _: PgPoolOptions,
        options: PgConnectOptions,
    ) {
        let pool = setup_pool(options).await;

        let group_user = create_user(&pool, "group-user").await;
        let default_user = create_user(&pool, "default-user").await;
        let group = Group::new("step-group")
            .save(&pool)
            .await
            .expect("failed to create group");
        group_user
            .add_to_group(&pool, &group)
            .await
            .expect("failed to add user to group");

        let mut network = WireguardNetwork::default()
            .try_set_address("10.0.0.1/24")
            .expect("failed to set network address")
            .save(&pool)
            .await
            .expect("failed to create network");
        network.mfa_enabled = true;
        network.save(&pool).await.expect("failed to enable MFA");

        // Group-scoped flow: two steps (TOTP -> Email). Default flow: one step (Oidc).
        let mut tx = pool.begin().await.expect("failed to begin tx");
        let (group_flow, _) = MfaFlow::create(
            &mut tx,
            "group-flow".to_owned(),
            vec![
                vec![VpnClientMfaMethod::Totp],
                vec![VpnClientMfaMethod::Email],
            ],
        )
        .await
        .expect("failed to create group flow");
        let (default_flow, _) = MfaFlow::create(
            &mut tx,
            "default-flow".to_owned(),
            vec![vec![VpnClientMfaMethod::Oidc]],
        )
        .await
        .expect("failed to create default flow");
        MfaFlow::assign_to_location(
            &mut tx,
            network.id,
            &[
                LocationMfaFlowAssignment {
                    flow_id: group_flow.id,
                    is_default: false,
                    group_ids: vec![group.id],
                },
                LocationMfaFlowAssignment {
                    flow_id: default_flow.id,
                    is_default: true,
                    group_ids: vec![],
                },
            ],
        )
        .await
        .expect("failed to assign flows");
        tx.commit().await.expect("failed to commit tx");

        // The group user resolves to the group-scoped flow's two steps, in order.
        let group_device = create_device(&pool, group_user.id).await;
        let wireguard_network_device = WireguardNetworkDevice::new(
            network.id,
            group_device.id,
            vec![IpAddr::V4(Ipv4Addr::new(10, 0, 0, 10))],
        );
        let mut conn = pool.acquire().await.expect("failed to acquire connection");
        let config =
            build_device_config(&mut conn, &network, &wireguard_network_device, &group_user)
                .await
                .expect("failed to build config");
        assert_eq!(config.steps.len(), 2);
        assert_eq!(config.steps[0].methods, vec![VpnClientMfaMethod::Totp]);
        assert_eq!(config.steps[1].methods, vec![VpnClientMfaMethod::Email]);

        // The default user falls through to the default flow's single step.
        let default_device = create_device(&pool, default_user.id).await;
        let wireguard_network_device = WireguardNetworkDevice::new(
            network.id,
            default_device.id,
            vec![IpAddr::V4(Ipv4Addr::new(10, 0, 0, 11))],
        );
        let mut conn = pool.acquire().await.expect("failed to acquire connection");
        let config = build_device_config(
            &mut conn,
            &network,
            &wireguard_network_device,
            &default_user,
        )
        .await
        .expect("failed to build config");
        assert_eq!(config.steps.len(), 1);
        assert_eq!(config.steps[0].methods, vec![VpnClientMfaMethod::Oidc]);
    }
}
