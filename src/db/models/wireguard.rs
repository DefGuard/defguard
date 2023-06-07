use super::{device::Device, error::ModelError, DbPool, User, UserInfo};
use base64::Engine;
use chrono::{Duration, NaiveDateTime, Utc};
use ipnetwork::{IpNetwork, IpNetworkError, NetworkSize};
use model_derive::Model;
use rand_core::OsRng;
use sqlx::{query_as, query_scalar, Error as SqlxError, FromRow};
use std::{
    collections::HashMap,
    fmt::Debug,
    net::{IpAddr, Ipv4Addr},
    str::FromStr,
};
use x25519_dalek::{PublicKey, StaticSecret};

pub static WIREGUARD_MAX_HANDSHAKE_MINUTES: u32 = 5;
pub static PEER_STATS_LIMIT: i64 = 6 * 60;

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

#[derive(Clone, Debug)]
pub enum GatewayEvent {
    NetworkCreated(WireguardNetwork),
    NetworkModified(WireguardNetwork),
    NetworkDeleted(String),
    DeviceCreated(Device),
    DeviceModified(Device),
    DeviceDeleted(String),
}

/// Stores configuration required to setup a wireguard network
#[derive(Clone, Debug, Model, Deserialize, Serialize, PartialEq)]
#[table(wireguard_network)]
pub struct WireguardNetwork {
    pub id: Option<i64>,
    pub name: String,
    #[model(enum)]
    pub address: IpNetwork,
    pub port: i32,
    pub pubkey: String,
    #[serde(default, skip_serializing)]
    pub prvkey: String,
    pub endpoint: String,
    pub dns: Option<String>,
    #[model(ref)]
    pub allowed_ips: Vec<IpNetwork>,
    pub connected_at: Option<NaiveDateTime>,
}

pub struct WireguardKey {
    pub private: String,
    pub public: String,
}

impl WireguardNetwork {
    pub fn new(
        name: String,
        address: IpNetwork,
        port: i32,
        endpoint: String,
        dns: Option<String>,
        allowed_ips: Vec<IpNetwork>,
    ) -> Result<Self, IpNetworkError> {
        let prvkey = StaticSecret::random_from_rng(OsRng);
        let pubkey = PublicKey::from(&prvkey);
        Ok(Self {
            id: None,
            name,
            address,
            port,
            pubkey: base64::prelude::BASE64_STANDARD.encode(pubkey.to_bytes()),
            prvkey: base64::prelude::BASE64_STANDARD.encode(prvkey.to_bytes()),
            endpoint,
            dns,
            allowed_ips,
            connected_at: None,
        })
    }

    /// Return number of devices that use this network.
    async fn device_count(&self, pool: &DbPool) -> Result<i64, SqlxError> {
        // FIXME: currently there is only one hard-coded network with id = 1.
        query_scalar!("SELECT count(*) \"count!\" FROM device")
            .fetch_one(pool)
            .await
    }

    /// Utility method to create wireguard keypair
    #[must_use]
    pub fn genkey() -> WireguardKey {
        let private = StaticSecret::random_from_rng(OsRng);
        let public = PublicKey::from(&private);
        WireguardKey {
            private: base64::prelude::BASE64_STANDARD.encode(private.to_bytes()),
            public: base64::prelude::BASE64_STANDARD.encode(public.to_bytes()),
        }
    }

    /// Try to set `address` from `&str`.
    pub fn try_set_address(&mut self, address: &str) -> Result<IpNetwork, IpNetworkError> {
        IpNetwork::from_str(address).map(|network| {
            self.address = network;
            network
        })
    }

