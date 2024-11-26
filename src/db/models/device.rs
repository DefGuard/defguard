use std::{fmt, net::IpAddr};

use base64::{prelude::BASE64_STANDARD, Engine};
use chrono::{NaiveDateTime, Utc};
use ipnetwork::IpNetwork;
use model_derive::Model;
use sqlx::{query, query_as, Error as SqlxError, FromRow, PgConnection, PgExecutor, PgPool};
use thiserror::Error;
use utoipa::ToSchema;

use super::{
    error::ModelError,
    wireguard::{WireguardNetwork, WIREGUARD_MAX_HANDSHAKE_MINUTES},
};
use crate::{
    db::{Id, NoId},
    KEY_LENGTH,
};

#[derive(Serialize)]
pub struct DeviceConfig {
    pub(crate) network_id: Id,
    pub(crate) network_name: String,
    pub(crate) config: String,
    pub(crate) address: IpAddr,
    pub(crate) endpoint: String,
    pub(crate) allowed_ips: Vec<IpNetwork>,
    pub(crate) pubkey: String,
    pub(crate) dns: Option<String>,
    pub(crate) mfa_enabled: bool,
    pub(crate) keepalive_interval: i32,
}

#[derive(Clone, Deserialize, Model, Serialize, Debug, ToSchema)]
pub struct Device<I = NoId> {
    pub id: I,
    pub name: String,
    pub wireguard_pubkey: String,
    pub user_id: Id,
    pub created: NaiveDateTime,
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
    pub async fn from_device<'e, E>(executor: E, device: Device<Id>) -> Result<Self, ModelError>
    where
        E: PgExecutor<'e>,
    {
        debug!("Generating device info for {device}");
        let network_info = query_as!(
            DeviceNetworkInfo,
            "SELECT wireguard_network_id network_id, wireguard_ip \"device_wireguard_ip: IpAddr\", preshared_key, is_authorized \
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

#[derive(Clone, Debug, Deserialize, Serialize)]
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
            SELECT \
                n.id network_id, n.name network_name, n.endpoint gateway_endpoint, \
                wnd.wireguard_ip \"device_wireguard_ip: IpAddr\", stats.endpoint device_endpoint, \
                stats.latest_handshake \"latest_handshake?\", \
                COALESCE (((NOW() - stats.latest_handshake) < $1 * interval '1 minute'), false) as \"is_active!\" \
            FROM wireguard_network_device wnd \
            JOIN wireguard_network n ON n.id = wnd.wireguard_network_id \
            LEFT JOIN stats on n.id = stats.network \
            WHERE wnd.device_id = $2",
            WIREGUARD_MAX_HANDSHAKE_MINUTES as f64,
            device.id,
        )
        .fetch_all(pool)
        .await?;

        let networks_info: Vec<UserDeviceNetworkInfo> = result
            .into_iter()
            .map(|r| {
                let device_ip = match r.device_endpoint {
                    Some(endpoint) => endpoint.split(':').next().map(ToString::to_string),
                    None => None,
                };
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
}

impl WireguardNetworkDevice {
    #[must_use]
    pub fn new(network_id: Id, device_id: Id, wireguard_ip: IpAddr) -> Self {
        Self {
            wireguard_network_id: network_id,
            wireguard_ip,
            device_id,
            preshared_key: None,
            is_authorized: false,
            authorized_at: None,
        }
    }

    pub async fn insert<'e, E>(&self, executor: E) -> Result<(), SqlxError>
    where
        E: PgExecutor<'e>,
    {
        query!(
            "INSERT INTO wireguard_network_device \
                (device_id, wireguard_network_id, wireguard_ip, is_authorized, authorized_at, preshared_key) \
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

    pub async fn update<'e, E>(&self, executor: E) -> Result<(), SqlxError>
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

    pub async fn delete<'e, E>(&self, executor: E) -> Result<(), SqlxError>
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

    pub async fn find<'e, E>(
        executor: E,
        device_id: Id,
        network_id: Id,
    ) -> Result<Option<Self>, SqlxError>
    where
        E: PgExecutor<'e>,
    {
        let res = query_as!(
            Self,
            "SELECT device_id, wireguard_network_id, wireguard_ip \"wireguard_ip: IpAddr\", preshared_key, is_authorized, authorized_at \
            FROM wireguard_network_device \
            WHERE device_id = $1 AND wireguard_network_id = $2",
            device_id,
            network_id
        )
        .fetch_optional(executor)
        .await?;

        Ok(res)
    }

    pub async fn find_by_device(
        pool: &PgPool,
        device_id: Id,
    ) -> Result<Option<Vec<Self>>, SqlxError> {
        let result = query_as!(
            Self,
            "SELECT device_id, wireguard_network_id, wireguard_ip \"wireguard_ip: IpAddr\", preshared_key, is_authorized, authorized_at \
            FROM wireguard_network_device WHERE device_id = $1",
            device_id
        )
        .fetch_all(pool)
        .await?;

        Ok(if result.is_empty() {
            None
        } else {
            Some(result)
        })
    }

    pub async fn all_for_network<'e, E>(executor: E, network_id: Id) -> Result<Vec<Self>, SqlxError>
    where
        E: PgExecutor<'e>,
    {
        let res = query_as!(
            Self,
            "SELECT device_id, wireguard_network_id, wireguard_ip \"wireguard_ip: IpAddr\", preshared_key, is_authorized, authorized_at \
            FROM wireguard_network_device \
            WHERE wireguard_network_id = $1",
            network_id
        )
        .fetch_all(executor)
        .await?;

        Ok(res)
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
    pub fn new(name: String, wireguard_pubkey: String, user_id: Id) -> Self {
        Self {
            id: NoId,
            name,
            wireguard_pubkey,
            user_id,
            created: Utc::now().naive_utc(),
        }
    }
}

impl Device<Id> {
    pub fn update_from(&mut self, other: ModifyDevice) {
        self.name = other.name;
        self.wireguard_pubkey = other.wireguard_pubkey;
    }

    /// Create WireGuard config for device.
    #[must_use]
    pub fn create_config(
        &self,
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

    pub async fn find_by_ip<'e, E>(
        executor: E,
        ip: IpAddr,
        network_id: Id,
    ) -> Result<Option<Self>, SqlxError>
    where
        E: PgExecutor<'e>,
    {
        query_as!(
            Self,
            "SELECT d.id, d.name, d.wireguard_pubkey, d.user_id, d.created \
            FROM device d \
            JOIN wireguard_network_device wnd \
            ON d.id = wnd.device_id \
            WHERE wnd.wireguard_ip = $1 AND wnd.wireguard_network_id = $2",
            IpNetwork::from(ip),
            network_id
        )
        .fetch_optional(executor)
        .await
    }

    pub async fn find_by_pubkey<'e, E>(executor: E, pubkey: &str) -> Result<Option<Self>, SqlxError>
    where
        E: PgExecutor<'e>,
    {
        query_as!(
            Self,
            "SELECT id, name, wireguard_pubkey, user_id, created \
            FROM device WHERE wireguard_pubkey = $1",
            pubkey
        )
        .fetch_optional(executor)
        .await
    }

    pub async fn find_by_id_and_username(
        pool: &PgPool,
        id: Id,
        username: &str,
    ) -> Result<Option<Self>, SqlxError> {
        query_as!(
            Self,
            "SELECT device.id, name, wireguard_pubkey, user_id, created \
            FROM device JOIN \"user\" ON device.user_id = \"user\".id \
            WHERE device.id = $1 AND \"user\".username = $2",
            id,
            username
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn find_by_id_and_user_id(
        pool: &PgPool,
        id: Id,
        user_id: Id,
    ) -> Result<Option<Self>, SqlxError> {
        query_as!(
            Self,
            "SELECT device.id, name, wireguard_pubkey, user_id, created \
            FROM device JOIN \"user\" ON device.user_id = \"user\".id \
            WHERE device.id = $1 AND \"user\".id = $2",
            id,
            user_id
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn get_ip(&self, pool: &PgPool, network_id: Id) -> Result<Option<String>, SqlxError> {
        let result = query!(
            "SELECT wireguard_ip \
            FROM wireguard_network_device \
            WHERE device_id = $1 AND wireguard_network_id = $2",
            self.id,
            network_id
        )
        .fetch_one(pool)
        .await?;

        Ok(Some(result.wireguard_ip.to_string()))
    }

    pub async fn all_for_username(pool: &PgPool, username: &str) -> Result<Vec<Self>, SqlxError> {
        query_as!(
            Self,
            "SELECT device.id, name, wireguard_pubkey, user_id, created \
            FROM device JOIN \"user\" ON device.user_id = \"user\".id \
            WHERE \"user\".username = $1",
            username
        )
        .fetch_all(pool)
        .await
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

                let config = self.create_config(&network, &wireguard_network_device);
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
    pub(crate) async fn assign_network_ip(
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

    pub fn validate_pubkey(pubkey: &str) -> Result<(), String> {
        if let Ok(key) = BASE64_STANDARD.decode(pubkey) {
            if key.len() == KEY_LENGTH {
                return Ok(());
            }
        }

        Err(format!("{pubkey} is not a valid pubkey"))
    }
}

#[cfg(test)]
mod test {
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
                for ip in &*address {
                    if ip == net_ip || ip == net_network || ip == net_broadcast {
                        continue;
                    }
                    // Break loop if IP is unassigned and return device
                    if Self::find_by_ip(pool, ip, network.id).await?.is_none() {
                        let device = Device::new(name.clone(), pubkey, user_id)
                            .save(pool)
                            .await?;
                        let wireguard_network_device =
                            WireguardNetworkDevice::new(network.id, device.id, ip);
                        wireguard_network_device.insert(pool).await?;

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
}
