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
use tracing::{debug, error, info};

use crate::{
    KEY_LENGTH,
    db::{
        Id, NoId,
        models::{
            ModelError, WireguardNetwork,
            mfa_flow::MfaFlowStep,
            user::User,
            vpn_client_session::{VpnClientSession, VpnClientSessionState},
            wireguard::{LocationMfaMode, NetworkAddressError, ServiceLocationMode},
        },
    },
};

#[derive(Serialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct DeviceConfig {
    pub network_id: Id,
    pub network_name: String,
    pub config: String,
    #[cfg_attr(feature = "openapi", schema(value_type = Vec<String>))]
    pub address: Vec<IpAddr>,
    pub endpoint: String,
    #[cfg_attr(feature = "openapi", schema(value_type = Vec<String>))]
    pub allowed_ips: Vec<IpNetwork>,
    pub pubkey: String,
    pub dns: Option<String>,
    pub keepalive_interval: i32,
    /// MTU for the client, from the location's `client_mtu`.
    pub mtu: Option<i32>,
    /// Whether the location requires MFA. This is the authoritative flag, read from the stored
    /// `wireguard_network.mfa_enabled` column.
    pub mfa_enabled: bool,
    /// Legacy single-factor mode, derived in memory for backward-compatible locations only.
    /// `None` when the location's flow configuration cannot be expressed as a legacy mode, which
    /// includes every location that has no flows at all. Consumers deciding whether a location
    /// requires MFA must use `mfa_enabled`, not the absence of this field.
    pub location_mfa_mode: Option<LocationMfaMode>,
    pub service_location_mode: ServiceLocationMode,
    pub posture_check_required: bool,
    /// The MFA flow steps resolved for this location and user, ordered by step position. Empty
    /// when the location has MFA disabled or no flow resolves for the user.
    #[serde(skip)]
    pub steps: Vec<MfaFlowStep<Id>>,
}

// The type of a device:
// User: A device of a user, which may be in multiple networks, e.g. a laptop
// Network: A stand-alone device added by a user permanently bound to one network, e.g. a printer
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, Type)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
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

#[derive(Clone, Debug, Deserialize, FromRow, Model, Serialize, PartialEq)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct Device<I = NoId> {
    #[cfg_attr(feature = "openapi", schema(value_type = i64))]
    pub id: I,
    pub name: String,
    pub wireguard_pubkey: String,
    pub user_id: Id,
    pub created: NaiveDateTime,
    #[model(enum)]
    pub device_type: DeviceType,
    pub description: Option<String>,
    /// Whether the device is ready to use. Unconfigured devices are not sent to the gateway.
    /// Such a device is already added to all its networks, but is still missing something,
    /// for example its public key.
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
    pub fn from_authorized_vpn_session<I>(
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
                    WHEN NOT n.mfa_enabled THEN NULL::text \
                    ELSE active_session.preshared_key \
                END \"preshared_key?\", \
                CASE \
                    WHEN NOT n.mfa_enabled THEN TRUE \
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
#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct UserDevice {
    #[serde(flatten)]
    pub device: Device<Id>,
    pub networks: Vec<UserDeviceNetworkInfo>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct UserDeviceNetworkInfo {
    pub network_id: Id,
    pub network_name: String,
    pub network_gateway_ip: String,
    pub device_wireguard_ips: Vec<String>,
    pub last_connected_ip: Option<String>,
    pub last_connected_at: Option<NaiveDateTime>,
    pub is_active: bool,
    pub mfa_enabled: bool,
}

impl UserDevice {
    pub async fn from_device(pool: &PgPool, device: Device<Id>) -> sqlx::Result<Option<Self>> {
        // fetch device config and connection info for all allowed networks
        let result = query!(
            "SELECT n.id network_id, n.name network_name, n.endpoint gateway_endpoint, \
	            wnd.wireguard_ips \"device_wireguard_ips: Vec<IpAddr>\", \
				latest_successful_stats.endpoint \"device_endpoint?\", \
	            latest_successful_session.connected_at \"last_connected_at?\", \
	            latest_successful_session.state \"state?: VpnClientSessionState\", \
                n.mfa_enabled \
            FROM wireguard_network_device wnd \
            JOIN wireguard_network n ON n.id = wnd.wireguard_network_id \
            LEFT JOIN LATERAL ( \
				SELECT id, state, connected_at \
				FROM vpn_client_session \
				WHERE location_id = n.id AND device_id = wnd.device_id \
				AND connected_at IS NOT NULL \
				ORDER BY connected_at DESC, id DESC \
				LIMIT 1 \
	            ) latest_successful_session ON true \
	            LEFT JOIN LATERAL ( \
				SELECT endpoint \
				FROM vpn_session_stats \
				WHERE session_id = latest_successful_session.id \
				ORDER BY collected_at DESC, id DESC \
				LIMIT 1 \
	            ) latest_successful_stats ON true \
            WHERE wnd.device_id = $1",
            device.id,
        )
        .fetch_all(pool)
        .await?;

        let networks_info = result
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
                    last_connected_at: r.last_connected_at,
                    is_active,
                    mfa_enabled: r.mfa_enabled,
                }
            })
            .collect::<Vec<_>>();

        Ok(Some(Self {
            device,
            networks: networks_info,
        }))
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WireguardNetworkDevice {
    pub wireguard_network_id: Id,
    pub wireguard_ips: Vec<IpAddr>,
    pub device_id: Id,
}

#[derive(Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct AddDevice {
    pub name: String,
    pub wireguard_pubkey: String,
}

#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
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
        if !network.mfa_enabled {
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
        let (preshared_key, is_authorized) = if network.mfa_enabled {
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
            "SELECT device_id, wireguard_network_id, wireguard_ips \"wireguard_ips: Vec<IpAddr>\" \
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
            "SELECT device_id, wireguard_network_id, wireguard_ips \"wireguard_ips: Vec<IpAddr>\" \
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
    pub async fn assign_network_ips(
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
        if let Ok(key) = BASE64_STANDARD.decode(pubkey)
            && key.len() == KEY_LENGTH
        {
            return Ok(());
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
            from_ldap, ldap_pass_randomized, ldap_rdn, ldap_user_path, ldap_remote_enrollment_completed, enrollment_pending \
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
mod tests;
