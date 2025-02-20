use std::{fmt, net::IpAddr};

use base64::{prelude::BASE64_STANDARD, Engine};
use chrono::{NaiveDateTime, Utc};
use ipnetwork::IpNetwork;
use model_derive::Model;
use sqlx::{
    postgres::types::PgInterval, query, query_as, Error as SqlxError, FromRow, PgConnection,
    PgExecutor, PgPool, Type,
};
use thiserror::Error;
use utoipa::ToSchema;

use super::{
    error::ModelError,
    wireguard::{WireguardNetwork, WIREGUARD_MAX_HANDSHAKE},
};
use crate::{
    db::{Id, NoId, User},
    KEY_LENGTH,
};

#[derive(Serialize, ToSchema)]
pub struct DeviceConfig {
    pub(crate) network_id: Id,
    pub(crate) network_name: String,
    pub(crate) config: String,
    #[schema(value_type = String)]
    pub(crate) address: IpAddr,
    pub(crate) endpoint: String,
    #[schema(value_type = String)]
    pub(crate) allowed_ips: Vec<IpNetwork>,
    pub(crate) pubkey: String,
    pub(crate) dns: Option<String>,
    pub(crate) mfa_enabled: bool,
    pub(crate) keepalive_interval: i32,
}

// The type of a device:
// User: A device of a user, which may be in multiple networks, e.g. a laptop
// Network: A standalone device added by a user permamently bound to one network, e.g. a printer
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, ToSchema, Type)]
#[sqlx(type_name = "device_type", rename_all = "snake_case")]
pub enum DeviceType {
    User,
    Network,
}

impl fmt::Display for DeviceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::User => write!(f, "user"),
            Self::Network => write!(f, "network"),
        }
    }
}

impl From<DeviceType> for String {
    fn from(device_type: DeviceType) -> Self {
        device_type.to_string()
    }
}

#[derive(Clone, Debug, Deserialize, FromRow, Model, Serialize, ToSchema)]
pub struct Device<I = NoId> {
    pub id: I,
    pub name: String,
    pub wireguard_pubkey: String,
    pub user_id: Id,
    pub created: NaiveDateTime,
    #[model(enum)]
    pub device_type: DeviceType,
    pub description: Option<String>,
    /// Whether the device should be considered as setup and ready to use
    /// or does it require some additional steps to be taken. Not configured devices
    /// won't be sent to the gateway. It is assumed that an unconfigured device is already
    /// added to all networks it should be in, but it's not ready to be used yet due to
    /// e.g. public key not properly set up yet.
    pub configured: bool,
}

impl fmt::Display for Device<NoId> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl fmt::Display for Device<Id> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[ID {}] {}", self.id, self.name)
    }
}

// helper struct which includes network configurations for a given device
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DeviceInfo {
    #[serde(flatten)]
    pub device: Device<Id>,
    pub network_info: Vec<DeviceNetworkInfo>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DeviceNetworkInfo {
    pub network_id: Id,
    pub device_wireguard_ip: IpAddr,
    #[serde(skip_serializing)]
    pub preshared_key: Option<String>,
    pub is_authorized: bool,
}

impl DeviceInfo {
    pub(crate) async fn from_device<'e, E>(
        executor: E,
        device: Device<Id>,
    ) -> Result<Self, ModelError>
    where
        E: PgExecutor<'e>,
    {
        debug!("Generating device info for {device}");
        let network_info = query_as!(
            DeviceNetworkInfo,
            "SELECT wireguard_network_id network_id, wireguard_ip \"device_wireguard_ip: IpAddr\", \
            preshared_key, is_authorized \
            FROM wireguard_network_device \
            WHERE device_id = $1",
            device.id
        )
        .fetch_all(executor)
        .await?;

        Ok(Self {
            device,
            network_info,
        })
    }
}

