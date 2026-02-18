use std::{net::IpAddr, str::FromStr};

use defguard_common::{
    csv::AsCsv,
    db::{
        Id,
        models::{
            Device, DeviceType, Settings, User, WireguardNetwork,
            device::WireguardNetworkDevice,
            wireguard::{LocationMfaMode, ServiceLocationMode},
        },
    },
};
use defguard_proto::proxy::{
    DeviceConfig as ProtoDeviceConfig, DeviceConfigResponse, DeviceInfo,
    LocationMfaMode as ProtoLocationMfaMode,
};
use sqlx::PgPool;
use tonic::Status;

use super::InstanceInfo;
use crate::{
    enterprise::db::models::{
        enterprise_settings::EnterpriseSettings, openid_provider::OpenIdProvider,
    },
    grpc::{client_version::ClientFeature, should_prevent_service_location_usage},
};

pub async fn build_device_config_response(
    pool: &PgPool,
    device: Device<Id>,
    token: Option<String>,
    device_info: Option<DeviceInfo>,
) -> Result<DeviceConfigResponse, Status> {
    let settings = Settings::get_current_settings();

    let openid_provider = OpenIdProvider::get_current(pool).await.map_err(|err| {
        error!("Failed to get OpenID provider: {err}");
        Status::internal(format!("unexpected error: {err}"))
    })?;

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

            if network.service_location_mode != ServiceLocationMode::Disabled {
                error!(
                    "Network device {} tried to fetch config for service location {}, which is unsupported.",
                    device.name, network.name
                );
                return Err(Status::permission_denied(
                    "service location mode is not available for network devices",
                ));
            }

            // DEPRECATED(1.5): superseeded by location_mfa_mode
            let mfa_enabled = network.location_mfa_mode == LocationMfaMode::Internal;
            let config =
                ProtoDeviceConfig {
                    config: Device::create_config(&network, &wireguard_network_device),
                    network_id: network.id,
                    network_name: network.name,
                    assigned_ip: wireguard_network_device.wireguard_ips.as_csv(),
                    endpoint: format!("{}:{}", network.endpoint, network.port),
                    pubkey: network.pubkey,
                    allowed_ips: network.allowed_ips.as_csv(),
                    dns: network.dns,
                    keepalive_interval: network.keepalive_interval,
                    #[allow(deprecated)]
                    mfa_enabled,
                    location_mfa_mode: Some(
                        <LocationMfaMode as Into<ProtoLocationMfaMode>>::into(
                            network.location_mfa_mode,
                        )
                        .into(),
                    ),
                    service_location_mode:
                        Some(
                            <ServiceLocationMode as Into<
                                defguard_proto::proxy::ServiceLocationMode,
                            >>::into(network.service_location_mode)
                            .into(),
                        ),
                };
            configs.push(config);
        }
    } else {
        for network in networks {
            let wireguard_network_device = WireguardNetworkDevice::find(
                pool, device.id, network.id,
            )
            .await
            .map_err(|err| {
                error!(
                    "Failed to fetch WireGuard network device for device {} and network {}: {err}",
                    device.id, network.id
                );
                Status::internal(format!("unexpected error: {err}"))
            })?;
            if should_prevent_service_location_usage(&network) {
                warn!(
                    "Tried to use service location {} with disabled enterprise features.",
                    network.name
                );
                continue;
            }
            if network.service_location_mode != ServiceLocationMode::Disabled
                && !ClientFeature::ServiceLocations.is_supported_by_device(device_info.as_ref())
            {
                info!(
                    "Device {} does not support service locations feature, skipping sending network {} configuration to device {}.",
                    device.name, network.name, device.name
                );
                continue;
            }
            // DEPRECATED(1.5): superseeded by location_mfa_mode
            let mfa_enabled = network.location_mfa_mode == LocationMfaMode::Internal;
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
                    keepalive_interval: network.keepalive_interval,
                    #[allow(deprecated)]
                    mfa_enabled,
                    location_mfa_mode: Some(
                        <LocationMfaMode as Into<ProtoLocationMfaMode>>::into(
                            network.location_mfa_mode,
                        )
                        .into(),
                    ),
                    service_location_mode:
                        Some(
                            <ServiceLocationMode as Into<
                                defguard_proto::proxy::ServiceLocationMode,
                            >>::into(network.service_location_mode)
                            .into(),
                        ),
                };
                configs.push(config);
            }
        }
    }

    info!(
        "User {}({}) device {}({}) automatically fetched the newest configuration.",
        user.username, user.id, device.name, device.id
    );

    let instance_info = InstanceInfo::new(
        settings,
        &user.username,
        &enterprise_settings,
        openid_provider,
    )
    .map_err(|err| {
        error!("Failed to build instance info: {err}");
        Status::internal(format!("unexpected error: {err}"))
    })?;

    Ok(DeviceConfigResponse {
        device: Some(device.into()),
        configs,
        instance: Some(instance_info.into()),
        token,
    })
}

/// Parses `DeviceInfo` returning client IP address and user agent.
pub fn parse_client_ip_agent(info: &Option<DeviceInfo>) -> Result<(IpAddr, String), String> {
    let Some(info) = info else {
        error!("Missing DeviceInfo in proxy request");
        return Err("missing device info".to_string());
    };

    let ip = IpAddr::from_str(&info.ip_address).map_err(|_| {
        let msg = format!("invalid IP address: {}", info.ip_address);
        error!(msg);
        msg
    })?;
    let user_agent = info.user_agent.clone().unwrap_or_else(String::new);
    let escaped_agent = tera::escape_html(&user_agent);

    Ok((ip, escaped_agent))
}
