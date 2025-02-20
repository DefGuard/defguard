use std::{
    net::IpAddr,
    str::FromStr,
    sync::{Arc, Mutex},
};

use axum::{
    extract::{Json, Path, Query, State},
    http::StatusCode,
    Extension,
};
use chrono::{DateTime, NaiveDateTime, TimeDelta, Utc};
use ipnetwork::IpNetwork;
use serde_json::{json, Value};
use sqlx::PgPool;
use utoipa::ToSchema;
use uuid::Uuid;

use super::{device_for_admin_or_self, user_for_admin_or_self, ApiResponse, ApiResult, WebError};
use crate::{
    appstate::AppState,
    auth::{AdminRole, Claims, ClaimsType, SessionInfo},
    db::{
        models::{
            device::{
                DeviceConfig, DeviceInfo, DeviceNetworkInfo, DeviceType, ModifyDevice,
                WireguardNetworkDevice,
            },
            wireguard::{
                DateTimeAggregation, MappedDevice, WireguardDeviceStatsRow, WireguardNetworkInfo,
                WireguardUserStatsRow,
            },
        },
        AddDevice, Device, GatewayEvent, Id, WireguardNetwork,
    },
    enterprise::{handlers::CanManageDevices, limits::update_counts},
    grpc::GatewayMap,
    handlers::mail::send_new_device_added_email,
    server_config,
    templates::TemplateLocation,
    wg_config::{parse_wireguard_config, ImportedDevice},
};

/// Parse a string with comma-separated IP addresses.
/// Invalid addresses will be silently ignored.
pub(crate) fn parse_address_list(ips: &str) -> Vec<IpNetwork> {
    ips.split(',')
        .filter_map(|ip| ip.trim().parse().ok())
        .collect()
}

#[derive(Deserialize, Serialize, ToSchema)]
pub struct WireguardNetworkData {
    pub name: String,
    pub address: String, // comma-separated list of addresses
    pub endpoint: String,
    pub port: i32,
    pub allowed_ips: Option<String>,
    pub dns: Option<String>,
    pub allowed_groups: Vec<String>,
    pub mfa_enabled: bool,
    pub keepalive_interval: i32,
    pub peer_disconnect_threshold: i32,
}

impl WireguardNetworkData {
    pub(crate) fn parse_allowed_ips(&self) -> Vec<IpNetwork> {
        self.allowed_ips
            .as_ref()
            .map_or(Vec::new(), |ips| parse_address_list(ips))
    }
}

// Used in process of importing network from WireGuard config
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MappedDevices {
    pub devices: Vec<MappedDevice>,
}

#[derive(Deserialize)]
pub struct ImportNetworkData {
    pub name: String,
    pub endpoint: String,
    pub config: String,
    pub allowed_groups: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct ImportedNetworkData {
    pub network: WireguardNetwork<Id>,
    pub devices: Vec<ImportedDevice>,
}

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
    Json(data): Json<WireguardNetworkData>,
) -> ApiResult {
    let network_name = data.name.clone();
    debug!(
        "User {} creating WireGuard network {network_name}",
        session.user.username
    );
    let allowed_ips = data.parse_allowed_ips();
    let network = WireguardNetwork::new(
        data.name,
        parse_address_list(&data.address),
        data.port,
        data.endpoint,
        data.dns,
        allowed_ips,
        data.mfa_enabled,
        data.keepalive_interval,
        data.peer_disconnect_threshold,
    )
    .map_err(|_| WebError::Serialization("Invalid network address".into()))?;

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
    update_counts(&appstate.pool).await?;

    Ok(ApiResponse {
        json: json!(network),
        status: StatusCode::CREATED,
    })
}

