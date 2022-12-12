use super::{
    device_for_admin_or_self, user_for_admin_or_self, ApiResponse, ApiResult, OriWebError,
};
use crate::{
    appstate::AppState,
    auth::{AdminRole, Claims, ClaimsType, SessionInfo},
    db::{
        models::wireguard::DateTimeAggregation, AddDevice, DbPool, Device, GatewayEvent,
        WireguardNetwork,
    },
    grpc::GatewayState,
};
use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use ipnetwork::IpNetwork;
use rocket::{
    http::Status,
    serde::{
        json::{json, Json},
        Deserialize,
    },
    State,
};
use std::{
    str::FromStr,
    sync::{Arc, Mutex},
};

#[derive(Deserialize, Serialize)]
pub struct WireguardNetworkData {
    pub name: String,
    pub address: IpNetwork,
    pub endpoint: String,
    pub port: i32,
    pub allowed_ips: Option<String>,
    pub dns: Option<String>,
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

#[derive(Serialize)]
struct ConnectionInfo {
    connected: bool,
}

#[post("/", format = "json", data = "<data>")]
pub async fn create_network(
    _admin: AdminRole,
    data: Json<WireguardNetworkData>,
    appstate: &State<AppState>,
) -> ApiResult {
    debug!("Creating WireGuard network",);
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
    network.save(&appstate.pool).await?;
    appstate.send_wireguard_event(GatewayEvent::NetworkCreated(network.clone()));
    info!("Created WireGuard network");
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
) -> ApiResult {
    debug!("Modifying network id: {}", id);
    let mut network = find_network(id, &appstate.pool).await?;
    let data = data.into_inner();
    network.allowed_ips = data.parse_allowed_ips();
    network.name = data.name;
    network.change_address(&appstate.pool, data.address).await?;
    network.endpoint = data.endpoint;
    network.port = data.port;
    network.dns = data.dns;
    network.save(&appstate.pool).await?;
    appstate.send_wireguard_event(GatewayEvent::NetworkModified(network.clone()));
    info!("Modified network id: {}", id);
    Ok(ApiResponse {
        json: json!(network),
        status: Status::Ok,
    })
}

#[delete("/<id>")]
pub async fn delete_network(_admin: AdminRole, id: i64, appstate: &State<AppState>) -> ApiResult {
    debug!("Deleting network id: {}", id);
    let network = find_network(id, &appstate.pool).await?;
    let network_name = network.name.clone();
    network.delete(&appstate.pool).await?;
    appstate.send_wireguard_event(GatewayEvent::NetworkDeleted(network_name));
    info!("Deleted network id: {}", id);
    Ok(ApiResponse::default())
}

#[get("/", format = "json")]
pub async fn list_networks(_admin: AdminRole, appstate: &State<AppState>) -> ApiResult {
    debug!("Listing WireGuard networks");
    let networks = WireguardNetwork::all(&appstate.pool).await?;
    info!("Listed WireGuard networks");

    Ok(ApiResponse {
        json: json!(networks),
        status: Status::Ok,
    })
}

#[get("/<network_id>", format = "json")]
pub async fn network_details(
    network_id: i64,
    _admin: AdminRole,
    appstate: &State<AppState>,
) -> ApiResult {
    debug!("Displaying network details for network {}", network_id);
    let network = WireguardNetwork::find_by_id(&appstate.pool, network_id).await?;
    info!("Displayed network details for network {}", network_id);

    Ok(ApiResponse {
        json: json!(network),
        status: Status::Ok,
    })
}

#[post("/device/<username>", format = "json", data = "<data>")]
pub async fn add_device(
    session: SessionInfo,
    appstate: &State<AppState>,
    username: &str,
    data: Json<AddDevice>,
) -> ApiResult {
    let user = user_for_admin_or_self(&appstate.pool, &session, username).await?;
    debug!("Adding device for user: {}", username);
    // FIXME: hard-coded network id
    if let Ok(Some(network)) = WireguardNetwork::find_by_id(&appstate.pool, 1).await {
        if network.pubkey == data.wireguard_pubkey {
            return Ok(ApiResponse {
                json: json!({"msg": "device's pubkey must be differnet from server's pubkey"}),
                status: Status::BadRequest,
            });
        }

        let add_device = data.into_inner();
        let mut device = Device::assign_device_ip(
            &appstate.pool,
            user.id.unwrap(),
            add_device.name,
            add_device.wireguard_pubkey,
            &network,
        )
        .await?;
        device.save(&appstate.pool).await?;
        appstate.send_wireguard_event(GatewayEvent::DeviceCreated(device.clone()));
        info!(
            "Added WireGuard device {} for user: {}",
            device.id.unwrap(),
            username
        );
        let config = device.create_config(network);
        Ok(ApiResponse {
            json: json!(config),
            status: Status::Created,
        })
    } else {
        error!("No network found, can't add device");
        Ok(ApiResponse {
            json: json!({}),
            status: Status::BadRequest,
        })
    }
}

#[put("/device/<id>", format = "json", data = "<data>")]
pub async fn modify_device(
    session: SessionInfo,
    id: i64,
    data: Json<Device>,
    appstate: &State<AppState>,
) -> ApiResult {
    debug!("Modifying device with id: {}", id);
    let mut device = device_for_admin_or_self(&appstate.pool, &session, id).await?;

    // FIXME: hard-coded network id
    if let Ok(Some(network)) = WireguardNetwork::find_by_id(&appstate.pool, 1).await {
        if network.pubkey == data.wireguard_pubkey {
            return Ok(ApiResponse {
                json: json!({"msg": "device's pubkey must be differnet from server's pubkey"}),
                status: Status::BadRequest,
            });
        }

        device.update_from(data.into_inner());
        device.save(&appstate.pool).await?;
        appstate.send_wireguard_event(GatewayEvent::DeviceModified(device.clone()));
        Ok(ApiResponse {
            json: json!(device),
            status: Status::Ok,
        })
    } else {
        error!("No network found can't add device");
        Ok(ApiResponse {
            json: json!({}),
            status: Status::BadRequest,
        })
    }
}

#[get("/device/<id>", format = "json")]
pub async fn get_device(session: SessionInfo, id: i64, appstate: &State<AppState>) -> ApiResult {
    debug!("Retrieving device with id: {}", id);
    let device = device_for_admin_or_self(&appstate.pool, &session, id).await?;
    Ok(ApiResponse {
        json: json!(device),
        status: Status::Ok,
    })
}

#[delete("/device/<id>")]
pub async fn delete_device(session: SessionInfo, id: i64, appstate: &State<AppState>) -> ApiResult {
    debug!("Removing device with id: {}", id);
    let device = device_for_admin_or_self(&appstate.pool, &session, id).await?;
    let device_pubkey = device.wireguard_pubkey.clone();
    device.delete(&appstate.pool).await?;
    appstate.send_wireguard_event(GatewayEvent::DeviceDeleted(device_pubkey));
    info!("Removed device with id: {}", id);
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
    _session: SessionInfo,
    appstate: &State<AppState>,
    username: &str,
) -> ApiResult {
    debug!("Listing devices for user: {}", username);
    let devices = Device::all_for_username(&appstate.pool, username).await?;
    info!("Listed devices for user: {}", username);

    Ok(ApiResponse {
        json: json!(devices),
        status: Status::Ok,
    })
}

// FIXME: conflicts with /device/user/<username>
#[get("/device/<id>/config", rank = 2, format = "json")]
pub async fn download_config(
    session: SessionInfo,
    appstate: &State<AppState>,
    id: i64,
) -> Result<String, OriWebError> {
    let network = find_network(1, &appstate.pool).await?;
    let device = device_for_admin_or_self(&appstate.pool, &session, id).await?;

    Ok(device.create_config(network))
}

#[get("/token/<id>", format = "json")]
pub async fn create_network_token(
    _admin: AdminRole,
    appstate: &State<AppState>,
    id: i64,
) -> ApiResult {
    let network = find_network(id, &appstate.pool).await?;
    let token = Claims::new(
        ClaimsType::Gateway,
        network.name.clone(),
        String::new(),
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
        json: json!({ "token": token }),
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

#[get("/stats/users?<from>", format = "json")]
pub async fn user_stats(
    _admin: AdminRole,
    appstate: &State<AppState>,
    from: Option<String>,
) -> ApiResult {
    debug!("Displaying wireguard user stats");
    let from = parse_timestamp(from)?.naive_utc();
    let aggregation = get_aggregation(from)?;
    let stats = WireguardNetwork::user_stats(&appstate.pool, &from, &aggregation).await?;
    info!("Displayed wireguard user stats");

    Ok(ApiResponse {
        json: json!(stats),
        status: Status::Ok,
    })
}

#[get("/stats?<from>", format = "json")]
pub async fn network_stats(
    _admin: AdminRole,
    appstate: &State<AppState>,
    from: Option<String>,
) -> ApiResult {
    debug!("Displaying wireguard network stats");
    let from = parse_timestamp(from)?.naive_utc();
    let aggregation = get_aggregation(from)?;
    let stats = WireguardNetwork::network_stats(&appstate.pool, &from, &aggregation).await?;
    info!("Displayed wireguard network stats");

    Ok(ApiResponse {
        json: json!(stats),
        status: Status::Ok,
    })
}

#[get("/connection", format = "json")]
pub async fn connection_info(
    _admin: AdminRole,
    gateway_state: &State<Arc<Mutex<GatewayState>>>,
) -> ApiResult {
    debug!("Checking gateway connection info");
    let info = ConnectionInfo {
        connected: gateway_state.lock().unwrap().connected,
    };
    info!("Checked gateway connection info");

    Ok(ApiResponse {
        json: json!(info),
        status: Status::Ok,
    })
}
