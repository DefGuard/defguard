use std::{
    net::{AddrParseError, IpAddr},
    str::FromStr,
};

use axum::{
    extract::{Json, Path, Query, State},
    http::StatusCode,
};
use chrono::NaiveDateTime;
use defguard_common::{
    csv::AsCsv,
    db::{
        Id,
        models::{
            Device, DeviceConfig, DeviceType, Settings, User, WireguardNetwork,
            device::{DeviceInfo, WireguardNetworkDevice},
            wireguard::{LocationMfaMode, NetworkAddressError},
        },
    },
    utils::{SplitIp, split_ip},
};
use serde_json::json;
use sqlx::PgConnection;
use utoipa::ToSchema;

use super::{ApiErrorResponse, ApiResponse, ApiResult, WebError};
use crate::{
    appstate::AppState,
    auth::{AdminRole, SessionInfo},
    device_access::{build_device_config, join_device_to_network},
    enrollment_management::start_desktop_configuration,
    enterprise::{
        db::models::enterprise_settings::EnterpriseSettings,
        firewall::try_get_location_firewall_config, limits::update_counts,
    },
    events::{ApiEvent, ApiEventType, ApiRequestContext},
    grpc::GatewayCommand,
    handlers::{
        device_for_admin_or_self,
        pagination::{PaginatedApiResponse, PaginatedApiResult, PaginationParams},
    },
    mail::templates::{TemplateLocation, new_device_added_mail},
};

#[derive(Serialize, ToSchema)]
struct NetworkDeviceLocation {
    id: Id,
    name: String,
}

#[derive(Serialize, ToSchema)]
pub(crate) struct NetworkDeviceInfo {
    id: Id,
    name: String,
    #[schema(value_type = Vec<String>)]
    assigned_ips: Vec<IpAddr>,
    description: Option<String>,
    added_by: String,
    added_date: NaiveDateTime,
    location: NetworkDeviceLocation,
    wireguard_pubkey: String,
    configured: bool,
    split_ips: Vec<SplitIp>,
}

impl NetworkDeviceInfo {
    async fn from_device(
        device: Device<Id>,
        transaction: &mut PgConnection,
    ) -> Result<Self, WebError> {
        let network = WireguardNetwork::find_network_device_networks(&mut *transaction, device.id)
            .await?
            .pop()
            .ok_or(WebError::ObjectNotFound(format!(
                "Failed to find the network with which the network device {} is associated",
                device.name
            )))?;
        let wireguard_device =
            WireguardNetworkDevice::find(&mut *transaction, device.id, network.id)
                .await?
                .ok_or(WebError::ObjectNotFound(format!(
                    "Failed to find network device {} network information in network {}",
                    device.name, network.name
                )))?;
        let added_by = device.get_owner(&mut *transaction).await?;
        let split_ips = wireguard_device
            .wireguard_ips
            .iter()
            .copied()
            .map(|ip| {
                network
                    .get_containing_network(ip)
                    .map(|net_addr| split_ip(&ip, &net_addr))
                    .ok_or_else(|| {
                        WebError::ObjectNotFound(format!(
                            "Failed to find the network address for network {}",
                            network.name
                        ))
                    })
            })
            .collect::<Result<_, _>>()?;
        Ok(Self {
            id: device.id,
            name: device.name,
            assigned_ips: wireguard_device.wireguard_ips,
            description: device.description,
            added_by: added_by.username,
            added_date: device.created,
            wireguard_pubkey: device.wireguard_pubkey,
            location: NetworkDeviceLocation {
                id: wireguard_device.wireguard_network_id,
                name: network.name,
            },
            configured: device.configured,
            split_ips,
        })
    }
}

#[derive(Serialize)]
pub(crate) struct DeviceWireGuardConfig {
    pub(crate) network_id: Id,
    pub(crate) network_name: String,
    pub(crate) config: String,
    /// Authoritative flag for whether the location requires MFA.
    pub(crate) mfa_enabled: bool,
    /// Legacy derived mode. Absent when the location's MFA flow configuration has no legacy
    /// equivalent, which includes every location with no flows, so it must not be used to infer
    /// whether MFA is required.
    pub(crate) location_mfa_mode: Option<LocationMfaMode>,
    pub(crate) posture_check_required: bool,
}

