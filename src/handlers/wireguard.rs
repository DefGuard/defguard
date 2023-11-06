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
use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use ipnetwork::IpNetwork;
use serde_json::{json, Value};
use uuid::Uuid;

use super::{device_for_admin_or_self, user_for_admin_or_self, ApiResponse, ApiResult, WebError};
use crate::{
    appstate::AppState,
    auth::{AdminRole, Claims, ClaimsType, SessionInfo},
    db::{
        models::{
            device::{
                DeviceConfig, DeviceInfo, DeviceNetworkInfo, ModifyDevice, WireguardNetworkDevice,
            },
            wireguard::{DateTimeAggregation, MappedDevice, WireguardNetworkInfo},
        },
        AddDevice, DbPool, Device, GatewayEvent, WireguardNetwork,
    },
    grpc::GatewayMap,
    handlers::mail::send_new_device_added_email,
    templates::TemplateLocation,
    wg_config::{parse_wireguard_config, ImportedDevice},
};

#[derive(Deserialize, Serialize)]
pub struct WireguardNetworkData {
    pub name: String,
    pub address: IpNetwork,
    pub endpoint: String,
    pub port: i32,
    pub allowed_ips: Option<String>,
    pub dns: Option<String>,
    pub allowed_groups: Vec<String>,
}

impl WireguardNetworkData {
    pub(crate) fn parse_allowed_ips(&self) -> Vec<IpNetwork> {
        self.allowed_ips.as_ref().map_or(Vec::new(), |ips| {
            ips.split(',')
                .filter_map(|ip| ip.trim().parse().ok())
                .collect()
        })
    }
}

// Used in process of importing network from wireguard config
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MappedDevices {
    pub devices: Vec<MappedDevice>,
}

#[derive(Serialize)]
struct ConnectionInfo {
    connected: bool,
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
    pub network: WireguardNetwork,
    pub devices: Vec<ImportedDevice>,
}

pub async fn create_network(
    _admin: AdminRole,
    State(appstate): State<AppState>,
    session: SessionInfo,
    Json(data): Json<WireguardNetworkData>,
) -> ApiResult {
    let network_name = data.name.clone();
    debug!(
        "User {} creating WireGuard network {}",
        session.user.username, network_name
    );
    let allowed_ips = data.parse_allowed_ips();
    let mut network = WireguardNetwork::new(
        data.name,
        data.address,
        data.port,
        data.endpoint,
        data.dns,
        allowed_ips,
    )
    .map_err(|_| WebError::Serialization("Invalid network address".into()))?;

    let mut transaction = appstate.pool.begin().await?;
    network.save(&mut *transaction).await?;
    network
        .set_allowed_groups(&mut transaction, data.allowed_groups)
        .await?;

    // generate IP addresses for existing devices
    network
        .add_all_allowed_devices(&mut transaction, &appstate.config.admin_groupname)
        .await?;
    info!("Assigning IPs for existing devices in network {network}");

    match &network.id {
        Some(network_id) => {
            appstate
                .send_wireguard_event(GatewayEvent::NetworkCreated(*network_id, network.clone()));
        }
        None => {
            error!("Network {} ID was not created during network creation, gateway event was not send!", &network.name);
            return Ok(ApiResponse {
                json: json!({}),
                status: StatusCode::INTERNAL_SERVER_ERROR,
            });
        }
    }

    transaction.commit().await?;

    info!(
        "User {} created WireGuard network {}",
        session.user.username, network_name
    );
    Ok(ApiResponse {
        json: json!(network),
        status: StatusCode::CREATED,
    })
}

async fn find_network(id: i64, pool: &DbPool) -> Result<WireguardNetwork, WebError> {
    WireguardNetwork::find_by_id(pool, id)
        .await?
        .ok_or_else(|| WebError::ObjectNotFound(format!("Network {id} not found")))
}

