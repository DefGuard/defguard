use super::{
    device_for_admin_or_self, user_for_admin_or_self, ApiResponse, ApiResult, OriWebError,
};
use crate::db::models::device::{DeviceInfo, DeviceNetworkInfo};
use crate::db::models::wireguard::WireguardNetworkInfo;
use crate::grpc::GatewayMap;
use crate::{
    appstate::AppState,
    auth::{AdminRole, Claims, ClaimsType, SessionInfo},
    db::{
        models::{
            device::{ModifyDevice, WireguardNetworkDevice},
            wireguard::DateTimeAggregation,
        },
        AddDevice, DbPool, Device, GatewayEvent, WireguardNetwork,
    },
    wg_config::{parse_wireguard_config, ImportedDevice},
};
use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use ethers::utils::__serde_json::Value;
use ipnetwork::IpNetwork;
use rocket::{
    http::Status,
    serde::{
        json::{json, Json},
        Deserialize,
    },
    State,
};
use std::net::IpAddr;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

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
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MappedDevice {
    pub user_id: i64,
    pub name: String,
    pub wireguard_pubkey: String,
    pub wireguard_ip: IpAddr,
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

#[post("/", format = "json", data = "<data>")]
pub async fn create_network(
    _admin: AdminRole,
    data: Json<WireguardNetworkData>,
    appstate: &State<AppState>,
    session: SessionInfo,
) -> ApiResult {
    let network_name = data.name.clone();
    debug!(
        "User {} creating WireGuard network {}",
        session.user.username, network_name
    );
    let data = data.into_inner();
    let allowed_ips = data.parse_allowed_ips();
    let mut network = WireguardNetwork::new(
        data.name,
        data.address,
        data.port,
        data.endpoint,
        data.dns,
        allowed_ips,
    )
    .map_err(|_| OriWebError::Serialization("Invalid network address".into()))?;

    let mut transaction = appstate.pool.begin().await?;
    network.save(&mut transaction).await?;
    network
        .set_allowed_groups(&mut transaction, data.allowed_groups)
        .await?;

    // generate IP addresses for existing devices
    network
        .add_all_allowed_devices(&mut transaction, &appstate.config.admin_groupname)
        .await?;
    info!("Assigning IPs for existing devices in network {}", network);

    match &network.id {
        Some(network_id) => {
            appstate
                .send_wireguard_event(GatewayEvent::NetworkCreated(*network_id, network.clone()));
        }
        None => {
            error!("Network {} ID was not created during network creation, gateway event was not send!", &network.name);
            return Ok(ApiResponse {
                json: json!({}),
                status: Status::InternalServerError,
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
        status: Status::Created,
    })
}

async fn find_network(id: i64, pool: &DbPool) -> Result<WireguardNetwork, OriWebError> {
    WireguardNetwork::find_by_id(pool, id)
        .await?
        .ok_or_else(|| OriWebError::ObjectNotFound(format!("Network {} not found", id)))
}

#[put("/<id>", format = "json", data = "<data>")]
pub async fn modify_network(
    _admin: AdminRole,
    id: i64,
    data: Json<WireguardNetworkData>,
    appstate: &State<AppState>,
    session: SessionInfo,
) -> ApiResult {
    debug!(
        "User {} updating WireGuard network {}",
        session.user.username, id
    );
    let mut network = find_network(id, &appstate.pool).await?;
    let data = data.into_inner();
    network.allowed_ips = data.parse_allowed_ips();
    network.name = data.name;

    // initialize DB transaction
    let mut transaction = appstate.pool.begin().await?;

    network.endpoint = data.endpoint;
    network.port = data.port;
    network.dns = data.dns;
    network.address = data.address;
    network.save(&mut transaction).await?;
    network
        .set_allowed_groups(&mut transaction, data.allowed_groups)
        .await?;
    let events = network
        .sync_allowed_devices(&mut transaction, &appstate.config.admin_groupname, None)
        .await?;

    match &network.id {
        Some(network_id) => {
            appstate
                .send_wireguard_event(GatewayEvent::NetworkModified(*network_id, network.clone()));
        }
        &None => {
            error!(
                "Network {} id not found, gateway update not send!",
                network.name
            );
        }
    }
    // send gateway events for changed devices
    appstate.send_multiple_wireguard_events(events);

    // commit DB transaction
    transaction.commit().await?;

    info!(
        "User {} updated WireGuard network {}",
        session.user.username, id
    );
    Ok(ApiResponse {
        json: json!(network),
        status: Status::Ok,
    })
}

#[delete("/<id>")]
pub async fn delete_network(
    _admin: AdminRole,
    id: i64,
    appstate: &State<AppState>,
    session: SessionInfo,
) -> ApiResult {
    debug!(
        "User {} deleting WireGuard network {}",
        session.user.username, id
    );
    let network = find_network(id, &appstate.pool).await?;
    let network_name = network.name.clone();
    network.delete(&appstate.pool).await?;
    appstate.send_wireguard_event(GatewayEvent::NetworkDeleted(id, network_name));
    info!(
        "User {} deleted WireGuard network {}",
        session.user.username, id
    );
    Ok(ApiResponse::default())
}

#[get("/", format = "json")]
pub async fn list_networks(
    _admin: AdminRole,
    appstate: &State<AppState>,
    gateway_state: &State<Arc<Mutex<GatewayMap>>>,
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
            })
        }
    }
    debug!("Listed WireGuard networks");

    Ok(ApiResponse {
        json: json!(network_info),
        status: Status::Ok,
    })
}