/// Get the WireGuard configuration of a network device
///
/// Returns one configuration per location the device belongs to.
#[utoipa::path(
    get,
    path = "/api/v1/device/network/{device_id}/config",
    tag = "network device",
    params(
        ("device_id" = i64, Path, description = "ID of the network device."),
    ),
    responses(
        (status = 200, description = "Network device configuration for each location of the device.", body = [Object], example = json!([
            {"network_id": 1, "network_name": "office", "config": "[Interface]\n...", "mfa_enabled": false, "posture_check_required": false}
        ])),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges or the request must target your own account.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 404, description = "Network device not found.", body = ApiErrorResponse, example = json!({"msg": "device not found"})),
        (status = 500, description = "Unable to get network device configuration.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn network_device_configs(
    session: SessionInfo,
    State(appstate): State<AppState>,
    Path(device_id): Path<Id>,
) -> ApiResult {
    debug!("Creating a WireGuard config for network device {device_id}.");

    let settings = EnterpriseSettings::get(&appstate.pool).await?;
    if settings.only_client_activation && !session.is_admin {
        warn!(
            "User {} tried to download device config, but manual device management is disabled",
            session.user.username
        );
        return Err(WebError::Forbidden("Manual device management is disabled"));
    }

    let device = device_for_admin_or_self(&appstate.pool, &session, device_id).await?;
    let user = User::find_by_id(&appstate.pool, device.user_id)
        .await?
        .ok_or(WebError::ObjectNotFound(format!(
            "User {} not found",
            device.user_id
        )))?;
    let networks =
        WireguardNetwork::find_network_device_networks(&appstate.pool, device_id).await?;

    let mut result = Vec::new();
    for network in networks {
        let network_device = WireguardNetworkDevice::find(&appstate.pool, device_id, network.id)
            .await?
            .ok_or(WebError::ObjectNotFound(format!(
                "No IP address found for device: {}({})",
                device.name, device.id
            )))?;
        debug!(
            "Created a WireGuard config for network device {device_id} in network {}.",
            network.name
        );
        let mut conn = appstate.pool.acquire().await?;
        let device_config =
            build_device_config(&mut conn, &network, &network_device, &user).await?;
        let device_config = DeviceWireGuardConfig {
            network_id: device_config.network_id,
            network_name: device_config.network_name,
            config: device_config.config,
            mfa_enabled: device_config.mfa_enabled,
            location_mfa_mode: device_config.location_mfa_mode,
            posture_check_required: device_config.posture_check_required,
        };
        result.push(device_config);
    }

    Ok(ApiResponse::json(result, StatusCode::OK))
}

/// Get a network device
#[utoipa::path(
    get,
    path = "/api/v1/device/network/{device_id}",
    tag = "network device",
    params(
        ("device_id" = i64, Path, description = "ID of the network device."),
    ),
    responses(
        (status = 200, description = "Network device details.", body = NetworkDeviceInfo),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 404, description = "Network device not found.", body = ApiErrorResponse, example = json!({"msg": "device not found"})),
        (status = 500, description = "Unable to get network device.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn get_network_device(
    _admin_role: AdminRole,
    session: SessionInfo,
    Path(device_id): Path<Id>,
    State(appstate): State<AppState>,
) -> ApiResult {
    debug!(
        "User {} is retrieving network device with id: {device_id}",
        session.user.username
    );

    let device = Device::find_by_id(&appstate.pool, device_id).await?;
    if let Some(device) = device
        && device.device_type == DeviceType::Network
    {
        let mut transaction = appstate.pool.begin().await?;
        let network_device_info = NetworkDeviceInfo::from_device(device, &mut transaction).await?;
        transaction.commit().await?;
        return Ok(ApiResponse::json(network_device_info, StatusCode::OK));
    }
    error!(
        "Failed to retrieve network device with id: {device_id}, such network device doesn't exist."
    );
    Err(WebError::ObjectNotFound(format!(
        "Network device with ID {device_id} not found"
    )))
}

