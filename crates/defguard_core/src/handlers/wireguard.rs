use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
};

use axum::{
    extract::{Json, Path, Query, State},
    http::StatusCode,
};
use chrono::{DateTime, NaiveDateTime, TimeDelta, Utc};
use defguard_common::{
    csv::AsCsv,
    db::{
        Id,
        models::{
            Device, DeviceConfig, DeviceNetworkInfo, DeviceType, WireguardNetwork,
            device::{AddDevice, DeviceInfo, ModifyDevice, WireguardNetworkDevice},
            gateway::Gateway,
            wireguard::{
                DateTimeAggregation, LocationMfaMode, MappedDevice, ServiceLocationMode,
                WireguardDeviceStatsRow, WireguardNetworkStats, WireguardUserStatsRow,
                networks_stats,
            },
        },
    },
    utils::{parse_address_list, parse_network_address_list},
};
use defguard_mail::templates::{TemplateLocation, new_device_added_mail};
use ipnetwork::IpNetwork;
use serde_json::{Value, json};
use sqlx::PgPool;
use utoipa::ToSchema;

use super::{ApiResponse, ApiResult, WebError, device_for_admin_or_self, user_for_admin_or_self};
use crate::{
    appstate::AppState,
    auth::{AdminRole, SessionInfo},
    enterprise::{
        db::models::{enterprise_settings::EnterpriseSettings, openid_provider::OpenIdProvider},
        firewall::try_get_location_firewall_config,
        handlers::CanManageDevices,
        is_business_license_active, is_enterprise_license_active,
        license::get_cached_license,
        limits::{get_counts, update_counts},
    },
    events::{ApiEvent, ApiEventType, ApiRequestContext},
    grpc::gateway::events::GatewayEvent,
    location_management::{
        allowed_peers::get_location_allowed_peers, handle_imported_devices, handle_mapped_devices,
        sync_location_allowed_devices,
    },
    wg_config::{ImportedDevice, parse_wireguard_config},
};

#[derive(Serialize, ToSchema)]
pub(crate) struct GatewayInfo {
    id: Id,
    network_id: Id,
    url: String,
    hostname: Option<String>,
    connected_at: Option<NaiveDateTime>,
    disconnected_at: Option<NaiveDateTime>,
    connected: bool,
}

impl From<Gateway<Id>> for GatewayInfo {
    fn from(gateway: Gateway<Id>) -> Self {
        let connected = gateway.is_connected();
        Self {
            id: gateway.id,
            network_id: gateway.network_id,
            url: gateway.url,
            hostname: gateway.hostname,
            connected_at: gateway.connected_at,
            disconnected_at: gateway.disconnected_at,
            connected,
        }
    }
}

#[derive(Serialize, ToSchema)]
pub(crate) struct WireguardNetworkInfo {
    #[serde(flatten)]
    network: WireguardNetwork<Id>,
    connected: bool,
    gateways: Vec<GatewayInfo>,
    allowed_groups: Vec<String>,
}

#[derive(Deserialize, Serialize, ToSchema)]
pub struct WireguardNetworkData {
    pub name: String,
    pub address: String, // comma-separated list of addresses
    pub endpoint: String,
    pub port: i32,
    pub allowed_ips: Option<String>,
    pub dns: Option<String>,
    pub mtu: i32,
    pub fwmark: i64,
    pub allowed_groups: Vec<String>,
    pub keepalive_interval: i32,
    pub peer_disconnect_threshold: i32,
    pub acl_enabled: bool,
    pub acl_default_allow: bool,
    pub location_mfa_mode: LocationMfaMode,
    pub service_location_mode: ServiceLocationMode,
}

impl WireguardNetworkData {
    pub(crate) fn parse_allowed_ips(&self) -> Vec<IpNetwork> {
        self.allowed_ips
            .as_ref()
            .map_or(Vec::new(), |ips| parse_network_address_list(ips))
    }

    pub(crate) fn parse_addresses(&self) -> Result<Vec<IpNetwork>, WebError> {
        // first parse the addresses
        let subnets = parse_address_list(self.address.as_ref());

        // check if address list is not empty
        if subnets.is_empty() {
            return Err(WebError::BadRequest(
                "Must provide at least one valid network address".to_owned(),
            ));
        }

        // check if any subnet has an invalid /0 netmask
        for subnet in &subnets {
            if subnet.prefix() == 0 {
                return Err(WebError::BadRequest(format!(
                    "{subnet} is not a valid address"
                )));
            }
        }

        Ok(subnets)
    }

    pub(crate) async fn validate_location_mfa_mode<'e, E: sqlx::PgExecutor<'e>>(
        &self,
        executor: E,
    ) -> Result<(), WebError> {
        // if external MFA was chosen verify if enterprise features are enabled
        // and external OpenID provider is configured
        if self.location_mfa_mode == LocationMfaMode::External {
            if !is_business_license_active() {
                error!(
                    "Unable to create location with external MFA. External OpenID provider is not configured"
                );

                return Err(WebError::Forbidden(
                    "Cannot enable external MFA. Enterprise features are disabled".into(),
                ));
            }

            if OpenIdProvider::get_current(executor).await?.is_none() {
                error!(
                    "Unable to create location with external MFA. External OpenID provider is not configured"
                );
                return Err(WebError::BadRequest(
                    "Cannot enable external MFA. External OpenID provider is not configured".into(),
                ));
            }
        }

        Ok(())
    }
}

// Used in process of importing network from WireGuard config.
#[derive(Deserialize)]
pub(crate) struct MappedDevices {
    devices: Vec<MappedDevice>,
}

#[derive(Deserialize)]
pub(crate) struct ImportNetworkData {
    name: String,
    endpoint: String,
    config: String,
    allowed_groups: Vec<String>,
}

#[derive(Deserialize, Serialize)]
pub struct ImportedNetworkData {
    pub network: WireguardNetwork<Id>,
    pub devices: Vec<ImportedDevice>,
}

