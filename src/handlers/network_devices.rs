use std::{net::IpAddr, str::FromStr};

use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
};
use chrono::NaiveDateTime;
use sqlx::PgConnection;

use super::{ApiResponse, ApiResult, WebError};
use crate::{
    appstate::AppState,
    auth::{AdminRole, SessionInfo},
    db::{
        models::device::{
            DeviceConfig, DeviceInfo, DeviceNetworkInfo, DeviceType, ModifyDevice,
            WireguardNetworkDevice,
        },
        Device, GatewayEvent, Id, WireguardNetwork,
    },
    enterprise::limits::update_counts,
    handlers::mail::send_new_device_added_email,
    templates::TemplateLocation,
};

#[derive(Serialize)]
struct NetworkDeviceLocation {
    id: Id,
    name: String,
}

#[derive(Serialize)]
struct NetworkDeviceInfo {
    id: Id,
    name: String,
    assigned_ip: IpAddr,
    description: Option<String>,
    added_by: String,
    added_date: NaiveDateTime,
    location: NetworkDeviceLocation,
}

impl NetworkDeviceInfo {
    async fn from_device(
        device: Device<Id>,
        transaction: &mut PgConnection,
    ) -> Result<Self, WebError> {
        let mut wireguard_devices =
            WireguardNetworkDevice::find_by_device(&mut *transaction, device.id)
                .await?
                .ok_or_else(|| {
                    WebError::ObjectNotFound(format!(
                        "Failed to find the network with which the network device {} is associated",
                        device.name
                    ))
                })?;
        if wireguard_devices.len() > 1 {
            warn!(
                "Found multiple networks for a network device with ID {}, picking the last one",
                device.id
            );
        }
        let wireguard_device = wireguard_devices.pop().ok_or_else(|| {
            WebError::ObjectNotFound(format!(
                "Failed to find the network with which the network device {} is associated",
                device.name
            ))
        })?;

        let added_by = device.get_owner(&mut *transaction).await?;
        let network = device
            .find_device_networks(&mut *transaction)
            .await?
            .pop()
            .ok_or_else(|| {
                WebError::ObjectNotFound(format!(
                    "Failed to find the network with which the network device {} is associated",
                    device.name
                ))
            })?;

        Ok(NetworkDeviceInfo {
            id: device.id,
            name: device.name,
            assigned_ip: wireguard_device.wireguard_ip,
            description: device.description,
            added_by: added_by.username,
            added_date: device.created,
            location: NetworkDeviceLocation {
                id: wireguard_device.wireguard_network_id,
                name: network.name,
            },
        })
    }
}

pub async fn get_network_device(
    _admin_role: AdminRole,
    session: SessionInfo,
    Path(device_id): Path<i64>,
    State(appstate): State<AppState>,
) -> ApiResult {
    debug!(
        "User {} is retrieving network device with id: {device_id}",
        session.user.username
    );

    let device = Device::find_by_id(&appstate.pool, device_id).await?;
    if let Some(device) = device {
        if device.device_type == DeviceType::Network {
            let mut transaction = appstate.pool.begin().await?;
            let network_device_info =
                NetworkDeviceInfo::from_device(device, &mut transaction).await?;
            transaction.commit().await?;
            return Ok(ApiResponse {
                json: json!(network_device_info),
                status: StatusCode::OK,
            });
        }
    }
    error!("Failed to retrieve network device with id: {device_id}, such network device doesn't exist.");
    Err(WebError::ObjectNotFound(format!(
        "Network device with ID {device_id} not found"
    )))
}