    /// Try to change network address, changing device addresses if necessary.
    pub async fn change_address(
        &mut self,
        pool: &DbPool,
        new_address: IpNetwork,
    ) -> Result<(), ModelError> {
        let old_address = self.address;

        // check if new network size will fit all existing devices
        let new_size = new_address.size();
        if new_size < old_address.size() {
            // include address, network, and broadcast in the calculation
            let count = self.device_count(pool).await? + 3;
            match new_size {
                NetworkSize::V4(size) => {
                    if count as u32 > size {
                        return Err(ModelError::NetworkTooSmall);
                    }
                }
                NetworkSize::V6(size) => {
                    if count as u128 > size {
                        return Err(ModelError::NetworkTooSmall);
                    }
                }
            }
        }

        // re-address all devices
        if new_address.network() != old_address.network() {
            let transaction = pool.begin().await?;

            let mut devices = Device::all(pool).await?;
            let net_ip = new_address.ip();
            let net_network = new_address.network();
            let net_broadcast = new_address.broadcast();
            let mut devices_iter = devices.iter_mut();
            for ip in new_address.iter() {
                if ip == net_ip || ip == net_network || ip == net_broadcast {
                    continue;
                }
                match devices_iter.next() {
                    Some(device) => {
                        device.wireguard_ip = ip.to_string();
                        device.save(pool).await?;
                    }
                    None => break,
                }
            }

            transaction.commit().await?;
        }

        self.address = new_address;
        Ok(())
    }

    async fn fetch_latest_stats(
        conn: &DbPool,
        device_id: i64,
    ) -> Result<Option<WireguardPeerStats>, SqlxError> {
        let stats = query_as!(
            WireguardPeerStats,
            r#"
            SELECT id "id?", device_id "device_id!", collected_at "collected_at!", network "network!",
                endpoint, upload "upload!", download "download!", latest_handshake "latest_handshake!", allowed_ips
            FROM wireguard_peer_stats
            WHERE device_id = $1
            ORDER BY collected_at DESC
            LIMIT 1
            "#,
            device_id
        )
        .fetch_optional(conn)
        .await?;
        Ok(stats)
    }

    /// Parse WireGuard IP address
    fn parse_wireguard_ip(stats: &WireguardPeerStats) -> Option<String> {
        stats
            .allowed_ips
            .as_ref()
            .and_then(|ips| Some(ips.split('/').next()?.to_owned()))
    }

    /// Parse public IP address
    fn parse_public_ip(stats: &WireguardPeerStats) -> Option<String> {
        stats
            .endpoint
            .as_ref()
            .and_then(|ep| Some(ep.split(':').next()?.to_owned()))
    }

    /// Finds when the device connected based on handshake timestamps
    async fn connected_at(
        conn: &DbPool,
        device_id: i64,
    ) -> Result<Option<NaiveDateTime>, SqlxError> {
        let connected_at = query_scalar!(
            r#"
            SELECT
                latest_handshake "latest_handshake: NaiveDateTime"
            FROM wireguard_peer_stats_view
            WHERE device_id = $1
                AND latest_handshake IS NOT NULL
                AND (latest_handshake_diff > $2 * interval '1 minute' OR latest_handshake_diff IS NULL)
            ORDER BY collected_at DESC
            LIMIT 1
            "#,
            device_id,
            WIREGUARD_MAX_HANDSHAKE_MINUTES as f64,
        )
        .fetch_optional(conn)
        .await?;
        Ok(connected_at.flatten())
    }

    /// Retrieves stats for specified devices
    async fn device_stats(
        conn: &DbPool,
        devices: &[Device],
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
            .filter_map(|d| d.id.map(|id| id.to_string()))
            .collect::<Vec<String>>()
            .join(",");
        let query = format!(
            r#"
            SELECT
                device_id,
                date_trunc($1, collected_at) as collected_at,
                cast(sum(download) as bigint) as download,
                cast(sum(upload) as bigint) as upload
            FROM wireguard_peer_stats_view
            WHERE device_id IN ({})
            AND collected_at >= $2
            GROUP BY 1, 2
            ORDER BY 1, 2
            "#,
            device_ids
        );
        let stats: Vec<WireguardDeviceTransferRow> = query_as(&query)
            .bind(aggregation.fstring())
            .bind(from)
            .fetch_all(conn)
            .await?;
        let mut result = Vec::new();
        for device in devices {
            let latest_stats = Self::fetch_latest_stats(conn, device.id.unwrap()).await?;
            result.push(WireguardDeviceStatsRow {
                id: device.id.unwrap(),
                user_id: device.user_id,
                name: device.name.clone(),
                wireguard_ip: latest_stats.as_ref().and_then(Self::parse_wireguard_ip),
                public_ip: latest_stats.as_ref().and_then(Self::parse_public_ip),
                connected_at: Self::connected_at(conn, device.id.unwrap()).await?,
                // Filter stats for this device
                stats: stats
                    .iter()
                    .filter(|s| Some(s.device_id) == device.id)
                    .cloned()
                    .collect(),
            })
        }
        Ok(result)
    }

