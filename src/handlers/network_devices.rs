use std::{
    iter::zip,
    net::{AddrParseError, IpAddr, Ipv4Addr, Ipv6Addr},
    str::FromStr,
};

use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
};
use chrono::NaiveDateTime;
use ipnetwork::IpNetwork;
use serde_json::json;
use sqlx::PgConnection;

use super::{ApiResponse, ApiResult, WebError};
use crate::{
    appstate::AppState,
    auth::{AdminRole, SessionInfo},
    db::{
        models::device::{DeviceConfig, DeviceInfo, DeviceType, WireguardNetworkDevice},
        Device, GatewayEvent, Id, User, WireguardNetwork,
    },
    enterprise::limits::update_counts,
    handlers::mail::send_new_device_added_email,
    server_config,
    templates::TemplateLocation,
    CommaSeparated,
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
        let network = device
            .find_network_device_networks(&mut *transaction)
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
        let net_addr = network
            .address
            .first()
            .ok_or(WebError::ObjectNotFound(format!(
                "Failed to find the network address for network {}",
                network.name
            )))?;
        // TODO(jck) deal with all ips
        // let split_ip = split_ip(
        //     wireguard_device
        //         .wireguard_ip
        //         .first()
        //         .expect("missing NetworkDevice IP"),
        //     net_addr,
        // );
        let split_ips = wireguard_device
            .wireguard_ip
            .iter()
            .map(|ip| split_ip(ip, net_addr))
            .collect::<Vec<SplitIp>>();
        Ok(NetworkDeviceInfo {
            id: device.id,
            name: device.name,
            assigned_ips: wireguard_device.wireguard_ip,
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

pub async fn download_network_device_config(
    _admin_role: AdminRole,
    State(appstate): State<AppState>,
    Path(device_id): Path<i64>,
) -> Result<String, WebError> {
    debug!("Creating a WireGuard config for network device {device_id}.");
    let device =
        Device::find_by_id(&appstate.pool, device_id)
            .await?
            .ok_or(WebError::ObjectNotFound(format!(
                "Network device with ID {device_id} not found"
            )))?;
    let network = device
        .find_network_device_networks(&appstate.pool)
        .await?
        .pop()
        .ok_or(WebError::ObjectNotFound(format!(
            "No network found for network device: {}({})",
            device.name, device.id
        )))?;
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
    Ok(Device::create_config(&network, &network_device))
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
    _admin_role: AdminRole,
    State(appstate): State<AppState>,
) -> ApiResult {
    debug!("Listing all network devices");
    let mut devices_response: Vec<NetworkDeviceInfo> = vec![];
    let mut transaction = appstate.pool.begin().await?;
    let devices = Device::find_by_type(&mut *transaction, DeviceType::Network).await?;
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
    transaction.commit().await?;

    info!("Listed {} network devices", devices_response.len());
    Ok(ApiResponse {
        json: json!(devices_response),
        status: StatusCode::OK,
    })
}

#[derive(Serialize, Deserialize, Debug)]
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

/// Checks if the IP addresses fall into the range of the network
/// and if they are not already assigned to another device.
async fn check_ips(
    ip_addrs: &[IpAddr],
    network: &WireguardNetwork<Id>,
    transaction: &mut PgConnection,
) -> Result<(), WebError> {
    // if let Some(network_address) = network.address.first() {
    //     if !network_address.contains(ip_addr) {
    //         return Err(WebError::BadRequest(format!(
    //             "Provided IP address {ip_addr} is not in the network ({}) range {network_address}",
    //             network.name,
    //         )));
    //     }
    //     if ip_addr == network_address.network() || ip_addr == network_address.broadcast() {
    //         return Err(WebError::BadRequest(format!(
    //             "Provided IP address {ip_addr} is network or broadcast address of network {}",
    //             network.name
    //         )));
    //     }
    //     if ip_addr == network_address.ip() {
    //         return Err(WebError::BadRequest(format!(
    //             "Provided IP address {ip_addr} may overlap with the network's gateway IP in network {}",
    //             network.name
    //         )));
    //     }

    //     let device = Device::find_by_ip(transaction, ip_addr, network.id).await?;
    //     if let Some(device) = device {
    //         return Err(WebError::BadRequest(format!(
    //             "Provided IP address {ip_addr} is already assigned to device {} in network {}",
    //             device.name, network.name
    //         )));
    //     }
    // }

    let networks = ip_addrs
        .iter()
        .map(|ip| network.get_containing_network(*ip).ok_or(()))
        .collect::<Result<Vec<IpNetwork>, ()>>()
        .map_err(|_| {
            WebError::BadRequest(format!(
                // "Provided IP address {ip_addrs} is not in the network ({}) range {network_address}",
                "Provided IP addresses {ip_addrs:?} are not in the network ({}) range {:?}",
                network.name, network.address,
            ))
        })?;
    // if !network.contains_all(ip_addrs) {
    //     return Err(WebError::BadRequest(format!(
    //         // "Provided IP address {ip_addrs} is not in the network ({}) range {network_address}",
    //         "Provided IP addresses {ip_addrs} are not in the network ({}) range {:?}",
    //         network.name, network.address,
    //     )));
    // }
    for (ip, network_address) in zip(ip_addrs, networks) {
        // if !network_address.contains(ip_addrs) {
        //     return Err(WebError::BadRequest(format!(
        //         "Provided IP address {ip_addrs} is not in the network ({}) range {network_address}",
        //         network.name,
        //     )));
        // }
        let net_ip = network_address.ip();
        let net_network = network_address.network();
        let net_broadcast = network_address.broadcast();
        if *ip == net_network || *ip == net_broadcast {
            return Err(WebError::BadRequest(format!(
                "Provided IP address {ip} is network or broadcast address of network {}",
                network.name
            )));
        }
        if *ip == net_ip {
            return Err(WebError::BadRequest(format!(
                "Provided IP address {ip} may overlap with the network's gateway IP {net_ip} in network {}",
                network.name
            )));
        }

        let device = Device::find_by_ip(&mut *transaction, *ip, network.id).await?;
        if let Some(device) = device {
            return Err(WebError::BadRequest(format!(
                "Provided IP address {ip} is already assigned to device {} in network {}",
                device.name, network.name
            )));
        }
    }

    Ok(())
}

#[derive(Deserialize)]
pub struct IpAvailabilityCheck {
    ip: String,
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
                "Failed to check IP availability for network with ID {network_id}, network not found",

            );
            WebError::BadRequest("Failed to check IP availability, network not found".into())
        })?;

    let Ok(ip) = IpAddr::from_str(&ip.ip) else {
        warn!(
            "Failed to check IP availability for network with ID {network_id}, invalid IP address",
        );
        return Ok(ApiResponse {
            json: json!({
                "available": false,
                "valid": false,
            }),
            status: StatusCode::OK,
        });
    };

    if let Some(network_address) = network.address.first() {
        if !network_address.contains(ip) {
            warn!(
                "Provided device IP address is not in the network ({}) range {network_address}",
                network.name
            );
            return Ok(ApiResponse {
                json: json!({
                    "available": false,
                    "valid": false,
                }),
                status: StatusCode::OK,
            });
        }
        if ip == network_address.network() || ip == network_address.broadcast() {
            warn!(
                "Provided device IP address is network or broadcast address of network {}",
                network.name
            );
            return Ok(ApiResponse {
                json: json!({
                    "available": false,
                    "valid": true,
                }),
                status: StatusCode::OK,
            });
        }
        if ip == network_address.ip() {
            warn!(
                "Provided device IP address may overlap with the gateway's IP address on network {}",
                network.name
            );
            return Ok(ApiResponse {
                json: json!({
                    "available": false,
                    "valid": true,
                }),
                status: StatusCode::OK,
            });
        }
    }

    if let Some(device) = Device::find_by_ip(&mut *transaction, ip, network.id).await? {
        warn!(
            "Provided device IP is already assigned to device {} in network {}",
            device.name, network.name
        );
        Ok(ApiResponse {
            json: json!({
                "available": false,
                "valid": true,
            }),
            status: StatusCode::OK,
        })
    } else {
        Ok(ApiResponse {
            json: json!({
                "available": true,
                "valid": true,
            }),
            status: StatusCode::OK,
        })
    }
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
    if let Some(network_address) = network.address.first() {
        let net_ip = network_address.ip();
        let net_network = network_address.network();
        let net_broadcast = network_address.broadcast();
        for ip in network_address {
            if ip == net_ip || ip == net_network || ip == net_broadcast {
                continue;
            }

            // Break loop if IP is unassigned and return network device
            if Device::find_by_ip(&mut *transaction, ip, network.id)
                .await?
                .is_none()
            {
                let split_ip = split_ip(&ip, network_address);
                transaction.commit().await?;
                return Ok(ApiResponse {
                    json: json!(split_ip),
                    status: StatusCode::OK,
                });
            }
        }
    }

    Ok(ApiResponse {
        json: json!({}),
        status: StatusCode::NOT_FOUND,
    })
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StartNetworkDeviceSetup {
    name: String,
    description: Option<String>,
    location_id: i64,
    assigned_ips: Vec<String>,
}

// Setup a network device to be later configured by a CLI client
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
            WebError::BadRequest("Failed to add device, network not found".to_string())
        })?;

    debug!(
        "Identified network location with ID {} as {}",
        setup_start.location_id, network.name
    );

    let mut transaction = appstate.pool.begin().await?;
    let device = Device::new(
        setup_start.name,
        "NOT_CONFIGURED".to_string(),
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

    // let ips: IpAddr = setup_start.assigned_ips.parse().map_err(|e| {
    //     error!("Failed to add network device {device_name}, invalid IP address: {e}");
    //     WebError::BadRequest("Invalid IP address".to_string())
    // })?;
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

    check_ips(&ips, &network, &mut transaction).await?;

    let (_, config) = device
        .add_to_network(&network, &ips, &mut transaction)
        .await?;

    info!(
        "User {} added a new unconfigured network device {device_name} with IPs {ips:?} to network {}",
        user.username, network.name
    );

    let result = AddNetworkDeviceResult {
        config,
        device: NetworkDeviceInfo::from_device(device, &mut transaction).await?,
    };
    let config = server_config();
    let configuration_token = user
        .start_remote_desktop_configuration(
            &mut transaction,
            &user,
            None,
            config.enrollment_token_timeout.as_secs(),
            config.enrollment_url.clone(),
            false,
            appstate.mail_tx.clone(),
            Some(result.device.id),
        )
        .await?;

    debug!(
        "Generated a new device CLI configuration token for a network device {device_name} with ID {}: {configuration_token}",
        result.device.id
    );

    update_counts(&mut *transaction).await?;

    transaction.commit().await?;

    Ok(ApiResponse {
        json: json!({"enrollment_token": configuration_token, "enrollment_url":  config.enrollment_url.to_string()}),
        status: StatusCode::CREATED,
    })
}

