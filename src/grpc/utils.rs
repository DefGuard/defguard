use ipnetwork::IpNetwork;
use tonic::Status;

use super::{
    proto::{DeviceConfig as ProtoDeviceConfig, DeviceConfigResponse},
    InstanceInfo,
};
use crate::{
    db::{
        models::{
            device::WireguardNetworkDevice, polling_token::PollingToken,
            wireguard::WireguardNetwork,
        },
        DbPool, Device, Settings, User,
    },
    enterprise::db::models::enterprise_settings::EnterpriseSettings,
};

pub(crate) async fn build_device_config_response(
    pool: &DbPool,
    pubkey: &str,
    // Whether to make a new polling token for the device
    new_token: bool,
) -> Result<DeviceConfigResponse, Status> {
    Device::validate_pubkey(pubkey).map_err(|_| {
        error!("Invalid pubkey {pubkey}");
        Status::invalid_argument("invalid pubkey")
    })?;
    // Find existing device by public key
    let device = Device::find_by_pubkey(pool, pubkey).await.map_err(|_| {
        error!("Failed to get device by its pubkey: {pubkey}");
        Status::internal("unexpected error")
    })?;
    let settings = Settings::get_settings(pool).await.map_err(|_| {
        error!("Failed to get settings");
        Status::internal("unexpected error")
    })?;

    let networks = WireguardNetwork::all(pool).await.map_err(|err| {
        error!("Failed to fetch all networks: {err}");
        Status::internal(format!("unexpected error: {err}"))
    })?;

    let enterprise_settings = EnterpriseSettings::get(pool).await.map_err(|err| {
        error!("Failed to get enterprise settings: {err}");
        Status::internal(format!("unexpected error: {err}"))
    })?;

    let mut configs: Vec<ProtoDeviceConfig> = Vec::new();
    let Some(device) = device else {
        return Err(Status::internal("device not found error"));
    };
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
    for network in networks {
        let (Some(device_id), Some(network_id)) = (device.id, network.id) else {
            continue;
        };
        let wireguard_network_device = WireguardNetworkDevice::find(pool, device_id, network_id)
            .await
            .map_err(|err| {
                error!(
                    "Failed to fetch wireguard network device for device {} and network {}: {err}",
                    device_id, network_id
                );
                Status::internal(format!("unexpected error: {err}"))
            })?;
        if let Some(wireguard_network_device) = wireguard_network_device {
            let allowed_ips = network
                .allowed_ips
                .iter()
                .map(IpNetwork::to_string)
                .collect::<Vec<String>>()
                .join(",");
            let config = ProtoDeviceConfig {
                config: device.create_config(&network, &wireguard_network_device),
                network_id,
                network_name: network.name,
                assigned_ip: wireguard_network_device.wireguard_ip.to_string(),
                endpoint: format!("{}:{}", network.endpoint, network.port),
                pubkey: network.pubkey,
                allowed_ips,
                dns: network.dns,
                mfa_enabled: network.mfa_enabled,
                keepalive_interval: network.keepalive_interval,
            };
            configs.push(config);
        }
    }

    let token = if new_token {
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
        PollingToken::delete_for_device_id(
            &mut *transaction,
            device.id.ok_or_else(|| {
                error!(
                    "Device {} has no id, can't delete polling token",
                    device.wireguard_pubkey
                );
                Status::internal("unexpected error")
            })?,
        )
        .await
        .map_err(|err| {
            error!("Failed to delete polling token: {err}");
            Status::internal(format!("unexpected error: {err}"))
        })?;
        let mut new_token = PollingToken::new(device.id.ok_or_else(|| {
            error!(
                "Device {} has no id, can't create a polling token",
                device.wireguard_pubkey
            );
            Status::internal("unexpected error")
        })?);
        new_token.save(&mut *transaction).await.map_err(|err| {
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

        Some(new_token.token)
    } else {
        None
    };

    info!(
        "User {}({:?}) device {}({:?}) config fetched",
        user.username, user.id, device.name, device.id,
    );

    Ok(DeviceConfigResponse {
        device: Some(device.into()),
        configs,
        instance: Some(InstanceInfo::new(settings, &user.username, enterprise_settings).into()),
        token,
    })
}
