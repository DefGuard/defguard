use std::{collections::HashSet, fmt, net::IpAddr};

use base64::{Engine, prelude::BASE64_STANDARD};
use chrono::{NaiveDate, NaiveDateTime, Timelike, Utc};
use ipnetwork::IpNetwork;
use model_derive::Model;
use rand::{
    Rng,
    distributions::{Alphanumeric, DistString, Standard},
    prelude::Distribution,
};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgConnection, PgExecutor, PgPool, Type, query, query_as, query_scalar};
use thiserror::Error;
use tracing::{debug, error, info, warn};
use utoipa::ToSchema;

use crate::{
    KEY_LENGTH,
    csv::AsCsv,
    db::{
        Id, NoId,
        models::{
            ModelError, WireguardNetwork,
            user::User,
            vpn_client_session::{VpnClientSession, VpnClientSessionState},
            wireguard::{
                LocationMfaMode, NetworkAddressError, ServiceLocationMode, WireguardNetworkError,
            },
        },
    },
};

#[derive(Serialize, ToSchema)]
pub struct DeviceConfig {
    pub network_id: Id,
    pub network_name: String,
    pub config: String,
    #[schema(value_type = Vec<String>)]
    pub address: Vec<IpAddr>,
    pub endpoint: String,
    #[schema(value_type = Vec<String>)]
    pub allowed_ips: Vec<IpNetwork>,
    pub pubkey: String,
    pub dns: Option<String>,
    pub keepalive_interval: i32,
    pub location_mfa_mode: LocationMfaMode,
    pub service_location_mode: ServiceLocationMode,
}

// The type of a device:
// User: A device of a user, which may be in multiple networks, e.g. a laptop
// Network: A stand-alone device added by a user permanently bound to one network, e.g. a printer
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, ToSchema, Type)]
#[sqlx(type_name = "device_type", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum DeviceType {
    User,
    Network,
}

impl fmt::Display for DeviceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::User => "user",
            Self::Network => "network",
        })
    }
}

impl From<DeviceType> for String {
    fn from(device_type: DeviceType) -> Self {
        device_type.to_string()
    }
}

#[derive(Clone, Debug, Deserialize, FromRow, Model, Serialize, ToSchema, PartialEq)]
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
        f.write_str(&self.name)
    }
}

impl fmt::Display for Device<Id> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[ID {}] {}", self.id, self.name)
    }
}

impl Distribution<Device> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Device {
        Device {
            id: NoId,
            name: Alphanumeric.sample_string(rng, 8),
            wireguard_pubkey: Alphanumeric.sample_string(rng, 32),
            user_id: rng.r#gen(),
            created: NaiveDate::from_ymd_opt(
                rng.gen_range(2000..2026),
                rng.gen_range(1..13),
                rng.gen_range(1..29),
            )
            .unwrap()
            .and_hms_opt(
                rng.gen_range(1..24),
                rng.gen_range(1..60),
                rng.gen_range(1..60),
            )
            .unwrap(),
            device_type: match rng.gen_range(0..2) {
                0 => DeviceType::Network,
                _ => DeviceType::User,
            },
            description: rng
                .r#gen::<bool>()
                .then_some(Alphanumeric.sample_string(rng, 20)),
            configured: rng.r#gen(),
        }
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
    pub device_wireguard_ips: Vec<IpAddr>,
    #[serde(skip_serializing)]
    pub preshared_key: Option<String>,
    pub is_authorized: bool,
}

impl DeviceNetworkInfo {
    #[must_use]
    pub fn from_authorized_mfa_session<I>(
        network_id: Id,
        device_wireguard_ips: I,
        preshared_key: String,
    ) -> Self
    where
        I: Into<Vec<IpAddr>>,
    {
        Self {
            network_id,
            device_wireguard_ips: device_wireguard_ips.into(),
            preshared_key: Some(preshared_key),
            is_authorized: true,
        }
    }
}