/// List network devices
#[utoipa::path(
    get,
    path = "/api/v1/device/network",
    tag = "network device",
    params(
        ("page" = Option<u32>, Query, description = "Page number. Defaults to 1."),
        ("per_page" = Option<u32>, Query, description = "Number of items per page, from 1 to 100. Defaults to 50."),
    ),
    responses(
        (status = 200, description = "Paginated list of network devices.", body = PaginatedApiResponse<NetworkDeviceInfo>),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 500, description = "Unable to list network devices.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn list_network_devices(
    _admin_role: AdminRole,
    State(appstate): State<AppState>,
    pagination: Query<PaginationParams>,
) -> PaginatedApiResult<NetworkDeviceInfo> {
    let pagination = pagination.0;

    debug!("Listing network devices");

    let mut devices_response = Vec::new();
    let mut transaction = appstate.pool.begin().await?;
    let devices = Device::find_by_type_paginated(
        &mut *transaction,
        DeviceType::Network,
        i64::from(pagination.per_page()),
        i64::from(pagination.offset()),
    )
    .await?;
    for device in devices {
        match NetworkDeviceInfo::from_device(device, &mut transaction).await {
            Ok(device_info) => {
                devices_response.push(device_info);
            }
            Err(err) => {
                error!(
                    "Failed to get network information for network device. This device will not be
                    displayed. Error details: {err}"
                );
            }
        }
    }
    let count = Device::count_by_type(&mut *transaction, DeviceType::Network).await?;
    transaction.commit().await?;

    info!("Listed {} network devices", devices_response.len());

    Ok(PaginatedApiResponse::new(
        devices_response,
        pagination,
        count as u32,
    ))
}

#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct AddNetworkDevice {
    pub name: String,
    pub description: Option<String>,
    pub location_id: i64,
    pub assigned_ips: Vec<String>,
    pub wireguard_pubkey: String,
}

#[derive(Serialize)]
pub struct AddNetworkDeviceResult {
    config: DeviceConfig,
    device: NetworkDeviceInfo,
}

#[derive(Deserialize, ToSchema)]
pub struct IpAvailabilityCheck {
    ips: Vec<String>,
    device_id: Option<Id>,
}

#[derive(Serialize)]
pub struct IpAvailabilityCheckResult {
    available: bool,
    valid: bool,
}

impl IpAvailabilityCheckResult {
    #[must_use]
    pub fn new(available: bool, valid: bool) -> Self {
        Self { available, valid }
    }
}