// Make a new CLI configuration token for an already added network device
pub(crate) async fn start_network_device_setup_for_device(
    _admin_role: AdminRole,
    session: SessionInfo,
    Path(device_id): Path<i64>,
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
    let config = server_config();
    let configuration_token = user
        .start_remote_desktop_configuration(
            &mut transaction,
            &user,
            None,
            config.enrollment_token_timeout.as_secs(),
            config.enrollment_url.clone(),
            false,
            appstate.mail_tx.clone(),
            Some(device.id),
        )
        .await?;
    transaction.commit().await?;

    debug!(
        "Generated a new device CLI configuration token for already existing network
        device {} with ID {}: {configuration_token}",
        device.name, device.id
    );
    Ok(ApiResponse {
        json: json!({
            "enrollment_token": configuration_token,
            "enrollment_url": config.enrollment_url.to_string()
        }),
        status: StatusCode::CREATED,
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
    check_ips(&ips, &network, &mut transaction).await?;

    let (network_info, config) = device
        .add_to_network(&network, &ips, &mut transaction)
        .await?;

    appstate.send_wireguard_event(GatewayEvent::DeviceCreated(DeviceInfo {
        device: device.clone(),
        network_info: vec![network_info.clone()],
    }));

    update_counts(&mut *transaction).await?;

    // send firewall update event if ACLs & enterprise features are enabled
    if let Some(firewall_config) = network.try_get_firewall_config(&mut transaction).await? {
        appstate.send_wireguard_event(GatewayEvent::FirewallConfigChanged(
            network.id,
            firewall_config,
        ));
    }

    let template_locations = vec![TemplateLocation {
        name: config.network_name.clone(),
        assigned_ip: config.address.comma_separated(),
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

    Ok(ApiResponse {
        json: json!(result),
        status: StatusCode::CREATED,
    })
}

#[derive(Debug, Deserialize)]
pub struct ModifyNetworkDevice {
    name: String,
    description: Option<String>,
    assigned_ips: Vec<String>,
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
        .find_network_device_networks(&mut *transaction)
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
    let new_ips = data
        .assigned_ips
        .iter()
        .map(|ip| IpAddr::from_str(ip))
        .collect::<Result<Vec<IpAddr>, AddrParseError>>()
        .map_err(|e| {
            let msg = format!("Failed to update device {device_id}, invalid IP address: {e}");
            error!(msg);
            WebError::BadRequest(msg)
        })?;

    device.name = data.name;
    device.description = data.description;
    device.save(&mut *transaction).await?;

    // IP address has changed, so remove device from network and add it again with new IP address.
    // TODO(jck) order-insensitive comparison
    if new_ips != *wireguard_network_device.wireguard_ip {
        check_ips(&new_ips, &device_network, &mut transaction).await?;
        // TODO(jck)
        wireguard_network_device.wireguard_ip = new_ips.clone();
        wireguard_network_device.update(&mut *transaction).await?;
        let device_info = DeviceInfo::from_device(&mut *transaction, device.clone()).await?;
        appstate.send_wireguard_event(GatewayEvent::DeviceModified(device_info));

        // send firewall update event if ACLs are enabled
        if device_network.acl_enabled {
            if let Some(firewall_config) = device_network
                .try_get_firewall_config(&mut transaction)
                .await?
            {
                appstate.send_wireguard_event(GatewayEvent::FirewallConfigChanged(
                    device_network.id,
                    firewall_config,
                ));
            }
        }

        info!(
            "User {} changed IP addresses of network device {} from {} to {new_ips:?} in network {}",
            session.user.username,
            device.name,
            // TODO(jck)
            wireguard_network_device.wireguard_ip.comma_separated(),
            device_network.name
        );
    }

    let network_device_info = NetworkDeviceInfo::from_device(device, &mut transaction).await?;
    transaction.commit().await?;

    Ok(ApiResponse {
        json: json!(network_device_info),
        status: StatusCode::OK,
    })
}

#[derive(Debug, Serialize)]
struct SplitIp {
    network_part: String,
    modifiable_part: String,
    network_prefix: String,
    ip: String,
}

/// Splits the IP address (IPv4 or IPv6) into three parts: network part, modifiable part and prefix
/// The network part is the part that can't be changed by the user.
/// This is to display an IP address in the UI like this: 192.168.(1.1)/16, where the part in the parenthesis can be changed by the user.
/// The algorithm works as follows:
/// 1. Get the network address, last address and IP address segments, e.g. 192.1.1.1 would be [192, 1, 1, 1]
/// 2. Iterate over the segments and compare the last address and network segments, as long as the current segments are equal, append the segment to the network part.
///    If they are not equal, we found the first modifiable segment (one of the segments of an address that may change between hosts in the same network),
///    append the rest of the segments to the modifiable part.
/// 3. Join the segments with the delimiter and return the network part, modifiable part and the network prefix
fn split_ip(ip: &IpAddr, network: &IpNetwork) -> SplitIp {
    let network_addr = network.network();
    let network_prefix = network.prefix();

    let ip_segments = match ip {
        IpAddr::V4(ip) => ip.octets().iter().map(|x| u16::from(*x)).collect(),
        IpAddr::V6(ip) => ip.segments().to_vec(),
    };

    let last_addr_segments = match network {
        IpNetwork::V4(net) => {
            let last_ip = u32::from(net.ip()) | (!u32::from(net.mask()));
            let last_ip: Ipv4Addr = last_ip.into();
            last_ip.octets().iter().map(|x| u16::from(*x)).collect()
        }
        IpNetwork::V6(net) => {
            let last_ip = u128::from(net.ip()) | (!u128::from(net.mask()));
            let last_ip: Ipv6Addr = last_ip.into();
            last_ip.segments().to_vec()
        }
    };

    let network_segments = match network_addr {
        IpAddr::V4(ip) => ip.octets().iter().map(|x| u16::from(*x)).collect(),
        IpAddr::V6(ip) => ip.segments().to_vec(),
    };

    let mut network_part = String::new();
    let mut modifiable_part = String::new();
    let delimiter = if ip.is_ipv4() { "." } else { ":" };
    let formatter = |x: &u16| {
        if ip.is_ipv4() {
            x.to_string()
        } else {
            format!("{x:04x}")
        }
    };

    for (i, ((last_addr_segment, network_segment), ip_segment)) in last_addr_segments
        .iter()
        .zip(network_segments.iter())
        .zip(ip_segments.iter())
        .enumerate()
    {
        if last_addr_segment != network_segment {
            let parts = ip_segments.split_at(i).1;
            let joined = parts
                .iter()
                .map(formatter)
                .collect::<Vec<String>>()
                .join(delimiter);
            modifiable_part.push_str(&joined);
            break;
        }
        let formatted = formatter(ip_segment);
        network_part.push_str(&format!("{formatted}{delimiter}"));
    }

    SplitIp {
        ip: ip.to_string(),
        network_part,
        modifiable_part,
        network_prefix: network_prefix.to_string(),
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_ip_splitter() {
        let net = split_ip(
            &IpAddr::from_str("192.168.3.1").unwrap(),
            &IpNetwork::from_str("192.168.3.1/30").unwrap(),
        );

        assert_eq!(net.network_part, "192.168.3.");
        assert_eq!(net.modifiable_part, "1");
        assert_eq!(net.network_prefix, "30");

        let net = split_ip(
            &IpAddr::from_str("192.168.5.7").unwrap(),
            &IpNetwork::from_str("192.168.3.1/24").unwrap(),
        );

        assert_eq!(net.network_part, "192.168.5.");
        assert_eq!(net.modifiable_part, "7");
        assert_eq!(net.network_prefix, "24");

        let net = split_ip(
            &IpAddr::from_str("2001:0db8:85a3::8a2e:0370:7334").unwrap(),
            &IpNetwork::from_str("2001:0db8:85a3::8a2e:0370:7334/64").unwrap(),
        );

        assert_eq!(net.network_part, "2001:0db8:85a3:0000:");
        assert_eq!(net.modifiable_part, "0000:8a2e:0370:7334");
        assert_eq!(net.network_prefix, "64");

        let net = split_ip(
            &IpAddr::from_str("2001:0db8::0010:8a2e:0370:aaaa").unwrap(),
            &IpNetwork::from_str("2001:db8::10:8a2e:370:aaa8/125").unwrap(),
        );

        assert_eq!(net.network_part, "2001:0db8:0000:0000:0010:8a2e:0370:");
        assert_eq!(net.modifiable_part, "aaaa");
        assert_eq!(net.network_prefix, "125");
    }
}
