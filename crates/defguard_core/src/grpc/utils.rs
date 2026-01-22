use std::{net::IpAddr, str::FromStr};

use defguard_common::{
    csv::AsCsv,
    db::{Id, models::Settings},
};
use defguard_proto::proxy::{
    DeviceConfig as ProtoDeviceConfig, DeviceConfigResponse, DeviceInfo,
    LocationMfaMode as ProtoLocationMfaMode,
};
use sqlx::PgPool;
use tonic::Status;

use super::InstanceInfo;
use crate::{
    db::{
        Device, User,
        models::{
            device::{DeviceType, WireguardNetworkDevice},
            polling_token::PollingToken,
            wireguard::{
                LocationMfaMode, ServiceLocationMode, WireguardNetwork, get_allowed_ips_for_device,
            },
        },
    },
    enterprise::db::models::{
        enterprise_settings::EnterpriseSettings, openid_provider::OpenIdProvider,
    },
    grpc::client_version::ClientFeature,
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
    device_info: Option<DeviceInfo>,
) -> Result<DeviceConfigResponse, Status> {
    let settings = Settings::get_current_settings();

    let openid_provider = OpenIdProvider::get_current(pool).await.map_err(|err| {
        error!("Failed to get OpenID provider: {err}");
        Status::internal(format!("unexpected error: {err}"))
    })?;

    let locations = WireguardNetwork::all(pool).await.map_err(|err| {
        error!("Failed to fetch all locations: {err}");
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
            let location = wireguard_network_device
                .network(pool)
                .await
                .map_err(|err| {
                    error!(
                        "Failed to fetch location for WireGuard network device {}: {err}",
                        device.name
                    );
                    Status::internal(format!("unexpected error: {err}"))
                })?;

            if location.service_location_mode != ServiceLocationMode::Disabled {
                error!(
                    "Network device {} tried to fetch config for service location {}, which is unsupported.",
                    device.name, location.name
                );
                return Err(Status::permission_denied(
                    "service location mode is not available for network devices",
                ));
            }

            // DEPRECATED(1.5): superseeded by location_mfa_mode
            let mfa_enabled = location.location_mfa_mode == LocationMfaMode::Internal;
            let allowed_ips = get_allowed_ips_for_device(&enterprise_settings, &location).as_csv();
            let config =
                ProtoDeviceConfig {
                    config: Device::create_config(
                        &location,
                        &wireguard_network_device,
                        &enterprise_settings,
                    ),
                    network_id: location.id,
                    network_name: location.name,
                    assigned_ip: wireguard_network_device.wireguard_ips.as_csv(),
                    endpoint: format!("{}:{}", location.endpoint, location.port),
                    pubkey: location.pubkey,
                    allowed_ips,
                    dns: location.dns,
                    keepalive_interval: location.keepalive_interval,
                    #[allow(deprecated)]
                    mfa_enabled,
                    location_mfa_mode: Some(
                        <LocationMfaMode as Into<ProtoLocationMfaMode>>::into(
                            location.location_mfa_mode,
                        )
                        .into(),
                    ),
                    service_location_mode:
                        Some(
                            <ServiceLocationMode as Into<
                                defguard_proto::proxy::ServiceLocationMode,
                            >>::into(location.service_location_mode)
                            .into(),
                        ),
                };
            configs.push(config);
        }
    } else {
        for location in locations {
            let wireguard_network_device = WireguardNetworkDevice::find(
                pool,
                device.id,
                location.id,
            )
            .await
            .map_err(|err| {
                error!(
                    "Failed to fetch WireGuard network device for device {} and network {}: {err}",
                    device.id, location.id
                );
                Status::internal(format!("unexpected error: {err}"))
            })?;
            if location.should_prevent_service_location_usage() {
                warn!(
                    "Tried to use service location {} with disabled enterprise features.",
                    location.name
                );
                continue;
            }
            if location.service_location_mode != ServiceLocationMode::Disabled
                && !ClientFeature::ServiceLocations.is_supported_by_device(device_info.as_ref())
            {
                info!(
                    "Device {} does not support service locations feature, skipping sending network {} configuration to device {}.",
                    device.name, location.name, device.name
                );
                continue;
            }
            // DEPRECATED(1.5): superseeded by location_mfa_mode
            let mfa_enabled = location.location_mfa_mode == LocationMfaMode::Internal;
            let allowed_ips = get_allowed_ips_for_device(&enterprise_settings, &location).as_csv();
            if let Some(wireguard_network_device) = wireguard_network_device {
                let config = ProtoDeviceConfig {
                    config: Device::create_config(
                        &location,
                        &wireguard_network_device,
                        &enterprise_settings,
                    ),
                    network_id: location.id,
                    network_name: location.name,
                    assigned_ip: wireguard_network_device.wireguard_ips.as_csv(),
                    endpoint: format!("{}:{}", location.endpoint, location.port),
                    pubkey: location.pubkey,
                    allowed_ips,
                    dns: location.dns,
                    keepalive_interval: location.keepalive_interval,
                    #[allow(deprecated)]
                    mfa_enabled,
                    location_mfa_mode: Some(
                        <LocationMfaMode as Into<ProtoLocationMfaMode>>::into(
                            location.location_mfa_mode,
                        )
                        .into(),
                    ),
                    service_location_mode:
                        Some(
                            <ServiceLocationMode as Into<
                                defguard_proto::proxy::ServiceLocationMode,
                            >>::into(location.service_location_mode)
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

    Ok(DeviceConfigResponse {
        device: Some(device.into()),
        configs,
        instance: Some(
            InstanceInfo::new(
                settings,
                &user.username,
                &enterprise_settings,
                openid_provider,
            )
            .into(),
        ),
        token,
    })
}

/// Parses `DeviceInfo` returning client IP address and user agent.
pub(crate) fn parse_client_ip_agent(info: &Option<DeviceInfo>) -> Result<(IpAddr, String), String> {
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