// helper struct which includes full device info
// including network activity metadata
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct UserDevice {
    #[serde(flatten)]
    pub device: Device<Id>,
    pub networks: Vec<UserDeviceNetworkInfo>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct UserDeviceNetworkInfo {
    pub network_id: Id,
    pub network_name: String,
    pub network_gateway_ip: String,
    pub device_wireguard_ip: String,
    pub last_connected_ip: Option<String>,
    pub last_connected_location: Option<String>,
    pub last_connected_at: Option<NaiveDateTime>,
    pub is_active: bool,
}

impl UserDevice {
    pub async fn from_device(pool: &PgPool, device: Device<Id>) -> Result<Option<Self>, SqlxError> {
        // fetch device config and connection info for all networks
        let result = query!(
            "WITH stats AS ( \
                SELECT DISTINCT ON (network) network, endpoint, latest_handshake \
                FROM wireguard_peer_stats \
                WHERE device_id = $2 \
                ORDER BY network, collected_at DESC \
            ) \
            SELECT n.id network_id, n.name network_name, n.endpoint gateway_endpoint, \
            wnd.wireguard_ip \"device_wireguard_ip: IpAddr\", stats.endpoint device_endpoint, \
            stats.latest_handshake \"latest_handshake?\", \
            COALESCE((NOW() - stats.latest_handshake) < $1, FALSE) \"is_active!\" \
            FROM wireguard_network_device wnd \
            JOIN wireguard_network n ON n.id = wnd.wireguard_network_id \
            LEFT JOIN stats ON n.id = stats.network \
            WHERE wnd.device_id = $2",
            PgInterval::try_from(WIREGUARD_MAX_HANDSHAKE).unwrap(),
            device.id,
        )
        .fetch_all(pool)
        .await?;

        let networks_info: Vec<UserDeviceNetworkInfo> = result
            .into_iter()
            .map(|r| {
                // TODO: merge below enclosure with WireguardPeerStats::endpoint_without_port().
                let device_ip = r.device_endpoint.and_then(|endpoint| {
                    let mut addr = endpoint.rsplit_once(':')?.0;
                    // Strip square brackets.
                    if addr.starts_with('[') && addr.ends_with(']') {
                        let end = addr.len() - 1;
                        addr = &addr[1..end];
                    }
                    Some(addr.to_owned())
                });
                UserDeviceNetworkInfo {
                    network_id: r.network_id,
                    network_name: r.network_name,
                    network_gateway_ip: r.gateway_endpoint,
                    device_wireguard_ip: r.device_wireguard_ip.to_string(),
                    last_connected_ip: device_ip,
                    last_connected_location: None,
                    last_connected_at: r.latest_handshake,
                    is_active: r.is_active,
                }
            })
            .collect();

        Ok(Some(Self {
            device,
            networks: networks_info,
        }))
    }
}

#[derive(Clone, Debug, Deserialize, FromRow, Serialize)]
pub struct WireguardNetworkDevice {
    pub wireguard_network_id: Id,
    pub wireguard_ip: IpAddr,
    pub device_id: Id,
    pub preshared_key: Option<String>,
    pub is_authorized: bool,
    pub authorized_at: Option<NaiveDateTime>,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct AddDevice {
    pub name: String,
    pub wireguard_pubkey: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ModifyDevice {
    pub name: String,
    pub wireguard_pubkey: String,
    pub description: Option<String>,
}

impl WireguardNetworkDevice {
    #[must_use]
    pub(crate) fn new(network_id: Id, device_id: Id, wireguard_ip: IpAddr) -> Self {
        Self {
            wireguard_network_id: network_id,
            wireguard_ip,
            device_id,
            preshared_key: None,
            is_authorized: false,
            authorized_at: None,
        }
    }

    pub(crate) async fn insert<'e, E>(&self, executor: E) -> Result<(), SqlxError>
    where
        E: PgExecutor<'e>,
    {
        query!(
            "INSERT INTO wireguard_network_device \
            (device_id, wireguard_network_id, wireguard_ip, is_authorized, authorized_at, \
            preshared_key) \
            VALUES ($1, $2, $3, $4, $5, $6) \
            ON CONFLICT ON CONSTRAINT device_network \
            DO UPDATE SET wireguard_ip = $3, is_authorized = $4",
            self.device_id,
            self.wireguard_network_id,
            IpNetwork::from(self.wireguard_ip.clone()),
            self.is_authorized,
            self.authorized_at,
            self.preshared_key
        )
        .execute(executor)
        .await?;

        Ok(())
    }

    pub(crate) async fn update<'e, E>(&self, executor: E) -> Result<(), SqlxError>
    where
        E: PgExecutor<'e>,
    {
        query!(
            "UPDATE wireguard_network_device \
            SET wireguard_ip = $3, is_authorized = $4, authorized_at = $5, preshared_key = $6 \
            WHERE device_id = $1 AND wireguard_network_id = $2",
            self.device_id,
            self.wireguard_network_id,
            IpNetwork::from(self.wireguard_ip.clone()),
            self.is_authorized,
            self.authorized_at,
            self.preshared_key,
        )
        .execute(executor)
        .await?;

        Ok(())
    }

    pub(crate) async fn delete<'e, E>(&self, executor: E) -> Result<(), SqlxError>
    where
        E: PgExecutor<'e>,
    {
        query!(
            "DELETE FROM wireguard_network_device \
            WHERE device_id = $1 AND wireguard_network_id = $2",
            self.device_id,
            self.wireguard_network_id,
        )
        .execute(executor)
        .await?;

        Ok(())
    }

    pub(crate) async fn find<'e, E>(
        executor: E,
        device_id: Id,
        network_id: Id,
    ) -> Result<Option<Self>, SqlxError>
    where
        E: PgExecutor<'e>,
    {
        let res = query_as!(
            Self,
            "SELECT device_id, wireguard_network_id, wireguard_ip \"wireguard_ip: IpAddr\", \
            preshared_key, is_authorized, authorized_at \
            FROM wireguard_network_device \
            WHERE device_id = $1 AND wireguard_network_id = $2",
            device_id,
            network_id
        )
        .fetch_optional(executor)
        .await?;

        Ok(res)
    }

    /// Get a first network the device was added to. Useful for network devices to
    /// make sure they always pull only one network's config.
    pub(crate) async fn find_first<'e, E>(
        executor: E,
        device_id: Id,
    ) -> Result<Option<Self>, SqlxError>
    where
        E: PgExecutor<'e>,
    {
        let res = query_as!(
            Self,
            "SELECT device_id, wireguard_network_id, wireguard_ip \"wireguard_ip: IpAddr\", \
            preshared_key, is_authorized, authorized_at \
            FROM wireguard_network_device \
            WHERE device_id = $1 ORDER BY id LIMIT 1",
            device_id
        )
        .fetch_optional(executor)
        .await?;

        Ok(res)
    }

    pub async fn find_by_device<'e, E>(
        executor: E,
        device_id: Id,
    ) -> Result<Option<Vec<Self>>, SqlxError>
    where
        E: PgExecutor<'e>,
    {
        let result = query_as!(
            Self,
            "SELECT device_id, wireguard_network_id, wireguard_ip \"wireguard_ip: IpAddr\", \
            preshared_key, is_authorized, authorized_at \
            FROM wireguard_network_device WHERE device_id = $1",
            device_id
        )
        .fetch_all(executor)
        .await?;

        Ok(if result.is_empty() {
            None
        } else {
            Some(result)
        })
    }

    pub(crate) async fn all_for_network<'e, E>(
        executor: E,
        network_id: Id,
    ) -> Result<Vec<Self>, SqlxError>
    where
        E: PgExecutor<'e>,
    {
        let res = query_as!(
            Self,
            "SELECT device_id, wireguard_network_id, wireguard_ip \"wireguard_ip: IpAddr\", \
            preshared_key, is_authorized, authorized_at \
            FROM wireguard_network_device \
            WHERE wireguard_network_id = $1",
            network_id
        )
        .fetch_all(executor)
        .await?;

        Ok(res)
    }

    /// Get all devices for a given network and user
    /// Note: doesn't return network devices added by the user
    /// as they are not considered to be bound to the user
    pub(crate) async fn all_for_network_and_user<'e, E>(
        executor: E,
        network_id: Id,
        user_id: Id,
    ) -> Result<Vec<Self>, SqlxError>
    where
        E: PgExecutor<'e>,
    {
        let res = query_as!(
            Self,
            "SELECT device_id, wireguard_network_id, wireguard_ip \"wireguard_ip: IpAddr\", \
            preshared_key, is_authorized, authorized_at \
            FROM wireguard_network_device \
            WHERE wireguard_network_id = $1 AND device_id IN \
            (SELECT id FROM device WHERE user_id = $2 AND device_type = 'user'::device_type)",
            network_id,
            user_id
        )
        .fetch_all(executor)
        .await?;

        Ok(res)
    }

    pub(crate) async fn network<'e, E>(
        &self,
        executor: E,
    ) -> Result<WireguardNetwork<Id>, SqlxError>
    where
        E: PgExecutor<'e>,
    {
        query_as!(
            WireguardNetwork,
            "SELECT id, name, address, port, pubkey, prvkey, endpoint, dns, allowed_ips, \
            connected_at, mfa_enabled, keepalive_interval, peer_disconnect_threshold \
            FROM wireguard_network WHERE id = $1",
            self.wireguard_network_id
        )
        .fetch_one(executor)
        .await
    }
}