#[get("/<network_id>", format = "json")]
pub async fn network_details(
    network_id: i64,
    _admin: AdminRole,
    appstate: &State<AppState>,
    gateway_state: &State<Arc<Mutex<GatewayMap>>>,
) -> ApiResult {
    debug!("Displaying network details for network {}", network_id);
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
                status: Status::Ok,
            }
        }
        None => ApiResponse {
            json: Value::Null,
            status: Status::NotFound,
        },
    };
    debug!("Displayed network details for network {}", network_id);

    Ok(response)
}

#[get("/<network_id>/gateways", format = "json")]
pub async fn gateway_status(
    network_id: i64,
    _admin: AdminRole,
    gateway_state: &State<Arc<Mutex<GatewayMap>>>,
) -> ApiResult {
    debug!("Displaying gateway status for network {}", network_id);
    let gateway_state = gateway_state
        .lock()
        .expect("Failed to acquire gateway state lock");

    Ok(ApiResponse {
        json: json!(gateway_state.get_network_gateway_status(network_id)),
        status: Status::Ok,
    })
}

#[delete("/<network_id>/gateways/<gateway_id>")]
pub async fn remove_gateway(
    network_id: i64,
    gateway_id: String,
    _admin: AdminRole,
    gateway_state: &State<Arc<Mutex<GatewayMap>>>,
) -> ApiResult {
    info!("Removing gateway {} in network {}", gateway_id, network_id);
    let mut gateway_state = gateway_state
        .lock()
        .expect("Failed to acquire gateway state lock");

    gateway_state.remove_gateway(
        network_id,
        Uuid::from_str(&gateway_id).map_err(|_| OriWebError::Http(Status::InternalServerError))?,
    )?;

    Ok(ApiResponse {
        json: Value::Null,
        status: Status::Ok,
    })
}