/// Create new network
///
/// Create new network based on `WireguardNetworkData` object.
///
/// # Returns
/// - `WireguardNetwork` object
///
/// - `WebError` if error occurs
#[utoipa::path(
    post,
    path = "/api/v1/network",
    request_body = WireguardNetworkData,
    responses(
        (status = 201, description = "Successfully created network.", body = WireguardNetwork),
        (status = 401, description = "Unauthorized to create network.", body = ApiResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "You don't have permission to create a network.", body = ApiResponse, example = json!({"msg": "access denied"})),
        (status = 500, description = "Unable to create network.", body = ApiResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn create_network(
    _role: AdminRole,
    State(appstate): State<AppState>,
    session: SessionInfo,
    context: ApiRequestContext,
    Json(data): Json<WireguardNetworkData>,
) -> ApiResult {
    let network_name = data.name.clone();
    debug!(
        "User {} creating WireGuard network {network_name}",
        session.user.username
    );

    // check if adding new network will go over license limits
    let location_count = get_counts().location();

    if get_cached_license()
        .as_ref()
        .and_then(|l| l.limits.as_ref())
        .is_some_and(|l| l.locations == location_count)
    {
        error!("Adding location {network_name} blocked! License limit reached.");
        return Ok(WebError::Forbidden("License limit reached.".into()).into());
    }

    // check if tries to add service location without active enterprise
    if data.service_location_mode != ServiceLocationMode::Disabled
        && !is_enterprise_license_active()
    {
        error!("Adding location {network_name} blocked! Enterprise license required.");
        return Ok(ApiResponse {
            json: json!({
                "msg": "Enterprise license required.",
            }),
            status: StatusCode::FORBIDDEN,
        });
    }

    data.validate_location_mfa_mode(&appstate.pool).await?;

    let allowed_ips = data.parse_allowed_ips();
    let network = WireguardNetwork::new(
        data.name,
        parse_address_list(&data.address),
        data.port,
        data.endpoint,
        data.dns,
        data.mtu,
        data.fwmark,
        allowed_ips,
        data.keepalive_interval,
        data.peer_disconnect_threshold,
        data.acl_enabled,
        data.acl_default_allow,
        data.location_mfa_mode,
        data.service_location_mode,
    );

    let mut transaction = appstate.pool.begin().await?;
    let network = network.save(&mut *transaction).await?;
    network
        .set_allowed_groups(&mut transaction, data.allowed_groups)
        .await?;

    // generate IP addresses for existing devices
    network.add_all_allowed_devices(&mut transaction).await?;
    info!("Assigning IPs for existing devices in network {network}");

    appstate.send_wireguard_event(GatewayEvent::NetworkCreated(network.id, network.clone()));

    transaction.commit().await?;

    info!(
        "User {} created WireGuard network {network_name}",
        session.user.username
    );

    appstate.emit_event(ApiEvent {
        context,
        event: Box::new(ApiEventType::VpnLocationAdded {
            location: network.clone(),
        }),
    })?;
    update_counts(&appstate.pool).await?;

    Ok(ApiResponse::json(network, StatusCode::CREATED))
}

async fn find_network(id: Id, pool: &PgPool) -> Result<WireguardNetwork<Id>, WebError> {
    WireguardNetwork::find_by_id(pool, id)
        .await?
        .ok_or_else(|| WebError::ObjectNotFound(format!("Network {id} not found")))
}

/// Modify network
///
/// Modify existing network basing on `WireguardNetworkData` object.
///
/// # Returns
/// - `WireguardNetwork` object
///
/// - `WebError` if error occurs
#[utoipa::path(
    put,
    path = "/api/v1/network/{network_id}",
    request_body = WireguardNetworkData,
    responses(
        (status = 200, description = "Successfully modified network.", body = WireguardNetwork),
        (status = 401, description = "Unauthorized to modify network.", body = ApiResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "You don't have permission to modify a network.", body = ApiResponse, example = json!({"msg": "access denied"})),
        (status = 404, description = "Network not found", body = ApiResponse, example = json!({"msg": "network not found"})),
        (status = 500, description = "Unable to modify network.", body = ApiResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn modify_network(
    _role: AdminRole,
    Path(network_id): Path<i64>,
    State(appstate): State<AppState>,
    session: SessionInfo,
    context: ApiRequestContext,
    Json(data): Json<WireguardNetworkData>,
) -> ApiResult {
    debug!(
        "User {} updating WireGuard network {network_id}",
        session.user.username
    );

    // check if tries to modify service location without active enterprise
    if data.service_location_mode != ServiceLocationMode::Disabled
        && !is_enterprise_license_active()
    {
        let name = data.name;
        error!("Modification of location {name} blocked! Enterprise license required.");
        return Ok(ApiResponse {
            json: json!({
                "msg": "Enterprise license required.",
            }),
            status: StatusCode::BAD_REQUEST,
        });
    }

    data.validate_location_mfa_mode(&appstate.pool).await?;

    let mut network = find_network(network_id, &appstate.pool).await?;
    // store network before mods
    let before = network.clone();
    network.address = data.parse_addresses()?;

    network.allowed_ips = data.parse_allowed_ips();
    network.name = data.name;

    // initialize DB transaction
    let mut transaction = appstate.pool.begin().await?;

    network.endpoint = data.endpoint;
    network.port = data.port;
    network.dns = data.dns;
    network.keepalive_interval = data.keepalive_interval;
    network.mtu = data.mtu;
    network.fwmark = data.fwmark;
    network.peer_disconnect_threshold = data.peer_disconnect_threshold;
    network.acl_enabled = data.acl_enabled;
    network.acl_default_allow = data.acl_default_allow;
    network.service_location_mode = if data.location_mfa_mode == LocationMfaMode::Disabled {
        data.service_location_mode
    } else {
        warn!(
            "Disabling service location mode for location {} because location MFA is enabled",
            network.name
        );
        ServiceLocationMode::Disabled
    };
    network.location_mfa_mode = data.location_mfa_mode;

    network.save(&mut *transaction).await?;
    network
        .set_allowed_groups(&mut transaction, data.allowed_groups)
        .await?;
    let _events = sync_location_allowed_devices(&network, &mut transaction, None).await?;

    let peers = get_location_allowed_peers(&network, &mut *transaction).await?;
    let maybe_firewall_config =
        try_get_location_firewall_config(&network, &mut transaction).await?;
    appstate.send_wireguard_event(GatewayEvent::NetworkModified(
        network.id,
        network.clone(),
        peers,
        maybe_firewall_config,
    ));

    // commit DB transaction
    transaction.commit().await?;

    info!(
        "User {} updated WireGuard network {network_id}",
        session.user.username,
    );
    appstate.emit_event(ApiEvent {
        context,
        event: Box::new(ApiEventType::VpnLocationModified {
            before,
            after: network.clone(),
        }),
    })?;
    Ok(ApiResponse::json(network, StatusCode::OK))
}

/// Delete network
///
/// # Returns
/// - empty JSON
///
/// - `WebError` if error occurs
#[utoipa::path(
    delete,
    path = "/api/v1/network/{network_id}",
    responses(
        (status = 200, description = "Successfully deleted network.", body = ApiResponse),
        (status = 401, description = "Unauthorized to delete network.", body = ApiResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "You don't have permission to delete a network.", body = ApiResponse, example = json!({"msg": "access denied"})),
        (status = 404, description = "Network not found", body = ApiResponse, example = json!({"msg": "network not found"})),
        (status = 500, description = "Unable to delete network.", body = ApiResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn delete_network(
    _role: AdminRole,
    Path(network_id): Path<i64>,
    State(appstate): State<AppState>,
    session: SessionInfo,
    context: ApiRequestContext,
) -> ApiResult {
    debug!(
        "User {} deleting WireGuard network {network_id}",
        session.user.username,
    );
    let network = find_network(network_id, &appstate.pool).await?;
    let network_name = network.name.clone();
    let mut transaction = appstate.pool.begin().await?;
    let network_devices = network
        .get_devices_by_type(&mut *transaction, DeviceType::Network)
        .await?;
    for device in network_devices {
        device.delete(&mut *transaction).await?;
    }
    network.clone().delete(&mut *transaction).await?;
    transaction.commit().await?;
    appstate.send_wireguard_event(GatewayEvent::NetworkDeleted(network_id, network_name));
    info!(
        "User {} deleted WireGuard network {network_id}",
        session.user.username,
    );
    appstate.emit_event(ApiEvent {
        context,
        event: Box::new(ApiEventType::VpnLocationRemoved { location: network }),
    })?;
    update_counts(&appstate.pool).await?;

    Ok(ApiResponse::default())
}

/// List of all networks
///
/// Retrieve list of all networks
///
/// # Returns
/// - List of `WireguardNetworkInfo` objects
///
/// - `WebError` if error occurs
#[utoipa::path(
    get,
    path = "/api/v1/network",
    responses(
        (status = 200, description = "List of all networks", body = [WireguardNetworkInfo]),
        (status = 401, description = "Unauthorized to list all networks.", body = ApiResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "You don't have permission to list all networks.", body = ApiResponse, example = json!({"msg": "access denied"})),
        (status = 500, description = "Unable to list all networks.", body = ApiResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn list_networks(_role: AdminRole, State(appstate): State<AppState>) -> ApiResult {
    debug!("Listing WireGuard networks");
    let mut network_info = Vec::new();
    let networks = WireguardNetwork::all(&appstate.pool).await?;

    for network in networks {
        let allowed_groups = network.fetch_allowed_groups(&appstate.pool).await?;
        let gateways = Gateway::find_by_network_id(&appstate.pool, network.id).await?;
        network_info.push(WireguardNetworkInfo {
            network,
            connected: false, // FIXME: was: gateway_state.connected(network_id),
            gateways: gateways.into_iter().map(Into::into).collect(),
            allowed_groups,
        });
    }
    debug!("Listed WireGuard networks");

    Ok(ApiResponse::json(network_info, StatusCode::OK))
}

/// Details of network
///
/// Retrieve details about network with `network_id`.
///
/// # Returns
/// - `WireguardNetworkInfo` object
///
/// - `WebError` if error occurs
#[utoipa::path(
    get,
    path = "/api/v1/network/{network_id}",
    responses(
        (status = 200, description = "Network details", body = WireguardNetworkInfo),
        (status = 401, description = "Unauthorized to get network details.", body = ApiResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "You don't have permission to get network details.", body = ApiResponse, example = json!({"msg": "access denied"})),
        (status = 404, description = "Network not found", body = ApiResponse, example = json!({"msg": "network not found"})),
        (status = 500, description = "Unable to get network details.", body = ApiResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn network_details(
    Path(network_id): Path<i64>,
    _role: AdminRole,
    State(appstate): State<AppState>,
) -> ApiResult {
    debug!("Displaying network details for network {network_id}");
    let network = WireguardNetwork::find_by_id(&appstate.pool, network_id).await?;
    let response = match network {
        Some(network) => {
            let allowed_groups = network.fetch_allowed_groups(&appstate.pool).await?;
            let gateways = Gateway::find_by_network_id(&appstate.pool, network_id).await?;
            let network_info = WireguardNetworkInfo {
                network,
                connected: false, // FIXME: was: gateway_state.connected(network_id),
                gateways: gateways.into_iter().map(Into::into).collect(),
                allowed_groups,
            };
            ApiResponse::json(network_info, StatusCode::OK)
        }
        None => ApiResponse::new(Value::Null, StatusCode::NOT_FOUND),
    };
    debug!("Displayed network details for network {network_id}");

    Ok(response)
}

/// Returns state of gateways in a given network
///
/// # Returns
/// Returns `Vec<Gateway>` for requested network.
pub(crate) async fn gateway_status(
    Path(network_id): Path<i64>,
    _role: AdminRole,
    State(appstate): State<AppState>,
) -> ApiResult {
    debug!("Displaying gateway status for network {network_id}");

    let gateways = Gateway::find_by_network_id(&appstate.pool, network_id)
        .await?
        .into_iter()
        .map(GatewayInfo::from)
        .collect::<Vec<_>>();

    debug!("Displayed gateway status for network {network_id}");

    Ok(ApiResponse::json(gateways, StatusCode::OK))
}

/// Returns state of gateways for all networks
///
/// Returns current state of gateways as `HashMap<Id, Vec<GatewayInfo>>` where key is ID of
/// `WireguardNetwork`.
pub(crate) async fn all_gateways_status(
    _role: AdminRole,
    State(appstate): State<AppState>,
) -> ApiResult {
    debug!("Displaying gateways status for all networks.");

    let mut map = HashMap::new();
    let gateways = Gateway::all(&appstate.pool).await?;
    for gateway in gateways {
        let entry: &mut Vec<GatewayInfo> = map.entry(gateway.network_id).or_default();
        entry.push(gateway.into());
    }

    Ok(ApiResponse::json(map, StatusCode::OK))
}

#[derive(Deserialize)]
pub(crate) struct GatewayData {
    url: String,
}

/// Change gateway (PUT).
pub(crate) async fn change_gateway(
    Path((network_id, gateway_id)): Path<(Id, Id)>,
    _role: AdminRole,
    State(appstate): State<AppState>,
    Json(data): Json<GatewayData>,
) -> ApiResult {
    debug!("Changing gateway {gateway_id} in network {network_id}");

    if let Some(mut gateway) = Gateway::find_by_id(&appstate.pool, gateway_id).await? {
        if gateway.network_id == network_id {
            gateway.url = data.url;
            gateway.save(&appstate.pool).await?;
            info!("Changed gateway");
            return Ok(ApiResponse::json(
                GatewayInfo::from(gateway),
                StatusCode::OK,
            ));
        }
    }

    info!("Changed gateway {gateway_id} in network {network_id}");

    Ok(ApiResponse::new(Value::Null, StatusCode::NOT_FOUND))
}

/// Remove gateway (DELETE).
pub(crate) async fn remove_gateway(
    Path((network_id, gateway_id)): Path<(Id, Id)>,
    _role: AdminRole,
    State(appstate): State<AppState>,
) -> ApiResult {
    debug!("Removing gateway {gateway_id} in network {network_id}");

    Gateway::delete_by_id(&appstate.pool, gateway_id, network_id).await?;

    info!("Removed gateway {gateway_id} in network {network_id}");

    Ok(ApiResponse::new(Value::Null, StatusCode::OK))
}

pub(crate) async fn import_network(
    _role: AdminRole,
    State(appstate): State<AppState>,
    context: ApiRequestContext,
    Json(data): Json<ImportNetworkData>,
) -> ApiResult {
    debug!("Importing network from config file");
    let (mut network, imported_devices) =
        parse_wireguard_config(&data.config).map_err(|error| {
            error!("{error}");
            WebError::Http(StatusCode::UNPROCESSABLE_ENTITY)
        })?;
    network.name = data.name;
    network.endpoint = data.endpoint;

    let mut transaction = appstate.pool.begin().await?;
    let network = network.save(&mut *transaction).await?;
    network
        .set_allowed_groups(&mut transaction, data.allowed_groups)
        .await?;

    info!("New network {network} created");
    appstate.send_wireguard_event(GatewayEvent::NetworkCreated(network.id, network.clone()));

    let reserved_ips = imported_devices
        .iter()
        .flat_map(|dev| dev.wireguard_ips.clone())
        .collect::<Vec<_>>();
    let (devices, gateway_events) =
        handle_imported_devices(&network, &mut transaction, imported_devices).await?;
    appstate.send_multiple_wireguard_events(gateway_events);

    // assign IPs for other existing devices
    debug!("Assigning IPs in imported network for remaining existing devices");
    let gateway_events =
        sync_location_allowed_devices(&network, &mut transaction, Some(&reserved_ips)).await?;
    appstate.send_multiple_wireguard_events(gateway_events);
    debug!("Assigned IPs in imported network for remaining existing devices");

    transaction.commit().await?;

    info!("Imported network {network} with {} devices", devices.len());
    appstate.emit_event(ApiEvent {
        context,
        event: Box::new(ApiEventType::VpnLocationAdded {
            location: network.clone(),
        }),
    })?;
    update_counts(&appstate.pool).await?;

    Ok(ApiResponse::json(
        ImportedNetworkData { network, devices },
        StatusCode::CREATED,
    ))
}

// This is used exclusively for the wizard to map imported devices to users.
pub(crate) async fn add_user_devices(
    _role: AdminRole,
    session: SessionInfo,
    State(appstate): State<AppState>,
    Path(network_id): Path<i64>,
    Json(request_data): Json<MappedDevices>,
) -> ApiResult {
    let mapped_devices = request_data.devices.clone();
    let user = session.user;
    let device_count = mapped_devices.len();

    debug!(
        "User {} mapping {device_count} devices for network {network_id}",
        user.username,
    );

    // finish early if no devices were provided in request
    if mapped_devices.is_empty() {
        debug!("No devices provided in request, skipping mapping");
        return Ok(ApiResponse::with_status(StatusCode::NO_CONTENT));
    }

    if let Some(network) = WireguardNetwork::find_by_id(&appstate.pool, network_id).await? {
        // wrap loop in transaction to abort if a device is invalid
        let mut transaction = appstate.pool.begin().await?;
        let events = handle_mapped_devices(&network, &mut transaction, mapped_devices).await?;
        appstate.send_multiple_wireguard_events(events);
        transaction.commit().await?;

        info!(
            "User {} mapped {device_count} devices for {network_id} network",
            user.username,
        );
        update_counts(&appstate.pool).await?;

        Ok(ApiResponse::with_status(StatusCode::CREATED))
    } else {
        error!("Failed to map devices, network {network_id} not found");
        Err(WebError::ObjectNotFound(format!(
            "Network {network_id} not found"
        )))
    }
}

// assign IPs and generate configs for each network
#[derive(Serialize, ToSchema)]
pub(crate) struct AddDeviceResult {
    configs: Vec<DeviceConfig>,
    device: Device<Id>,
}

/// Add device
///
/// Add a new device for a user by sending `AddDevice` object.
///
/// Notice that `wireguard_pubkey` must be unique to successfully add the device.
///
/// You can't add devices for `disabled` users, unless you are an admin.
///
/// Device will be added to all networks in your company infrastructure.
///
/// User will receive all new device details on email.
///
/// # Returns
/// - `AddDeviceResult` object
///
/// - `WebError` if error occurs
#[utoipa::path(
    post,
    path = "/api/v1/device/{device_id}",
    params(
        ("device_id" = String, description = "ID of device.")
    ),
    request_body = AddDevice,
    responses(
        (status = 201, description = "Successfully added a new device for a user.", body = AddDeviceResult, example = json!(
            {
                "configs": [
                    {
                        "network_id": 0,
                        "network_name": "network_name",
                        "config": "config",
                        "address": "0.0.0.0:8000",
                        "endpoint": "endpoint",
                        "allowed_ips": ["0.0.0.0:8000"],
                        "pubkey": "pubkey",
                        "dns": "8.8.8.8",
                        "keepalive_interval": 5,
			            "location_mfa_mode": "disabled",
                        "service_location_mode": "disabled"
                    }
                ],
                "device": {
                    "id": 0,
                    "name": "name",
                    "wireguard_pubkey": "wireguard_pubkey",
                    "user_id": 0,
                    "created": "2024-07-10T10:25:43.231Z"
                }
            }
        )),
        (status = 400, description = "Bad request, no networks found or device with pubkey that you want to send with already exists.", body = ApiResponse, example = json!({})),
        (status = 401, description = "Unauthorized to add a new device for a user.", body = ApiResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "You don't have permission to add a new device for a user. You can't add a new device for a disabled user.", body = ApiResponse, example = json!({"msg": "requires privileged access"})),
        (status = 500, description = "Cannot add a new device for a user.", body = ApiResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn add_device(
    _can_manage_devices: CanManageDevices,
    session: SessionInfo,
    context: ApiRequestContext,
    State(appstate): State<AppState>,
    // Alias, because otherwise `axum` reports conflicting routes.
    Path(username): Path<String>,
    Json(add_device): Json<AddDevice>,
) -> ApiResult {
    let device_name = add_device.name.clone();
    debug!(
        "User {} adding device {device_name} for user {username}",
        session.user.username,
    );

    let user = user_for_admin_or_self(&appstate.pool, &session, &username).await?;

    let settings = EnterpriseSettings::get(&appstate.pool).await?;
    if settings.only_client_activation && !session.is_admin {
        warn!(
            "User {} tried to add a device, but manual device management is disaled",
            session.user.username
        );
        return Err(WebError::Forbidden(
            "Manual device management is disabled".into(),
        ));
    }

    // Let admins manage devices for disabled users
    if !user.is_active && !session.is_admin {
        warn!(
            "User {} tried to add a device for a disabled user {username}",
            session.user.username
        );

        return Err(WebError::Forbidden("User is disabled.".into()));
    }

    let networks = WireguardNetwork::all(&appstate.pool).await?;
    if networks.is_empty() {
        error!("Failed to add device {device_name}, no networks found");
        return Ok(ApiResponse::with_status(StatusCode::BAD_REQUEST));
    }

    Device::validate_pubkey(&add_device.wireguard_pubkey).map_err(WebError::PubkeyValidation)?;

    // Make sure there is no device with the same pubkey, such state may lead to unexpected issues
    if Device::find_by_pubkey(&appstate.pool, &add_device.wireguard_pubkey)
        .await?
        .is_some()
    {
        return Err(WebError::PubkeyExists(format!(
            "Failed to add device {device_name}, identical pubkey ({}) already exists",
            add_device.wireguard_pubkey
        )));
    }

    // save the device
    let mut transaction = appstate.pool.begin().await?;
    let device = Device::new(
        add_device.name,
        add_device.wireguard_pubkey,
        user.id,
        DeviceType::User,
        None,
        true,
    )
    .save(&mut *transaction)
    .await?;

    let (network_info, configs) = device.add_to_all_networks(&mut transaction).await?;

    // prepare a list of gateway events to be sent
    let mut events = Vec::new();

    // get all locations affected by device being added
    let mut affected_location_ids = HashSet::new();
    for network_info_item in network_info.clone() {
        affected_location_ids.insert(network_info_item.network_id);
    }

    // send firewall config updates to affected locations
    // if they have ACL enabled & enterprise features are active
    for location_id in affected_location_ids {
        if let Some(location) = WireguardNetwork::find_by_id(&mut *transaction, location_id).await?
        {
            if let Some(firewall_config) =
                try_get_location_firewall_config(&location, &mut transaction).await?
            {
                debug!(
                    "Sending firewall config update for location {location} affected by adding new user {username} devices"
                );
                events.push(GatewayEvent::FirewallConfigChanged(
                    location_id,
                    firewall_config,
                ));
            }
        }
    }

    // add peer on relevant gateways
    events.push(GatewayEvent::DeviceCreated(DeviceInfo {
        device: device.clone(),
        network_info: network_info.clone(),
    }));

    appstate.send_multiple_wireguard_events(events);

    let template_locations = configs
        .iter()
        .map(|c| TemplateLocation {
            name: c.network_name.clone(),
            assigned_ips: c.address.as_csv(),
        })
        .collect::<Vec<_>>();

    // hide session info if triggered by admin for other user
    let (session_ip, session_device_info) = if session.is_admin && session.user != user {
        (None, None)
    } else {
        (
            Some(session.session.ip_address.as_str()),
            session.session.device_info.clone(),
        )
    };
    new_device_added_mail(
        &user.email,
        &mut transaction,
        &device.name,
        &device.wireguard_pubkey,
        &template_locations,
        session_ip,
        session_device_info.as_deref(),
    )
    .await?;

    transaction.commit().await?;

    info!(
        "User {} added device {device_name} for user {username}",
        session.user.username
    );

    let result = AddDeviceResult {
        configs,
        device: device.clone(),
    };

    update_counts(&appstate.pool).await?;

    appstate.emit_event(ApiEvent {
        context,
        event: Box::new(ApiEventType::UserDeviceAdded {
            device,
            owner: user,
        }),
    })?;

    Ok(ApiResponse::json(result, StatusCode::CREATED))
}

/// Modify device
///
/// Update a device for a user by sending `ModifyDevice` object.
///
/// Notice that `wireguard_pubkey` must be different from server's pubkey.
///
/// Endpoint will trigger new update in gateway server.
///
/// # Returns
/// - `Device` object
///
/// - `WebError` if error occurs
#[utoipa::path(
    put,
    path = "/api/v1/device/{device_id}",
    params(
        ("device_id" = i64, description = "ID of device.")
    ),
    request_body = ModifyDevice,
    responses(
        (status = 200, description = "Successfully updated a device.", body = Device, example = json!(
            {
                "id": 0,
                "name": "name",
                "wireguard_pubkey": "wireguard_pubkey",
                "user_id": 0,
                "created": "2024-07-10T10:25:43.231Z"
            }
        )),
        (status = 400, description = "Bad request, no networks found or device with pubkey that you want to send with is a server's pubkey.", body = ApiResponse, example = json!({"msg": "device's pubkey must be different from server's pubkey"})),
        (status = 401, description = "Unauthorized to update a device.", body = ApiResponse, example = json!({"msg": "Session is required"})),
        (status = 404, description = "Device not found.", body = ApiResponse, example = json!({"msg": "device id <id> not found"})),
        (status = 500, description = "Cannot update a device.", body = ApiResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn modify_device(
    _can_manage_devices: CanManageDevices,
    session: SessionInfo,
    context: ApiRequestContext,
    Path(device_id): Path<i64>,
    State(appstate): State<AppState>,
    Json(data): Json<ModifyDevice>,
) -> ApiResult {
    debug!("User {} updating device {device_id}", session.user.username);

    let settings = EnterpriseSettings::get(&appstate.pool).await?;
    if settings.only_client_activation && !session.is_admin {
        warn!(
            "User {} tried to add a device, but manual device management is disaled",
            session.user.username
        );
        return Err(WebError::Forbidden(
            "Manual device management is disabled".into(),
        ));
    }

    let mut device = device_for_admin_or_self(&appstate.pool, &session, device_id).await?;
    // store device before mods
    let before = device.clone();
    let networks = WireguardNetwork::all(&appstate.pool).await?;

    if networks.is_empty() {
        error!("Failed to update device {device_id}, no networks found");
        return Ok(ApiResponse::with_status(StatusCode::BAD_REQUEST));
    }

    // check pubkeys
    for network in &networks {
        if network.pubkey == data.wireguard_pubkey {
            error!(
                "Failed to update device {device_id}, device's pubkey must be different from server's pubkey"
            );
            return Ok(ApiResponse::new(
                json!({"msg": "device's pubkey must be different from server's pubkey"}),
                StatusCode::BAD_REQUEST,
            ));
        }
    }

    // update device info
    device.update_from(data);

    // clone to use later

    device.save(&appstate.pool).await?;

    // send update to gateway's
    let mut network_info = Vec::new();
    for network in &networks {
        let wireguard_network_device =
            WireguardNetworkDevice::find(&appstate.pool, device.id, network.id).await?;
        if let Some(wireguard_network_device) = wireguard_network_device {
            let device_network_info = DeviceNetworkInfo {
                network_id: network.id,
                device_wireguard_ips: wireguard_network_device.wireguard_ips,
                preshared_key: wireguard_network_device.preshared_key,
                is_authorized: wireguard_network_device.is_authorized,
            };
            network_info.push(device_network_info);
        }
    }
    appstate.send_wireguard_event(GatewayEvent::DeviceModified(DeviceInfo {
        device: device.clone(),
        network_info,
    }));

    info!("User {} updated device {device_id}", session.user.username);

    let owner = device.get_owner(&appstate.pool).await?;
    appstate.emit_event(ApiEvent {
        context,
        event: Box::new(ApiEventType::UserDeviceModified {
            owner,
            before,
            after: device.clone(),
        }),
    })?;

    Ok(ApiResponse::json(device, StatusCode::OK))
}

/// Get device
///
/// Retrieve information about device based on their `device_id`
///
/// # Returns
/// - `Device` object
///
/// - `WebError` if error occurs
#[utoipa::path(
    get,
    path = "/api/v1/device/{device_id}",
    params(
        ("device_id" = i64, description = "ID of device to update details.")
    ),
    responses(
        (status = 200, description = "Successfully updated a device.", body = Device, example = json!(
            {
                "id": 0,
                "name": "name",
                "wireguard_pubkey": "wireguard_pubkey",
                "user_id": 0,
                "created": "2024-07-10T10:25:43.231Z"
            }
        )),
        (status = 400, description = "Bad request, no networks found or device with pubkey that you want to send with is a server's pubkey.", body = ApiResponse, example = json!({"msg": "device's pubkey must be different from server's pubkey"})),
        (status = 401, description = "Unauthorized to update a device.", body = ApiResponse, example = json!({"msg": "Session is required"})),
        (status = 404, description = "Device not found.", body = ApiResponse, example = json!({"msg": "device id <id> not found"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn get_device(
    session: SessionInfo,
    Path(device_id): Path<i64>,
    State(appstate): State<AppState>,
) -> ApiResult {
    debug!("Retrieving device with id: {device_id}");
    let device = device_for_admin_or_self(&appstate.pool, &session, device_id).await?;
    debug!("Retrieved device with id: {device_id}");
    Ok(ApiResponse::json(device, StatusCode::OK))
}

/// Delete device
///
/// Delete user device and trigger new update in gateway server.
///
/// # Returns
/// - empty JSON
///
/// - `WebError` if error occurs
#[utoipa::path(
    delete,
    path = "/api/v1/device/{device_id}",
    params(
        ("device_id" = i64, description = "ID of device to update details.")
    ),
    responses(
        (status = 200, description = "Successfully deleted device."),
        (status = 401, description = "Unauthorized to update a device.", body = ApiResponse, example = json!({"msg": "Session is required"})),
        (status = 404, description = "Device not found.", body = ApiResponse, example = json!({"msg": "device id <id> not found"})),
        (status = 500, description = "Cannot update a device.", body = ApiResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn delete_device(
    _can_manage_devices: CanManageDevices,
    session: SessionInfo,
    context: ApiRequestContext,
    Path(device_id): Path<i64>,
    State(appstate): State<AppState>,
) -> ApiResult {
    // bind username to a variable for easier reference
    let username = &session.user.username;

    debug!("User {username} deleting device {device_id}");
    let mut transaction = appstate.pool.begin().await?;

    let device = device_for_admin_or_self(&mut *transaction, &session, device_id).await?;

    let mut events = Vec::new();

    // prepare device info
    let device_info = DeviceInfo::from_device(&mut *transaction, device.clone()).await?;

    // delete device before firewall config is generated
    device.clone().delete(&mut *transaction).await?;

    update_counts(&mut *transaction).await?;

    // prepare firewall update for affected networks if ACL & enterprise features are enabled
    for info in &device_info.network_info {
        if let Some(location) =
            WireguardNetwork::find_by_id(&mut *transaction, info.network_id).await?
        {
            if let Some(firewall_config) =
                try_get_location_firewall_config(&location, &mut transaction).await?
            {
                debug!(
                    "Sending firewall config update for location {location} affected by deleting user {username} device"
                );
                events.push(GatewayEvent::FirewallConfigChanged(
                    location.id,
                    firewall_config,
                ));
            }
        }
    }

    let device_id = device_info.device.id;
    events.push(GatewayEvent::DeviceDeleted(device_info.clone()));

    // send generated gateway events
    appstate.send_multiple_wireguard_events(events);

    // Emit event specific to the device type.
    match device.device_type {
        DeviceType::User => {
            let owner = device_info.device.get_owner(&mut *transaction).await?;
            appstate.emit_event(ApiEvent {
                context,
                event: Box::new(ApiEventType::UserDeviceRemoved { device, owner }),
            })?;
        }
        DeviceType::Network => {
            if let Some(network_info) = device_info.network_info.first() {
                let location =
                    WireguardNetwork::find_by_id(&mut *transaction, network_info.network_id)
                        .await?;
                if let Some(location) = location {
                    appstate.emit_event(ApiEvent {
                        context,
                        event: Box::new(ApiEventType::NetworkDeviceRemoved { device, location }),
                    })?;
                } else {
                    error!(
                        "Network device {}({}) is assigned to non-existent location {}",
                        device.name, device.id, network_info.network_id
                    );
                }
            } else {
                error!(
                    "Network device {}({}) has no network assigned",
                    device.name, device.id
                );
            }
        }
    }
    transaction.commit().await?;
    info!("User {username} deleted device {device_id}");

    Ok(ApiResponse::default())
}

/// List all devices
///
/// Retrieves all devices
///
/// # Returns
/// - List of `Device` objects
///
/// - `WebError` if error occurs
#[utoipa::path(
    get,
    path = "/api/v1/device",
    responses(
        (status = 200, description = "List all devices.", body = [Device], example = json!([
            {
                "id": 0,
                "name": "name",
                "wireguard_pubkey": "wireguard_pubkey",
                "user_id": 0,
                "created": "2024-07-10T10:25:43.231Z"
            }
        ])),
        (status = 401, description = "Unauthorized to list all devices.", body = ApiResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "You don't have permission to list all devices.", body = ApiResponse, example = json!({"msg": "requires privileged access"})),
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn list_devices(_role: AdminRole, State(appstate): State<AppState>) -> ApiResult {
    debug!("Listing devices");
    let devices = Device::all(&appstate.pool).await?;
    info!("Listed {} devices", devices.len());

    Ok(ApiResponse::json(devices, StatusCode::OK))
}

/// List user devices
///
/// Retrieve all devices that belong to specific `username`.
///
/// This endpoint requires `admin` role.
///
/// # Returns
/// - List of `Device` objects
///
/// - `WebError` object if error occurs.
#[utoipa::path(
    get,
    path = "/api/v1/device/user/{username}",
    params(
        ("username" = String, description = "Name of a user.")
    ),
    responses(
        (status = 200, description = "List user devices.", body = [Device], example = json!([
            {
                "id": 0,
                "name": "name",
                "wireguard_pubkey": "wireguard_pubkey",
                "user_id": 0,
                "created": "2024-07-10T10:25:43.231Z"
            }
        ])),
        (status = 401, description = "Unauthorized to list user devices.", body = ApiResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "You don't have permission to list user devices.", body = ApiResponse, example = json!({"msg": "Admin access required"})),
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn list_user_devices(
    session: SessionInfo,
    State(appstate): State<AppState>,
    Path(username): Path<String>,
) -> ApiResult {
    // only allow for admin or user themselves
    if !session.is_admin && session.user.username != username {
        warn!(
            "User {} tried to list devices for user {username}, but is not an admin",
            session.user.username
        );
        return Err(WebError::Forbidden("Admin access required".into()));
    }
    debug!("Listing devices for user: {username}");
    let devices = Device::all_for_username(&appstate.pool, &username).await?;
    info!("Listed {} devices for user: {username}", devices.len());

    Ok(ApiResponse::json(devices, StatusCode::OK))
}

pub(crate) async fn download_config(
    session: SessionInfo,
    State(appstate): State<AppState>,
    Path((network_id, device_id)): Path<(i64, i64)>,
) -> Result<String, WebError> {
    debug!("Creating config for device {device_id} in network {network_id}");

    let settings = EnterpriseSettings::get(&appstate.pool).await?;
    if settings.only_client_activation && !session.is_admin {
        warn!(
            "User {} tried to download device config, but manual device management is disaled",
            session.user.username
        );
        return Err(WebError::Forbidden(
            "Manual device management is disabled".into(),
        ));
    }

    let network = find_network(network_id, &appstate.pool).await?;
    let device = device_for_admin_or_self(&appstate.pool, &session, device_id).await?;
    let wireguard_network_device =
        WireguardNetworkDevice::find(&appstate.pool, device_id, network_id).await?;
    if let Some(wireguard_network_device) = wireguard_network_device {
        info!("Created config for device {}({device_id})", device.name);
        Ok(Device::create_config(&network, &wireguard_network_device))
    } else {
        error!(
            "Failed to create config, no IP address found for device: {}({})",
            device.name, device.id
        );
        Err(WebError::ObjectNotFound(format!(
            "No IP address found for device: {}({})",
            device.name, device.id
        )))
    }
}

/// Returns appropriate aggregation level depending on the `from` date param
/// If `from` is >= than 6 hours ago, returns `Hour` aggregation
/// Otherwise returns `Minute` aggregation
fn get_aggregation(from: NaiveDateTime) -> Result<DateTimeAggregation, StatusCode> {
    // Use hourly aggregation for longer periods
    let aggregation = match Utc::now().naive_utc() - from {
        duration if duration >= TimeDelta::hours(6) => Ok(DateTimeAggregation::Hour),
        duration if duration < TimeDelta::zero() => Err(StatusCode::BAD_REQUEST),
        _ => Ok(DateTimeAggregation::Minute),
    }?;
    Ok(aggregation)
}

#[derive(Deserialize)]
pub(crate) struct QueryFrom {
    from: Option<String>,
}

impl QueryFrom {
    /// If `datetime` is Some, parses the date string, otherwise returns `DateTime` one hour ago.
    fn parse_timestamp(&self) -> Result<DateTime<Utc>, StatusCode> {
        Ok(match &self.from {
            Some(from) => DateTime::<Utc>::from_str(from).map_err(|_| StatusCode::BAD_REQUEST)?,
            None => Utc::now() - TimeDelta::hours(1),
        })
    }
}

#[derive(Serialize)]
pub(crate) struct DevicesStatsResponse {
    user_devices: Vec<WireguardUserStatsRow>,
    network_devices: Vec<WireguardDeviceStatsRow>,
}

/// Returns network statistics for users and their devices
///
/// # Returns
/// Returns an `DevicesStatsResponse` for requested network and time period
pub(crate) async fn devices_stats(
    _role: AdminRole,
    State(appstate): State<AppState>,
    Path(network_id): Path<i64>,
    Query(query_from): Query<QueryFrom>,
) -> ApiResult {
    debug!("Displaying WireGuard user stats for network {network_id}");
    let Some(network) = WireguardNetwork::find_by_id(&appstate.pool, network_id).await? else {
        return Err(WebError::ObjectNotFound(format!(
            "Requested network ({network_id}) not found",
        )));
    };
    let from = query_from.parse_timestamp()?.naive_utc();
    let aggregation = get_aggregation(from)?;
    let user_devices_stats = network
        .user_stats(&appstate.pool, &from, &aggregation)
        .await?;
    let network_devices_stats = network
        .distinct_device_stats(&appstate.pool, &from, &aggregation, DeviceType::Network)
        .await?;
    let response = DevicesStatsResponse {
        user_devices: user_devices_stats,
        network_devices: network_devices_stats,
    };

    debug!("Displayed WireGuard user stats for network {network_id}");

    Ok(ApiResponse::json(response, StatusCode::OK))
}

/// Returns statistics for requested network
///
/// # Returns
/// Returns an `WireguardNetworkStats` based on requested network and time period
pub(crate) async fn network_stats(
    _role: AdminRole,
    State(appstate): State<AppState>,
    Path(network_id): Path<i64>,
    Query(query_from): Query<QueryFrom>,
) -> ApiResult {
    debug!("Displaying WireGuard network stats for location {network_id}");
    let Some(location) = WireguardNetwork::find_by_id(&appstate.pool, network_id).await? else {
        return Err(WebError::ObjectNotFound(format!(
            "Requested location ({network_id}) not found"
        )));
    };
    let from = query_from.parse_timestamp()?.naive_utc();
    let aggregation: DateTimeAggregation = get_aggregation(from)?;
    let stats: WireguardNetworkStats = location
        .network_stats(&appstate.pool, &from, &aggregation)
        .await?;
    debug!("Displayed WireGuard network stats for network {network_id}");

    Ok(ApiResponse::json(stats, StatusCode::OK))
}

/// Returns statistics for all networks
///
/// # Returns
/// Returns an `WireguardNetworkStats` based on stats from all networks in requested time period
pub(crate) async fn networks_overview_stats(
    _role: AdminRole,
    State(appstate): State<AppState>,
    Query(query_from): Query<QueryFrom>,
) -> ApiResult {
    debug!("Preparing networks overview stats");
    let from = query_from.parse_timestamp()?.naive_utc();
    let aggregation = get_aggregation(from)?;
    let all_networks_stats = networks_stats(&appstate.pool, &from, &aggregation).await?;
    debug!("Finished processing networks overview stats");
    Ok(ApiResponse::json(all_networks_stats, StatusCode::OK))
}