    /// Retrieves network stats grouped by currently active users since `from` timestamp
    pub async fn user_stats(
        conn: &DbPool,
        from: &NaiveDateTime,
        aggregation: &DateTimeAggregation,
    ) -> Result<Vec<WireguardUserStatsRow>, SqlxError> {
        let mut user_map: HashMap<i64, Vec<WireguardDeviceStatsRow>> = HashMap::new();
        let oldest_handshake =
            (Utc::now() - Duration::minutes(WIREGUARD_MAX_HANDSHAKE_MINUTES.into())).naive_utc();
        // Retrieve connected devices from database
        let devices = query_as!(
            Device,
            r#"
            WITH s AS (
                SELECT DISTINCT ON (device_id) *
                FROM wireguard_peer_stats
                ORDER BY device_id, latest_handshake DESC
            )
            SELECT
                d.id "id?", d.name, d.wireguard_ip, d.wireguard_pubkey, d.user_id, d.created
            FROM device d
            JOIN s ON d.id = s.device_id
            WHERE s.latest_handshake > $1
            "#,
            oldest_handshake,
        )
        .fetch_all(conn)
        .await?;
        // Retrieve data series for all active devices and assign them to users
        let device_stats = Self::device_stats(conn, &devices, from, aggregation).await?;
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
                user: UserInfo::from_user(conn, user).await?,
                devices: u.1.clone(),
            });
        }
        Ok(stats)
    }

    /// Retrieves total active users/devices since `from` timestamp
    async fn total_activity(
        conn: &DbPool,
        from: &NaiveDateTime,
    ) -> Result<WireguardNetworkActivityStats, SqlxError> {
        let activity_stats = query_as!(
            WireguardNetworkActivityStats,
            r#"
            SELECT
                COALESCE(COUNT(DISTINCT(u.id)), 0) as "active_users!",
                COALESCE(COUNT(DISTINCT(s.device_id)), 0) as "active_devices!"
            FROM "user" u
                JOIN device d ON d.user_id = u.id
                JOIN wireguard_peer_stats s ON s.device_id = d.id
                WHERE latest_handshake >= $1
            "#,
            from,
        )
        .fetch_one(conn)
        .await?;
        Ok(activity_stats)
    }
    /// Retrievies currently connected users
    async fn current_activity(conn: &DbPool) -> Result<WireguardNetworkActivityStats, SqlxError> {
        // Add 2 minutes margin because gateway sends stats in 1 minute period
        let from = Utc::now()
            .naive_utc()
            .checked_sub_signed(Duration::minutes(2));
        let activity_stats = query_as!(
            WireguardNetworkActivityStats,
            r#"
            SELECT
                COALESCE(COUNT(DISTINCT(u.id)), 0) as "active_users!",
                COALESCE(COUNT(DISTINCT(s.device_id)), 0) as "active_devices!"
            FROM "user" u
                JOIN device d ON d.user_id = u.id
                JOIN wireguard_peer_stats s ON s.device_id = d.id
                WHERE latest_handshake >= $1
            "#,
            from,
        )
        .fetch_one(conn)
        .await?;
        Ok(activity_stats)
    }

    /// Retrieves network upload & download time series since `from` timestamp
    /// using `aggregation` (hour/minute) aggregation level
    async fn transfer_series(
        conn: &DbPool,
        from: &NaiveDateTime,
        aggregation: &DateTimeAggregation,
    ) -> Result<Vec<WireguardStatsRow>, SqlxError> {
        let stats = query_as!(
            WireguardStatsRow,
            r#"
            SELECT
                date_trunc($1, collected_at) "collected_at: NaiveDateTime",
                cast(sum(upload) AS bigint) upload, cast(sum(download) AS bigint) download
            FROM wireguard_peer_stats_view
            WHERE collected_at >= $2
            GROUP BY 1
            ORDER BY 1
            LIMIT $3
            "#,
            aggregation.fstring(),
            from,
            PEER_STATS_LIMIT,
        )
        .fetch_all(conn)
        .await?;
        Ok(stats)
    }

    /// Retrieves network stats
    pub async fn network_stats(
        conn: &DbPool,
        from: &NaiveDateTime,
        aggregation: &DateTimeAggregation,
    ) -> Result<WireguardNetworkStats, SqlxError> {
        let total_activity = Self::total_activity(conn, from).await?;
        let current_activity = Self::current_activity(conn).await?;
        let transfer_series = Self::transfer_series(conn, from, aggregation).await?;
        Ok(WireguardNetworkStats {
            current_active_users: current_activity.active_users,
            current_active_devices: current_activity.active_devices,
            active_users: total_activity.active_users,
            active_devices: total_activity.active_devices,
            upload: transfer_series.iter().filter_map(|t| t.upload).sum(),
            download: transfer_series.iter().filter_map(|t| t.download).sum(),
            transfer_series,
        })
    }
}