#[derive(Debug, Error)]
pub enum DeviceError {
    #[error("Device {0} pubkey is the same as gateway pubkey for network {1}")]
    PubkeyConflict(Device<Id>, String),
    #[error("Database error")]
    DatabaseError(#[from] sqlx::Error),
    #[error("Model error")]
    ModelError(#[from] ModelError),
    #[error("Unexpected error: {0}")]
    Unexpected(String),
}

impl Device {
    #[must_use]
    pub fn new(
        name: String,
        wireguard_pubkey: String,
        user_id: Id,
        device_type: DeviceType,
        description: Option<String>,
        configured: bool,
    ) -> Self {
        Self {
            id: NoId,
            name,
            wireguard_pubkey,
            user_id,
            created: Utc::now().naive_utc(),
            device_type,
            description,
            configured,
        }
    }
}

impl Device<Id> {
    pub(crate) fn update_from(&mut self, other: ModifyDevice) {
        self.name = other.name;
        self.wireguard_pubkey = other.wireguard_pubkey;
        self.description = other.description;
    }

    /// Create WireGuard config for device.
    #[must_use]
    pub(crate) fn create_config(
        network: &WireguardNetwork<Id>,
        wireguard_network_device: &WireguardNetworkDevice,
    ) -> String {
        let dns = match &network.dns {
            Some(dns) => {
                if dns.is_empty() {
                    String::new()
                } else {
                    format!("DNS = {dns}")
                }
            }
            None => String::new(),
        };

        let allowed_ips = if network.allowed_ips.is_empty() {
            String::new()
        } else {
            format!(
                "AllowedIPs = {}\n",
                network
                    .allowed_ips
                    .iter()
                    .map(IpNetwork::to_string)
                    .collect::<Vec<String>>()
                    .join(",")
            )
        };

        format!(
            "[Interface]\n\
            PrivateKey = YOUR_PRIVATE_KEY\n\
            Address = {}\n\
            {dns}\n\
            \n\
            [Peer]\n\
            PublicKey = {}\n\
            {allowed_ips}\
            Endpoint = {}:{}\n\
            PersistentKeepalive = 300",
            wireguard_network_device.wireguard_ip, network.pubkey, network.endpoint, network.port,
        )
    }

    pub(crate) async fn find_by_ip<'e, E>(
        executor: E,
        ip: IpAddr,
        network_id: Id,
    ) -> Result<Option<Self>, SqlxError>
    where
        E: PgExecutor<'e>,
    {
        query_as!(
            Self,
            "SELECT d.id, d.name, d.wireguard_pubkey, d.user_id, d.created, d.description, \
            d.device_type  \"device_type: DeviceType\", configured \
            FROM device d \
            JOIN wireguard_network_device wnd ON d.id = wnd.device_id \
            WHERE wnd.wireguard_ip = $1 AND wnd.wireguard_network_id = $2",
            IpNetwork::from(ip),
            network_id
        )
        .fetch_optional(executor)
        .await
    }

    pub(crate) async fn find_by_pubkey<'e, E>(
        executor: E,
        pubkey: &str,
    ) -> Result<Option<Self>, SqlxError>
    where
        E: PgExecutor<'e>,
    {
        query_as!(
            Self,
            "SELECT id, name, wireguard_pubkey, user_id, created, description, \
            device_type \"device_type: DeviceType\", configured \
            FROM device WHERE wireguard_pubkey = $1",
            pubkey
        )
        .fetch_optional(executor)
        .await
    }

