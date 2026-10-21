use std::{collections::HashMap, net::IpAddr, str::FromStr};

use defguard_common::{
    csv::AsCsv,
    db::{
        Id,
        models::{
            Device, DeviceType, Settings, User, WireguardNetwork,
            device::{DeviceConfig, WireguardNetworkDevice},
            mfa_flow::MfaFlowStep,
            vpn_client_session::VpnClientMfaMethod,
            wireguard::{LocationMfaMode, ServiceLocationMode},
        },
    },
};
use defguard_proto::{
    client_types::{
        DeviceConfig as ProtoDeviceConfig, DeviceConfigResponse,
        LocationMfaMode as ProtoLocationMfaMode, MfaMethod, MfaStep, MfaStepMethod,
    },
    proxy::DeviceInfo,
};
use sqlx::PgPool;
use tonic::Status;

use super::InstanceInfo;
use crate::{
    device_access::build_device_config,
    enterprise::db::models::openid_provider::OpenIdProvider,
    grpc::{
        client_version::{ClientFeature, should_omit_location_for_device},
        should_prevent_mfa_location_usage, should_prevent_service_location_usage,
    },
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
    let instance_info =
        InstanceInfo::build(pool, &settings, &user, openid_provider, Some(device.id))
            .await
            .map_err(|err| {
                error!("Failed to build instance info: {err}");
                Status::internal(format!("unexpected error: {err}"))
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
                        "Failed to fetch network for WireGuard network device {}: {err}",
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

            let mut conn = pool.acquire().await.map_err(|err| {
                error!("Failed to acquire connection: {err}");
                Status::internal(format!("unexpected error: {err}"))
            })?;

            let device_config =
                build_device_config(&mut conn, &location, &wireguard_network_device, &user)
                    .await
                    .map_err(|err| {
                        error!("Failed to build device config: {err}");
                        Status::internal(format!("unexpected error: {err}"))
                    })?;

            if device_config.mfa_enabled {
                error!(
                    "Network device {} tried to fetch config for location {} with MFA enabled, which is unsupported.",
                    device.name, location.name
                );
                return Err(Status::failed_precondition(
                    "network devices cannot connect to locations with MFA enabled",
                ));
            }

            let config =
                to_wire_device_config(device_config, &instance_info.configured_methods, false)?;
            configs.push(config);
        }
    } else {
        let mut assignments = WireguardNetworkDevice::find_by_device(pool, device.id)
            .await
            .map_err(|err| {
                error!("Failed to fetch networks for device {}: {err}", device.id);
                Status::internal(format!("unexpected error: {err}"))
            })?
            .unwrap_or_default()
            .into_iter()
            .map(|assignment| (assignment.wireguard_network_id, assignment))
            .collect::<HashMap<_, _>>();
        let locations = WireguardNetwork::all(pool).await.map_err(|err| {
            error!("Failed to fetch all networks: {err}");
            Status::internal(format!("unexpected error: {err}"))
        })?;
        let supports_multi_step_mfa =
            ClientFeature::MultiStepMfa.is_supported_by_device(device_info.as_ref());
        for location in locations {
            let Some(wireguard_network_device) = assignments.remove(&location.id) else {
                continue;
            };
            if should_prevent_service_location_usage(&location) {
                warn!(
                    "Tried to use service location {} with disabled enterprise features.",
                    location.name
                );
                continue;
            }
            if should_prevent_mfa_location_usage(pool, &location)
                .await
                .map_err(|err| {
                    error!(
                        "Failed to read the MFA policy of location {}: {err}",
                        location.name
                    );
                    Status::internal(format!("unexpected error: {err}"))
                })?
            {
                warn!(
                    "Tried to use location {} whose MFA policy is above the active licence.",
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
            let mut conn = pool.acquire().await.map_err(|err| {
                error!("Failed to acquire connection: {err}");
                Status::internal(format!("unexpected error: {err}"))
            })?;

            let device_config =
                build_device_config(&mut conn, &location, &wireguard_network_device, &user)
                    .await
                    .map_err(|err| {
                        error!("Failed to build device config: {err}");
                        Status::internal(format!("unexpected error: {err}"))
                    })?;

            if device_config.posture_check_required
                && !ClientFeature::PostureChecks.is_supported_by_device(device_info.as_ref())
            {
                info!(
                    "Device {} does not support posture checks feature, skipping sending network {} configuration to device {}.",
                    device.name, location.name, device.name
                );
                continue;
            }

            if should_omit_location_for_device(
                device_config.location_mfa_mode.clone(),
                device_info.as_ref(),
            ) {
                info!(
                    "Device {} does not support multi-step MFA, skipping sending network {} configuration to device {}.",
                    device.name, location.name, device.name
                );
                continue;
            }

            let config = to_wire_device_config(
                device_config,
                &instance_info.configured_methods,
                supports_multi_step_mfa,
            )?;
            configs.push(config);
        }
    }

    info!(
        "User {}({}) device {}({}) automatically fetched the newest configuration.",
        user.username, user.id, device.name, device.id
    );

    Ok(DeviceConfigResponse {
        device: Some(device.into()),
        configs,
        instance: Some(instance_info.into()),
        token,
    })
}

/// Maps resolved MFA flow steps to wire steps using the methods configured for this device.
#[must_use]
pub fn build_wire_steps(
    steps: &[MfaFlowStep<Id>],
    configured_methods: &[VpnClientMfaMethod],
) -> Vec<MfaStep> {
    let mut wire_steps = Vec::with_capacity(steps.len());
    for step in steps {
        let mut methods = Vec::with_capacity(step.methods.len());
        for method in VpnClientMfaMethod::ordered_set(&step.methods) {
            methods.push(MfaStepMethod {
                method: <VpnClientMfaMethod as Into<MfaMethod>>::into(method) as i32,
                configured: configured_methods.contains(&method),
            });
        }
        wire_steps.push(MfaStep { methods });
    }
    wire_steps
}

/// Computes the wire `steps` for a location, applying per-client capability branching and the
/// fail-closed empty-flow guard. Returns the resolved flow for clients that support multi-step MFA
/// and an empty list for legacy clients. Rejects an MFA-enabled location that has no resolvable
/// flow, so it is never advertised to a capable client as `steps = []`.
pub fn wire_steps_for_device(
    location_mfa_mode_is_none: bool,
    resolved_steps: &[MfaFlowStep<Id>],
    network_name: &str,
    configured_methods: &[VpnClientMfaMethod],
    supports_multi_step_mfa: bool,
) -> Result<Vec<MfaStep>, Status> {
    if location_mfa_mode_is_none && resolved_steps.is_empty() {
        return Err(Status::failed_precondition(format!(
            "location {network_name} has MFA enabled but no MFA flow is configured"
        )));
    }
    if supports_multi_step_mfa {
        Ok(build_wire_steps(resolved_steps, configured_methods))
    } else {
        Ok(Vec::new())
    }
}

/// Builds a wire `DeviceConfig` from the resolved internal config, applying per-client
/// capability branching and the fail-closed empty-flow guard.
pub fn to_wire_device_config(
    device_config: DeviceConfig,
    configured_methods: &[VpnClientMfaMethod],
    supports_multi_step_mfa: bool,
) -> Result<ProtoDeviceConfig, Status> {
    let steps = wire_steps_for_device(
        device_config.location_mfa_mode.is_none(),
        &device_config.steps,
        &device_config.network_name,
        configured_methods,
        supports_multi_step_mfa,
    )?;

    Ok(ProtoDeviceConfig {
        config: device_config.config,
        network_id: device_config.network_id,
        network_name: device_config.network_name,
        assigned_ip: device_config.address.as_csv(),
        endpoint: device_config.endpoint,
        pubkey: device_config.pubkey,
        allowed_ips: device_config.allowed_ips.as_csv(),
        dns: device_config.dns,
        keepalive_interval: device_config.keepalive_interval,
        mtu: device_config.mtu.map(i32::cast_unsigned),
        #[allow(deprecated)]
        mfa_enabled: device_config.mfa_enabled,
        #[allow(deprecated)]
        location_mfa_mode: device_config
            .location_mfa_mode
            .map(|mode| <LocationMfaMode as Into<ProtoLocationMfaMode>>::into(mode).into()),
        service_location_mode: Some(
            <ServiceLocationMode as Into<defguard_proto::client_types::ServiceLocationMode>>::into(
                device_config.service_location_mode,
            )
            .into(),
        ),
        posture_check_required: Some(device_config.posture_check_required),
        steps,
    })
}

/// Parses `DeviceInfo` returning client IP address and user agent.
pub fn parse_client_ip_agent(info: &Option<DeviceInfo>) -> Result<(IpAddr, String), String> {
    let Some(info) = info else {
        error!("Missing DeviceInfo in proxy request");
        return Err("missing device info".to_owned());
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

#[cfg(test)]
mod tests;
