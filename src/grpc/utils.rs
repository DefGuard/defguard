use super::{proto::InstanceConfigResponse, InstanceInfo};
use ipnetwork::IpNetwork;
use tonic::Status;

use super::proto::{DeviceConfig as ProtoDeviceConfig, DeviceConfigResponse};
use crate::{
    db::{
        models::{device::WireguardNetworkDevice, wireguard::WireguardNetwork},
        DbPool, Device, Settings, User,
    },
    enterprise::{
        db::models::enterprise_settings::EnterpriseSettings,
        license::{get_cached_license, validate_license},
    },
};

pub(crate) async fn build_device_config_response(
    pool: &DbPool,
    pubkey: &str,
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

    info!("Device {} configs fetched", device.name);

    Ok(DeviceConfigResponse {
        device: Some(device.into()),
        configs,
        instance: Some(InstanceInfo::new(settings, &user.username).into()),
        token: None,
    })
}

pub(crate) async fn build_instance_config_response(
    pool: &DbPool,
) -> Result<InstanceConfigResponse, Status> {
    debug!("Building instance config response");
    let enterprise = validate_license(get_cached_license().as_ref()).is_ok();
    let enterprise_settings = EnterpriseSettings::get(pool).await.map_err(|_| {
        error!("Failed to get enterprise settings while building instance config response");
        Status::internal("unexpected error")
    })?;
    debug!("Instance config response built");

    Ok(InstanceConfigResponse {
        enterprise,
        disable_route_all_traffic: enterprise_settings.disable_all_traffic,
    })
}