    pub(crate) async fn find_by_id_and_username(
        pool: &PgPool,
        id: Id,
        username: &str,
    ) -> Result<Option<Self>, SqlxError> {
        query_as!(
            Self,
            "SELECT device.id, name, wireguard_pubkey, user_id, created, description, \
            device_type \"device_type: DeviceType\", configured \
            FROM device JOIN \"user\" ON device.user_id = \"user\".id \
            WHERE device.id = $1 AND \"user\".username = $2",
            id,
            username
        )
        .fetch_optional(pool)
        .await
    }

    pub(crate) async fn all_for_username(
        pool: &PgPool,
        username: &str,
    ) -> Result<Vec<Self>, SqlxError> {
        query_as!(
            Self,
            "SELECT device.id, name, wireguard_pubkey, user_id, created, description, \
            device_type \"device_type: DeviceType\", configured \
            FROM device JOIN \"user\" ON device.user_id = \"user\".id \
            WHERE \"user\".username = $1",
            username
        )
        .fetch_all(pool)
        .await
    }

    pub(crate) async fn get_network_configs(
        &self,
        network: &WireguardNetwork<Id>,
        transaction: &mut PgConnection,
    ) -> Result<(DeviceNetworkInfo, DeviceConfig), DeviceError> {
        let wireguard_network_device =
            WireguardNetworkDevice::find(&mut *transaction, self.id, network.id)
                .await?
                .ok_or_else(|| DeviceError::Unexpected("Device not found in network".into()))?;
        let device_network_info = DeviceNetworkInfo {
            network_id: network.id,
            device_wireguard_ip: wireguard_network_device.wireguard_ip,
            preshared_key: wireguard_network_device.preshared_key.clone(),
            is_authorized: wireguard_network_device.is_authorized,
        };

        let config = Self::create_config(network, &wireguard_network_device);
        let device_config = DeviceConfig {
            network_id: network.id,
            network_name: network.name.clone(),
            config,
            endpoint: format!("{}:{}", network.endpoint, network.port),
            address: wireguard_network_device.wireguard_ip,
            allowed_ips: network.allowed_ips.clone(),
            pubkey: network.pubkey.clone(),
            dns: network.dns.clone(),
            mfa_enabled: network.mfa_enabled,
            keepalive_interval: network.keepalive_interval,
        };

        Ok((device_network_info, device_config))
    }