// [`IpNetwork`] does not implement [`Default`]
impl Default for WireguardNetwork {
    fn default() -> Self {
        Self {
            id: Option::default(),
            name: String::default(),
            address: IpNetwork::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0).unwrap(),
            port: i32::default(),
            pubkey: String::default(),
            prvkey: String::default(),
            endpoint: String::default(),
            dns: Option::default(),
            allowed_ips: Vec::default(),
            connected_at: Option::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct WireguardStatsRow {
    pub collected_at: Option<NaiveDateTime>,
    pub upload: Option<i64>,
    pub download: Option<i64>,
}

#[derive(FromRow, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct WireguardDeviceTransferRow {
    pub device_id: i64,
    pub collected_at: Option<NaiveDateTime>,
    pub upload: i64,
    pub download: i64,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct WireguardDeviceStatsRow {
    pub id: i64,
    pub stats: Vec<WireguardDeviceTransferRow>,
    pub user_id: i64,
    pub name: String,
    pub wireguard_ip: Option<String>,
    pub public_ip: Option<String>,
    pub connected_at: Option<NaiveDateTime>,
}

#[derive(Serialize, Deserialize)]
pub struct WireguardUserStatsRow {
    pub user: UserInfo,
    pub devices: Vec<WireguardDeviceStatsRow>,
}

#[derive(Model, Serialize, Deserialize, Debug)]
#[table(wireguard_peer_stats)]
pub struct WireguardPeerStats {
    pub id: Option<i64>,
    pub device_id: i64,
    pub collected_at: NaiveDateTime,
    pub network: i64,
    pub endpoint: Option<String>,
    pub upload: i64,
    pub download: i64,
    pub latest_handshake: NaiveDateTime,
    // FIXME: can contain multiple IP addresses
    pub allowed_ips: Option<String>,
}

pub struct WireguardNetworkActivityStats {
    pub active_users: i64,
    pub active_devices: i64,
}

pub struct WireguardNetworkTransferStats {
    pub upload: i64,
    pub download: i64,
}

#[derive(Serialize, Deserialize)]
pub struct WireguardNetworkStats {
    pub current_active_users: i64,
    pub current_active_devices: i64,
    pub active_users: i64,
    pub active_devices: i64,
    pub upload: i64,
    pub download: i64,
    pub transfer_series: Vec<WireguardStatsRow>,
}

#[cfg(test)]
mod test {
    use chrono::{Duration, SubsecRound};

    use super::*;

    async fn add_devices(pool: &DbPool, network: &WireguardNetwork, count: usize) {
        let mut user = User::new(
            "testuser".to_string(),
            "hunter2",
            "Tester".to_string(),
            "Test".to_string(),
            "test@test.com".to_string(),
            None,
        );
        user.save(pool).await.unwrap();
        for i in 0..count {
            let mut device = Device::assign_device_ip(
                pool,
                user.id.unwrap(),
                format!("dev{i}"),
                format!("key{i}"),
                network,
            )
            .await
            .unwrap();
            device.save(pool).await.unwrap();
        }
    }

    #[sqlx::test]
    async fn test_change_address(pool: DbPool) {
        let mut network = WireguardNetwork::default();
        network.try_set_address("10.1.1.1/29").unwrap();

        add_devices(&pool, &network, 3).await;

        network
            .change_address(&pool, "10.2.2.2/28".parse().unwrap())
            .await
            .unwrap();

        let dev0 = Device::find_by_pubkey(&pool, "key0")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(dev0.wireguard_ip, "10.2.2.1");

        let dev1 = Device::find_by_pubkey(&pool, "key1")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(dev1.wireguard_ip, "10.2.2.3");

        let dev2 = Device::find_by_pubkey(&pool, "key2")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(dev2.wireguard_ip, "10.2.2.4");
    }

    #[sqlx::test]
    async fn test_change_address_wont_fit(pool: DbPool) {
        let mut network = WireguardNetwork::default();
        network.try_set_address("10.1.1.1/29").unwrap();

        add_devices(&pool, &network, 3).await;

        assert!(network
            .change_address(&pool, "10.2.2.2/30".parse().unwrap())
            .await
            .is_err());
        assert!(network
            .change_address(&pool, "10.2.2.2/29".parse().unwrap())
            .await
            .is_ok());
    }

    #[sqlx::test]
    async fn test_connected_at_reconnection(pool: DbPool) {
        let mut user = User::new(
            "testuser".to_string(),
            "hunter2",
            "Tester".to_string(),
            "Test".to_string(),
            "test@test.com".to_string(),
            None,
        );
        user.save(&pool).await.unwrap();
        let mut device = Device::new(
            String::new(),
            String::new(),
            String::new(),
            user.id.unwrap(),
        );
        device.save(&pool).await.unwrap();

        // insert stats
        let samples = 60; // 1 hour of samples
        let now = Utc::now().naive_utc();
        for i in 0..=samples {
            // simulate connection 30 minutes ago
            let handshake_minutes = i * if i < 31 { 1 } else { 10 };
            let mut wps = WireguardPeerStats {
                id: None,
                device_id: device.id.unwrap(),
                collected_at: now - Duration::minutes(i),
                network: 1,
                endpoint: Some("11.22.33.44".into()),
                upload: (samples - i) * 10,
                download: (samples - i) * 20,
                latest_handshake: now - Duration::minutes(handshake_minutes),
                allowed_ips: Some("10.1.1.0/24".into()),
            };
            wps.save(&pool).await.unwrap();
        }

        let connected_at = WireguardNetwork::connected_at(&pool, device.id.unwrap())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            connected_at,
            // Postgres stores 6 sub-second digits while chrono stores 9
            (now - Duration::minutes(30)).trunc_subsecs(6),
        );
    }

    #[sqlx::test]
    async fn test_connected_at_always_connected(pool: DbPool) {
        let mut user = User::new(
            "testuser".to_string(),
            "hunter2",
            "Tester".to_string(),
            "Test".to_string(),
            "test@test.com".to_string(),
            None,
        );
        user.save(&pool).await.unwrap();
        let mut device = Device::new(
            String::new(),
            String::new(),
            String::new(),
            user.id.unwrap(),
        );
        device.save(&pool).await.unwrap();

        // insert stats
        let samples = 60; // 1 hour of samples
        let now = Utc::now().naive_utc();
        for i in 0..=samples {
            let mut wps = WireguardPeerStats {
                id: None,
                device_id: device.id.unwrap(),
                collected_at: now - Duration::minutes(i),
                network: 1,
                endpoint: Some("11.22.33.44".into()),
                upload: (samples - i) * 10,
                download: (samples - i) * 20,
                latest_handshake: now - Duration::minutes(i), // handshake every minute
                allowed_ips: Some("10.1.1.0/24".into()),
            };
            wps.save(&pool).await.unwrap();
        }

        let connected_at = WireguardNetwork::connected_at(&pool, device.id.unwrap())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            connected_at,
            // Postgres stores 6 sub-second digits while chrono stores 9
            (now - Duration::minutes(samples)).trunc_subsecs(6),
        );
    }
}