#[post("/import", format = "json", data = "<data>")]
pub async fn import_network(
    _admin: AdminRole,
    appstate: &State<AppState>,
    data: Json<ImportNetworkData>,
) -> ApiResult {
    info!("Importing network from config file");
    let data = data.into_inner();
    let (mut network, imported_devices) =
        parse_wireguard_config(&data.config).map_err(|error| {
            error!("{}", error);
            OriWebError::Http(Status::UnprocessableEntity)
        })?;
    network.name = data.name;
    network.endpoint = data.endpoint;

    let mut transaction = appstate.pool.begin().await?;
    network.save(&mut transaction).await?;
    network
        .set_allowed_groups(&mut transaction, data.allowed_groups)
        .await?;

    info!("New network {} created", network);
    match network.id {
        Some(network_id) => {
            appstate
                .send_wireguard_event(GatewayEvent::NetworkCreated(network_id, network.clone()));
        }
        None => {
            error!("Network {} id not found, gateway event not sent!", network);
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
        status: Status::Created,
    })
}

// This is used exclusively during wizard for mapping imported devices to users
#[post("/<network_id>/devices", format = "json", data = "<data>")]
pub async fn add_user_devices(
    _admin: AdminRole,
    session: SessionInfo,
    appstate: &State<AppState>,
    data: Json<MappedDevices>,
    network_id: i64,
) -> ApiResult {
    let request_data = data.into_inner();
    let mapped_devices = request_data.devices.clone();
    let user = session.user;
    let device_count = mapped_devices.len();

    // finish early if no devices were provided in request
    if mapped_devices.is_empty() {
        return Ok(ApiResponse {
            json: json!({}),
            status: Status::NoContent,
        });
    }

    match WireguardNetwork::find_by_id(&appstate.pool, network_id).await? {
        Some(network) => {
            info!(
                "User {} mapping {} devices for network {}",
                user.username, device_count, network_id
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
                "User {} mapped {} devices for {} network",
                user.username, device_count, network_id
            );

            Ok(ApiResponse {
                json: json!({}),
                status: Status::Created,
            })
        }
        None => Err(OriWebError::ObjectNotFound(format!(
            "Network {} not found",
            network_id
        ))),
    }
}

#[derive(Serialize)]
pub struct DeviceConfig {
    pub(crate) network_id: i64,
    pub(crate) network_name: String,
    pub(crate) config: String,
}

#[post("/device/<username>", format = "json", data = "<data>")]
pub async fn add_device(
    session: SessionInfo,
    appstate: &State<AppState>,
    username: &str,
    data: Json<AddDevice>,
) -> ApiResult {
    let device_name = data.name.clone();
    debug!(
        "User {} adding device {} for user {}",
        session.user.username, device_name, username
    );
    let user = user_for_admin_or_self(&appstate.pool, &session, username).await?;
    let networks = WireguardNetwork::all(&appstate.pool).await?;
    if networks.is_empty() {
        error!("No network found, can't add device");
        return Ok(ApiResponse {
            json: json!({}),
            status: Status::BadRequest,
        });
    }

    Device::validate_pubkey(&data.wireguard_pubkey).map_err(OriWebError::PubkeyValidation)?;

    // save device
    let add_device = data.into_inner();
    let user_id = match user.id {
        Some(id) => id,
        None => {
            return Err(OriWebError::ModelError("User has no id".to_string()));
        }
    };
    let mut device = Device::new(add_device.name, add_device.wireguard_pubkey, user_id);

    let mut transaction = appstate.pool.begin().await?;
    device.save(&mut transaction).await?;

    // assign IPs and generate configs for each network
    #[derive(Serialize)]
    struct AddDeviceResult {
        configs: Vec<DeviceConfig>,
        device: Device,
    }

    let (network_info, configs) = device
        .add_to_all_networks(&mut transaction, &appstate.config.admin_groupname)
        .await?;

    appstate.send_wireguard_event(GatewayEvent::DeviceCreated(DeviceInfo {
        device: device.clone(),
        network_info,
    }));

    transaction.commit().await?;

    info!(
        "User {} added device {} for user {}",
        session.user.username, device_name, username
    );

    let result = AddDeviceResult { device, configs };

    Ok(ApiResponse {
        json: json!(result),
        status: Status::Created,
    })
}

