use std::{
    net::{AddrParseError, IpAddr, Ipv4Addr, Ipv6Addr},
    str::FromStr,
};

use axum::{
    extract::{Json, Path, State},
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
            wireguard::NetworkAddressError,
        },
    },
};
use defguard_mail::templates::{TemplateLocation, new_device_added_mail};
use ipnetwork::IpNetwork;
use serde_json::json;
use sqlx::PgConnection;

use super::{ApiResponse, ApiResult, WebError};
use crate::{
    appstate::AppState,
    auth::{AdminRole, SessionInfo},
    enrollment_management::start_desktop_configuration,
    enterprise::{firewall::try_get_location_firewall_config, limits::update_counts},
    events::{ApiEvent, ApiEventType, ApiRequestContext},
    grpc::GatewayEvent,
    server_config,
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
        let split_ips: Vec<SplitIp> = wireguard_device
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
        Ok(NetworkDeviceInfo {
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
            return Ok(ApiResponse::json(network_device_info, StatusCode::OK));
        }
    }
    error!(
        "Failed to retrieve network device with id: {device_id}, such network device doesn't exist."
    );
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
    Ok(ApiResponse::json(devices_response, StatusCode::OK))
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

#[derive(Deserialize)]
pub struct IpAvailabilityCheck {
    ips: Vec<String>,
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

pub(crate) async fn check_ip_availability(
    _admin_role: AdminRole,
    Path(network_id): Path<i64>,
    State(appstate): State<AppState>,
    Json(check): Json<IpAvailabilityCheck>,
) -> ApiResult {
    let mut transaction = appstate.pool.begin().await?;

    // fetch relevant WireGuard location
    let location = WireguardNetwork::find_by_id(&appstate.pool, network_id)
        .await?
        .ok_or_else(|| {
            error!(
                "Failed to check IP availability for location with ID {network_id}, location not found",

            );
            WebError::BadRequest("Failed to check IP availability, location not found".into())
        })?;

    // process IPs one by one and preserve order in response
    let mut validation_results = Vec::new();
    for ip in &check.ips {
        match IpAddr::from_str(ip) {
            Ok(ip) => {
                debug!(
                    "Checking if IP address {ip} can be assigned to a device in location {location}",
                );
                let result = match location.can_assign_ips(&mut transaction, &[ip], None).await {
                    Ok(()) => IpAvailabilityCheckResult::new(true, true),
                    Err(NetworkAddressError::NoContainingNetwork(name, ip, networks)) => {
                        warn!(
                            "Provided device IP address {ip} is not in the network {name} range: {networks:?}"
                        );
                        IpAvailabilityCheckResult::new(false, false)
                    }
                    Err(NetworkAddressError::ReservedForGateway(name, ip)) => {
                        warn!(
                            "Provided device IP address {ip} may overlap with the gateway's IP address on network {name}",
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
                    "Failed to check IP availability for location {location}, invalid IP address {ip}",
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

pub(crate) async fn find_available_ips(
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
    let mut split_ips = Vec::new();
    for network_address in &network.address {
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
    if split_ips.len() == network.address.len() {
        debug!(
            "Found addresses {:?} for new device i network {} ({:?})",
            split_ips, network.name, network.address
        );
        Ok(ApiResponse::json(split_ips, StatusCode::OK))
    } else {
        warn!(
            "Failed to find available IPs for new device in network {} ({:?})",
            network.name, network.address
        );
        Ok(ApiResponse::with_status(StatusCode::NOT_FOUND))
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StartNetworkDeviceSetup {
    name: String,
    description: Option<String>,
    location_id: i64,
    assigned_ips: Vec<String>,
}

impl From<NetworkAddressError> for WebError {
    fn from(error: NetworkAddressError) -> Self {
        WebError::BadRequest(error.to_string())
    }
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
    let settings = Settings::get_current_settings();
    let configuration_token = start_desktop_configuration(
        &user,
        &mut transaction,
        &user,
        None,
        config.enrollment_token_timeout.as_secs(),
        settings.proxy_public_url()?.clone(),
        false,
        Some(result.device.id),
    )
    .await?;

    debug!(
        "Generated a new device CLI configuration token for a network device {device_name} with ID {}: {configuration_token}",
        result.device.id
    );

    update_counts(&mut *transaction).await?;

    transaction.commit().await?;

    Ok(ApiResponse::new(
        json!({"enrollment_token": configuration_token, "enrollment_url":  settings.proxy_public_url()?.to_string()}),
        StatusCode::CREATED,
    ))
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
    let settings = Settings::get_current_settings();
    let configuration_token = start_desktop_configuration(
        &user,
        &mut transaction,
        &user,
        None,
        config.enrollment_token_timeout.as_secs(),
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
    network.can_assign_ips(&mut transaction, &ips, None).await?;

    let (network_info, config) = device
        .add_to_network(&network, &ips, &mut transaction)
        .await?;

    appstate.send_wireguard_event(GatewayEvent::DeviceCreated(DeviceInfo {
        device: device.clone(),
        network_info: vec![network_info.clone()],
    }));

    update_counts(&mut *transaction).await?;

    // send firewall update event if ACLs & enterprise features are enabled
    if let Some(firewall_config) =
        try_get_location_firewall_config(&network, &mut transaction).await?
    {
        appstate.send_wireguard_event(GatewayEvent::FirewallConfigChanged(
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

#[derive(Debug, Deserialize)]
pub struct ModifyNetworkDevice {
    name: String,
    description: Option<String>,
    assigned_ips: Vec<IpAddr>,
}

pub async fn modify_network_device(
    _admin_role: AdminRole,
    session: SessionInfo,
    context: ApiRequestContext,
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
    // store device before modifications
    let before = device.clone();
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
        appstate.send_wireguard_event(GatewayEvent::DeviceModified(device_info));

        // send firewall update event if ACLs are enabled
        if device_network.acl_enabled {
            if let Some(firewall_config) =
                try_get_location_firewall_config(&device_network, &mut transaction).await?
            {
                appstate.send_wireguard_event(GatewayEvent::FirewallConfigChanged(
                    device_network.id,
                    firewall_config,
                ));
            }
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
        network_part.push_str(&formatted);
        network_part.push_str(delimiter);
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