pub async fn modify_network(
    _admin: AdminRole,
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
    network.address = data.address;
    network.save(&mut *transaction).await?;
    network
        .set_allowed_groups(&mut transaction, data.allowed_groups)
        .await?;
    let _events = network
        .sync_allowed_devices(&mut transaction, &appstate.config.admin_groupname, None)
        .await?;

    match &network.id {
        Some(network_id) => {
            let peers = network.get_peers(&mut *transaction).await?;
            appstate.send_wireguard_event(GatewayEvent::NetworkModified(
                *network_id,
                network.clone(),
                peers,
            ));
        }
        &None => {
            error!(
                "Network {} id not found, gateway update not send!",
                network.name
            );
        }
    }

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

pub async fn delete_network(
    _admin: AdminRole,
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
    network.delete(&appstate.pool).await?;
    appstate.send_wireguard_event(GatewayEvent::NetworkDeleted(network_id, network_name));
    info!(
        "User {} deleted WireGuard network {network_id}",
        session.user.username,
    );
    Ok(ApiResponse::default())
}

pub async fn list_networks(
    _admin: AdminRole,
    State(appstate): State<AppState>,
    Extension(gateway_state): Extension<Arc<Mutex<GatewayMap>>>,
) -> ApiResult {
    debug!("Listing WireGuard networks");
    let mut network_info = Vec::new();
    let networks = WireguardNetwork::all(&appstate.pool).await?;

    for network in networks {
        let network_id = network.id.expect("Network does not have an ID");
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

pub async fn network_details(
    Path(network_id): Path<i64>,
    _admin: AdminRole,
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

pub async fn gateway_status(
    Path(network_id): Path<i64>,
    _admin: AdminRole,
    Extension(gateway_state): Extension<Arc<Mutex<GatewayMap>>>,
) -> ApiResult {
    debug!("Displaying gateway status for network {network_id}");
    let gateway_state = gateway_state
        .lock()
        .expect("Failed to acquire gateway state lock");

    Ok(ApiResponse {
        json: json!(gateway_state.get_network_gateway_status(network_id)),
        status: StatusCode::OK,
    })
}

pub async fn remove_gateway(
    Path((network_id, gateway_id)): Path<(i64, String)>,
    _admin: AdminRole,
    Extension(gateway_state): Extension<Arc<Mutex<GatewayMap>>>,
) -> ApiResult {
    info!("Removing gateway {gateway_id} in network {network_id}");
    let mut gateway_state = gateway_state
        .lock()
        .expect("Failed to acquire gateway state lock");

    gateway_state.remove_gateway(
        network_id,
        Uuid::from_str(&gateway_id)
            .map_err(|_| WebError::Http(StatusCode::INTERNAL_SERVER_ERROR))?,
    )?;

    Ok(ApiResponse {
        json: Value::Null,
        status: StatusCode::OK,
    })
}

pub async fn import_network(
    _admin: AdminRole,
    State(appstate): State<AppState>,
    Json(data): Json<ImportNetworkData>,
) -> ApiResult {
    info!("Importing network from config file");
    let (mut network, imported_devices) =
        parse_wireguard_config(&data.config).map_err(|error| {
            error!("{error}");
            WebError::Http(StatusCode::UNPROCESSABLE_ENTITY)
        })?;
    network.name = data.name;
    network.endpoint = data.endpoint;

    let mut transaction = appstate.pool.begin().await?;
    network.save(&mut *transaction).await?;
    network
        .set_allowed_groups(&mut transaction, data.allowed_groups)
        .await?;

    info!("New network {network} created");
    match network.id {
        Some(network_id) => {
            appstate
                .send_wireguard_event(GatewayEvent::NetworkCreated(network_id, network.clone()));
        }
        None => {
            error!("Network {network} id not found, gateway event not sent!");
        }
    }

    let reserved_ips: Vec<IpAddr> = imported_devices
        .iter()
        .map(|dev| dev.wireguard_ip)
        .collect();
    let (devices, gateway_events) = network
        .handle_imported_devices(
            &mut transaction,
            imported_devices,
            &appstate.config.admin_groupname,
        )
        .await?;
    appstate.send_multiple_wireguard_events(gateway_events);

    // assign IPs for other existing devices
    info!("Assigning IPs in imported network for remaining existing devices");
    let gateway_events = network
        .sync_allowed_devices(
            &mut transaction,
            &appstate.config.admin_groupname,
            Some(&reserved_ips),
        )
        .await?;
    appstate.send_multiple_wireguard_events(gateway_events);

    transaction.commit().await?;

    Ok(ApiResponse {
        json: json!(ImportedNetworkData { network, devices }),
        status: StatusCode::CREATED,
    })
}

// This is used exclusively for the wizard to map imported devices to users.
pub async fn add_user_devices(
    _admin: AdminRole,
    session: SessionInfo,
    State(appstate): State<AppState>,
    Path(network_id): Path<i64>,
    Json(request_data): Json<MappedDevices>,
) -> ApiResult {
    let mapped_devices = request_data.devices.clone();
    let user = session.user;
    let device_count = mapped_devices.len();

    // finish early if no devices were provided in request
    if mapped_devices.is_empty() {
        return Ok(ApiResponse {
            json: json!({}),
            status: StatusCode::NO_CONTENT,
        });
    }

    match WireguardNetwork::find_by_id(&appstate.pool, network_id).await? {
        Some(network) => {
            info!(
                "User {} mapping {device_count} devices for network {network_id}",
                user.username,
            );

            // wrap loop in transaction to abort if a device is invalid
            let mut transaction = appstate.pool.begin().await?;
            let events = network
                .handle_mapped_devices(
                    &mut transaction,
                    mapped_devices,
                    &appstate.config.admin_groupname,
                )
                .await?;
            appstate.send_multiple_wireguard_events(events);
            transaction.commit().await?;

            info!(
                "User {} mapped {device_count} devices for {network_id} network",
                user.username,
            );

            Ok(ApiResponse {
                json: json!({}),
                status: StatusCode::CREATED,
            })
        }
        None => Err(WebError::ObjectNotFound(format!(
            "Network {network_id} not found"
        ))),
    }
}

pub async fn add_device(
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
    let networks = WireguardNetwork::all(&appstate.pool).await?;
    if networks.is_empty() {
        error!("No network found, can't add device");
        return Ok(ApiResponse {
            json: json!({}),
            status: StatusCode::BAD_REQUEST,
        });
    }

    Device::validate_pubkey(&add_device.wireguard_pubkey).map_err(WebError::PubkeyValidation)?;

    // save device
    let Some(user_id) = user.id else {
        return Err(WebError::ModelError("User has no id".to_string()));
    };
    let mut device = Device::new(add_device.name, add_device.wireguard_pubkey, user_id);

    let mut transaction = appstate.pool.begin().await?;
    device.save(&mut *transaction).await?;

    // assign IPs and generate configs for each network
    #[derive(Serialize)]
    struct AddDeviceResult {
        configs: Vec<DeviceConfig>,
        device: Device,
    }

    let (network_info, configs) = device
        .add_to_all_networks(&mut transaction, &appstate.config.admin_groupname)
        .await?;

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

    send_new_device_added_email(
        &device.name,
        &device.wireguard_pubkey,
        &template_locations,
        &user.email,
        &appstate.mail_tx,
    )
    .await?;

    info!(
        "User {} added device {device_name} for user {username}",
        session.user.username
    );

    let result = AddDeviceResult { configs, device };

    Ok(ApiResponse {
        json: json!(result),
        status: StatusCode::CREATED,
    })
}

pub async fn modify_device(
    session: SessionInfo,
    Path(device_id): Path<i64>,
    State(appstate): State<AppState>,
    Json(data): Json<ModifyDevice>,
) -> ApiResult {
    debug!("User {} updating device {device_id}", session.user.username);
    let mut device = device_for_admin_or_self(&appstate.pool, &session, device_id).await?;
    let networks = WireguardNetwork::all(&appstate.pool).await?;

    if networks.is_empty() {
        error!("No network found can't modify device");
        return Ok(ApiResponse {
            json: json!({}),
            status: StatusCode::BAD_REQUEST,
        });
    }

    // check pubkeys
    for network in &networks {
        if network.pubkey == data.wireguard_pubkey {
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
        if let Some(network_id) = network.id {
            if let Some(device_id) = device.id {
                let wireguard_network_device =
                    WireguardNetworkDevice::find(&appstate.pool, device_id, network_id).await?;
                if let Some(wireguard_network_device) = wireguard_network_device {
                    let device_network_info = DeviceNetworkInfo {
                        network_id,
                        device_wireguard_ip: wireguard_network_device.wireguard_ip,
                    };
                    network_info.push(device_network_info);
                }
            }
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

pub async fn get_device(
    session: SessionInfo,
    Path(device_id): Path<i64>,
    State(appstate): State<AppState>,
    // TypedHeader(user_agent): TypedHeader<UserAgent>,
) -> ApiResult {
    debug!("Retrieving device with id: {device_id}");
    let device = device_for_admin_or_self(&appstate.pool, &session, device_id).await?;
    Ok(ApiResponse {
        json: json!(device),
        status: StatusCode::OK,
    })
}

pub async fn delete_device(
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
    Ok(ApiResponse::default())
}

pub async fn list_devices(_admin: AdminRole, State(appstate): State<AppState>) -> ApiResult {
    debug!("Listing devices");
    let devices = Device::all(&appstate.pool).await?;
    info!("Listed devices");

    Ok(ApiResponse {
        json: json!(devices),
        status: StatusCode::OK,
    })
}

pub async fn list_user_devices(
    session: SessionInfo,
    State(appstate): State<AppState>,
    Path(username): Path<String>,
) -> ApiResult {
    // only allow for admin or user themselves
    if !session.is_admin && session.user.username != username {
        return Err(WebError::Forbidden("Admin access required".into()));
    };
    debug!("Listing devices for user: {username}");
    let devices = Device::all_for_username(&appstate.pool, &username).await?;

    Ok(ApiResponse {
        json: json!(devices),
        status: StatusCode::OK,
    })
}

pub async fn download_config(
    session: SessionInfo,
    State(appstate): State<AppState>,
    Path((network_id, device_id)): Path<(i64, i64)>,
) -> Result<String, WebError> {
    let network = find_network(network_id, &appstate.pool).await?;
    let device = device_for_admin_or_self(&appstate.pool, &session, device_id).await?;
    let wireguard_network_device =
        WireguardNetworkDevice::find(&appstate.pool, device_id, network_id).await?;
    if let Some(wireguard_network_device) = wireguard_network_device {
        Ok(device.create_config(&network, &wireguard_network_device))
    } else {
        let device_id = if let Some(id) = device.id {
            id.to_string()
        } else {
            String::new()
        };
        Err(WebError::ObjectNotFound(format!(
            "No ip found for device: {}({device_id})",
            device.name
        )))
    }
}

pub async fn create_network_token(
    _admin: AdminRole,
    State(appstate): State<AppState>,
    Path(network_id): Path<i64>,
) -> ApiResult {
    info!("Generating a new token for network ID {network_id}");
    let network = find_network(network_id, &appstate.pool).await?;
    let token = Claims::new(
        ClaimsType::Gateway,
        format!("DEFGUARD-NETWORK-{network_id}"),
        network_id.to_string(),
        u32::MAX.into(),
    )
    .to_jwt()
    .map_err(|_| {
        WebError::Authorization(format!(
            "Failed to create token for gateway {}",
            network.name
        ))
    })?;
    Ok(ApiResponse {
        json: json!({"token": token, "grpc_url": appstate.config.grpc_url.to_string()}),
        status: StatusCode::OK,
    })
}

/// Returns appropriate aggregation level depending on the `from` date param
/// If `from` is >= than 6 hours ago, returns `Hour` aggregation
/// Otherwise returns `Minute` aggregation
fn get_aggregation(from: NaiveDateTime) -> Result<DateTimeAggregation, StatusCode> {
    // Use hourly aggregation for longer periods
    let aggregation = match Utc::now().naive_utc() - from {
        duration if duration >= Duration::hours(6) => Ok(DateTimeAggregation::Hour),
        duration if duration < Duration::zero() => Err(StatusCode::BAD_REQUEST),
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
            None => Utc::now() - Duration::hours(1),
        })
    }
}

pub async fn user_stats(
    _admin: AdminRole,
    State(appstate): State<AppState>,
    Path(network_id): Path<i64>,
    Query(query_from): Query<QueryFrom>,
) -> ApiResult {
    debug!("Displaying WireGuard user stats");
    let Some(network) = WireguardNetwork::find_by_id(&appstate.pool, network_id).await? else {
        return Err(WebError::ObjectNotFound(format!(
            "Requested network ({network_id}) not found",
        )));
    };
    let from = query_from.parse_timestamp()?.naive_utc();
    let aggregation = get_aggregation(from)?;
    let stats = network
        .user_stats(&appstate.pool, &from, &aggregation)
        .await?;
    debug!("Displayed WireGuard user stats");

    Ok(ApiResponse {
        json: json!(stats),
        status: StatusCode::OK,
    })
}

pub async fn network_stats(
    _admin: AdminRole,
    State(appstate): State<AppState>,
    Path(network_id): Path<i64>,
    Query(query_from): Query<QueryFrom>,
) -> ApiResult {
    debug!("Displaying WireGuard network stats");
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
    debug!("Displayed WireGuard network stats");

    Ok(ApiResponse {
        json: json!(stats),
        status: StatusCode::OK,
    })
}
