use std::{
    collections::HashMap,
    fmt::{self, Display},
    iter::zip,
    net::{IpAddr, Ipv4Addr},
};

use crate::{
    auth::claims::{Claims, ClaimsType},
    db::{
        Id, NoId,
        models::{
            ModelError,
            group::{Group, Permission},
            vpn_client_session::{VpnClientSession, VpnClientSessionState},
            wireguard_peer_stats::WireguardPeerStats,
        },
    },
    types::user_info::UserInfo,
};
use base64::prelude::{BASE64_STANDARD, Engine};
use chrono::{NaiveDateTime, TimeDelta, Utc};
use ipnetwork::{IpNetwork, IpNetworkError, NetworkSize};
use model_derive::Model;
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use sqlx::{
    Error as SqlxError, FromRow, PgConnection, PgExecutor, PgPool, Type,
    postgres::types::PgInterval, query, query_as, query_scalar,
};
use thiserror::Error;
use tracing::{debug, info};
use utoipa::ToSchema;
use x25519_dalek::{PublicKey, StaticSecret};

use super::{
    device::{Device, DeviceError, DeviceType, WireguardNetworkDevice},
    user::User,
};

pub const DEFAULT_KEEPALIVE_INTERVAL: i32 = 25;
pub const DEFAULT_DISCONNECT_THRESHOLD: i32 = 300;

// Used in process of importing network from wireguard config
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MappedDevice {
    pub user_id: Id,
    pub name: String,
    pub wireguard_pubkey: String,
    pub wireguard_ips: Vec<IpAddr>,
}

pub const WIREGUARD_MAX_HANDSHAKE: TimeDelta = TimeDelta::minutes(8);
pub const PEER_STATS_LIMIT: i64 = 6 * 60;

/// Defines datetime aggregation levels
pub enum DateTimeAggregation {
    Hour,
    Minute,
}

impl DateTimeAggregation {
    /// Returns database format string for given aggregation variant
    fn fstring(&self) -> &str {
        match self {
            Self::Hour => "hour",
            Self::Minute => "minute",
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize, ToSchema, Type)]
#[sqlx(type_name = "location_mfa_mode", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum LocationMfaMode {
    #[default]
    Disabled,
    Internal,
    External,
}

impl Display for LocationMfaMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LocationMfaMode::Disabled => write!(f, "MFA disabled"),
            LocationMfaMode::Internal => write!(f, "Internal MFA"),
            LocationMfaMode::External => write!(f, "External MFA"),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize, ToSchema, Type)]
#[sqlx(type_name = "service_location_mode", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum ServiceLocationMode {
    #[default]
    Disabled,
    PreLogon,
    AlwaysOn,
}

/// Stores configuration required to setup a WireGuard network
#[derive(Clone, Deserialize, Eq, Hash, Model, PartialEq, Serialize, ToSchema)]
#[table(wireguard_network)]
pub struct WireguardNetwork<I = NoId> {
    pub id: I,
    pub name: String,
    #[model(ref)]
    #[schema(value_type = String)]
    pub address: Vec<IpNetwork>,
    pub port: i32,
    pub pubkey: String,
    #[serde(default, skip_serializing)]
    pub prvkey: String,
    pub endpoint: String,
    pub dns: Option<String>,
    #[model(ref)]
    #[schema(value_type = String)]
    pub allowed_ips: Vec<IpNetwork>,
    pub connected_at: Option<NaiveDateTime>,
    pub acl_enabled: bool,
    pub acl_default_allow: bool,
    pub keepalive_interval: i32,
    pub peer_disconnect_threshold: i32,
    #[model(enum)]
    pub location_mfa_mode: LocationMfaMode,
    #[model(enum)]
    pub service_location_mode: ServiceLocationMode,
}

pub struct WireguardKey {
    pub private: String,
    pub public: String,
}

impl fmt::Display for WireguardNetwork<NoId> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl fmt::Display for WireguardNetwork<Id> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[ID {}] {}", self.id, self.name)
    }
}

impl fmt::Debug for WireguardNetwork<Id> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WireguardNetwork")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("address", &self.address)
            .field("port", &self.port)
            .field("pubkey", &self.pubkey)
            .field("prvkey", &"***")
            .field("endpoint", &self.endpoint)
            .field("dns", &self.dns)
            .field("allowed_ips", &self.allowed_ips)
            .field("connected_at", &self.connected_at)
            .field("acl_enabled", &self.acl_enabled)
            .field("acl_default_allow", &self.acl_default_allow)
            .field("keepalive_interval", &self.keepalive_interval)
            .field("peer_disconnect_threshold", &self.peer_disconnect_threshold)
            .field("location_mfa_mode", &self.location_mfa_mode)
            .field("service_location_mode", &self.service_location_mode)
            .finish()
    }
}

#[cfg(test)]
impl Default for WireguardNetwork<Id> {
    fn default() -> Self {
        Self {
            id: Id::default(),
            name: String::default(),
            address: vec![IpNetwork::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0).unwrap()],
            port: i32::default(),
            pubkey: String::default(),
            prvkey: String::default(),
            endpoint: String::default(),
            dns: Option::default(),
            allowed_ips: Vec::default(),
            connected_at: Option::default(),
            keepalive_interval: DEFAULT_KEEPALIVE_INTERVAL,
            peer_disconnect_threshold: DEFAULT_DISCONNECT_THRESHOLD,
            acl_default_allow: false,
            acl_enabled: false,
            location_mfa_mode: LocationMfaMode::default(),
            service_location_mode: ServiceLocationMode::default(),
        }
    }
}