    pub(crate) async fn add_to_network(
        &self,
        network: &WireguardNetwork<Id>,
        ip: IpAddr,
        transaction: &mut PgConnection,
    ) -> Result<(DeviceNetworkInfo, DeviceConfig), DeviceError> {
        let wireguard_network_device = self
            .assign_network_ip(&mut *transaction, network, ip)
            .await?;
        let device_network_info = DeviceNetworkInfo {
            network_id: network.id,
            device_wireguard_ip: wireguard_network_device.wireguard_ip,
            preshared_key: wireguard_network_device.preshared_key.clone(),
            is_authorized: wireguard_network_device.is_authorized,
        };

        let config = Self::create_config(network, &wireguard_network_device);
        let device_config = DeviceConfig {
            network_id: network.id,
            network_name: network.name.clone(),
            config,
            endpoint: format!("{}:{}", network.endpoint, network.port),
            address: wireguard_network_device.wireguard_ip,
            allowed_ips: network.allowed_ips.clone(),
            pubkey: network.pubkey.clone(),
            dns: network.dns.clone(),
            mfa_enabled: network.mfa_enabled,
            keepalive_interval: network.keepalive_interval,
        };

        Ok((device_network_info, device_config))
    }

    // Add device to all existing networks
    pub async fn add_to_all_networks(
        &self,
        transaction: &mut PgConnection,
    ) -> Result<(Vec<DeviceNetworkInfo>, Vec<DeviceConfig>), DeviceError> {
        info!("Adding device {} to all existing networks", self.name);
        let networks = WireguardNetwork::all(&mut *transaction).await?;

        let mut configs = Vec::new();
        let mut network_info = Vec::new();
        for network in networks {
            debug!(
                "Assigning IP for device {} (user {}) in network {network}",
                self.name, self.user_id
            );
            // check for pubkey conflicts with networks
            if network.pubkey == self.wireguard_pubkey {
                return Err(DeviceError::PubkeyConflict(self.clone(), network.name));
            }
            if WireguardNetworkDevice::find(&mut *transaction, self.id, network.id)
                .await?
                .is_some()
            {
                debug!("Device {self} already has an IP within network {network}. Skipping...",);
                continue;
            }

            if let Ok(wireguard_network_device) = network
                .add_device_to_network(&mut *transaction, self, None)
                .await
            {
                debug!(
                    "Assigned IP {} for device {} (user {}) in network {network}",
                    wireguard_network_device.wireguard_ip, self.name, self.user_id
                );
                let device_network_info = DeviceNetworkInfo {
                    network_id: network.id,
                    device_wireguard_ip: wireguard_network_device.wireguard_ip,
                    preshared_key: wireguard_network_device.preshared_key.clone(),
                    is_authorized: wireguard_network_device.is_authorized,
                };
                network_info.push(device_network_info);

                let config = Self::create_config(&network, &wireguard_network_device);
                configs.push(DeviceConfig {
                    network_id: network.id,
                    network_name: network.name,
                    config,
                    endpoint: format!("{}:{}", network.endpoint, network.port),
                    address: wireguard_network_device.wireguard_ip,
                    allowed_ips: network.allowed_ips,
                    pubkey: network.pubkey,
                    dns: network.dns,
                    mfa_enabled: network.mfa_enabled,
                    keepalive_interval: network.keepalive_interval,
                });
            }
        }
        Ok((network_info, configs))
    }