/// Check whether the given IP addresses are free in a location
#[utoipa::path(
    post,
    path = "/api/v1/device/network/ip/{network_id}",
    tag = "network device",
    request_body = IpAvailabilityCheck,
    params(
        ("network_id" = i64, Path, description = "ID of the network."),
    ),
    responses(
        (status = 200, description = "Availability of the requested IP addresses.", body = [Object], example = json!([{"available": true, "valid": true}])),
        (status = 400, description = "Location not found.", body = ApiErrorResponse, example = json!({"msg": "Failed to check IP availability, location not found"})),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 500, description = "Unable to check IP availability.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn check_ip_availability(
    _admin_role: AdminRole,
    Path(network_id): Path<Id>,
    State(appstate): State<AppState>,
    Json(check): Json<IpAvailabilityCheck>,
) -> ApiResult {
    let mut transaction = appstate.pool.begin().await?;

    // fetch relevant WireGuard location
    let location = WireguardNetwork::find_by_id(&appstate.pool, network_id)
        .await?
        .ok_or_else(|| {
            error!(
                "Failed to check IP availability for location with ID {network_id}, location not \
                found",
            );
            WebError::BadRequest("Failed to check IP availability, location not found".into())
        })?;

    // process IPs one by one and preserve order in response
    let mut validation_results = Vec::new();
    for ip in &check.ips {
        match IpAddr::from_str(ip) {
            Ok(ip) => {
                debug!(
                    "Checking if IP address {ip} can be assigned to a device in location {location}"
                );
                let result = match location
                    .can_assign_ips(&mut transaction, &[ip], check.device_id)
                    .await
                {
                    Ok(()) => IpAvailabilityCheckResult::new(true, true),
                    Err(NetworkAddressError::NoContainingNetwork(name, ip, networks)) => {
                        warn!(
                            "Provided device IP address {ip} is not in the network {name} range: \
                            {networks:?}"
                        );
                        IpAvailabilityCheckResult::new(false, false)
                    }
                    Err(NetworkAddressError::ReservedForGateway(name, ip)) => {
                        warn!(
                            "Provided device IP address {ip} may overlap with the gateway's IP \
                            address on network {name}",
                        );
                        IpAvailabilityCheckResult::new(false, true)
                    }
                    Err(NetworkAddressError::IsBroadcastAddress(name, ip)) => {
                        warn!(
                            "Provided device IP address {ip} is broadcast address of network {name}"
                        );
                        IpAvailabilityCheckResult::new(false, true)
                    }
                    Err(NetworkAddressError::IsNetworkAddress(name, ip)) => {
                        warn!(
                            "Provided device IP address {ip} is network address of network {name}"
                        );
                        IpAvailabilityCheckResult::new(false, true)
                    }
                    Err(NetworkAddressError::AddressAlreadyAssigned(name, ip)) => {
                        warn!("Provided device IP {ip} is already assigned in network {name}");
                        IpAvailabilityCheckResult::new(false, true)
                    }
                    Err(NetworkAddressError::DbError(err)) => Err(err)?,
                };
                validation_results.push(result);
            }
            Err(_err) => {
                warn!(
                    "Failed to check IP availability for location {location}, invalid IP address \
                    {ip}",
                );
                validation_results.push(IpAvailabilityCheckResult {
                    available: false,
                    valid: false,
                });
            }
        }
    }

    Ok(ApiResponse::json(validation_results, StatusCode::OK))
}

/// Suggest free IP addresses in a location
#[utoipa::path(
    get,
    path = "/api/v1/device/network/ip/{network_id}",
    tag = "network device",
    params(
        ("network_id" = i64, Path, description = "ID of the network."),
    ),
    responses(
        (status = 200, description = "Suggested IP addresses.", body = [Object], example = json!([
            {"network_part": "10.0.0.", "modifiable_part": "15", "network_prefix": "/24", "ip": "10.0.0.15"}
        ])),
        (status = 400, description = "Location not found.", body = ApiErrorResponse, example = json!({"msg": "Failed to find available IP, network not found"})),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 500, description = "Unable to find available IP addresses.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn find_available_ips(
    _admin_role: AdminRole,
    Path(network_id): Path<Id>,
    State(appstate): State<AppState>,
) -> ApiResult {
    let network = WireguardNetwork::find_by_id(&appstate.pool, network_id)
        .await?
        .ok_or_else(|| {
            error!(
                "Failed to find available IP for network with ID {}",
                network_id
            );
            WebError::BadRequest("Failed to find available IP, network not found".to_owned())
        })?;

    let mut transaction = appstate.pool.begin().await?;
    let mut split_ips = Vec::new();
    for network_address in network.address() {
        let net_ip = network_address.ip();
        let net_network = network_address.network();
        let net_broadcast = network_address.broadcast();
        for ip in network_address {
            if ip == net_ip || ip == net_network || ip == net_broadcast {
                continue;
            }

            // Break the loop if IP is unassigned and return network device
            if Device::find_by_ip(&mut *transaction, ip, network.id)
                .await?
                .is_none()
            {
                split_ips.push(split_ip(&ip, network_address));
                break;
            }
        }
    }

    transaction.commit().await?;
    if split_ips.len() != network.address().len() {
        warn!(
            "Failed to find available IPs for new device in network {} ({:?})",
            network.name,
            network.address()
        );
        return Err(WebError::NetworkFull(format!(
            "Network {} is full, no IP addresses available",
            network.name
        )));
    }
    debug!(
        "Found addresses {:?} for new device in network {} ({:?})",
        split_ips,
        network.name,
        network.address()
    );
    Ok(ApiResponse::json(split_ips, StatusCode::OK))
}