#[derive(Debug, Error)]
pub enum WireguardNetworkError {
    #[error("Network address space cannot fit all devices")]
    NetworkTooSmall,
    #[error(transparent)]
    IpNetworkError(#[from] IpNetworkError),
    #[error("Database error")]
    DbError(#[from] sqlx::Error),
    #[error("Model error")]
    ModelError(#[from] ModelError),
    #[error("Unexpected error: {0}")]
    Unexpected(String),
    #[error("Invalid device pubkey: {0}")]
    InvalidDevicePubkey(String),
    #[error("Device {0} not allowed in network")]
    DeviceNotAllowed(String),
    #[error("Device error")]
    DeviceError(#[from] DeviceError),
    #[error(transparent)]
    TokenError(#[from] jsonwebtoken::errors::Error),
}

#[derive(Debug, Error)]
pub enum NetworkAddressError {
    #[error(
        "Location {0} has no network that could contain IP address {1}, available networks: {2:?}"
    )]
    NoContainingNetwork(String, IpAddr, Vec<IpNetwork>),
    #[error("IP address {1} is reserved for gateway in location {0}")]
    ReservedForGateway(String, IpAddr),
    #[error("IP address {1} is network broadcast address in location {0}")]
    IsBroadcastAddress(String, IpAddr),
    #[error("IP address {1} is network address in location {0}")]
    IsNetworkAddress(String, IpAddr),
    #[error("IP address {1} is already assigned in location {0}")]
    AddressAlreadyAssigned(String, IpAddr),
    #[error(transparent)]
    DbError(#[from] sqlx::Error),
}

impl WireguardNetwork {
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn new(
        name: String,
        address: Vec<IpNetwork>,
        port: i32,
        endpoint: String,
        dns: Option<String>,
        allowed_ips: Vec<IpNetwork>,
        keepalive_interval: i32,
        peer_disconnect_threshold: i32,
        acl_enabled: bool,
        acl_default_allow: bool,
        location_mfa_mode: LocationMfaMode,
        service_location_mode: ServiceLocationMode,
    ) -> Self {
        let prvkey = StaticSecret::random_from_rng(OsRng);
        let pubkey = PublicKey::from(&prvkey);
        Self {
            id: NoId,
            name,
            address,
            port,
            pubkey: BASE64_STANDARD.encode(pubkey.to_bytes()),
            prvkey: BASE64_STANDARD.encode(prvkey.to_bytes()),
            endpoint,
            dns,
            allowed_ips,
            connected_at: None,

            keepalive_interval,
            peer_disconnect_threshold,
            acl_enabled,
            acl_default_allow,
            location_mfa_mode,
            service_location_mode,
        }
    }

    /// Try to set `address` from `&str`.
    pub fn try_set_address(&mut self, address: &str) -> Result<(), IpNetworkError> {
        use crate::utils::parse_address_list;

        let address = parse_address_list(address);
        if address.is_empty() {
            return Err(IpNetworkError::InvalidAddr("invalid address".into()));
        }
        self.address = address;

        Ok(())
    }
}

impl WireguardNetwork<Id> {
    pub async fn find_by_name<'e, E>(
        executor: E,
        name: &str,
    ) -> Result<Option<Vec<Self>>, WireguardNetworkError>
    where
        E: PgExecutor<'e>,
    {
        let networks = query_as!(
            WireguardNetwork,
            "SELECT id, name, address, port, pubkey, prvkey, endpoint, dns, allowed_ips, \
            connected_at, keepalive_interval, peer_disconnect_threshold, \
            acl_enabled, acl_default_allow, location_mfa_mode \"location_mfa_mode: LocationMfaMode\", \
            service_location_mode \"service_location_mode: ServiceLocationMode\" \
            FROM wireguard_network WHERE name = $1",
            name
        )
        .fetch_all(executor)
        .await?;

        if networks.is_empty() {
            return Ok(None);
        }

        Ok(Some(networks))
    }

    #[allow(clippy::result_large_err)]
    pub fn validate_network_size(&self, device_count: usize) -> Result<(), WireguardNetworkError> {
        debug!("Checking if {device_count} devices can fit in networks used by location {self}");
        // if given location uses multiple subnets validate devices can fit them all
        for subnet in &self.address {
            debug!("Checking if {device_count} devices can fit in network {subnet}");
            let network_size = subnet.size();
            // include address, network, and broadcast in the calculation
            match network_size {
                NetworkSize::V4(size) => {
                    if device_count as u32 > size {
                        return Err(WireguardNetworkError::NetworkTooSmall);
                    }
                }
                NetworkSize::V6(size) => {
                    if device_count as u128 > size {
                        return Err(WireguardNetworkError::NetworkTooSmall);
                    }
                }
            }
        }

        Ok(())
    }

    /// Utility method to create WireGuard keypair
    #[must_use]
    pub fn genkey() -> WireguardKey {
        let private = StaticSecret::random_from_rng(OsRng);
        let public = PublicKey::from(&private);
        WireguardKey {
            private: BASE64_STANDARD.encode(private.to_bytes()),
            public: BASE64_STANDARD.encode(public.to_bytes()),
        }
    }

    /// Get a list of all devices belonging to users in allowed groups.
    /// Admin users should always be allowed to access a network.
    /// Note: Doesn't check if the devices are really in the network.
    pub async fn get_allowed_devices(
        &self,
        transaction: &mut PgConnection,
    ) -> Result<Vec<Device<Id>>, ModelError> {
        debug!("Fetching all allowed devices for network {}", self);
        let devices = match self.get_allowed_groups(&mut *transaction).await? {
            // devices need to be filtered by allowed group
            Some(allowed_groups) => {
                query_as!(
                Device,
                "SELECT DISTINCT ON (d.id) d.id, d.name, d.wireguard_pubkey, d.user_id, d.created, d.description, d.device_type \"device_type: DeviceType\", \
                configured
                FROM device d \
                JOIN \"user\" u ON d.user_id = u.id \
                JOIN group_user gu ON u.id = gu.user_id \
                JOIN \"group\" g ON gu.group_id = g.id \
                WHERE g.\"name\" IN (SELECT * FROM UNNEST($1::text[])) \
                AND u.is_active = true \
                AND d.device_type = 'user'::device_type \
                ORDER BY d.id ASC",
                &allowed_groups
            )
                .fetch_all(&mut *transaction)
                .await?
            }
            // all devices of enabled users are allowed
            None => {
                query_as!(
                    Device,
                    "SELECT d.id, d.name, d.wireguard_pubkey, d.user_id, d.created, d.description, d.device_type \"device_type: DeviceType\", \
                    configured \
                    FROM device d \
                    JOIN \"user\" u ON d.user_id = u.id \
                    WHERE u.is_active = true \
                    AND d.device_type = 'user'::device_type \
                    ORDER BY d.id ASC"
                )
                .fetch_all(&mut *transaction)
                .await?
            }
        };
        Ok(devices)
    }

    /// Get a list of devices belonging to a user which are also in the network's allowed groups.
    /// Admin users should always be allowed to access a network.
    /// Note: Doesn't check if the devices are really in the network.
    pub async fn get_allowed_devices_for_user(
        &self,
        transaction: &mut PgConnection,
        user_id: Id,
    ) -> Result<Vec<Device<Id>>, ModelError> {
        debug!("Fetching all allowed devices for network {self}, user ID {user_id}");
        let devices = match self.get_allowed_groups(&mut *transaction).await? {
            // devices need to be filtered by allowed group
            Some(allowed_groups) => {
                query_as!(
                Device,
                "SELECT DISTINCT ON (d.id) d.id, d.name, d.wireguard_pubkey, d.user_id, d.created, d.description, d.device_type \"device_type: DeviceType\", \
                configured
                FROM device d \
                JOIN \"user\" u ON d.user_id = u.id \
                JOIN group_user gu ON u.id = gu.user_id \
                JOIN \"group\" g ON gu.group_id = g.id \
                WHERE g.\"name\" IN (SELECT * FROM UNNEST($1::text[])) \
                AND u.is_active = true \
                AND d.device_type = 'user'::device_type \
                AND d.user_id = $2 \
                ORDER BY d.id ASC",
                &allowed_groups, user_id
            )
                .fetch_all(&mut *transaction)
                .await?
            }
            // all devices of enabled users are allowed
            None => {
                query_as!(
                    Device,
                    "SELECT d.id, d.name, d.wireguard_pubkey, d.user_id, d.created, d.description, d.device_type \"device_type: DeviceType\", \
                    configured \
                    FROM device d \
                    JOIN \"user\" u ON d.user_id = u.id \
                    WHERE u.is_active = true \
                    AND d.device_type = 'user'::device_type \
                    AND d.user_id = $1 \
                    ORDER BY d.id ASC", user_id
                )
                .fetch_all(&mut *transaction)
                .await?
            }
        };

        Ok(devices)
    }

    /// Generate network IPs for all existing devices
    /// If `allowed_groups` is set, devices should be filtered accordingly
    pub async fn add_all_allowed_devices(
        &self,
        transaction: &mut PgConnection,
    ) -> Result<(), ModelError> {
        info!(
            "Assigning IPs in network {} for all existing devices ",
            self
        );
        let devices = self.get_allowed_devices(&mut *transaction).await?;
        for device in devices {
            device
                .assign_next_network_ip(&mut *transaction, self, None, None)
                .await?;
        }
        Ok(())
    }

    /// Generate network IPs for a device if it's allowed in network
    pub(crate) async fn add_device_to_network(
        &self,
        transaction: &mut PgConnection,
        device: &Device<Id>,
        reserved_ips: Option<&[IpAddr]>,
    ) -> Result<WireguardNetworkDevice, WireguardNetworkError> {
        info!("Assigning IP in network {self} for {device}");
        let allowed_devices = self.get_allowed_devices(&mut *transaction).await?;
        let allowed_device_ids: Vec<i64> = allowed_devices.iter().map(|dev| dev.id).collect();
        if allowed_device_ids.contains(&device.id) {
            let wireguard_network_device = device
                .assign_next_network_ip(&mut *transaction, self, reserved_ips, None)
                .await?;
            Ok(wireguard_network_device)
        } else {
            info!("Device {device} not allowed in network {self}");
            Err(WireguardNetworkError::DeviceNotAllowed(format!("{device}")))
        }
    }

    /// Checks if all device addresses are contained in at least one of the network addresses
    #[must_use]
    pub fn contains_all(&self, addresses: &[IpAddr]) -> bool {
        addresses
            .iter()
            .all(|addr| self.address.iter().any(|net| net.contains(*addr)))
    }

    /// Finds [`IpNetwork`] containing given [`IpAddr`]
    #[must_use]
    pub fn get_containing_network(&self, addr: IpAddr) -> Option<IpNetwork> {
        self.address.iter().find(|net| net.contains(addr)).copied()
    }

    /// Finds when the device connected based on handshake timestamps.
    async fn connected_at(
        &self,
        conn: &PgPool,
        device_id: Id,
    ) -> Result<Option<NaiveDateTime>, SqlxError> {
        // Find a first handshake gap longer than WIREGUARD_MAX_HANDSHAKE.
        // We assume that this gap indicates a time when the device was not connected.
        // So, the handshake after this gap is the moment the last connection was established.
        // If no such gap is found, the device may be connected from the beginning, return the first handshake in this case.
        let connected_at = query_scalar!(
            "WITH stats AS (SELECT * FROM wireguard_peer_stats_view WHERE device_id = $1 AND network = $2) \
            SELECT \
                COALESCE( \
                    ( \
                        SELECT latest_handshake \"latest_handshake: NaiveDateTime\" \
                        FROM stats WHERE latest_handshake_diff > $3 \
                        ORDER BY collected_at DESC LIMIT 1 \
                    ), \
                    ( \
                        SELECT latest_handshake \"latest_handshake: NaiveDateTime\" \
                        FROM stats ORDER BY collected_at LIMIT 1 \
                    ) \
                ) \
            AS latest_handshake",
            device_id,
            self.id,
            PgInterval::try_from(WIREGUARD_MAX_HANDSHAKE).unwrap()
        )
        .fetch_one(conn)
        .await?;

        Ok(connected_at)
    }

    /// Retrieves stats for specified devices
    pub(crate) async fn device_stats(
        &self,
        conn: &PgPool,
        devices: &[Device<Id>],
        from: &NaiveDateTime,
        aggregation: &DateTimeAggregation,
    ) -> Result<Vec<WireguardDeviceStatsRow>, SqlxError> {
        if devices.is_empty() {
            return Ok(Vec::new());
        }
        // query_as! macro doesn't work with `... WHERE ... IN (...) `
        // so we'll have to use format! macro
        // https://github.com/launchbadge/sqlx/issues/875
        // https://github.com/launchbadge/sqlx/issues/656
        let device_ids = devices
            .iter()
            .map(|d| d.id.to_string())
            .collect::<Vec<String>>()
            .join(",");
        let query = format!(
            "SELECT device_id, device.name, device.user_id, \
            date_trunc($1, collected_at) collected_at, \
            CAST(sum(download) AS bigint) download, \
            CAST(sum(upload) AS bigint) upload \
            FROM wireguard_peer_stats_view wpsv \
            JOIN device ON wpsv.device_id = device.id \
            WHERE device_id IN ({device_ids}) \
            AND collected_at >= $2 \
            AND network = $3 \
            GROUP BY 1, 2, 3, 4 ORDER BY 1, 4"
        );
        let stats: Vec<WireguardDeviceTransferRow> = query_as(&query)
            .bind(aggregation.fstring())
            .bind(from)
            .bind(self.id)
            .fetch_all(conn)
            .await?;
        let mut result = Vec::new();
        for device in devices {
            let latest_stats = WireguardPeerStats::fetch_latest(conn, device.id, self.id).await?;
            let wireguard_ips = if let Some(stats) = &latest_stats {
                stats.trim_allowed_ips()
            } else {
                Vec::new()
            };
            result.push(WireguardDeviceStatsRow {
                id: device.id,
                user_id: device.user_id,
                name: device.name.clone(),
                wireguard_ips,
                public_ip: latest_stats
                    .as_ref()
                    .and_then(WireguardPeerStats::endpoint_without_port),
                connected_at: self.connected_at(conn, device.id).await?,
                // Filter stats for this device
                stats: stats
                    .iter()
                    .filter(|s| s.device_id == device.id)
                    .cloned()
                    .collect(),
            });
        }
        Ok(result)
    }

    pub async fn distinct_device_stats(
        &self,
        conn: &PgPool,
        from: &NaiveDateTime,
        aggregation: &DateTimeAggregation,
        device_type: DeviceType,
    ) -> Result<Vec<WireguardDeviceStatsRow>, SqlxError> {
        let oldest_handshake = (Utc::now() - WIREGUARD_MAX_HANDSHAKE).naive_utc();
        // Retrieve connected devices from database
        let devices = query_as!(
            Device,
            "SELECT DISTINCT ON (d.id) d.id, d.name, d.wireguard_pubkey, d.user_id, d.created, \
            d.description, d.device_type \"device_type: DeviceType\", d.configured \
            FROM device d JOIN wireguard_peer_stats s ON d.id = s.device_id \
            WHERE s.latest_handshake >= $1 AND s.network = $2 \
            AND d.device_type = $3",
            oldest_handshake,
            self.id,
            &device_type as &DeviceType,
        )
        .fetch_all(conn)
        .await?;
        // Retrieve data series for all active devices and assign them to users
        self.device_stats(conn, &devices, from, aggregation).await
    }

    /// Retrieves network stats grouped by currently active users since `from` timestamp.
    pub async fn user_stats(
        &self,
        conn: &PgPool,
        from: &NaiveDateTime,
        aggregation: &DateTimeAggregation,
    ) -> Result<Vec<WireguardUserStatsRow>, SqlxError> {
        let mut user_map: HashMap<Id, Vec<WireguardDeviceStatsRow>> = HashMap::new();
        // Retrieve data series for all active devices and assign them to users
        let device_stats = self
            .distinct_device_stats(conn, from, aggregation, DeviceType::User)
            .await?;
        for stats in device_stats {
            user_map.entry(stats.user_id).or_default().push(stats);
        }
        // Reshape final result
        let mut stats = Vec::new();
        for u in user_map {
            let user = User::find_by_id(conn, u.0)
                .await?
                .ok_or(SqlxError::RowNotFound)?;
            stats.push(WireguardUserStatsRow {
                user: UserInfo::from_user(conn, &user).await?,
                devices: u.1.clone(),
            });
        }

        Ok(stats)
    }

    /// Retrieves total active users/devices since `from` timestamp
    async fn total_activity(
        &self,
        conn: &PgPool,
        from: &NaiveDateTime,
    ) -> Result<WireguardNetworkActivityStats, SqlxError> {
        let activity_stats = query_as!(
            WireguardNetworkActivityStats,
            "SELECT \
                    COALESCE(COUNT(DISTINCT CASE WHEN d.device_type = 'user' THEN u.id END), 0) \"active_users!\", \
                    COALESCE(COUNT(DISTINCT CASE WHEN d.device_type = 'user' THEN d.id END), 0) \"active_user_devices!\", \
                    COALESCE(COUNT(DISTINCT CASE WHEN d.device_type = 'network' THEN d.id END), 0) \"active_network_devices!\" \
                FROM wireguard_peer_stats s \
                JOIN device d ON d.id = s.device_id \
                LEFT JOIN \"user\" u ON u.id = d.user_id \
                WHERE latest_handshake >= $1 AND s.network = $2",
            from,
            self.id,
        )
        .fetch_one(conn)
        .await?;

        Ok(activity_stats)
    }

    /// Retrieves currently connected users
    async fn current_activity(
        &self,
        conn: &PgPool,
    ) -> Result<WireguardNetworkActivityStats, SqlxError> {
        let from = (Utc::now() - WIREGUARD_MAX_HANDSHAKE).naive_utc();
        let activity_stats = query_as!(
            WireguardNetworkActivityStats,
            "SELECT \
                    COALESCE(COUNT(DISTINCT CASE WHEN d.device_type = 'user' THEN u.id END), 0) \"active_users!\", \
                    COALESCE(COUNT(DISTINCT CASE WHEN d.device_type = 'user' THEN d.id END), 0) \"active_user_devices!\", \
                    COALESCE(COUNT(DISTINCT CASE WHEN d.device_type = 'network' THEN d.id END), 0) \"active_network_devices!\" \
                FROM wireguard_peer_stats s \
                JOIN device d ON d.id = s.device_id \
                LEFT JOIN \"user\" u ON u.id = d.user_id \
                WHERE latest_handshake >= $1 AND s.network = $2",
            from,
            self.id
        )
        .fetch_one(conn)
        .await?;

        Ok(activity_stats)
    }

    /// Retrieves network upload & download time series since `from` timestamp
    /// using `aggregation` (hour/minute) aggregation level
    async fn transfer_series(
        &self,
        conn: &PgPool,
        from: &NaiveDateTime,
        aggregation: &DateTimeAggregation,
    ) -> Result<Vec<WireguardStatsRow>, SqlxError> {
        let stats = query_as!(
            WireguardStatsRow,
            "SELECT \
                date_trunc($1, collected_at) \"collected_at: NaiveDateTime\", \
                cast(sum(upload) AS bigint) upload, cast(sum(download) AS bigint) download \
            FROM wireguard_peer_stats_view \
            WHERE collected_at >= $2 AND network = $3 \
            GROUP BY 1 \
            ORDER BY 1 \
            LIMIT $4",
            aggregation.fstring(),
            from,
            self.id,
            PEER_STATS_LIMIT,
        )
        .fetch_all(conn)
        .await?;

        Ok(stats)
    }

    /// Retrieves network stats
    pub async fn network_stats(
        &self,
        conn: &PgPool,
        from: &NaiveDateTime,
        aggregation: &DateTimeAggregation,
    ) -> Result<WireguardNetworkStats, SqlxError> {
        let total_activity = self.total_activity(conn, from).await?;
        let current_activity = self.current_activity(conn).await?;
        let transfer_series = self.transfer_series(conn, from, aggregation).await?;
        Ok(WireguardNetworkStats {
            active_users: total_activity.active_users,
            active_network_devices: total_activity.active_network_devices,
            active_user_devices: total_activity.active_user_devices,
            current_active_network_devices: current_activity.active_network_devices,
            current_active_user_devices: current_activity.active_user_devices,
            current_active_users: current_activity.active_users,
            upload: transfer_series.iter().filter_map(|t| t.upload).sum(),
            download: transfer_series.iter().filter_map(|t| t.download).sum(),
            transfer_series,
        })
    }

    pub async fn get_devices_by_type<'e, E>(
        &self,
        executor: E,
        device_type: DeviceType,
    ) -> Result<Vec<Device<Id>>, SqlxError>
    where
        E: PgExecutor<'e>,
    {
        query_as!(
            Device,
            "SELECT \
                id, name, wireguard_pubkey, user_id, created, description, device_type \"device_type: DeviceType\", \
                configured \
            FROM device WHERE id in (SELECT device_id FROM wireguard_network_device WHERE wireguard_network_id = $1) \
            AND device_type = $2",
            self.id,
            device_type as DeviceType
        )
        .fetch_all(executor)
        .await
    }

    /// Determine if a set of IP addresses can be safely assigned on this network.
    ///
    /// This method runs three categories of checks in sequence:
    /// 1. **Range validation**
    ///    Every address in `ip_addrs` must lie within one of the network's CIDR.
    ///    Fails with `NoContainingNetwork` if any IP falls outside.
    ///
    /// 2. **Reserved‐address checks**
    ///    - Rejects the network address itself (`IsNetworkAddress`).
    ///    - Rejects the broadcast address (`IsBroadcastAddress`).
    ///    - Rejects the gateway/reserved host address (`ReservedForGateway`).
    ///
    /// 3. **Conflict detection**
    ///    Queries the database to see if an IP is already claimed.
    ///    - If `device_id` is `Some(id)`, any IP already bound to that same device is exempt.
    ///    - Otherwise, or if bound to a different device, fails with `AddressAlreadyAssigned`.
    ///
    /// # Parameters
    ///
    /// - `transaction`: Open PostgreSQL transaction to check existing assignments.
    /// - `ip_addrs`: Candidate `IpAddr`s to validate.
    /// - `device_id`: If `Some(id)`, IPs already assigned to this device are treated as free;
    ///   if `None`, all existing assignments count as conflicts.
    ///
    /// # Returns
    ///
    /// - `Ok(())`: All addresses passed every check.
    /// - `Err(NetworkIpAssignmentError)`: The first failing check.
    pub async fn can_assign_ips(
        &self,
        transaction: &mut PgConnection,
        ip_addrs: &[IpAddr],
        device_id: Option<Id>,
    ) -> Result<(), NetworkAddressError> {
        // Ensure the network contains all provided IP addresses
        let networks = ip_addrs
            .iter()
            .map(|ip| self.get_containing_network(*ip).ok_or(*ip))
            .collect::<Result<Vec<IpNetwork>, IpAddr>>()
            .map_err(|ip| {
                NetworkAddressError::NoContainingNetwork(
                    self.name.clone(),
                    ip,
                    self.address.clone(),
                )
            })?;
        for (ip, network_address) in zip(ip_addrs, networks) {
            if *ip == network_address.network() {
                return Err(NetworkAddressError::IsNetworkAddress(
                    self.name.clone(),
                    *ip,
                ));
            } else if *ip == network_address.broadcast() {
                return Err(NetworkAddressError::IsBroadcastAddress(
                    self.name.clone(),
                    *ip,
                ));
            } else if *ip == network_address.ip() {
                return Err(NetworkAddressError::ReservedForGateway(
                    self.name.clone(),
                    *ip,
                ));
            }

            // Make sure the IP address is not assigned
            let device = Device::find_by_ip(&mut *transaction, *ip, self.id).await?;
            if device.is_some_and(|device| device_id != Some(device.id)) {
                return Err(NetworkAddressError::AddressAlreadyAssigned(
                    self.name.clone(),
                    *ip,
                ));
            }
        }

        Ok(())
    }

    #[must_use]
    pub fn mfa_enabled(&self) -> bool {
        match self.location_mfa_mode {
            LocationMfaMode::Internal | LocationMfaMode::External => true,
            LocationMfaMode::Disabled => false,
        }
    }

    // fetch all locations using external MFA
    pub async fn all_using_external_mfa<'e, E>(
        executor: E,
    ) -> Result<Vec<Self>, WireguardNetworkError>
    where
        E: PgExecutor<'e>,
    {
        let locations = query_as!(
            WireguardNetwork,
            "SELECT id, name, address, port, pubkey, prvkey, endpoint, dns, allowed_ips, \
            connected_at, keepalive_interval, peer_disconnect_threshold, acl_enabled, \
            acl_default_allow, location_mfa_mode \"location_mfa_mode: LocationMfaMode\", \
            service_location_mode \"service_location_mode: ServiceLocationMode\" \
            FROM wireguard_network WHERE location_mfa_mode = 'external'::location_mfa_mode",
        )
        .fetch_all(executor)
        .await?;

        Ok(locations)
    }

    /// Generates auth token for a VPN gateway
    #[allow(clippy::result_large_err)]
    pub fn generate_gateway_token(&self) -> Result<String, WireguardNetworkError> {
        let location_id = self.id;

        let token = Claims::new(
            ClaimsType::Gateway,
            format!("DEFGUARD-NETWORK-{location_id}"),
            location_id.to_string(),
            u32::MAX.into(),
        )
        .to_jwt()?;

        Ok(token)
    }

    /// Fetch a list of all allowed groups for a given network from DB
    pub async fn fetch_allowed_groups<'e, E>(&self, executor: E) -> Result<Vec<String>, ModelError>
    where
        E: PgExecutor<'e>,
    {
        debug!("Fetching all allowed groups for network {self}");
        let groups = query_scalar!(
            "SELECT name FROM wireguard_network_allowed_group wag \
            JOIN \"group\" g ON wag.group_id = g.id WHERE wag.network_id = $1",
            self.id
        )
        .fetch_all(executor)
        .await?;

        Ok(groups)
    }

    /// Return a list of allowed groups for a given network.
    /// Admin group should always be included.
    /// If no `allowed_groups` are specified for a network then all devices are allowed.
    /// In this case `None` is returned to signify that there's no filtering.
    /// This helper method is meant for use in all business logic gating
    /// access to networks based on allowed groups.
    pub async fn get_allowed_groups(
        &self,
        conn: &mut PgConnection,
    ) -> Result<Option<Vec<String>>, ModelError> {
        debug!("Returning a list of allowed groups for network {self}");
        let admin_groups = Group::find_by_permission(&mut *conn, Permission::IsAdmin).await?;

        // get allowed groups from DB
        let mut groups = self.fetch_allowed_groups(&mut *conn).await?;

        // if no allowed groups are set then all groups are allowed
        if groups.is_empty() {
            return Ok(None);
        }

        for group in admin_groups {
            if !groups.iter().any(|name| name == &group.name) {
                groups.push(group.name);
            }
        }

        Ok(Some(groups))
    }

    /// Set allowed groups, removing or adding groups as necessary.
    pub async fn set_allowed_groups(
        &self,
        transaction: &mut PgConnection,
        allowed_groups: Vec<String>,
    ) -> Result<(), ModelError> {
        info!("Setting allowed groups for network {self} to: {allowed_groups:?}");
        if allowed_groups.is_empty() {
            return self.clear_allowed_groups(transaction).await;
        }

        // get list of current allowed groups
        let mut current_groups = self.fetch_allowed_groups(&mut *transaction).await?;

        // add to group if not already a member
        for group in &allowed_groups {
            if !current_groups.contains(group) {
                self.add_to_group(transaction, group).await?;
            }
        }

        // remove groups which are no longer present
        current_groups.retain(|group| !allowed_groups.contains(group));
        if !current_groups.is_empty() {
            self.remove_from_groups(transaction, current_groups).await?;
        }

        Ok(())
    }

    pub async fn add_to_group(
        &self,
        transaction: &mut PgConnection,
        group: &str,
    ) -> Result<(), ModelError> {
        info!("Adding allowed group {group} for network {self}");
        query!(
            "INSERT INTO wireguard_network_allowed_group (network_id, group_id) \
            SELECT $1, g.id FROM \"group\" g WHERE g.name = $2",
            self.id,
            group
        )
        .execute(transaction)
        .await?;
        Ok(())
    }

    pub async fn remove_from_groups(
        &self,
        transaction: &mut PgConnection,
        groups: Vec<String>,
    ) -> Result<(), ModelError> {
        info!("Removing allowed groups {groups:?} for network {self}");
        let result = query!(
            "DELETE FROM wireguard_network_allowed_group \
            WHERE network_id = $1 AND group_id IN ( \
                SELECT id FROM \"group\" \
                WHERE name IN (SELECT * FROM UNNEST($2::text[])) \
            )",
            self.id,
            &groups
        )
        .execute(transaction)
        .await?;
        info!(
            "Removed {} allowed groups for network {self}",
            result.rows_affected(),
        );
        Ok(())
    }

    /// Remove all allowed groups for a given network
    async fn clear_allowed_groups(&self, transaction: &mut PgConnection) -> Result<(), ModelError> {
        info!("Removing all allowed groups for network {self}");
        let result = query!(
            "DELETE FROM wireguard_network_allowed_group WHERE network_id=$1",
            self.id
        )
        .execute(transaction)
        .await?;
        info!(
            "Removed {} allowed groups for network {self}",
            result.rows_affected(),
        );
        Ok(())
    }

    /// Fetch all active VPN client sessions
    pub async fn get_active_vpn_sessions<'e, E: sqlx::PgExecutor<'e>>(
        &self,
        executor: E,
    ) -> Result<Vec<VpnClientSession<Id>>, SqlxError> {
        query_as!(
            VpnClientSession,
            "SELECT id, location_id, user_id, device_id, \
            created_at, connected_at, disconnected_at, mfa, state \"state: VpnClientSessionState\" \
            FROM vpn_client_session \
            WHERE location_id = $1 AND state = 'connected'::vpn_client_session_state",
            self.id,
        )
        .fetch_all(executor)
        .await
    }
}

// [`IpNetwork`] does not implement [`Default`]
impl Default for WireguardNetwork {
    fn default() -> Self {
        Self {
            id: NoId,
            name: String::default(),
            address: vec![IpNetwork::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0).unwrap()],
            port: i32::default(),
            pubkey: String::default(),
            prvkey: String::default(),
            endpoint: String::default(),
            dns: Option::default(),
            allowed_ips: Vec::default(),
            connected_at: Option::default(),
            keepalive_interval: DEFAULT_KEEPALIVE_INTERVAL,
            peer_disconnect_threshold: DEFAULT_DISCONNECT_THRESHOLD,
            acl_enabled: false,
            acl_default_allow: false,
            location_mfa_mode: LocationMfaMode::default(),
            service_location_mode: ServiceLocationMode::default(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub struct WireguardStatsRow {
    pub collected_at: Option<NaiveDateTime>,
    pub upload: Option<i64>,
    pub download: Option<i64>,
}

#[derive(Clone, Debug, Deserialize, FromRow, PartialEq, Serialize)]
pub struct WireguardDeviceTransferRow {
    pub device_id: Id,
    pub collected_at: NaiveDateTime,
    pub upload: i64,
    pub download: i64,
}

#[derive(Clone, Default, Deserialize, Serialize)]
pub struct WireguardDeviceStatsRow {
    pub id: Id,
    pub stats: Vec<WireguardDeviceTransferRow>,
    pub user_id: Id,
    pub name: String,
    pub wireguard_ips: Vec<String>,
    pub public_ip: Option<String>,
    pub connected_at: Option<NaiveDateTime>,
}

#[derive(Deserialize, Serialize)]
pub struct WireguardUserStatsRow {
    pub user: UserInfo,
    pub devices: Vec<WireguardDeviceStatsRow>,
}

pub struct WireguardNetworkActivityStats {
    pub active_users: i64,
    pub active_user_devices: i64,
    pub active_network_devices: i64,
}

pub struct WireguardNetworkTransferStats {
    pub upload: i64,
    pub download: i64,
}

#[derive(Deserialize, Serialize)]
pub struct WireguardNetworkStats {
    pub current_active_users: i64,
    pub current_active_user_devices: i64,
    pub current_active_network_devices: i64,
    pub active_users: i64,
    pub active_user_devices: i64,
    pub active_network_devices: i64,
    pub upload: i64,
    pub download: i64,
    pub transfer_series: Vec<WireguardStatsRow>,
}

pub async fn networks_stats(
    conn: &PgPool,
    from: &NaiveDateTime,
    aggregation: &DateTimeAggregation,
) -> Result<WireguardNetworkStats, SqlxError> {
    let total_activity = query_as!(
        WireguardNetworkActivityStats,
        "SELECT \
                COALESCE(COUNT(DISTINCT CASE WHEN d.device_type = 'user' THEN u.id END), 0) \"active_users!\", \
                COALESCE(COUNT(DISTINCT CASE WHEN d.device_type = 'user' THEN d.id END), 0) \"active_user_devices!\", \
                COALESCE(COUNT(DISTINCT CASE WHEN d.device_type = 'network' THEN d.id END), 0) \"active_network_devices!\" \
            FROM wireguard_peer_stats s \
            JOIN device d ON d.id = s.device_id \
            LEFT JOIN \"user\" u ON u.id = d.user_id \
            WHERE latest_handshake >= $1",
        from
    )
    .fetch_one(conn)
    .await?;
    let current_activity_from = (Utc::now() - WIREGUARD_MAX_HANDSHAKE).naive_utc();
    let current_activity = query_as!(
        WireguardNetworkActivityStats,
        "SELECT \
                COALESCE(COUNT(DISTINCT CASE WHEN d.device_type = 'user' THEN u.id END), 0) \"active_users!\", \
                COALESCE(COUNT(DISTINCT CASE WHEN d.device_type = 'user' THEN d.id END), 0) \"active_user_devices!\", \
                COALESCE(COUNT(DISTINCT CASE WHEN d.device_type = 'network' THEN d.id END), 0) \"active_network_devices!\" \
            FROM wireguard_peer_stats s \
            JOIN device d ON d.id = s.device_id \
            LEFT JOIN \"user\" u ON u.id = d.user_id \
            WHERE latest_handshake >= $1",
        current_activity_from
    )
    .fetch_one(conn)
    .await?;
    let transfer_series = query_as!(
        WireguardStatsRow,
        "SELECT \
                date_trunc($1, collected_at) \"collected_at: NaiveDateTime\", \
                cast(sum(upload) AS bigint) upload, cast(sum(download) AS bigint) download \
            FROM wireguard_peer_stats_view \
            WHERE collected_at >= $2 \
            GROUP BY 1 \
            ORDER BY 1 \
            LIMIT $3",
        aggregation.fstring(),
        from,
        PEER_STATS_LIMIT,
    )
    .fetch_all(conn)
    .await?;
    Ok(WireguardNetworkStats {
        current_active_users: current_activity.active_users,
        current_active_network_devices: current_activity.active_network_devices,
        current_active_user_devices: current_activity.active_user_devices,
        active_users: total_activity.active_users,
        active_network_devices: total_activity.active_network_devices,
        active_user_devices: total_activity.active_user_devices,
        download: transfer_series.iter().filter_map(|t| t.download).sum(),
        upload: transfer_series.iter().filter_map(|t| t.upload).sum(),
        transfer_series,
    })
}

#[cfg(test)]
mod test {
    use std::str::FromStr;

    use crate::db::setup_pool;
    use chrono::{SubsecRound, TimeDelta, Utc};
    use matches::assert_matches;
    use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

    use super::*;

    #[sqlx::test]
    async fn test_connected_at_reconnection(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;
        let mut network = WireguardNetwork::default();
        network.try_set_address("10.1.1.1/29").unwrap();
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
        let device = Device::new(
            String::new(),
            String::new(),
            user.id,
            DeviceType::User,
            None,
            true,
        )
        .save(&pool)
        .await
        .unwrap();

        // insert stats
        let samples = 60; // 1 hour of samples
        let now = Utc::now().naive_utc();
        for i in 0..=samples {
            // simulate connection 30 minutes ago
            let handshake_minutes = i * if i < 31 { 1 } else { 10 };
            WireguardPeerStats {
                id: NoId,
                device_id: device.id,
                collected_at: now - TimeDelta::minutes(i),
                network: network.id,
                endpoint: Some("11.22.33.44".into()),
                upload: (samples - i) * 10,
                download: (samples - i) * 20,
                latest_handshake: now - TimeDelta::minutes(handshake_minutes),
                allowed_ips: Some("10.1.1.0/24".into()),
            }
            .save(&pool)
            .await
            .unwrap();
        }

        let connected_at = network
            .connected_at(&pool, device.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            connected_at,
            // PostgreSQL stores 6 sub-second digits while chrono stores 9.
            (now - TimeDelta::minutes(30)).trunc_subsecs(6),
        );
    }

    #[sqlx::test]
    async fn test_connected_at_always_connected(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;
        let mut network = WireguardNetwork::default();
        network.try_set_address("10.1.1.1/29").unwrap();
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
        let device = Device::new(
            String::new(),
            String::new(),
            user.id,
            DeviceType::User,
            None,
            true,
        )
        .save(&pool)
        .await
        .unwrap();

        // insert stats
        let samples = 60; // 1 hour of samples
        let now = Utc::now().naive_utc();
        for i in 0..=samples {
            WireguardPeerStats {
                id: NoId,
                device_id: device.id,
                collected_at: now - TimeDelta::minutes(i),
                network: network.id,
                endpoint: Some("11.22.33.44".into()),
                upload: (samples - i) * 10,
                download: (samples - i) * 20,
                latest_handshake: now - TimeDelta::minutes(i), // handshake every minute
                allowed_ips: Some("10.1.1.0/24".into()),
            }
            .save(&pool)
            .await
            .unwrap();
        }

        let connected_at = network
            .connected_at(&pool, device.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            connected_at,
            // PostgreSQL stores 6 sub-second digits while chrono stores 9.
            (now - TimeDelta::minutes(samples)).trunc_subsecs(6),
        );
    }

    #[sqlx::test]
    async fn test_get_allowed_devices_for_user(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;
        let mut network = WireguardNetwork::default();
        network.try_set_address("10.1.1.1/29").unwrap();
        let network = network.save(&pool).await.unwrap();

        let user1 = User::new(
            "user1",
            Some("pass1"),
            "Test",
            "User1",
            "user1@test.com",
            None,
        )
        .save(&pool)
        .await
        .unwrap();

        let user2 = User::new(
            "user2",
            Some("pass2"),
            "Test",
            "User2",
            "user2@test.com",
            None,
        )
        .save(&pool)
        .await
        .unwrap();

        let device1 = Device::new(
            "device1".into(),
            "key1".into(),
            user1.id,
            DeviceType::User,
            None,
            true,
        )
        .save(&pool)
        .await
        .unwrap();

        let device2 = Device::new(
            "device2".into(),
            "key2".into(),
            user1.id,
            DeviceType::User,
            None,
            true,
        )
        .save(&pool)
        .await
        .unwrap();

        let device3 = Device::new(
            "device3".into(),
            "key3".into(),
            user2.id,
            DeviceType::User,
            None,
            true,
        )
        .save(&pool)
        .await
        .unwrap();

        let devices = network
            .get_allowed_devices_for_user(&mut pool.acquire().await.unwrap(), user1.id)
            .await
            .unwrap();
        assert_eq!(devices.len(), 2);
        assert!(devices.iter().any(|d| d.id == device1.id));
        assert!(devices.iter().any(|d| d.id == device2.id));

        let devices = network
            .get_allowed_devices_for_user(&mut pool.acquire().await.unwrap(), user2.id)
            .await
            .unwrap();
        assert_eq!(devices.len(), 1);
        assert!(devices.iter().any(|d| d.id == device3.id));

        let devices = network
            .get_allowed_devices_for_user(&mut pool.acquire().await.unwrap(), Id::from(999))
            .await
            .unwrap();
        assert!(devices.is_empty());
    }

    #[sqlx::test]
    async fn test_get_allowed_devices_for_user_with_groups(
        _: PgPoolOptions,
        options: PgConnectOptions,
    ) {
        let pool = setup_pool(options).await;
        let mut network = WireguardNetwork::default();
        network.try_set_address("10.1.1.1/29").unwrap();
        let network = network.save(&pool).await.unwrap();

        let user1 = User::new(
            "user1",
            Some("pass1"),
            "Test",
            "User1",
            "user1@test.com",
            None,
        )
        .save(&pool)
        .await
        .unwrap();

        let user2 = User::new(
            "user2",
            Some("pass2"),
            "Test",
            "User2",
            "user2@test.com",
            None,
        )
        .save(&pool)
        .await
        .unwrap();

        let group1 = Group::new("group1").save(&pool).await.unwrap();
        let group2 = Group::new("group2").save(&pool).await.unwrap();

        user1.add_to_group(&pool, &group1).await.unwrap();
        user2.add_to_group(&pool, &group2).await.unwrap();

        let device1 = Device::new(
            "device1".into(),
            "key1".into(),
            user1.id,
            DeviceType::User,
            None,
            true,
        )
        .save(&pool)
        .await
        .unwrap();

        Device::new(
            "device2".into(),
            "key2".into(),
            user2.id,
            DeviceType::User,
            None,
            true,
        )
        .save(&pool)
        .await
        .unwrap();

        let mut transaction = pool.begin().await.unwrap();

        network
            .set_allowed_groups(&mut transaction, vec![group1.name])
            .await
            .unwrap();

        let devices = network
            .get_allowed_devices_for_user(&mut transaction, user1.id)
            .await
            .unwrap();
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].id, device1.id);

        let devices = network
            .get_allowed_devices_for_user(&mut transaction, user2.id)
            .await
            .unwrap();
        assert!(devices.is_empty());
    }

    #[sqlx::test]
    async fn test_can_assign_ips(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;

        let network = WireguardNetwork::new(
            "network".to_string(),
            vec![IpNetwork::from_str("10.1.1.1/24").unwrap()],
            50051,
            String::new(),
            None,
            vec![IpNetwork::from_str("10.1.1.0/24").unwrap()],
            300,
            300,
            false,
            false,
            LocationMfaMode::Disabled,
            ServiceLocationMode::Disabled,
        )
        .save(&pool)
        .await
        .unwrap();

        // assign free address
        let addrs = vec![IpAddr::from_str("10.1.1.2").unwrap()];
        assert_matches!(
            network
                .can_assign_ips(&mut pool.acquire().await.unwrap(), &addrs, None)
                .await,
            Ok(())
        );

        // assign multiple free addresses
        let addrs = vec![
            IpAddr::from_str("10.1.1.2").unwrap(),
            IpAddr::from_str("10.1.1.3").unwrap(),
        ];
        assert_matches!(
            network
                .can_assign_ips(&mut pool.acquire().await.unwrap(), &addrs, None)
                .await,
            Ok(())
        );

        // try to assign address from another network
        let addrs = vec![IpAddr::from_str("10.2.1.2").unwrap()];
        assert_matches!(
            network
                .can_assign_ips(&mut pool.acquire().await.unwrap(), &addrs, None)
                .await,
            Err(NetworkAddressError::NoContainingNetwork(..))
        );

        // try to assign already assigned address
        let user = User::new(
            "hpotter",
            Some("pass123"),
            "Potter",
            "Harry",
            "h.potter@hogwart.edu.uk",
            None,
        )
        .save(&pool)
        .await
        .unwrap();

        let device = Device::new(
            "device".to_string(),
            String::new(),
            user.id,
            DeviceType::User,
            None,
            true,
        )
        .save(&pool)
        .await
        .unwrap();
        WireguardNetworkDevice::new(
            network.id,
            device.id,
            vec![IpAddr::from_str("10.1.1.2").unwrap()],
        )
        .insert(&pool)
        .await
        .unwrap();
        let addrs = vec![IpAddr::from_str("10.1.1.2").unwrap()];
        assert_matches!(
            network
                .can_assign_ips(&mut pool.acquire().await.unwrap(), &addrs, None)
                .await,
            Err(NetworkAddressError::AddressAlreadyAssigned(..))
        );

        // assign with exception for the device
        let addrs = vec![IpAddr::from_str("10.1.1.2").unwrap()];
        assert_matches!(
            network
                .can_assign_ips(&mut pool.acquire().await.unwrap(), &addrs, Some(device.id))
                .await,
            Ok(())
        );

        // try to assign gateway address
        let addrs = vec![IpAddr::from_str("10.1.1.1").unwrap()];
        assert_matches!(
            network
                .can_assign_ips(&mut pool.acquire().await.unwrap(), &addrs, None)
                .await,
            Err(NetworkAddressError::ReservedForGateway(..))
        );

        // try to assign network address
        let addrs = vec![IpAddr::from_str("10.1.1.0").unwrap()];
        assert_matches!(
            network
                .can_assign_ips(&mut pool.acquire().await.unwrap(), &addrs, None)
                .await,
            Err(NetworkAddressError::IsNetworkAddress(..))
        );

        // try to assign broadcast address
        let addrs = vec![IpAddr::from_str("10.1.1.255").unwrap()];
        assert_matches!(
            network
                .can_assign_ips(&mut pool.acquire().await.unwrap(), &addrs, None)
                .await,
            Err(NetworkAddressError::IsBroadcastAddress(..))
        );
    }

    #[sqlx::test]
    async fn test_can_assign_ips_multiple_addresses(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;

        let network = WireguardNetwork::new(
            "network".to_string(),
            vec![
                IpNetwork::from_str("10.1.1.1/24").unwrap(),
                IpNetwork::from_str("fc00::1/112").unwrap(),
            ],
            50051,
            String::new(),
            None,
            vec![IpNetwork::from_str("10.1.1.0/24").unwrap()],
            300,
            300,
            false,
            false,
            LocationMfaMode::Disabled,
            ServiceLocationMode::Disabled,
        )
        .save(&pool)
        .await
        .unwrap();

        // assign free addresses
        let addrs = vec![
            IpAddr::from_str("10.1.1.2").unwrap(),
            IpAddr::from_str("fc00::2").unwrap(),
        ];
        assert_matches!(
            network
                .can_assign_ips(&mut pool.acquire().await.unwrap(), &addrs, None)
                .await,
            Ok(())
        );

        // assign multiple free addresses
        let addrs = vec![
            IpAddr::from_str("10.1.1.2").unwrap(),
            IpAddr::from_str("10.1.1.3").unwrap(),
            IpAddr::from_str("fc00::2").unwrap(),
            IpAddr::from_str("fc00::3").unwrap(),
        ];
        assert_matches!(
            network
                .can_assign_ips(&mut pool.acquire().await.unwrap(), &addrs, None)
                .await,
            Ok(())
        );

        // try to assign address from another network
        let addrs = vec![IpAddr::from_str("fa::2").unwrap()];
        assert_matches!(
            network
                .can_assign_ips(&mut pool.acquire().await.unwrap(), &addrs, None)
                .await,
            Err(NetworkAddressError::NoContainingNetwork(..))
        );

        // try to assign already assigned address
        let user = User::new(
            "hpotter",
            Some("pass123"),
            "Potter",
            "Harry",
            "h.potter@hogwart.edu.uk",
            None,
        )
        .save(&pool)
        .await
        .unwrap();

        let device = Device::new(
            "device".to_string(),
            String::new(),
            user.id,
            DeviceType::User,
            None,
            true,
        )
        .save(&pool)
        .await
        .unwrap();
        WireguardNetworkDevice::new(
            network.id,
            device.id,
            vec![
                IpAddr::from_str("10.1.1.2").unwrap(),
                IpAddr::from_str("fc00::2").unwrap(),
            ],
        )
        .insert(&pool)
        .await
        .unwrap();
        let addrs = vec![IpAddr::from_str("fc00::2").unwrap()];
        assert_matches!(
            network
                .can_assign_ips(&mut pool.acquire().await.unwrap(), &addrs, None)
                .await,
            Err(NetworkAddressError::AddressAlreadyAssigned(..))
        );

        // assign with exception for the device
        let addrs = vec![IpAddr::from_str("fc00::2").unwrap()];
        assert_matches!(
            network
                .can_assign_ips(&mut pool.acquire().await.unwrap(), &addrs, Some(device.id))
                .await,
            Ok(())
        );

        // try to assign gateway address
        let addrs = vec![IpAddr::from_str("fc00::1").unwrap()];
        assert_matches!(
            network
                .can_assign_ips(&mut pool.acquire().await.unwrap(), &addrs, None)
                .await,
            Err(NetworkAddressError::ReservedForGateway(..))
        );

        // try to assign network address
        let addrs = vec![IpAddr::from_str("fc00::0").unwrap()];
        assert_matches!(
            network
                .can_assign_ips(&mut pool.acquire().await.unwrap(), &addrs, None)
                .await,
            Err(NetworkAddressError::IsNetworkAddress(..))
        );

        // try to assign broadcast address
        let addrs = vec![IpAddr::from_str("fc00::ffff").unwrap()];
        assert_matches!(
            network
                .can_assign_ips(&mut pool.acquire().await.unwrap(), &addrs, None)
                .await,
            Err(NetworkAddressError::IsBroadcastAddress(..))
        );
    }
}