    // Assign IP to the device in a given network
    pub(crate) async fn assign_next_network_ip(
        &self,
        transaction: &mut PgConnection,
        network: &WireguardNetwork<Id>,
        reserved_ips: Option<&[IpAddr]>,
    ) -> Result<WireguardNetworkDevice, ModelError> {
        if let Some(address) = network.address.first() {
            let net_ip = address.ip();
            let net_network = address.network();
            let net_broadcast = address.broadcast();
            for ip in address {
                if ip == net_ip || ip == net_network || ip == net_broadcast {
                    continue;
                }
                if let Some(reserved_ips) = reserved_ips {
                    if reserved_ips.contains(&ip) {
                        continue;
                    }
                }

                // Break loop if IP is unassigned and return network device
                if Self::find_by_ip(&mut *transaction, ip, network.id)
                    .await?
                    .is_none()
                {
                    info!("Assigned IP address {ip} for device: {}", self.name);
                    let wireguard_network_device =
                        WireguardNetworkDevice::new(network.id, self.id, ip);
                    wireguard_network_device.insert(&mut *transaction).await?;
                    return Ok(wireguard_network_device);
                }
            }
        }
        Err(ModelError::CannotCreate)
    }

    pub(crate) async fn assign_network_ip(
        &self,
        transaction: &mut PgConnection,
        network: &WireguardNetwork<Id>,
        ip: IpAddr,
    ) -> Result<WireguardNetworkDevice, ModelError> {
        if let Some(network_address) = network.address.first() {
            let net_ip = network_address.ip();
            let net_network = network_address.network();
            let net_broadcast = network_address.broadcast();
            if ip == net_ip || ip == net_network || ip == net_broadcast {
                return Err(ModelError::CannotCreate);
            }

            if Self::find_by_ip(&mut *transaction, ip, network.id)
                .await?
                .is_none()
            {
                info!("Assigned IP: {ip} for device: {}", self.name);
                let wireguard_network_device = WireguardNetworkDevice::new(network.id, self.id, ip);
                wireguard_network_device.insert(&mut *transaction).await?;
                return Ok(wireguard_network_device);
            }
        }
        Err(ModelError::CannotCreate)
    }