async fn find_network(id: Id, pool: &PgPool) -> Result<WireguardNetwork<Id>, WebError> {
    WireguardNetwork::find_by_id(pool, id)
        .await?
        .ok_or_else(|| WebError::ObjectNotFound(format!("Network {id} not found")))
}

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
    Json(data): Json<WireguardNetworkData>,
) -> ApiResult {
    debug!(
        "User {} updating WireGuard network {network_id}",
        session.user.username
    );
    let mut network = find_network(network_id, &appstate.pool).await?;
    network.allowed_ips = data.parse_allowed_ips();
    network.name = data.name;

    // initialize DB transaction
    let mut transaction = appstate.pool.begin().await?;

    network.endpoint = data.endpoint;
    network.port = data.port;
    network.dns = data.dns;
    network.address = parse_address_list(&data.address);
    network.mfa_enabled = data.mfa_enabled;
    network.keepalive_interval = data.keepalive_interval;
    network.peer_disconnect_threshold = data.peer_disconnect_threshold;

    network.save(&mut *transaction).await?;
    network
        .set_allowed_groups(&mut transaction, data.allowed_groups)
        .await?;
    let _events = network.sync_allowed_devices(&mut transaction, None).await?;

    let peers = network.get_peers(&mut *transaction).await?;
    appstate.send_wireguard_event(GatewayEvent::NetworkModified(
        network.id,
        network.clone(),
        peers,
    ));

    // commit DB transaction
    transaction.commit().await?;

    info!(
        "User {} updated WireGuard network {network_id}",
        session.user.username,
    );
    Ok(ApiResponse {
        json: json!(network),
        status: StatusCode::OK,
    })
}

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
    network.delete(&mut *transaction).await?;
    transaction.commit().await?;
    appstate.send_wireguard_event(GatewayEvent::NetworkDeleted(network_id, network_name));
    info!(
        "User {} deleted WireGuard network {network_id}",
        session.user.username,
    );
    update_counts(&appstate.pool).await?;

    Ok(ApiResponse::default())
}

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
pub(crate) async fn list_networks(
    _role: AdminRole,
    State(appstate): State<AppState>,
    Extension(gateway_state): Extension<Arc<Mutex<GatewayMap>>>,
) -> ApiResult {
    debug!("Listing WireGuard networks");
    let mut network_info = Vec::new();
    let networks = WireguardNetwork::all(&appstate.pool).await?;

    for network in networks {
        let network_id = network.id;
        let allowed_groups = network.fetch_allowed_groups(&appstate.pool).await?;
        {
            let gateway_state = gateway_state
                .lock()
                .expect("Failed to acquire gateway state lock");
            network_info.push(WireguardNetworkInfo {
                network,
                connected: gateway_state.connected(network_id),
                gateways: gateway_state.get_network_gateway_status(network_id),
                allowed_groups,
            });
        }
    }
    debug!("Listed WireGuard networks");

    Ok(ApiResponse {
        json: json!(network_info),
        status: StatusCode::OK,
    })
}

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
    Extension(gateway_state): Extension<Arc<Mutex<GatewayMap>>>,
) -> ApiResult {
    debug!("Displaying network details for network {network_id}");
    let network = WireguardNetwork::find_by_id(&appstate.pool, network_id).await?;
    let response = match network {
        Some(network) => {
            let allowed_groups = network.fetch_allowed_groups(&appstate.pool).await?;
            let gateway_state = gateway_state
                .lock()
                .expect("Failed to acquire gateway state lock");
            let network_info = WireguardNetworkInfo {
                network,
                connected: gateway_state.connected(network_id),
                gateways: gateway_state.get_network_gateway_status(network_id),
                allowed_groups,
            };
            ApiResponse {
                json: json!(network_info),
                status: StatusCode::OK,
            }
        }
        None => ApiResponse {
            json: Value::Null,
            status: StatusCode::NOT_FOUND,
        },
    };
    debug!("Displayed network details for network {network_id}");

    Ok(response)
}

pub(crate) async fn gateway_status(
    Path(network_id): Path<i64>,
    _role: AdminRole,
    Extension(gateway_state): Extension<Arc<Mutex<GatewayMap>>>,
) -> ApiResult {
    debug!("Displaying gateway status for network {network_id}");
    let gateway_state = gateway_state
        .lock()
        .expect("Failed to acquire gateway state lock");
    debug!("Displayed gateway status for network {network_id}");

    Ok(ApiResponse {
        json: json!(gateway_state.get_network_gateway_status(network_id)),
        status: StatusCode::OK,
    })
}

