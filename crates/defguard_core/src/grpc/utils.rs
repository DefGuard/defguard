use sqlx::PgPool;
use std::{net::IpAddr, str::FromStr};
use tonic::Status;

use super::{
    proto::proxy::{DeviceConfig as ProtoDeviceConfig, DeviceConfigResponse, DeviceInfo},
    InstanceInfo,
};
use crate::{
    db::{
        models::{
            device::{DeviceType, WireguardNetworkDevice},
            polling_token::PollingToken,
            wireguard::WireguardNetwork,
        },
        Device, Id, Settings, User,
    },
    enterprise::db::models::enterprise_settings::EnterpriseSettings,
    AsCsv,
};

// Create a new token for configuration polling.
pub(crate) async fn new_polling_token(
    pool: &PgPool,
    device: &Device<Id>,
) -> Result<String, Status> {
    debug!(
        "Making a new polling token for device {}",
        device.wireguard_pubkey
    );
    let mut transaction = pool.begin().await.map_err(|err| {
        error!("Failed to start transaction while making a new polling token: {err}");
        Status::internal(format!("unexpected error: {err}"))
    })?;

    // 1. Delete existing polling token for the device, if it exists
    // 2. Create a new polling token for the device
    PollingToken::delete_for_device_id(&mut *transaction, device.id)
        .await
        .map_err(|err| {
            error!("Failed to delete polling token: {err}");
            Status::internal(format!("unexpected error: {err}"))
        })?;
    let new_token = PollingToken::new(device.id)
        .save(&mut *transaction)
        .await
        .map_err(|err| {
            error!("Failed to save new polling token: {err}");
            Status::internal(format!("unexpected error: {err}"))
        })?;

    transaction.commit().await.map_err(|err| {
        error!("Failed to commit transaction while making a new polling token: {err}");
        Status::internal(format!("unexpected error: {err}"))
    })?;
    info!(
        "New polling token created for device {}",
        device.wireguard_pubkey
    );

    Ok(new_token.token)
}

pub(crate) async fn build_device_config_response(
    pool: &PgPool,
    device: Device<Id>,
    token: Option<String>,
) -> Result<DeviceConfigResponse, Status> {
    let settings = Settings::get_current_settings();

    let networks = WireguardNetwork::all(pool).await.map_err(|err| {
        error!("Failed to fetch all networks: {err}");
        Status::internal(format!("unexpected error: {err}"))
    })?;

    let enterprise_settings = EnterpriseSettings::get(pool).await.map_err(|err| {
        error!("Failed to get enterprise settings: {err}");
        Status::internal(format!("unexpected error: {err}"))
    })?;

    let mut configs = Vec::new();
    let user = User::find_by_id(pool, device.user_id)
        .await
        .map_err(|_| {
            error!("Failed to get user: {}", device.user_id);
            Status::internal("unexpected error")
        })?
        .ok_or_else(|| {
            error!("User not found: {}", device.user_id);
            Status::internal("unexpected error")
        })?;
    if device.device_type == DeviceType::Network {
        let wireguard_network_device = WireguardNetworkDevice::find_first(pool, device.id)
            .await
            .map_err(|err| {
                error!(
                    "Failed to fetch WireGuard network device for device {}: {err}",
                    device.id
                );
                Status::internal(format!("unexpected error: {err}"))
            })?;
        if let Some(wireguard_network_device) = wireguard_network_device {
            let network = wireguard_network_device
                .network(pool)
                .await
                .map_err(|err| {
                    error!(
                        "Failed to fetch network for WireGuard network device {}: {err}",
                        device.name
                    );
                    Status::internal(format!("unexpected error: {err}"))
                })?;
            let config = ProtoDeviceConfig {
                config: Device::create_config(&network, &wireguard_network_device),
                network_id: network.id,
                network_name: network.name,
                assigned_ip: wireguard_network_device.wireguard_ips.as_csv(),
                endpoint: format!("{}:{}", network.endpoint, network.port),
                pubkey: network.pubkey,
                allowed_ips: network.allowed_ips.as_csv(),
                dns: network.dns,
                mfa_enabled: network.mfa_enabled,
                keepalive_interval: network.keepalive_interval,
            };
            configs.push(config);
        }
    } else {
        for network in networks {
            let wireguard_network_device =
                WireguardNetworkDevice::find(pool, device.id, network.id)
                    .await
                    .map_err(|err| {
                        error!(
                    "Failed to fetch WireGuard network device for device {} and network {}: {err}",
                    device.id, network.id
                );
                        Status::internal(format!("unexpected error: {err}"))
                    })?;
            if let Some(wireguard_network_device) = wireguard_network_device {
                let config = ProtoDeviceConfig {
                    config: Device::create_config(&network, &wireguard_network_device),
                    network_id: network.id,
                    network_name: network.name,
                    assigned_ip: wireguard_network_device.wireguard_ips.as_csv(),
                    endpoint: format!("{}:{}", network.endpoint, network.port),
                    pubkey: network.pubkey,
                    allowed_ips: network.allowed_ips.as_csv(),
                    dns: network.dns,
                    mfa_enabled: network.mfa_enabled,
                    keepalive_interval: network.keepalive_interval,
                };
                configs.push(config);
            }
        }
    }

    info!(
        "User {}({}) device {}({}) automatically fetched the newest configuration.",
        user.username, user.id, device.name, device.id,
    );

    Ok(DeviceConfigResponse {
        device: Some(device.into()),
        configs,
        instance: Some(InstanceInfo::new(settings, &user.username, &enterprise_settings).into()),
        token,
    })
}

/// Parses `DeviceInfo` returning client IP address and user agent.
pub(crate) fn parse_client_info(info: &Option<DeviceInfo>) -> Result<(IpAddr, String), String> {
    let Some(info) = info else {
        error!("Missing DeviceInfo in proxy request");
        return Err("missing device info".to_string());
    };

    let ip = IpAddr::from_str(&info.ip_address).map_err(|_| {
        let msg = format!("invalid IP address: {}", info.ip_address);
        error!(msg);
        msg
    })?;
    let user_agent = info
        .user_agent
        .clone()
        .unwrap_or_else(|| String::new());

    Ok((ip, user_agent))
}