pub(crate) async fn list_network_devices(
    session: SessionInfo,
    State(appstate): State<AppState>,
) -> ApiResult {
    // only allow for admin or user themselves
    if !session.is_admin || !session.user.is_active {
        warn!(
            "User {} tried to list network devices, but is not an admin",
            session.user.username
        );
        return Err(WebError::Forbidden("Admin access required".into()));
    };
    debug!("Listing all network devices");
    let mut devices_response: Vec<NetworkDeviceInfo> = vec![];
    let mut transaction = appstate.pool.begin().await?;
    let devices = Device::find_by_type(&mut *transaction, DeviceType::Network).await?;
    for device in devices {
        let network_device_info = NetworkDeviceInfo::from_device(device, &mut transaction).await?;
        devices_response.push(network_device_info);
    }
    transaction.commit().await?;

    info!("Listed {} network devices", devices_response.len());
    Ok(ApiResponse {
        json: json!(devices_response),
        status: StatusCode::OK,
    })
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AddNetworkDevice {
    name: String,
    description: Option<String>,
    location_id: i64,
    assigned_ip: String,
    wireguard_pubkey: String,
}

#[derive(Serialize)]
pub struct AddNetworkDeviceResult {
    config: DeviceConfig,
    device: NetworkDeviceInfo,
}

/// Checks if the IP falls into the range of the network
/// and if it is not already assigned to another device
async fn check_ip(
    ip: IpAddr,
    network: &WireguardNetwork<Id>,
    transaction: &mut PgConnection,
) -> Result<(), WebError> {
    let network_address = network.address;
    if !network_address.contains(ip) {
        let msg = format!(
            "Invalid IP address {ip}. It's not in the network {} range {network_address}",
            network.name,
        );
        error!(msg);
        return Err(WebError::BadRequest(msg));
    }

    let device = Device::find_by_ip(transaction, ip, network.id).await?;
    if let Some(device) = device {
        let msg = format!(
            "Invalid IP address {ip}. It's already assigned to device {} in network {}",
            device.name, network.name
        );
        error!(msg);
        return Err(WebError::BadRequest(msg));
    }

    Ok(())
}

#[derive(Deserialize)]
pub struct IpAvailabilityCheck {
    ip: IpAddr,
}

pub(crate) async fn check_ip_availability(
    _admin_role: AdminRole,
    Path(network_id): Path<i64>,
    State(appstate): State<AppState>,
    Json(ip): Json<IpAvailabilityCheck>,
) -> ApiResult {
    let mut transaction = appstate.pool.begin().await?;
    let network = WireguardNetwork::find_by_id(&appstate.pool, network_id)
        .await?
        .ok_or_else(|| {
            error!(
                "Failed to check IP availability for network with ID {}, network not found",
                network_id
            );
            WebError::BadRequest("Failed to check IP availability, network not found".to_string())
        })?;
    check_ip(ip.ip, &network, &mut transaction).await?;
    transaction.commit().await?;
    Ok(ApiResponse {
        json: json!({}),
        status: StatusCode::OK,
    })
}

pub(crate) async fn find_available_ip(
    _admin_role: AdminRole,
    Path(network_id): Path<i64>,
    State(appstate): State<AppState>,
) -> ApiResult {
    let network = WireguardNetwork::find_by_id(&appstate.pool, network_id)
        .await?
        .ok_or_else(|| {
            error!(
                "Failed to find available IP for network with ID {}",
                network_id
            );
            WebError::BadRequest("Failed to find available IP, network not found".to_string())
        })?;

    let mut transaction = appstate.pool.begin().await?;
    let net_ip = network.address.ip();
    let net_network = network.address.network();
    let net_broadcast = network.address.broadcast();
    for ip in &network.address {
        if ip == net_ip || ip == net_network || ip == net_broadcast {
            continue;
        }

        // Break loop if IP is unassigned and return network device
        if Device::find_by_ip(&mut *transaction, ip, network.id)
            .await?
            .is_none()
        {
            let ip = ip.to_string();
            transaction.commit().await?;
            return Ok(ApiResponse {
                json: json!({ "ip": ip }),
                status: StatusCode::OK,
            });
        }
    }

    Ok(ApiResponse {
        json: json!({}),
        status: StatusCode::NOT_FOUND,
    })
}

pub(crate) async fn add_network_device(
    _admin_role: AdminRole,
    session: SessionInfo,
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
            WebError::BadRequest("Failed to add device, network not found".to_string())
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
    )
    .save(&mut *transaction)
    .await?;

    let ip: IpAddr = add_network_device.assigned_ip.parse().map_err(|e| {
        error!("Failed to add network device {device_name}, invalid IP address: {e}");
        WebError::BadRequest("Invalid IP address".to_string())
    })?;
    check_ip(ip, &network, &mut transaction).await?;

    let (network_info, config) = device
        .add_to_network(&network, ip, &mut transaction)
        .await?;

    appstate.send_wireguard_event(GatewayEvent::DeviceCreated(DeviceInfo {
        device: device.clone(),
        network_info: vec![network_info.clone()],
    }));

    let template_locations = vec![TemplateLocation {
        name: config.network_name.clone(),
        assigned_ip: config.address.to_string(),
    }];

    send_new_device_added_email(
        &device.name,
        &device.wireguard_pubkey,
        &template_locations,
        &user.email,
        &appstate.mail_tx,
        Some(session.session.ip_address.as_str()),
        session.session.device_info.clone().as_deref(),
    )?;

    info!(
        "User {} added a new network device {device_name}.",
        user.username
    );

    let result = AddNetworkDeviceResult {
        config,
        device: NetworkDeviceInfo::from_device(device, &mut transaction).await?,
    };

    transaction.commit().await?;
    update_counts(&appstate.pool).await?;

    Ok(ApiResponse {
        json: json!(result),
        status: StatusCode::CREATED,
    })
}