#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct StartNetworkDeviceSetup {
    name: String,
    description: Option<String>,
    location_id: i64,
    assigned_ips: Vec<String>,
}

impl From<NetworkAddressError> for WebError {
    fn from(error: NetworkAddressError) -> Self {
        Self::BadRequest(error.to_string())
    }
}

// Setup a network device to be later configured by a CLI client
/// Start CLI setup for a new network device
///
/// Returns an enrollment token the `defguard-cli` client uses to configure itself.
#[utoipa::path(
    post,
    path = "/api/v1/device/network/start_cli",
    tag = "network device",
    request_body = StartNetworkDeviceSetup,
    responses(
        (status = 201, description = "Setup started. Returns the enrollment token and URL.", body = Object, example = json!({
            "enrollment_token": "yZbTsF0m9Xq7cVwPnR2Ld1Ku",
            "enrollment_url": "https://vpn.example.com/"
        })),
        (status = 400, description = "Invalid IP assignment.", body = ApiErrorResponse, example = json!({"msg": "Invalid IP address"})),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 500, description = "Unable to start network device setup.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn start_network_device_setup(
    _admin_role: AdminRole,
    session: SessionInfo,
    State(appstate): State<AppState>,
    Json(setup_start): Json<StartNetworkDeviceSetup>,
) -> ApiResult {
    let device_name = setup_start.name.clone();
    debug!(
        "User {} starting network device {device_name} setup in location with ID {}.",
        session.user.username, setup_start.location_id
    );

    let user = session.user;
    let network = WireguardNetwork::find_by_id(&appstate.pool, setup_start.location_id)
        .await?
        .ok_or_else(|| {
            error!(
                "Failed to add device {device_name}, network with ID {} not found",
                setup_start.location_id
            );
            WebError::BadRequest("Failed to add device, network not found".to_owned())
        })?;

    debug!(
        "Identified network location with ID {} as {}",
        setup_start.location_id, network.name
    );

    let mut transaction = appstate.pool.begin().await?;
    let device = Device::new(
        setup_start.name,
        "NOT_CONFIGURED".to_owned(),
        user.id,
        DeviceType::Network,
        setup_start.description,
        false,
    )
    .save(&mut *transaction)
    .await?;

    debug!(
        "Created a new unconfigured network device {device_name} with ID {}",
        device.id
    );

    let ips = setup_start
        .assigned_ips
        .iter()
        .map(|ip| IpAddr::from_str(ip))
        .collect::<Result<Vec<IpAddr>, AddrParseError>>()
        .map_err(|e| {
            let msg =
                format!("Failed to add network device {device_name}, invalid IP address: {e}");
            error!(msg);
            WebError::BadRequest(msg)
        })?;

    network.can_assign_ips(&mut transaction, &ips, None).await?;

    let (_, config) =
        join_device_to_network(&mut transaction, &device, &network, &user, &ips).await?;

    info!(
        "User {} added a new unconfigured network device {device_name} with IPs {ips:?} to network \
        {}",
        user.username, network.name
    );

    let result = AddNetworkDeviceResult {
        config,
        device: NetworkDeviceInfo::from_device(device, &mut transaction).await?,
    };
    let settings = Settings::get_current_settings();
    let configuration_token = start_desktop_configuration(
        &user,
        &mut transaction,
        &user,
        None,
        settings.enrollment_token_timeout().as_secs(),
        settings.proxy_public_url()?.clone(),
        false,
        Some(result.device.id),
    )
    .await?;

    debug!(
        "Generated a new device CLI configuration token for a network device {device_name} with ID \
        {}: {configuration_token}",
        result.device.id
    );

    update_counts(&mut *transaction).await?;

    transaction.commit().await?;

    Ok(ApiResponse::new(
        json!({
            "enrollment_token": configuration_token,
            "enrollment_url":  settings.proxy_public_url()?.to_string()
        }),
        StatusCode::CREATED,
    ))
}