pub(crate) async fn remove_gateway(
    Path((network_id, gateway_id)): Path<(i64, String)>,
    _role: AdminRole,
    Extension(gateway_state): Extension<Arc<Mutex<GatewayMap>>>,
) -> ApiResult {
    debug!("Removing gateway {gateway_id} in network {network_id}");
    let mut gateway_state = gateway_state
        .lock()
        .expect("Failed to acquire gateway state lock");

    gateway_state.remove_gateway(
        network_id,
        Uuid::from_str(&gateway_id)
            .map_err(|_| WebError::Http(StatusCode::INTERNAL_SERVER_ERROR))?,
    )?;

    info!("Removed gateway {gateway_id} in network {network_id}");

    Ok(ApiResponse {
        json: Value::Null,
        status: StatusCode::OK,
    })
}

pub(crate) async fn import_network(
    _role: AdminRole,
    State(appstate): State<AppState>,
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

    let reserved_ips: Vec<IpAddr> = imported_devices
        .iter()
        .map(|dev| dev.wireguard_ip)
        .collect();
    let (devices, gateway_events) = network
        .handle_imported_devices(&mut transaction, imported_devices)
        .await?;
    appstate.send_multiple_wireguard_events(gateway_events);

    // assign IPs for other existing devices
    debug!("Assigning IPs in imported network for remaining existing devices");
    let gateway_events = network
        .sync_allowed_devices(&mut transaction, Some(&reserved_ips))
        .await?;
    appstate.send_multiple_wireguard_events(gateway_events);
    debug!("Assigned IPs in imported network for remaining existing devices");

    transaction.commit().await?;

    info!("Imported network {network} with {} devices", devices.len());

    update_counts(&appstate.pool).await?;

    Ok(ApiResponse {
        json: json!(ImportedNetworkData { network, devices }),
        status: StatusCode::CREATED,
    })
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
        return Ok(ApiResponse {
            json: json!({}),
            status: StatusCode::NO_CONTENT,
        });
    }

    if let Some(network) = WireguardNetwork::find_by_id(&appstate.pool, network_id).await? {
        // wrap loop in transaction to abort if a device is invalid
        let mut transaction = appstate.pool.begin().await?;
        let events = network
            .handle_mapped_devices(&mut transaction, mapped_devices)
            .await?;
        appstate.send_multiple_wireguard_events(events);
        transaction.commit().await?;

        info!(
            "User {} mapped {device_count} devices for {network_id} network",
            user.username,
        );
        update_counts(&appstate.pool).await?;

        Ok(ApiResponse {
            json: json!({}),
            status: StatusCode::CREATED,
        })
    } else {
        error!("Failed to map devices, network {network_id} not found");
        Err(WebError::ObjectNotFound(format!(
            "Network {network_id} not found"
        )))
    }
}

// assign IPs and generate configs for each network
#[derive(Serialize, ToSchema)]
pub struct AddDeviceResult {
    configs: Vec<DeviceConfig>,
    device: Device<Id>,
}