#[put("/device/<device_id>", format = "json", data = "<data>")]
pub async fn modify_device(
    session: SessionInfo,
    device_id: i64,
    data: Json<ModifyDevice>,
    appstate: &State<AppState>,
) -> ApiResult {
    debug!(
        "User {} updating device {}",
        session.user.username, device_id
    );
    let mut device = device_for_admin_or_self(&appstate.pool, &session, device_id).await?;
    let networks = WireguardNetwork::all(&appstate.pool).await?;

    if networks.is_empty() {
        error!("No network found can't modify device");
        return Ok(ApiResponse {
            json: json!({}),
            status: Status::BadRequest,
        });
    }

    // check pubkeys
    for network in &networks {
        if network.pubkey == data.wireguard_pubkey {
            return Ok(ApiResponse {
                json: json!({"msg": "device's pubkey must be different from server's pubkey"}),
                status: Status::BadRequest,
            });
        }
    }

    // update device info
    device.update_from(data.into_inner());
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
                    network_info.push(device_network_info)
                }
            }
        }
    }
    appstate.send_wireguard_event(GatewayEvent::DeviceModified(DeviceInfo {
        device: device.clone(),
        network_info,
    }));

    info!(
        "User {} updated device {}",
        session.user.username, device_id
    );
    Ok(ApiResponse {
        json: json!(device),
        status: Status::Ok,
    })
}

#[get("/device/<device_id>", format = "json")]
pub async fn get_device(
    session: SessionInfo,
    device_id: i64,
    appstate: &State<AppState>,
) -> ApiResult {
    debug!("Retrieving device with id: {}", device_id);
    let device = device_for_admin_or_self(&appstate.pool, &session, device_id).await?;
    Ok(ApiResponse {
        json: json!(device),
        status: Status::Ok,
    })
}

#[delete("/device/<device_id>")]
pub async fn delete_device(
    session: SessionInfo,
    device_id: i64,
    appstate: &State<AppState>,
) -> ApiResult {
    debug!(
        "User {} deleting device {}",
        session.user.username, device_id
    );
    let device = device_for_admin_or_self(&appstate.pool, &session, device_id).await?;
    appstate.send_wireguard_event(GatewayEvent::DeviceDeleted(
        DeviceInfo::from_device(&appstate.pool, device.clone()).await?,
    ));
    device.delete(&appstate.pool).await?;
    info!(
        "User {} deleted device {}",
        session.user.username, device_id
    );
    Ok(ApiResponse::default())
}

#[get("/device", format = "json")]
pub async fn list_devices(_admin: AdminRole, appstate: &State<AppState>) -> ApiResult {
    debug!("Listing devices");
    let devices = Device::all(&appstate.pool).await?;
    info!("Listed devices");

    Ok(ApiResponse {
        json: json!(devices),
        status: Status::Ok,
    })
}

#[get("/device/user/<username>", format = "json")]
pub async fn list_user_devices(
    session: SessionInfo,
    appstate: &State<AppState>,
    username: &str,
) -> ApiResult {
    // only allow for admin or user themselves
    if !session.is_admin && session.user.username != username {
        return Err(OriWebError::Forbidden("Admin access required".into()));
    };
    debug!("Listing devices for user: {}", username);
    let devices = Device::all_for_username(&appstate.pool, username).await?;

    Ok(ApiResponse {
        json: json!(devices),
        status: Status::Ok,
    })
}

#[get("/<network_id>/device/<device_id>/config", rank = 2, format = "json")]
pub async fn download_config(
    session: SessionInfo,
    appstate: &State<AppState>,
    network_id: i64,
    device_id: i64,
) -> Result<String, OriWebError> {
    let network = find_network(network_id, &appstate.pool).await?;
    let device = device_for_admin_or_self(&appstate.pool, &session, device_id).await?;
    let wireguard_network_device =
        WireguardNetworkDevice::find(&appstate.pool, device_id, network_id).await?;
    match wireguard_network_device {
        Some(wireguard_network_device) => {
            Ok(device.create_config(&network, &wireguard_network_device))
        }
        None => {
            let device_id = match device.id {
                Some(id) => id.to_string(),
                None => "".to_string(),
            };
            Err(OriWebError::ObjectNotFound(format!(
                "No ip found for device: {}({})",
                device.name, device_id
            )))
        }
    }
}