#[derive(Debug, Deserialize)]
pub struct ModifyNetworkDevice {
    name: String,
    description: Option<String>,
    wireguard_pubkey: String,
    location_id: i64,
    assigned_ip: String,
}

impl From<ModifyNetworkDevice> for ModifyDevice {
    fn from(data: ModifyNetworkDevice) -> Self {
        ModifyDevice {
            name: data.name,
            description: data.description,
            wireguard_pubkey: data.wireguard_pubkey,
            device_type: DeviceType::Network,
        }
    }
}

pub async fn modify_network_device(
    _admin_role: AdminRole,
    session: SessionInfo,
    Path(device_id): Path<i64>,
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
    let device_network = device
        .find_device_networks(&mut *transaction)
        .await?
        .pop()
        .ok_or_else(|| {
            error!("Failed to update device {device_id}, device not found in any network");
            WebError::ObjectNotFound(format!("Device {device_id} not found in any network"))
        })?;
    if device_network.pubkey == data.wireguard_pubkey {
        error!("Failed to update device {device_id}, device's pubkey must be different from server's pubkey");
        return Ok(ApiResponse {
            json: json!({"msg": "device's pubkey must be different from server's pubkey"}),
            status: StatusCode::BAD_REQUEST,
        });
    }

    let new_network_id = data.location_id;
    let new_ip = IpAddr::from_str(&data.assigned_ip).map_err(|e| {
        error!("Failed to update device {device_id}, invalid IP address: {e}");
        WebError::BadRequest("Invalid IP address".to_string())
    })?;

    // update device info
    device.update_from(data.into());
    device.save(&mut *transaction).await?;

    // network changed, remove device from old network and add to new one
    if new_network_id != device_network.id {
        let new_network = WireguardNetwork::find_by_id(&mut *transaction, new_network_id)
            .await?
            .ok_or_else(|| {
                error!(
                    "Failed to update device {device_id}, new network with ID {} not found",
                    new_network_id
                );
                WebError::BadRequest("Failed to update device, new network not found".to_string())
            })?;

        check_ip(new_ip, &new_network, &mut transaction).await?;

        device
            .remove_from_network(&device_network, &mut transaction)
            .await?;
        let (network_info, config) = device
            .add_to_network(&new_network, new_ip, &mut transaction)
            .await?;
        appstate.send_wireguard_event(GatewayEvent::DeviceModified(DeviceInfo {
            device: device.clone(),
            network_info: vec![network_info.clone()],
        }));

        let template_locations = vec![TemplateLocation {
            name: config.network_name.clone(),
            assigned_ip: config.address.to_string(),
        }];

        send_new_device_added_email(
            &device.name,
            &device.wireguard_pubkey,
            &template_locations,
            &session.user.email,
            &appstate.mail_tx,
            Some(session.session.ip_address.as_str()),
            session.session.device_info.clone().as_deref(),
        )?;

        info!(
            "User {} moved network device {} to network {}",
            session.user.username, device.name, new_network.name
        );
    } else {
        let mut network_info = Vec::new();
        let wireguard_network_device =
            WireguardNetworkDevice::find(&mut *transaction, device.id, device_network.id).await?;

        if let Some(wireguard_network_device) = wireguard_network_device {
            let device_network_info = DeviceNetworkInfo {
                network_id: device_network.id,
                device_wireguard_ip: wireguard_network_device.wireguard_ip,
                preshared_key: wireguard_network_device.preshared_key,
                is_authorized: wireguard_network_device.is_authorized,
            };
            network_info.push(device_network_info);
        }

        appstate.send_wireguard_event(GatewayEvent::DeviceModified(DeviceInfo {
            device: device.clone(),
            network_info,
        }));

        info!(
            "User {} updated network device {device_id}",
            session.user.username
        );
    }

    let network_device_info = NetworkDeviceInfo::from_device(device, &mut transaction).await?;
    transaction.commit().await?;

    Ok(ApiResponse {
        json: json!(network_device_info),
        status: StatusCode::OK,
    })
}