// Make a new CLI configuration token for an already added network device
/// Start CLI setup for an existing network device
#[utoipa::path(
    post,
    path = "/api/v1/device/network/start_cli/{device_id}",
    tag = "network device",
    params(
        ("device_id" = i64, Path, description = "ID of the network device."),
    ),
    responses(
        (status = 201, description = "Setup started. Returns the enrollment token and URL.", body = Object, example = json!({
            "enrollment_token": "yZbTsF0m9Xq7cVwPnR2Ld1Ku",
            "enrollment_url": "https://vpn.example.com/"
        })),
        (status = 400, description = "Device not found, or it is not a network device.", body = ApiErrorResponse, example = json!({"msg": "Failed to start network device setup for device with ID 1, device not found"})),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 500, description = "Unable to start network device setup.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn start_network_device_setup_for_device(
    _admin_role: AdminRole,
    session: SessionInfo,
    Path(device_id): Path<Id>,
    State(appstate): State<AppState>,
) -> ApiResult {
    debug!(
        "User {} starting network device setup for already added device with ID {}.",
        session.user.username, device_id
    );
    let device = Device::find_by_id(&appstate.pool, device_id)
        .await?
        .ok_or_else(|| {
            WebError::BadRequest(format!(
                "Failed to start network device setup for device with ID {device_id},
                device not found"
            ))
        })?;

    if device.device_type != DeviceType::Network {
        return Err(WebError::BadRequest(format!(
            "Failed to start network device setup for a choosen device {},
            device is not a network device, device type: {:?}",
            device.name, device.device_type
        )));
    }

    let mut transaction = appstate.pool.begin().await?;
    let user = User::find_by_id(&mut *transaction, device.user_id)
        .await?
        .ok_or_else(|| {
            WebError::BadRequest(format!(
                "Failed to start network device setup for device with ID {device_id},
                user which added the device not found"
            ))
        })?;
    let settings = Settings::get_current_settings();
    let configuration_token = start_desktop_configuration(
        &user,
        &mut transaction,
        &user,
        None,
        settings.enrollment_token_timeout().as_secs(),
        settings.proxy_public_url()?,
        false,
        Some(device.id),
    )
    .await?;
    transaction.commit().await?;

    debug!(
        "Generated a new device CLI configuration token for already existing network
        device {} with ID {}: {configuration_token}",
        device.name, device.id
    );
    Ok(ApiResponse::new(
        json!({
            "enrollment_token": configuration_token,
            "enrollment_url": settings.proxy_public_url()?.to_string()
        }),
        StatusCode::CREATED,
    ))
}