#[get("/<network_id>/token", format = "json")]
pub async fn create_network_token(
    _admin: AdminRole,
    appstate: &State<AppState>,
    network_id: i64,
) -> ApiResult {
    info!("Generating a new token for network ID {}", network_id);
    let network = find_network(network_id, &appstate.pool).await?;
    let token = Claims::new(
        ClaimsType::Gateway,
        format!("DEFGUARD-NETWORK-{}", network_id),
        network_id.to_string(),
        u32::MAX.into(),
    )
    .to_jwt()
    .map_err(|_| {
        OriWebError::Authorization(format!(
            "Failed to create token for gateway {}",
            network.name
        ))
    })?;
    Ok(ApiResponse {
        json: json!({ "token": token, "grpc_url": appstate.config.grpc_url.to_string() }),
        status: Status::Ok,
    })
}

/// Returns appropriate aggregation level depending on the `from` date param
/// If `from` is >= than 6 hours ago, returns `Hour` aggregation
/// Otherwise returns `Minute` aggregation
fn get_aggregation(from: NaiveDateTime) -> Result<DateTimeAggregation, Status> {
    // Use hourly aggregation for longer periods
    let aggregation = match Utc::now().naive_utc() - from {
        duration if duration >= Duration::hours(6) => Ok(DateTimeAggregation::Hour),
        duration if duration < Duration::zero() => Err(Status::BadRequest),
        _ => Ok(DateTimeAggregation::Minute),
    }?;
    Ok(aggregation)
}

/// If `datetime` is Some, parses the date string, otherwise returns `DateTime` one hour ago.
fn parse_timestamp(datetime: Option<String>) -> Result<DateTime<Utc>, Status> {
    Ok(match datetime {
        Some(from) => DateTime::<Utc>::from_str(&from).map_err(|_| Status::BadRequest)?,
        None => Utc::now() - Duration::hours(1),
    })
}

#[get("/<network_id>/stats/users?<from>", format = "json")]
pub async fn user_stats(
    _admin: AdminRole,
    appstate: &State<AppState>,
    from: Option<String>,
    network_id: i64,
) -> ApiResult {
    debug!("Displaying wireguard user stats");
    let network = match WireguardNetwork::find_by_id(&appstate.pool, network_id).await? {
        Some(n) => n,
        None => {
            return Err(OriWebError::ObjectNotFound(format!(
                "Requested network ({}) not found",
                network_id
            )));
        }
    };
    let from = parse_timestamp(from)?.naive_utc();
    let aggregation = get_aggregation(from)?;
    let stats = network
        .user_stats(&appstate.pool, &from, &aggregation)
        .await?;
    debug!("Displayed wireguard user stats");

    Ok(ApiResponse {
        json: json!(stats),
        status: Status::Ok,
    })
}

#[get("/<network_id>/stats?<from>", format = "json")]
pub async fn network_stats(
    _admin: AdminRole,
    appstate: &State<AppState>,
    from: Option<String>,
    network_id: i64,
) -> ApiResult {
    debug!("Displaying wireguard network stats");
    let network = match WireguardNetwork::find_by_id(&appstate.pool, network_id).await? {
        Some(n) => n,
        None => {
            return Err(OriWebError::ObjectNotFound(format!(
                "Requested network ({}) not found",
                network_id
            )));
        }
    };
    let from = parse_timestamp(from)?.naive_utc();
    let aggregation = get_aggregation(from)?;
    let stats = network
        .network_stats(&appstate.pool, &from, &aggregation)
        .await?;
    debug!("Displayed wireguard network stats");

    Ok(ApiResponse {
        json: json!(stats),
        status: Status::Ok,
    })
}