impl DeviceInfo {
    pub async fn from_device<'e, E>(executor: E, device: Device<Id>) -> Result<Self, ModelError>
    where
        E: PgExecutor<'e>,
    {
        debug!("Generating device info for {device}");
        let network_info = query_as!(
            DeviceNetworkInfo,
            "SELECT wnd.wireguard_network_id network_id, \
                wnd.wireguard_ips \"device_wireguard_ips: Vec<IpAddr>\", \
                CASE \
                    WHEN n.location_mfa_mode = 'disabled'::location_mfa_mode THEN NULL::text \
                    ELSE active_session.preshared_key \
                END \"preshared_key?\", \
                CASE \
                    WHEN n.location_mfa_mode = 'disabled'::location_mfa_mode THEN TRUE \
                    ELSE active_session.preshared_key IS NOT NULL \
                END \"is_authorized!\" \
            FROM wireguard_network_device wnd \
            JOIN wireguard_network n ON n.id = wnd.wireguard_network_id \
            LEFT JOIN LATERAL ( \
                SELECT id, preshared_key \
                FROM vpn_client_session \
                WHERE location_id = wnd.wireguard_network_id \
                    AND device_id = wnd.device_id \
                    AND state IN ('new', 'connected') \
                ORDER BY created_at DESC, id DESC \
                LIMIT 1 \
            ) active_session ON true \
            WHERE wnd.device_id = $1 \
            ORDER BY wnd.wireguard_network_id ASC",
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
    pub device_wireguard_ips: Vec<String>,
    pub last_connected_ip: Option<String>,
    pub last_connected_at: Option<NaiveDateTime>,
    pub is_active: bool,
}

impl UserDevice {
    pub async fn from_device(pool: &PgPool, device: Device<Id>) -> sqlx::Result<Option<Self>> {
        // fetch device config and connection info for all allowed networks
        let result = query!(
            "SELECT n.id network_id, n.name network_name, n.endpoint gateway_endpoint, \
	            wnd.wireguard_ips \"device_wireguard_ips: Vec<IpAddr>\", vs.endpoint \"device_endpoint?\", \
	            vs.latest_handshake \"latest_handshake?\", \
	            vs.state \"state?: VpnClientSessionState\" \
            FROM wireguard_network_device wnd \
            JOIN wireguard_network n ON n.id = wnd.wireguard_network_id \
            LEFT JOIN LATERAL ( \
				SELECT id, state, location_id, endpoint, latest_handshake \
				FROM vpn_client_session \
	            LEFT JOIN LATERAL ( \
					SELECT session_id, endpoint, latest_handshake \
					FROM vpn_session_stats \
					WHERE session_id = vpn_client_session.id \
					ORDER BY collected_at DESC \
					LIMIT 1 \
	            ) vss ON vss.session_id = vpn_client_session.id \
				WHERE location_id = n.id and device_id = $1 \
				ORDER BY created_at DESC, id DESC \
				LIMIT 1 \
            ) vs ON vs.location_id = n.id \
            WHERE wnd.device_id = $1",
            device.id,
        )
        .fetch_all(pool)
        .await?;

        let networks_info: Vec<UserDeviceNetworkInfo> = result
            .into_iter()
            .map(|r| {
                // extract latest public IP from stats endpoint
                let device_ip = r.device_endpoint.and_then(|endpoint| {
                    let mut addr = endpoint.rsplit_once(':')?.0;
                    // Strip square brackets.
                    if addr.starts_with('[') && addr.ends_with(']') {
                        let end = addr.len() - 1;
                        addr = &addr[1..end];
                    }
                    Some(addr.to_owned())
                });

                let is_active = match r.state {
                    Some(session_state) => session_state == VpnClientSessionState::Connected,
                    None => false,
                };

                UserDeviceNetworkInfo {
                    network_id: r.network_id,
                    network_name: r.network_name,
                    network_gateway_ip: r.gateway_endpoint,
                    device_wireguard_ips: r
                        .device_wireguard_ips
                        .iter()
                        .map(IpAddr::to_string)
                        .collect(),
                    last_connected_ip: device_ip,
                    last_connected_at: r.latest_handshake,
                    is_active,
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
    pub wireguard_ips: Vec<IpAddr>,
    pub device_id: Id,
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
    async fn latest_active_session<'e, E>(
        executor: E,
        network: &WireguardNetwork<Id>,
        device_id: Id,
    ) -> sqlx::Result<Option<VpnClientSession<Id>>>
    where
        E: PgExecutor<'e>,
    {
        if !network.mfa_enabled() {
            return Ok(None);
        }

        VpnClientSession::try_get_active_session(executor, network.id, device_id).await
    }

    #[must_use]
    pub fn to_device_network_info(
        &self,
        network: &WireguardNetwork<Id>,
        active_session: Option<&VpnClientSession<Id>>,
    ) -> DeviceNetworkInfo {
        let (preshared_key, is_authorized) = if network.mfa_enabled() {
            let preshared_key = active_session.and_then(|session| session.preshared_key.clone());
            let is_authorized = preshared_key.is_some();
            (preshared_key, is_authorized)
        } else {
            (None, true)
        };

        DeviceNetworkInfo {
            network_id: network.id,
            device_wireguard_ips: self.wireguard_ips.clone(),
            preshared_key,
            is_authorized,
        }
    }

    pub async fn to_device_network_info_runtime<'e, E>(
        &self,
        executor: E,
        network: &WireguardNetwork<Id>,
    ) -> sqlx::Result<DeviceNetworkInfo>
    where
        E: PgExecutor<'e>,
    {
        let active_session = Self::latest_active_session(executor, network, self.device_id).await?;

        Ok(self.to_device_network_info(network, active_session.as_ref()))
    }

    #[must_use]
    pub fn new<I>(network_id: Id, device_id: Id, wireguard_ips: I) -> Self
    where
        I: Into<Vec<IpAddr>>,
    {
        Self {
            wireguard_network_id: network_id,
            wireguard_ips: wireguard_ips.into(),
            device_id,
        }
    }

    #[must_use]
    pub(crate) fn ips_as_network(&self) -> Vec<IpNetwork> {
        self.wireguard_ips
            .iter()
            .map(|ip| IpNetwork::from(*ip))
            .collect()
    }

    pub async fn insert<'e, E>(&self, executor: E) -> sqlx::Result<()>
    where
        E: PgExecutor<'e>,
    {
        query!(
            "INSERT INTO wireguard_network_device \
            (device_id, wireguard_network_id, wireguard_ips) \
            VALUES ($1, $2, $3) \
            ON CONFLICT ON CONSTRAINT device_network \
            DO UPDATE SET wireguard_ips = $3",
            self.device_id,
            self.wireguard_network_id,
            &self.ips_as_network(),
        )
        .execute(executor)
        .await?;

        Ok(())
    }

    pub async fn update<'e, E>(&self, executor: E) -> sqlx::Result<()>
    where
        E: PgExecutor<'e>,
    {
        query!(
            "UPDATE wireguard_network_device \
            SET wireguard_ips = $3 \
            WHERE device_id = $1 AND wireguard_network_id = $2",
            self.device_id,
            self.wireguard_network_id,
            &self.ips_as_network(),
        )
        .execute(executor)
        .await?;

        Ok(())
    }

    pub async fn delete<'e, E>(&self, executor: E) -> sqlx::Result<()>
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
    ) -> sqlx::Result<Option<Self>>
    where
        E: PgExecutor<'e>,
    {
        let res = query_as!(
            Self,
            "SELECT device_id, wireguard_network_id, \
                wireguard_ips \"wireguard_ips: Vec<IpAddr>\" \
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
    pub async fn find_first<'e, E>(executor: E, device_id: Id) -> sqlx::Result<Option<Self>>
    where
        E: PgExecutor<'e>,
    {
        let res = query_as!(
            Self,
            "SELECT device_id, wireguard_network_id, \
                wireguard_ips \"wireguard_ips: Vec<IpAddr>\" \
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
    ) -> sqlx::Result<Option<Vec<Self>>>
    where
        E: PgExecutor<'e>,
    {
        let result = query_as!(
            Self,
            "SELECT device_id, wireguard_network_id, \
                wireguard_ips \"wireguard_ips: Vec<IpAddr>\" \
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

    pub async fn all_for_network<'e, E>(executor: E, network_id: Id) -> sqlx::Result<Vec<Self>>
    where
        E: PgExecutor<'e>,
    {
        let res = query_as!(
            Self,
            "SELECT device_id, wireguard_network_id, \
                wireguard_ips \"wireguard_ips: Vec<IpAddr>\" \
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
    pub async fn all_for_network_and_user<'e, E>(
        executor: E,
        network_id: Id,
        user_id: Id,
    ) -> sqlx::Result<Vec<Self>>
    where
        E: PgExecutor<'e>,
    {
        let res = query_as!(
            Self,
            "SELECT device_id, wireguard_network_id, \
                wireguard_ips \"wireguard_ips: Vec<IpAddr>\" \
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

    pub async fn network<'e, E>(&self, executor: E) -> sqlx::Result<WireguardNetwork<Id>>
    where
        E: PgExecutor<'e>,
    {
        WireguardNetwork::find_by_id(executor, self.wireguard_network_id)
            .await?
            .ok_or(sqlx::Error::RowNotFound)
    }

    /// Check if any device is assigned to a given network.
    pub async fn has_devices_in_network<'e, E>(executor: E, network_id: Id) -> sqlx::Result<bool>
    where
        E: PgExecutor<'e>,
    {
        let result = query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM wireguard_network_device \
            WHERE wireguard_network_id = $1)",
            network_id
        )
        .fetch_one(executor)
        .await?;

        Ok(result.unwrap_or(false))
    }
}

#[derive(Debug, Error)]
pub enum DeviceError {
    #[error("Device pubkey {0} is the same as gateway pubkey")]
    PubkeyConflict(String),
    #[error("Database error")]
    DatabaseError(#[from] sqlx::Error),
    #[error(transparent)]
    ModelError(#[from] ModelError),
    #[error(transparent)]
    NetworkIpAssignmentError(#[from] NetworkAddressError),
    #[error("Unexpected error: {0}")]
    Unexpected(String),
    #[error("Network {0} is full, no IP addresses available for device")]
    NetworkFull(String),
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
        // FIXME: this is a workaround for reducing timestamp precision.
        // `chrono` has nanosecond precision by default, while Postgres only does microseconds.
        // It avoids issues when comparing to objects fetched from DB.
        let created = Utc::now().naive_utc();
        let created = created
            .with_nanosecond((created.nanosecond() / 1_000) * 1_000)
            .expect("failed to truncate timestamp precision");

        Self {
            id: NoId,
            name,
            wireguard_pubkey,
            user_id,
            created,
            device_type,
            description,
            configured,
        }
    }
}

impl Device<Id> {
    pub fn update_from(&mut self, other: ModifyDevice) {
        self.name = other.name;
        self.wireguard_pubkey = other.wireguard_pubkey;
        self.description = other.description;
    }

    /// Create WireGuard config for device.
    #[must_use]
    pub fn create_config(
        network: &WireguardNetwork<Id>,
        wireguard_network_device: &WireguardNetworkDevice,
    ) -> String {
        let dns = match &network.dns {
            Some(dns) if !dns.is_empty() => format!("DNS = {dns}"),
            _ => String::new(),
        };

        let allowed_ips = if network.allowed_ips.is_empty() {
            String::new()
        } else {
            format!("AllowedIPs = {}\n", network.allowed_ips.as_csv())
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
            wireguard_network_device.wireguard_ips.as_csv(),
            network.pubkey,
            network.endpoint,
            network.port,
        )
    }

    pub async fn find_by_ip<'e, E>(
        executor: E,
        ip: IpAddr,
        network_id: Id,
    ) -> sqlx::Result<Option<Self>>
    where
        E: PgExecutor<'e>,
    {
        query_as!(
            Self,
            "SELECT d.id, d.name, d.wireguard_pubkey, d.user_id, d.created, d.description, \
            d.device_type  \"device_type: DeviceType\", configured \
            FROM device d \
            JOIN wireguard_network_device wnd ON d.id = wnd.device_id \
            WHERE $1 = ANY(wnd.wireguard_ips) AND wnd.wireguard_network_id = $2",
            IpNetwork::from(ip),
            network_id
        )
        .fetch_optional(executor)
        .await
    }

    pub async fn find_by_pubkey<'e, E>(executor: E, pubkey: &str) -> sqlx::Result<Option<Self>>
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

    pub async fn find_by_id_and_username<'e, E: sqlx::PgExecutor<'e>>(
        executor: E,
        id: Id,
        username: &str,
    ) -> sqlx::Result<Option<Self>> {
        query_as!(
            Self,
            "SELECT device.id, name, wireguard_pubkey, user_id, created, description, \
            device_type \"device_type: DeviceType\", configured \
            FROM device JOIN \"user\" ON device.user_id = \"user\".id \
            WHERE device.id = $1 AND \"user\".username = $2",
            id,
            username
        )
        .fetch_optional(executor)
        .await
    }

    pub async fn all_for_username(pool: &PgPool, username: &str) -> sqlx::Result<Vec<Self>> {
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

    pub async fn get_network_configs(
        &self,
        network: &WireguardNetwork<Id>,
        transaction: &mut PgConnection,
    ) -> Result<(DeviceNetworkInfo, DeviceConfig), DeviceError> {
        let wireguard_network_device =
            WireguardNetworkDevice::find(&mut *transaction, self.id, network.id)
                .await?
                .ok_or_else(|| DeviceError::Unexpected("Device not found in network".into()))?;
        let device_network_info = wireguard_network_device
            .to_device_network_info_runtime(&mut *transaction, network)
            .await?;

        let config = Self::create_config(network, &wireguard_network_device);
        let device_config = DeviceConfig {
            network_id: network.id,
            network_name: network.name.clone(),
            config,
            endpoint: format!("{}:{}", network.endpoint, network.port),
            address: wireguard_network_device.wireguard_ips,
            allowed_ips: network.allowed_ips.clone(),
            pubkey: network.pubkey.clone(),
            dns: network.dns.clone(),
            keepalive_interval: network.keepalive_interval,
            location_mfa_mode: network.location_mfa_mode.clone(),
            service_location_mode: network.service_location_mode.clone(),
        };

        Ok((device_network_info, device_config))
    }

    pub async fn add_to_network(
        &self,
        network: &WireguardNetwork<Id>,
        ip: &[IpAddr],
        transaction: &mut PgConnection,
    ) -> Result<(DeviceNetworkInfo, DeviceConfig), DeviceError> {
        let wireguard_network_device = self
            .assign_network_ips(&mut *transaction, network, ip)
            .await?;
        let device_network_info = wireguard_network_device
            .to_device_network_info_runtime(&mut *transaction, network)
            .await?;

        let config = Self::create_config(network, &wireguard_network_device);
        let device_config = DeviceConfig {
            network_id: network.id,
            network_name: network.name.clone(),
            config,
            endpoint: format!("{}:{}", network.endpoint, network.port),
            address: wireguard_network_device.wireguard_ips,
            allowed_ips: network.allowed_ips.clone(),
            pubkey: network.pubkey.clone(),
            dns: network.dns.clone(),
            keepalive_interval: network.keepalive_interval,
            location_mfa_mode: network.location_mfa_mode.clone(),
            service_location_mode: network.service_location_mode.clone(),
        };

        Ok((device_network_info, device_config))
    }

    /// Add device to all existing networks.
    pub async fn add_to_all_networks(
        &self,
        conn: &mut PgConnection,
    ) -> Result<(Vec<DeviceNetworkInfo>, Vec<DeviceConfig>), DeviceError> {
        info!("Adding device {} to all existing networks", self.name);
        let networks = WireguardNetwork::all(&mut *conn).await?;

        let mut configs = Vec::new();
        let mut network_info = Vec::new();
        for network in networks {
            debug!(
                "Assigning IP for device {} (user {}) in network {network}",
                self.name, self.user_id
            );
            // check for pubkey conflicts with networks
            if network.pubkey == self.wireguard_pubkey {
                return Err(DeviceError::PubkeyConflict(self.wireguard_pubkey.clone()));
            }
            if WireguardNetworkDevice::find(&mut *conn, self.id, network.id)
                .await?
                .is_some()
            {
                debug!("Device {self} already has an IP within network {network}. Skipping...");
                continue;
            }

            let wireguard_network_device = match network
                .add_device_to_network(&mut *conn, self, None)
                .await
            {
                Ok(device) => device,
                Err(WireguardNetworkError::DeviceNotAllowed(_)) => {
                    debug!("Device {self} is not allowed in network {network}, skipping");
                    continue;
                }
                Err(WireguardNetworkError::DeviceError(DeviceError::NetworkFull(_))) => {
                    warn!("Network {network} is full, no IP addresses available for device {self}");
                    return Err(DeviceError::NetworkFull(network.name.clone()));
                }
                Err(err) => {
                    warn!("Failed to add device {self} to network {network}: {err}");
                    return Err(DeviceError::Unexpected(err.to_string()));
                }
            };

            debug!(
                "Assigned IPs {} for device {} (user {}) in network {network}",
                wireguard_network_device.wireguard_ips.as_csv(),
                self.name,
                self.user_id
            );
            let device_network_info = wireguard_network_device
                .to_device_network_info_runtime(&mut *conn, &network)
                .await?;
            network_info.push(device_network_info);

            let config = Self::create_config(&network, &wireguard_network_device);
            configs.push(DeviceConfig {
                network_id: network.id,
                network_name: network.name,
                config,
                endpoint: format!("{}:{}", network.endpoint, network.port),
                address: wireguard_network_device.wireguard_ips,
                allowed_ips: network.allowed_ips,
                pubkey: network.pubkey,
                dns: network.dns,
                keepalive_interval: network.keepalive_interval,
                location_mfa_mode: network.location_mfa_mode.clone(),
                service_location_mode: network.service_location_mode.clone(),
            });
        }
        Ok((network_info, configs))
    }

    /// Assign the next available IP address in each subnet of the network to this device.
    ///
    /// For every CIDR block in `network.address`, this function:
    /// 1. If `current_ips` contains an IP that already falls within the subnet, reuses it
    ///    immediately without consulting `used_ips` or scanning the address space.
    /// 2. Otherwise, iterates through the block's IPs in order and skips any IP that is:
    ///    - The network address, broadcast address, or the subnet's host IP (gateway), or
    ///    - Present in `used_ips` (already assigned to another device), or
    ///    - Present in the optional `reserved_ips`.
    /// 3. Selects the first remaining IP and records it.
    ///
    /// If any subnet has no valid, unassigned IP, the method returns `ModelError::CannotCreate`.
    ///
    /// # Parameters
    ///
    /// - `transaction`: Active PostgreSQL connection used to persist the assignment.
    /// - `network`: The `WireguardNetwork<Id>` whose subnets will be assigned.
    /// - `used_ips`: Set of IPs already assigned within the network (caller-maintained snapshot).
    /// - `reserved_ips`: Optional slice of IPs that must not be assigned, even if otherwise free.
    /// - `current_ips`: Optional slice of IPs already assigned to this device. An IP that still
    ///   falls within its subnet is reused as-is; only IPs that no longer fit their subnet are
    ///   replaced.
    ///
    /// # Returns
    ///
    /// - `Ok(WireguardNetworkDevice)`: A new relation linking this device to its assigned IPs across all subnets.
    /// - `Err(DeviceError::NetworkFull)`: If any subnet lacks an available IP.
    pub async fn assign_next_network_ip(
        &self,
        transaction: &mut PgConnection,
        network: &WireguardNetwork<Id>,
        used_ips: &HashSet<IpAddr>,
        reserved_ips: Option<&[IpAddr]>,
        current_ips: Option<&[IpAddr]>,
    ) -> Result<WireguardNetworkDevice, DeviceError> {
        debug!(
            "Assiging IP addresses for device: {} in network {}",
            self.name, network.name
        );
        let mut ips = Vec::new();
        let reserved = reserved_ips.unwrap_or_default();

        // Iterate over all network addresses and assign new IP for the device in each of them
        for address in network.address() {
            debug!(
                "Assigning address to device {} in network {} {address}",
                self.name, network.name,
            );
            // Don't reassign addresses for networks that didn't change
            if let Some(ip) =
                current_ips.and_then(|ips| ips.iter().find(|ip| address.contains(**ip)))
            {
                debug!(
                    "Skipping reassignment of already assigned valid IP {ip} for device {} in network {} with addresses {:?}",
                    self.name,
                    network.name,
                    network.address()
                );
                ips.push(*ip);
                continue;
            }
            let mut picked = None;
            for ip in address {
                if ip == address.network() || ip == address.broadcast() || ip == address.ip() {
                    continue;
                }

                if used_ips.contains(&ip) || reserved.contains(&ip) {
                    continue;
                }

                picked = Some(ip);
                break;
            }

            // Return error if no address can be assigned
            let ip = picked.ok_or_else(|| {
                error!(
                    "Failed to assign address for device {} in network {address:?}",
                    self.name,
                );
                DeviceError::NetworkFull(address.to_string())
            })?;

            // Otherwise, store the IP address
            debug!(
                "Found assignable address {ip} for device {} in network {} {address}",
                self.name, network.name,
            );
            ips.push(ip);
        }

        // Create relation record
        let wireguard_network_device =
            WireguardNetworkDevice::new(network.id, self.id, ips.clone());
        wireguard_network_device.insert(&mut *transaction).await?;

        info!(
            "Assigned IP addresses {ips:?} for device: {} in network {}",
            self.name, network.name
        );
        Ok(wireguard_network_device)
    }

    /// Assigns specific IP address to the device in specified [`WireguardNetwork`].
    /// This method is currently used only for network devices. For regular user
    /// devices use [`assign_next_network_ip`] method.
    pub(crate) async fn assign_network_ips(
        &self,
        transaction: &mut PgConnection,
        network: &WireguardNetwork<Id>,
        ips: &[IpAddr],
    ) -> Result<WireguardNetworkDevice, NetworkAddressError> {
        debug!(
            "Assigning IPs: {ips:?} for device: {} in network {}",
            self.name, network.name
        );
        // ensure assignment is valid
        network
            .can_assign_ips(&mut *transaction, ips, Some(self.id))
            .await
            .map_err(|err| {
                error!("Invalid network IP assignment: {err}");
                err
            })?;

        // insert relation record
        let wireguard_network_device = WireguardNetworkDevice::new(network.id, self.id, ips);
        wireguard_network_device.insert(&mut *transaction).await?;
        info!(
            "Assigned IPs: {ips:?} for device: {} in network {}",
            self.name, network.name
        );
        Ok(wireguard_network_device)
    }

    pub fn validate_pubkey(pubkey: &str) -> Result<(), String> {
        if let Ok(key) = BASE64_STANDARD.decode(pubkey) {
            if key.len() == KEY_LENGTH {
                return Ok(());
            }
        }

        Err(format!("{pubkey} is not a valid pubkey"))
    }

    pub async fn find_by_type<'e, E>(
        executor: E,
        device_type: DeviceType,
    ) -> sqlx::Result<Vec<Self>>
    where
        E: PgExecutor<'e>,
    {
        query_as!(
            Self,
            "SELECT id, name, wireguard_pubkey, user_id, created, description, \
            device_type \"device_type: DeviceType\", configured \
            FROM device WHERE device_type = $1 ORDER BY name",
            device_type as DeviceType
        )
        .fetch_all(executor)
        .await
    }

    pub async fn find_by_type_paginated<'e, E>(
        executor: E,
        device_type: DeviceType,
        limit: i64,
        offset: i64,
    ) -> sqlx::Result<Vec<Self>>
    where
        E: PgExecutor<'e>,
    {
        query_as!(
            Self,
            "SELECT id, name, wireguard_pubkey, user_id, created, description, \
            device_type \"device_type: DeviceType\", configured \
            FROM device WHERE device_type = $1 ORDER BY name \
            LIMIT $2 OFFSET $3",
            device_type as DeviceType,
            limit,
            offset
        )
        .fetch_all(executor)
        .await
    }

    pub async fn count_by_type<'e, E>(executor: E, device_type: DeviceType) -> sqlx::Result<i64>
    where
        E: PgExecutor<'e>,
    {
        let count = query_scalar!(
            "SELECT count(*) FROM device WHERE device_type = $1",
            device_type as DeviceType
        )
        .fetch_one(executor)
        .await?
        .unwrap_or_default();

        Ok(count)
    }

    pub async fn find_by_type_and_network<'e, E>(
        executor: E,
        device_type: DeviceType,
        network_id: Id,
    ) -> sqlx::Result<Vec<Self>>
    where
        E: PgExecutor<'e>,
    {
        query_as!(
            Self,
            "SELECT id, name, wireguard_pubkey, user_id, created, description, \
            device_type \"device_type: DeviceType\", configured \
            FROM device WHERE device_type = $1 \
            AND id IN \
            (SELECT device_id FROM wireguard_network_device WHERE wireguard_network_id = $2) \
            ORDER BY name",
            device_type as DeviceType,
            network_id
        )
        .fetch_all(executor)
        .await
    }

    pub async fn get_owner<'e, E>(&self, executor: E) -> sqlx::Result<User<Id>>
    where
        E: PgExecutor<'e>,
    {
        query_as!(
            User,
            "SELECT id, username, password_hash, last_name, first_name, email, phone, mfa_enabled, \
            totp_enabled, email_mfa_enabled, totp_secret, email_mfa_secret, \
            mfa_method \"mfa_method: _\", recovery_codes, is_active, openid_sub, \
            from_ldap, ldap_pass_randomized, ldap_rdn, ldap_user_path, enrollment_pending \
            FROM \"user\" WHERE id = $1",
            self.user_id
        )
        .fetch_one(executor)
        .await
    }

    pub async fn last_connected_at<'e, E: PgExecutor<'e>>(
        &self,
        executor: E,
        location_id: Id,
    ) -> sqlx::Result<Option<NaiveDateTime>> {
        query_scalar!(
            "SELECT connected_at \"connected_at!\" FROM vpn_client_session \
    		WHERE location_id = $1 AND device_id = $2 AND connected_at IS NOT NULL \
    		ORDER BY connected_at DESC LIMIT 1",
            location_id,
            self.id
        )
        .fetch_optional(executor)
        .await
    }
}

#[cfg(test)]
mod test {
    use std::{net::Ipv4Addr, str::FromStr};

    use claims::{assert_err, assert_ok};
    use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

    use super::*;
    use crate::db::{models::vpn_client_session::VpnClientMfaMethod, setup_pool};

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
            if let Some(address) = network.address().first() {
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
                            WireguardNetworkDevice::new(network.id, device.id, [ip]);
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
    async fn test_assign_device_ip(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;

        let network = WireguardNetwork::default()
            .try_set_address("10.1.1.1/30")
            .unwrap()
            .save(&pool)
            .await
            .unwrap();

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
        assert_eq!(wireguard_network_device.wireguard_ips.as_csv(), "10.1.1.2");

        let device = Device::new_with_ip(&pool, 1, "dev4".into(), "key4".into(), &network).await;
        assert!(device.is_err());
    }

    /// Test that assign_next_network_ip correctly preserves or reassigns device IPs
    /// when a network's address list changes.
    /// Initial network: 10.0.0.0/8, 123.10.0.0/16, 123.123.123.0/24
    /// Device IPs:      10.0.0.234,  123.10.33.44,  123.123.123.52
    /// New network:     10.0.0.0/16, 123.12.0.0/16, 123.123.0.0/16
    /// Expected:
    ///  - 10.0.0.234     KEPT    (still within 10.0.0.0/16)
    ///  - 123.10.33.44   CHANGED (not within 123.12.0.0/16)
    ///  - 123.123.123.52 KEPT    (still within 123.123.0.0/16)
    #[sqlx::test]
    async fn test_assign_next_network_ip_preserves_matching_subnets(
        _: PgPoolOptions,
        options: PgConnectOptions,
    ) {
        let pool = setup_pool(options).await;

        let network = WireguardNetwork::default()
            .try_set_address("10.0.0.1/8,123.10.0.1/16,123.123.123.1/24")
            .unwrap()
            .save(&pool)
            .await
            .unwrap();

        let user = User::new(
            "testuser",
            Some("password"),
            "Tester",
            "Test",
            "test@test.com",
            None,
        )
        .save(&pool)
        .await
        .unwrap();

        let device = Device::new(
            "dev1".into(),
            "key1".into(),
            user.id,
            DeviceType::User,
            None,
            true,
        )
        .save(&pool)
        .await
        .unwrap();

        let ip = IpAddr::from_str("10.0.0.234").unwrap();
        let ip2 = IpAddr::from_str("123.10.33.44").unwrap();
        let ip3 = IpAddr::from_str("123.123.123.52").unwrap();
        let initial_ips = vec![ip, ip2, ip3];

        let mut conn = pool.acquire().await.unwrap();
        WireguardNetworkDevice::new(network.id, device.id, initial_ips.clone())
            .insert(&mut *conn)
            .await
            .unwrap();

        let updated_network = network
            .clone()
            .set_address([
                "10.0.0.1/16".parse().unwrap(),
                "123.12.0.1/16".parse().unwrap(),
                "123.123.0.1/16".parse().unwrap(),
            ])
            .unwrap();
        updated_network.save(&mut *conn).await.unwrap();

        let used_ips = updated_network
            .all_used_ips_for_network(&mut conn)
            .await
            .unwrap();

        let result = device
            .assign_next_network_ip(
                &mut conn,
                &updated_network,
                &used_ips,
                None,
                Some(&initial_ips),
            )
            .await
            .unwrap();

        let new_ips = &result.wireguard_ips;
        assert_eq!(new_ips.len(), 3, "should have one IP per subnet");

        assert!(
            new_ips.contains(&ip),
            "10.0.0.234 should be kept – it is still within 10.0.0.0/16; got {new_ips:?}"
        );

        assert!(
            !new_ips.contains(&ip2),
            "123.10.33.44 should be reassigned – not within 123.12.0.0/16; got {new_ips:?}"
        );
        let network: IpNetwork = "123.12.0.0/16".parse().unwrap();
        assert!(
            new_ips.iter().any(|ip| network.contains(*ip)),
            "a new IP within 123.12.0.0/16 should be assigned; got {new_ips:?}"
        );

        assert!(
            new_ips.contains(&ip3),
            "123.123.123.52 should be kept – it is still within 123.123.0.0/16; got {new_ips:?}"
        );
    }
    /// Initial:  10.0.0.0/8  | 10.1.0.5
    /// Modified: 10.0.0.0/16 | 10.1.0.5 should be replaced with a 10.0.x.x address
    #[sqlx::test]
    async fn test_assign_next_network_ip_subnet_narrowed(
        _: PgPoolOptions,
        options: PgConnectOptions,
    ) {
        let pool = setup_pool(options).await;

        let network = WireguardNetwork::default()
            .try_set_address("10.0.0.1/8")
            .unwrap()
            .save(&pool)
            .await
            .unwrap();

        let user = User::new(
            "testuser",
            Some("password"),
            "Tester",
            "Test",
            "test@test.com",
            None,
        )
        .save(&pool)
        .await
        .unwrap();

        let device = Device::new(
            "dev1".into(),
            "key1".into(),
            user.id,
            DeviceType::User,
            None,
            true,
        )
        .save(&pool)
        .await
        .unwrap();

        let ip = IpAddr::from_str("10.1.0.5").unwrap();
        let initial_ips = vec![ip];

        let mut conn = pool.acquire().await.unwrap();
        WireguardNetworkDevice::new(network.id, device.id, initial_ips.clone())
            .insert(&mut *conn)
            .await
            .unwrap();

        let updated_network = network
            .clone()
            .set_address(["10.0.0.1/16".parse().unwrap()])
            .unwrap();
        updated_network.save(&mut *conn).await.unwrap();

        let used_ips = updated_network
            .all_used_ips_for_network(&mut conn)
            .await
            .unwrap();

        let result = device
            .assign_next_network_ip(
                &mut conn,
                &updated_network,
                &used_ips,
                None,
                Some(&initial_ips),
            )
            .await
            .unwrap();

        let new_ips = &result.wireguard_ips;
        assert_eq!(new_ips.len(), 1, "should have one IP per subnet");

        assert!(
            !new_ips.contains(&ip),
            "10.1.0.5 should be reassigned – outside narrowed 10.0.0.0/16; got {new_ips:?}"
        );
        let narrowed_net: IpNetwork = "10.0.0.0/16".parse().unwrap();
        assert!(
            new_ips.iter().all(|ip| narrowed_net.contains(*ip)),
            "new IP must be within 10.0.0.0/16; got {new_ips:?}"
        );
    }

    /// Initial:  123.123.123.0/24 | 123.123.123.254
    /// Modified: 123.123.0.0/16   | 123.123.123.254 still fits
    #[sqlx::test]
    async fn test_assign_next_network_ip_still_valid_after_widening(
        _: PgPoolOptions,
        options: PgConnectOptions,
    ) {
        let pool = setup_pool(options).await;

        let network = WireguardNetwork::default()
            .try_set_address("123.123.123.1/24")
            .unwrap()
            .save(&pool)
            .await
            .unwrap();

        let user = User::new(
            "testuser",
            Some("password"),
            "Tester",
            "Test",
            "test@test.com",
            None,
        )
        .save(&pool)
        .await
        .unwrap();

        let device = Device::new(
            "dev1".into(),
            "key1".into(),
            user.id,
            DeviceType::User,
            None,
            true,
        )
        .save(&pool)
        .await
        .unwrap();

        let ip = IpAddr::from_str("123.123.123.254").unwrap();
        let initial_ips = vec![ip];

        let mut conn = pool.acquire().await.unwrap();
        WireguardNetworkDevice::new(network.id, device.id, initial_ips.clone())
            .insert(&mut *conn)
            .await
            .unwrap();

        let updated_network = network
            .clone()
            .set_address(["123.123.0.1/16".parse().unwrap()])
            .unwrap();
        updated_network.save(&mut *conn).await.unwrap();

        let used_ips = updated_network
            .all_used_ips_for_network(&mut conn)
            .await
            .unwrap();

        let result = device
            .assign_next_network_ip(
                &mut conn,
                &updated_network,
                &used_ips,
                None,
                Some(&initial_ips),
            )
            .await
            .unwrap();

        let new_ips = &result.wireguard_ips;
        assert_eq!(new_ips.len(), 1, "should have one IP per subnet");

        assert!(
            new_ips.contains(&ip),
            "123.123.123.254 should be preserved – still within widened 123.123.0.0/16; got {new_ips:?}"
        );
    }

    #[test]
    fn test_pubkey_validation() {
        let invalid_test_key = "invalid_key";
        assert_err!(Device::validate_pubkey(invalid_test_key));

        let valid_test_key = "sejIy0WCLvOR7vWNchP9Elsayp3UTK/QCnEJmhsHKTc=";
        assert_ok!(Device::validate_pubkey(valid_test_key));
    }

    #[sqlx::test]
    async fn test_runtime_mfa_state_marks_mfa_session_without_preshared_key_as_unauthorized(
        _: PgPoolOptions,
        options: PgConnectOptions,
    ) {
        let pool = setup_pool(options).await;

        let network = WireguardNetwork::new(
            "runtime-mfa-network".into(),
            51820,
            "vpn.example.com".into(),
            None,
            Vec::<IpNetwork>::new(),
            false,
            false,
            false,
            LocationMfaMode::Internal,
            ServiceLocationMode::Disabled,
        )
        .try_set_address("10.1.1.1/24")
        .unwrap()
        .save(&pool)
        .await
        .unwrap();

        let wireguard_network_device = WireguardNetworkDevice {
            wireguard_network_id: network.id,
            wireguard_ips: vec![IpAddr::from_str("10.1.1.2").unwrap()],
            device_id: 1,
        };
        let active_session = VpnClientSession {
            id: 1,
            location_id: network.id,
            user_id: 1,
            device_id: wireguard_network_device.device_id,
            created_at: Utc::now().naive_utc(),
            connected_at: None,
            disconnected_at: None,
            mfa_method: Some(VpnClientMfaMethod::Totp),
            state: VpnClientSessionState::New,
            preshared_key: None,
        };

        let network_info =
            wireguard_network_device.to_device_network_info(&network, Some(&active_session));

        assert_eq!(network_info.preshared_key, None);
        assert!(!network_info.is_authorized);
    }

    #[sqlx::test]
    async fn test_runtime_mfa_state_keeps_session_preshared_key_for_authorized_runtime_reads(
        _: PgPoolOptions,
        options: PgConnectOptions,
    ) {
        let pool = setup_pool(options).await;

        let network = WireguardNetwork::new(
            "runtime-mfa-network".into(),
            51820,
            "vpn.example.com".into(),
            None,
            Vec::<IpNetwork>::new(),
            false,
            false,
            false,
            LocationMfaMode::Internal,
            ServiceLocationMode::Disabled,
        )
        .try_set_address("10.1.1.1/24")
        .unwrap()
        .save(&pool)
        .await
        .unwrap();

        let wireguard_network_device = WireguardNetworkDevice {
            wireguard_network_id: network.id,
            wireguard_ips: vec![IpAddr::from_str("10.1.1.2").unwrap()],
            device_id: 1,
        };
        let active_session = VpnClientSession {
            id: 1,
            location_id: network.id,
            user_id: 1,
            device_id: wireguard_network_device.device_id,
            created_at: Utc::now().naive_utc(),
            connected_at: Some(Utc::now().naive_utc()),
            disconnected_at: None,
            mfa_method: Some(VpnClientMfaMethod::Totp),
            state: VpnClientSessionState::Connected,
            preshared_key: Some("runtime-session-psk".into()),
        };

        let network_info =
            wireguard_network_device.to_device_network_info(&network, Some(&active_session));

        assert_eq!(
            network_info.preshared_key,
            Some("runtime-session-psk".into())
        );
        assert!(network_info.is_authorized);
    }

    #[sqlx::test]
    async fn test_device_info_marks_mfa_session_without_preshared_key_as_unauthorized(
        _: PgPoolOptions,
        options: PgConnectOptions,
    ) {
        let pool = setup_pool(options).await;

        let user = User::new(
            "testuser",
            Some("password"),
            "Tester",
            "Test",
            "test@test.com",
            None,
        )
        .save(&pool)
        .await
        .unwrap();

        let device = Device::new(
            "device".into(),
            "pubkey".into(),
            user.id,
            DeviceType::User,
            None,
            true,
        )
        .save(&pool)
        .await
        .unwrap();

        let network = WireguardNetwork::new(
            "device-info-network".into(),
            51820,
            "vpn.example.com".into(),
            None,
            Vec::<IpNetwork>::new(),
            false,
            false,
            false,
            LocationMfaMode::Internal,
            ServiceLocationMode::Disabled,
        )
        .try_set_address("10.1.1.1/24")
        .unwrap();
        let network = network.save(&pool).await.unwrap();

        let wireguard_network_device = WireguardNetworkDevice::new(
            network.id,
            device.id,
            [IpAddr::from_str("10.1.1.2").unwrap()],
        );
        wireguard_network_device.insert(&pool).await.unwrap();

        let session = VpnClientSession::new(
            network.id,
            user.id,
            device.id,
            None,
            Some(VpnClientMfaMethod::Totp),
        );
        session.save(&pool).await.unwrap();

        let device_info = DeviceInfo::from_device(&pool, device).await.unwrap();
        let network_info = device_info
            .network_info
            .into_iter()
            .find(|info| info.network_id == network.id)
            .unwrap();

        assert!(!network_info.is_authorized);
        assert_eq!(network_info.preshared_key, None);
    }

    #[sqlx::test]
    async fn test_device_info_keeps_mfa_session_preshared_key_for_authorized_full_sync_reads(
        _: PgPoolOptions,
        options: PgConnectOptions,
    ) {
        let pool = setup_pool(options).await;

        let user = User::new(
            "testuser",
            Some("password"),
            "Tester",
            "Test",
            "test@test.com",
            None,
        )
        .save(&pool)
        .await
        .unwrap();

        let device = Device::new(
            "device".into(),
            "pubkey".into(),
            user.id,
            DeviceType::User,
            None,
            true,
        )
        .save(&pool)
        .await
        .unwrap();

        let network = WireguardNetwork::new(
            "device-info-network".into(),
            51820,
            "vpn.example.com".into(),
            None,
            Vec::<IpNetwork>::new(),
            false,
            false,
            false,
            LocationMfaMode::Internal,
            ServiceLocationMode::Disabled,
        )
        .try_set_address("10.1.1.1/24")
        .unwrap()
        .save(&pool)
        .await
        .unwrap();

        let wireguard_network_device = WireguardNetworkDevice::new(
            network.id,
            device.id,
            [IpAddr::from_str("10.1.1.2").unwrap()],
        );
        wireguard_network_device.insert(&pool).await.unwrap();

        let mut session = VpnClientSession::new(
            network.id,
            user.id,
            device.id,
            Some(Utc::now().naive_utc()),
            Some(VpnClientMfaMethod::Totp),
        );
        session.preshared_key = Some("device-info-session-psk".into());
        session.save(&pool).await.unwrap();

        let device_info = DeviceInfo::from_device(&pool, device).await.unwrap();
        let network_info = device_info
            .network_info
            .into_iter()
            .find(|info| info.network_id == network.id)
            .unwrap();

        assert!(network_info.is_authorized);
        assert_eq!(
            network_info.preshared_key,
            Some("device-info-session-psk".into())
        );
    }

    #[sqlx::test]
    async fn test_device_info_keeps_non_mfa_location_authorized_without_exposing_session_preshared_key(
        _: PgPoolOptions,
        options: PgConnectOptions,
    ) {
        let pool = setup_pool(options).await;

        let user = User::new(
            "testuser",
            Some("password"),
            "Tester",
            "Test",
            "test@test.com",
            None,
        )
        .save(&pool)
        .await
        .unwrap();

        let device = Device::new(
            "device".into(),
            "pubkey".into(),
            user.id,
            DeviceType::User,
            None,
            true,
        )
        .save(&pool)
        .await
        .unwrap();

        let network = WireguardNetwork::new(
            "device-info-network".into(),
            51820,
            "vpn.example.com".into(),
            None,
            Vec::<IpNetwork>::new(),
            false,
            false,
            false,
            LocationMfaMode::Disabled,
            ServiceLocationMode::Disabled,
        )
        .try_set_address("10.1.1.1/24")
        .unwrap()
        .save(&pool)
        .await
        .unwrap();

        let wireguard_network_device = WireguardNetworkDevice::new(
            network.id,
            device.id,
            [IpAddr::from_str("10.1.1.2").unwrap()],
        );
        wireguard_network_device.insert(&pool).await.unwrap();

        let mut session = VpnClientSession::new(network.id, user.id, device.id, None, None);
        session.preshared_key = Some("legacy-session-psk".into());
        session.save(&pool).await.unwrap();

        let device_info = DeviceInfo::from_device(&pool, device).await.unwrap();
        let network_info = device_info
            .network_info
            .into_iter()
            .find(|info| info.network_id == network.id)
            .unwrap();

        assert!(network_info.is_authorized);
        assert_eq!(network_info.preshared_key, None);
    }

    #[sqlx::test]
    fn test_all_for_network_and_user(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;

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

        let mut network = WireguardNetwork::default()
            .try_set_address("10.1.1.1/24")
            .unwrap();
        network.allow_all_groups = true;
        let network = network.save(&pool).await.unwrap();
        let mut network_2 = WireguardNetwork::default()
            .try_set_address("10.1.2.1/24")
            .unwrap();
        network_2.name = "testnetwork2".into();
        network_2.allow_all_groups = true;
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
            [IpAddr::from_str("10.1.1.10").unwrap()],
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

    // Mimic what add_device handler does.
    #[sqlx::test]
    fn test_saturated_network(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;

        let user = User::new("tester", None, "Tester", "Test", "test@test.pl", None)
            .save(&pool)
            .await
            .unwrap();

        let mut network = WireguardNetwork::default()
            .set_address([IpNetwork::new(IpAddr::V4(Ipv4Addr::new(192, 168, 42, 4)), 29).unwrap()])
            .unwrap();
        network.allow_all_groups = true;
        let network = network.save(&pool).await.unwrap();

        let mut conn = pool.begin().await.unwrap();

        for (name, pubkey) in [
            ("device1", "LQKsT6/3HWKuJmMulH63R8iK+5sI8FyYEL6WDIi6lQU="),
            ("device2", "AJwxGkzvVVn5Q1xjpCDFo5RJSU9KOPHeoEixYaj+20M="),
            ("device3", "OLQNaEH3FxW0hiodaChEHoETzd+7UzcqIbsLs+X8rD0="),
            ("device4", "mgVXE8WcfStoD8mRatHcX5aaQ0DlcpjvPXibHEOr9y8="),
            ("device5", "hNuapt7lOxF93KUqZGUY00oKJxH8LYwwsUVB1uUa0y4="),
        ] {
            let device = Device::new(
                name.to_string(),
                pubkey.to_string(),
                user.id,
                DeviceType::User,
                None,
                true,
            )
            .save(&mut *conn)
            .await
            .unwrap();
            let (_, _) = device.add_to_all_networks(&mut conn).await.unwrap();
        }

        // This device won't fit in the address space.
        let _device = Device::new(
            "device6".to_string(),
            "fF9K0tgatZTEJRvzpNUswr0h8HqCIi+v39B45+QZZzE=".to_string(),
            user.id,
            DeviceType::User,
            None,
            true,
        )
        .save(&mut *conn)
        .await
        .unwrap();
        // FIXME: uncomment when `add_to_all_networks` is fixed.
        // assert!(device.add_to_all_networks(&mut conn).await.is_err());

        conn.commit().await.unwrap();

        let devices = Device::all(&pool).await.unwrap();
        assert_eq!(6, devices.len(), "{devices:#?}");
        let network_devices = WireguardNetworkDevice::all_for_network(&pool, network.id)
            .await
            .unwrap();
        assert_eq!(5, network_devices.len(), "{network_devices:#?}");
    }
}