/// Add device
///
/// Add a new device for a user by sending `AddDevice` object.
/// Notice that `wireguard_pubkey` must be unique to successfully add the device.
/// You can't add devices for `disabled` users, unless you are an admin.
///
/// Device will be added to all networks in your company infrastructure.
///
/// User will receive all new device details on email.
///
/// # Returns
/// Returns `AddDeviceResult` object or `WebError` object if error occurs.
#[utoipa::path(
    post,
    path = "/api/v1/device/{device_id}",
    params(
        ("device_id" = String, description = "Name of a user.")
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
                        "mfa_enabled": false,
                        "keepalive_interval": 5
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

    // Let admins manage devices for disabled users
    if !user.is_active && !session.is_admin {
        info!(
            "User {} tried to add a device for a disabled user {username}",
            session.user.username
        );

        return Err(WebError::Forbidden("User is disabled.".into()));
    }

    let networks = WireguardNetwork::all(&appstate.pool).await?;
    if networks.is_empty() {
        error!("Failed to add device {device_name}, no networks found");
        return Ok(ApiResponse {
            json: json!({}),
            status: StatusCode::BAD_REQUEST,
        });
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

    // save device
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

    let mut network_ips: Vec<String> = Vec::new();
    for network_info_item in network_info.clone() {
        network_ips.push(network_info_item.device_wireguard_ip.to_string());
    }

    appstate.send_wireguard_event(GatewayEvent::DeviceCreated(DeviceInfo {
        device: device.clone(),
        network_info: network_info.clone(),
    }));

    transaction.commit().await?;

    let template_locations: Vec<TemplateLocation> = configs
        .iter()
        .map(|c| TemplateLocation {
            name: c.network_name.clone(),
            assigned_ip: c.address.to_string(),
        })
        .collect();

    // hide session info if triggered by admin for other user
    let (session_ip, session_device_info) = if session.is_admin && session.user != user {
        (None, None)
    } else {
        (
            Some(session.session.ip_address.as_str()),
            session.session.device_info.clone(),
        )
    };
    send_new_device_added_email(
        &device.name,
        &device.wireguard_pubkey,
        &template_locations,
        &user.email,
        &appstate.mail_tx,
        session_ip,
        session_device_info.as_deref(),
    )?;

    info!(
        "User {} added device {device_name} for user {username}",
        session.user.username
    );

    let result = AddDeviceResult { configs, device };

    update_counts(&appstate.pool).await?;

    Ok(ApiResponse {
        json: json!(result),
        status: StatusCode::CREATED,
    })
}

/// Modify device
///
/// Update a device for a user by sending `ModifyDevice` object.
/// Notice that `wireguard_pubkey` must be diffrent from server's pubkey.
///
/// Endpoint will trigger new update in gateway server.
///
/// # Returns
/// Returns `Device` object or `WebError` object if error occurs.
#[utoipa::path(
    put,
    path = "/api/v1/device/{device_id}",
    params(
        ("device_id" = i64, description = "Id of device to update details.")
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
    Path(device_id): Path<i64>,
    State(appstate): State<AppState>,
    Json(data): Json<ModifyDevice>,
) -> ApiResult {
    debug!("User {} updating device {device_id}", session.user.username);
    let mut device = device_for_admin_or_self(&appstate.pool, &session, device_id).await?;
    let networks = WireguardNetwork::all(&appstate.pool).await?;

    if networks.is_empty() {
        error!("Failed to update device {device_id}, no networks found");
        return Ok(ApiResponse {
            json: json!({}),
            status: StatusCode::BAD_REQUEST,
        });
    }

    // check pubkeys
    for network in &networks {
        if network.pubkey == data.wireguard_pubkey {
            error!("Failed to update device {device_id}, device's pubkey must be different from server's pubkey");
            return Ok(ApiResponse {
                json: json!({"msg": "device's pubkey must be different from server's pubkey"}),
                status: StatusCode::BAD_REQUEST,
            });
        }
    }

    // update device info
    device.update_from(data);
    device.save(&appstate.pool).await?;

    // send update to gateway's
    let mut network_info = Vec::new();
    for network in &networks {
        let wireguard_network_device =
            WireguardNetworkDevice::find(&appstate.pool, device.id, network.id).await?;
        if let Some(wireguard_network_device) = wireguard_network_device {
            let device_network_info = DeviceNetworkInfo {
                network_id: network.id,
                device_wireguard_ip: wireguard_network_device.wireguard_ip,
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
    Ok(ApiResponse {
        json: json!(device),
        status: StatusCode::OK,
    })
}

/// Get device
///
/// # Returns
/// Returns `Device` object or `WebError` object if error occurs.
#[utoipa::path(
    get,
    path = "/api/v1/device/{device_id}",
    params(
        ("device_id" = i64, description = "Id of device to update details.")
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
    Ok(ApiResponse {
        json: json!(device),
        status: StatusCode::OK,
    })
}

/// Delete device
///
/// Delete user device and trigger new update in gateway server.
///
/// # Returns
/// If error occurs it returns `WebError` object.
#[utoipa::path(
    delete,
    path = "/api/v1/device/{device_id}",
    params(
        ("device_id" = i64, description = "Id of device to update details.")
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
    Path(device_id): Path<i64>,
    State(appstate): State<AppState>,
) -> ApiResult {
    debug!("User {} deleting device {device_id}", session.user.username);
    let device = device_for_admin_or_self(&appstate.pool, &session, device_id).await?;
    appstate.send_wireguard_event(GatewayEvent::DeviceDeleted(
        DeviceInfo::from_device(&appstate.pool, device.clone()).await?,
    ));
    device.delete(&appstate.pool).await?;
    info!("User {} deleted device {device_id}", session.user.username);
    update_counts(&appstate.pool).await?;
    Ok(ApiResponse::default())
}

/// List all devices
///
/// # Returns
/// Returns a list `Device` objects or `WebError` object if error occurs.
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

    Ok(ApiResponse {
        json: json!(devices),
        status: StatusCode::OK,
    })
}

/// List user devices
///
/// This endpoint requires `admin` role.
///
/// # Returns
/// Returns a list of `Device` object or `WebError` object if error occurs.
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
    };
    debug!("Listing devices for user: {username}");
    let devices = Device::all_for_username(&appstate.pool, &username).await?;
    info!("Listed {} devices for user: {username}", devices.len());

    Ok(ApiResponse {
        json: json!(devices),
        status: StatusCode::OK,
    })
}

pub(crate) async fn download_config(
    session: SessionInfo,
    State(appstate): State<AppState>,
    Path((network_id, device_id)): Path<(i64, i64)>,
) -> Result<String, WebError> {
    debug!("Creating config for device {device_id} in network {network_id}");
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

pub(crate) async fn create_network_token(
    _role: AdminRole,
    State(appstate): State<AppState>,
    Path(network_id): Path<i64>,
) -> ApiResult {
    debug!("Generating a new token for network ID {network_id}");
    let network = find_network(network_id, &appstate.pool).await?;
    let token = Claims::new(
        ClaimsType::Gateway,
        format!("DEFGUARD-NETWORK-{network_id}"),
        network_id.to_string(),
        u32::MAX.into(),
    )
    .to_jwt()
    .map_err(|_| {
        error!("Failed to create token for gateway {}", network.name);
        WebError::Authorization(format!(
            "Failed to create token for gateway {}",
            network.name
        ))
    })?;
    info!("Generated a new token for network ID {network_id}");
    Ok(ApiResponse {
        json: json!({"token": token, "grpc_url": server_config().grpc_url.to_string()}),
        status: StatusCode::OK,
    })
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
pub struct QueryFrom {
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
pub struct DevicesStatsResponse {
    pub user_devices: Vec<WireguardUserStatsRow>,
    pub network_devices: Vec<WireguardDeviceStatsRow>,
}

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

    Ok(ApiResponse {
        json: json!(response),
        status: StatusCode::OK,
    })
}

pub(crate) async fn network_stats(
    _role: AdminRole,
    State(appstate): State<AppState>,
    Path(network_id): Path<i64>,
    Query(query_from): Query<QueryFrom>,
) -> ApiResult {
    debug!("Displaying WireGuard network stats for network {network_id}");
    let Some(network) = WireguardNetwork::find_by_id(&appstate.pool, network_id).await? else {
        return Err(WebError::ObjectNotFound(format!(
            "Requested network ({network_id}) not found"
        )));
    };
    let from = query_from.parse_timestamp()?.naive_utc();
    let aggregation = get_aggregation(from)?;
    let stats = network
        .network_stats(&appstate.pool, &from, &aggregation)
        .await?;
    debug!("Displayed WireGuard network stats for network {network_id}");

    Ok(ApiResponse {
        json: json!(stats),
        status: StatusCode::OK,
    })
}