/// Create a network device
///
/// The device is created with the provided WireGuard public key.
#[utoipa::path(
    post,
    path = "/api/v1/device/network",
    tag = "network device",
    request_body(content = AddNetworkDevice, example = json!({"name": "office-printer", "location_id": 1, "assigned_ips": ["10.0.0.50"], "wireguard_pubkey": "xTIBA5rboUvnH4htodjb6e697QjLERt1NAB4mZqp8Dg=", "description": "Printer on the second floor"})),
    responses(
        (status = 201, description = "Network device created.", body = Object, example = json!({
            "config": {
                "network_id": 1,
                "network_name": "office",
                "config": "[Interface]\n...",
                "address": ["10.0.0.15"],
                "endpoint": "vpn.example.com:50051",
                "allowed_ips": ["10.0.0.0/24"],
                "pubkey": "Zm9vYmFyMDEyMzQ1Njc4OWFiY2RlZmdoaWprbG1ub3A=",
                "dns": "10.0.0.1",
                "keepalive_interval": 25,
                "mfa_enabled": false,
                "service_location_mode": "disabled",
                "posture_check_required": false
            },
            "device": {
                "id": 5,
                "name": "printer",
                "assigned_ips": ["10.0.0.15"],
                "description": null,
                "added_by": "admin",
                "added_date": "2026-08-04T10:15:00",
                "location": {"id": 1, "name": "office"},
                "wireguard_pubkey": "5ItSw7SLkVLXPFvNxLdEQaSMOFhLxD7YsTTAlR8CbCA=",
                "configured": true,
                "split_ips": [{"network_part": "10.0.0.", "modifiable_part": "15", "network_prefix": "/24", "ip": "10.0.0.15"}]
            }
        })),
        (status = 400, description = "Invalid public key or IP assignment.", body = ApiErrorResponse, example = json!({"msg": "Public key invalid"})),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 500, description = "Unable to create network device.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn add_network_device(
    _admin_role: AdminRole,
    session: SessionInfo,
    context: ApiRequestContext,
    State(appstate): State<AppState>,
    Json(add_network_device): Json<AddNetworkDevice>,
) -> ApiResult {
    let device_name = add_network_device.name.clone();
    debug!(
        "User {} adding network device {device_name} in location {}.",
        session.user.username, add_network_device.location_id
    );

    let user = session.user;
    let network = WireguardNetwork::find_by_id(&appstate.pool, add_network_device.location_id)
        .await?
        .ok_or_else(|| {
            error!(
                "Failed to add device {device_name}, network with ID {} not found",
                add_network_device.location_id
            );
            WebError::BadRequest("Failed to add device, network not found".to_owned())
        })?;

    Device::validate_pubkey(&add_network_device.wireguard_pubkey)
        .map_err(WebError::PubkeyValidation)?;

    // Make sure there is no device with the same pubkey, such state may lead to unexpected issues
    if Device::find_by_pubkey(&appstate.pool, &add_network_device.wireguard_pubkey)
        .await?
        .is_some()
    {
        return Err(WebError::PubkeyExists(format!(
            "Failed to add device {device_name}, identical pubkey ({}) already exists",
            add_network_device.wireguard_pubkey
        )));
    }

    let mut transaction = appstate.pool.begin().await?;
    let device = Device::new(
        add_network_device.name,
        add_network_device.wireguard_pubkey,
        user.id,
        DeviceType::Network,
        add_network_device.description,
        true,
    )
    .save(&mut *transaction)
    .await?;

    let ips = add_network_device
        .assigned_ips
        .iter()
        .map(|ip| IpAddr::from_str(ip))
        .collect::<Result<Vec<IpAddr>, AddrParseError>>()
        .map_err(|e| {
            let msg =
                format!("Failed to add network device {device_name}, invalid IP address: {e}");
            error!(msg);
            WebError::BadRequest(msg)
        })?;
    network.can_assign_ips(&mut transaction, &ips, None).await?;

    let (network_info, config) =
        join_device_to_network(&mut transaction, &device, &network, &user, &ips).await?;

    appstate.send_gateway_command(GatewayCommand::DeviceCreated(DeviceInfo {
        device: device.clone(),
        network_info: vec![network_info.clone()],
    }));

    update_counts(&mut *transaction).await?;

    // send firewall update event if ACLs & enterprise features are enabled
    if let Some(firewall_config) =
        try_get_location_firewall_config(&network, &mut transaction).await?
    {
        appstate.send_gateway_command(GatewayCommand::FirewallConfigChanged(
            network.id,
            firewall_config,
        ));
    }

    let template_locations = vec![TemplateLocation {
        name: config.network_name.clone(),
        assigned_ips: config.address.as_csv(),
    }];

    new_device_added_mail(
        &user.email,
        &mut transaction,
        &device.name,
        &device.wireguard_pubkey,
        &template_locations,
        Some(session.session.ip_address.as_str()),
        session.session.device_info.clone().as_deref(),
    )
    .await?;

    let result = AddNetworkDeviceResult {
        config,
        device: NetworkDeviceInfo::from_device(device.clone(), &mut transaction).await?,
    };

    transaction.commit().await?;

    info!(
        "User {} added a new network device {device_name}.",
        user.username
    );
    appstate.emit_event(ApiEvent {
        context,
        event: Box::new(ApiEventType::NetworkDeviceAdded {
            device,
            location: network,
        }),
    })?;

    Ok(ApiResponse::json(result, StatusCode::CREATED))
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ModifyNetworkDevice {
    name: String,
    description: Option<String>,
    #[schema(value_type = Vec<String>)]
    assigned_ips: Vec<IpAddr>,
}

/// Update a network device
#[utoipa::path(
    put,
    path = "/api/v1/device/network/{device_id}",
    tag = "network device",
    request_body = ModifyNetworkDevice,
    params(
        ("device_id" = i64, Path, description = "ID of the network device."),
    ),
    responses(
        (status = 200, description = "Network device updated.", body = NetworkDeviceInfo),
        (status = 400, description = "Invalid IP assignment.", body = ApiErrorResponse, example = json!({"msg": "Invalid IP address"})),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 404, description = "Network device not found.", body = ApiErrorResponse, example = json!({"msg": "device not found"})),
        (status = 500, description = "Unable to update network device.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub async fn modify_network_device(
    _admin_role: AdminRole,
    session: SessionInfo,
    context: ApiRequestContext,
    Path(device_id): Path<Id>,
    State(appstate): State<AppState>,
    Json(data): Json<ModifyNetworkDevice>,
) -> ApiResult {
    debug!("User {} updating device {device_id}", session.user.username);
    let mut transaction = appstate.pool.begin().await?;
    let mut device = Device::find_by_id(&mut *transaction, device_id)
        .await?
        .ok_or_else(|| {
            error!("Failed to update device {device_id}, device not found");
            WebError::ObjectNotFound(format!("Device {device_id} not found"))
        })?;
    // store device before modifications
    let before = device.clone();
    let device_network =
        WireguardNetwork::find_network_device_networks(&mut *transaction, device_id)
            .await?
            .pop()
            .ok_or_else(|| {
                error!("Failed to update device {device_id}, device not found in any network");
                WebError::ObjectNotFound(format!("Device {device_id} not found in any network"))
            })?;
    let mut wireguard_network_device =
        WireguardNetworkDevice::find(&mut *transaction, device.id, device_network.id)
            .await?
            .ok_or_else(|| {
                error!("Failed to update device {device_id}, device not found in any network");
                WebError::ObjectNotFound(format!("Device {device_id} not found in any network"))
            })?;
    device.name = data.name;
    device.description = data.description;
    device.save(&mut *transaction).await?;

    // IP address has changed, so remove device from network and add it again with new IP address.
    if data.assigned_ips != *wireguard_network_device.wireguard_ips {
        device_network
            .can_assign_ips(&mut transaction, &data.assigned_ips, Some(device.id))
            .await?;
        let old_ips = wireguard_network_device.wireguard_ips.clone();
        wireguard_network_device.wireguard_ips = data.assigned_ips;
        wireguard_network_device.update(&mut *transaction).await?;
        let device_info = DeviceInfo::from_device(&mut *transaction, device.clone()).await?;
        appstate.send_gateway_command(GatewayCommand::DeviceModified(device_info));

        // send firewall update event if ACLs are enabled
        if device_network.acl_enabled
            && let Some(firewall_config) =
                try_get_location_firewall_config(&device_network, &mut transaction).await?
        {
            appstate.send_gateway_command(GatewayCommand::FirewallConfigChanged(
                device_network.id,
                firewall_config,
            ));
        }

        info!(
            "User {} changed IP addresses of network device {} from {:?} to {:?} in network {}",
            session.user.username,
            device.name,
            old_ips,
            wireguard_network_device.wireguard_ips,
            device_network.name
        );
    }
    let network_device_info =
        NetworkDeviceInfo::from_device(device.clone(), &mut transaction).await?;
    transaction.commit().await?;

    appstate.emit_event(ApiEvent {
        context,
        event: Box::new(ApiEventType::NetworkDeviceModified {
            location: device_network,
            before,
            after: device,
        }),
    })?;
    Ok(ApiResponse::json(network_device_info, StatusCode::OK))
}