    /// Gets the first network of the network device
    /// FIXME: Return only one network, not a Vec
    pub async fn find_network_device_networks<'e, E>(
        &self,
        executor: E,
    ) -> Result<Vec<WireguardNetwork<Id>>, SqlxError>
    where
        E: PgExecutor<'e>,
    {
        query_as!(
            WireguardNetwork,
            "SELECT id, name, address, port, pubkey, prvkey, endpoint, dns, allowed_ips, \
            connected_at, mfa_enabled, keepalive_interval, peer_disconnect_threshold \
            FROM wireguard_network WHERE id IN \
            (SELECT wireguard_network_id FROM wireguard_network_device WHERE device_id = $1 ORDER BY id LIMIT 1)",
            self.id
        )
        .fetch_all(executor)
        .await
    }

    pub fn validate_pubkey(pubkey: &str) -> Result<(), String> {
        if let Ok(key) = BASE64_STANDARD.decode(pubkey) {
            if key.len() == KEY_LENGTH {
                return Ok(());
            }
        }

        Err(format!("{pubkey} is not a valid pubkey"))
    }

    pub(crate) async fn find_by_type<'e, E>(
        executor: E,
        device_type: DeviceType,
    ) -> Result<Vec<Self>, SqlxError>
    where
        E: PgExecutor<'e>,
    {
        query_as!(Self,
            "SELECT id, name, wireguard_pubkey, user_id, created, description, device_type \"device_type: DeviceType\", \
            configured \
            FROM device WHERE device_type = $1 ORDER BY name",
            device_type as DeviceType
        ).fetch_all(executor).await
    }

    pub(crate) async fn find_by_type_and_network<'e, E>(
        executor: E,
        device_type: DeviceType,
        network_id: Id,
    ) -> Result<Vec<Self>, SqlxError>
    where
        E: PgExecutor<'e>,
    {
        query_as!(Self,
            "SELECT id, name, wireguard_pubkey, user_id, created, description, device_type \"device_type: DeviceType\", \
            configured \
            FROM device WHERE device_type = $1 \
            AND id IN (SELECT device_id FROM wireguard_network_device WHERE wireguard_network_id = $2) \
            ORDER BY name",
            device_type as DeviceType,
            network_id
        ).fetch_all(executor).await
    }

    pub(crate) async fn get_owner<'e, E>(&self, executor: E) -> Result<User<Id>, SqlxError>
    where
        E: PgExecutor<'e>,
    {
        query_as!(
            User,
            "SELECT id, username, password_hash, last_name, first_name, email, \
            phone, mfa_enabled, totp_enabled, email_mfa_enabled, \
            totp_secret, email_mfa_secret, mfa_method \"mfa_method: _\", recovery_codes, is_active, openid_sub \
            FROM \"user\" WHERE id = $1",
            self.user_id
        ).fetch_one(executor).await
    }
}

#[cfg(test)]
mod test {
    use std::str::FromStr;

    use claims::{assert_err, assert_ok};

    use super::*;
    use crate::db::User;

    impl Device<Id> {
        /// Create new device and assign IP in a given network
        // TODO: merge with `assign_network_ip()`
        pub(crate) async fn new_with_ip(
            pool: &PgPool,
            user_id: Id,
            name: String,
            pubkey: String,
            network: &WireguardNetwork<Id>,
        ) -> Result<(Self, WireguardNetworkDevice), ModelError> {
            if let Some(address) = network.address.first() {
                let net_ip = address.ip();
                let net_network = address.network();
                let net_broadcast = address.broadcast();
                for ip in address {
                    if ip == net_ip || ip == net_network || ip == net_broadcast {
                        continue;
                    }
                    // Break loop if IP is unassigned and return device
                    if Self::find_by_ip(pool, ip, network.id).await?.is_none() {
                        let device = Device::new(
                            name.clone(),
                            pubkey,
                            user_id,
                            DeviceType::User,
                            None,
                            true,
                        )
                        .save(pool)
                        .await?;
                        info!("Created device: {}", device.name);
                        debug!("For user: {}", device.user_id);
                        let wireguard_network_device =
                            WireguardNetworkDevice::new(network.id, device.id, ip);
                        wireguard_network_device.insert(pool).await?;
                        info!(
                            "Assigned IP: {ip} for device: {name} in network: {}",
                            network.id
                        );
                        return Ok((device, wireguard_network_device));
                    }
                }
            }
            Err(ModelError::CannotCreate)
        }
    }

    #[sqlx::test]
    async fn test_assign_device_ip(pool: PgPool) {
        let mut network = WireguardNetwork::default();
        network.try_set_address("10.1.1.1/30").unwrap();
        let network = network.save(&pool).await.unwrap();

        let user = User::new(
            "testuser",
            Some("hunter2"),
            "Tester",
            "Test",
            "test@test.com",
            None,
        )
        .save(&pool)
        .await
        .unwrap();
        let (_device, wireguard_network_device) =
            Device::new_with_ip(&pool, user.id, "dev1".into(), "key1".into(), &network)
                .await
                .unwrap();
        assert_eq!(
            wireguard_network_device.wireguard_ip.to_string(),
            "10.1.1.2"
        );

        let device = Device::new_with_ip(&pool, 1, "dev4".into(), "key4".into(), &network).await;
        assert!(device.is_err());
    }

    #[test]
    fn test_pubkey_validation() {
        let invalid_test_key = "invalid_key";
        assert_err!(Device::validate_pubkey(invalid_test_key));

        let valid_test_key = "sejIy0WCLvOR7vWNchP9Elsayp3UTK/QCnEJmhsHKTc=";
        assert_ok!(Device::validate_pubkey(valid_test_key));
    }

    #[sqlx::test]
    fn test_all_for_network_and_user(pool: PgPool) {
        let user = User::new(
            "testuser",
            Some("hunter2"),
            "Tester",
            "Test",
            "email@email.com",
            None,
        )
        .save(&pool)
        .await
        .unwrap();

        let user2 = User::new(
            "testuser2",
            Some("hunter2"),
            "Tester",
            "Test",
            "email2@email.com",
            None,
        )
        .save(&pool)
        .await
        .unwrap();

        let mut network = WireguardNetwork::default();
        network.try_set_address("10.1.1.1/24").unwrap();
        let network = network.save(&pool).await.unwrap();
        let mut network_2 = WireguardNetwork::<NoId> {
            name: "testnetwork2".into(),
            ..Default::default()
        };
        network_2.try_set_address("10.1.2.1/24").unwrap();
        let network2 = network_2.save(&pool).await.unwrap();

        let device = Device::new(
            "testdevice".into(),
            "key".into(),
            user.id,
            DeviceType::User,
            None,
            true,
        )
        .save(&pool)
        .await
        .unwrap();

        let device2 = Device::new(
            "testdevice2".into(),
            "key2".into(),
            user.id,
            DeviceType::User,
            None,
            true,
        )
        .save(&pool)
        .await
        .unwrap();

        let device3 = Device::new(
            "testdevice3".into(),
            "key3".into(),
            user2.id,
            DeviceType::User,
            None,
            true,
        )
        .save(&pool)
        .await
        .unwrap();

        let device4 = Device::new(
            "testdevice4".into(),
            "key4".into(),
            user.id,
            DeviceType::Network,
            None,
            true,
        )
        .save(&pool)
        .await
        .unwrap();

        let mut transaction = pool.begin().await.unwrap();

        network
            .add_device_to_network(&mut transaction, &device, None)
            .await
            .unwrap();
        network2
            .add_device_to_network(&mut transaction, &device, None)
            .await
            .unwrap();
        network2
            .add_device_to_network(&mut transaction, &device2, None)
            .await
            .unwrap();
        network
            .add_device_to_network(&mut transaction, &device3, None)
            .await
            .unwrap();
        WireguardNetworkDevice::new(
            network.id,
            device4.id,
            IpAddr::from_str("10.1.1.10").unwrap(),
        )
        .insert(&mut *transaction)
        .await
        .unwrap();

        transaction.commit().await.unwrap();

        let devices = WireguardNetworkDevice::all_for_network_and_user(&pool, network.id, user.id)
            .await
            .unwrap();

        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].device_id, device.id);
    }
}
