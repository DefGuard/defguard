use super::{error::ModelError, wireguard::WireguardNetwork, DbPool};
use chrono::{NaiveDateTime, Utc};
use ipnetwork::IpNetwork;
use lazy_static::lazy_static;
use model_derive::Model;
use regex::Regex;
use sqlx::{query, query_as, Error as SqlxError, FromRow};

#[derive(Clone, Deserialize, Model, Serialize, Debug)]
pub struct Device {
    pub id: Option<i64>,
    pub name: String,
    pub wireguard_pubkey: String,
    pub user_id: i64,
    pub created: NaiveDateTime,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow)]
pub struct DeviceNetworkInfo {
    pub wireguard_network_id: Option<i64>,
    pub wireguard_ip: String,
    pub device_id: Option<i64>,
}

#[derive(Deserialize, Debug)]
pub struct AddDevice {
    pub name: String,
    pub wireguard_pubkey: String,
}

#[derive(Deserialize, Debug)]
pub struct ModifyDevice {
    pub name: String,
    pub wireguard_pubkey: String,
}

impl DeviceNetworkInfo {
    pub fn new(network_id: i64, device_id: i64, wireguard_ip: String) -> Self {
        Self {
            wireguard_network_id: Some(network_id),
            wireguard_ip,
            device_id: Some(device_id),
        }
    }

    pub async fn insert(&self, pool: &DbPool) -> Result<(), SqlxError> {
        query!(
            "INSERT INTO wireguard_network_device
                (device_id, wireguard_network_id, wireguard_ip)
                VALUES ($1, $2, $3)",
            self.device_id,
            self.wireguard_network_id,
            self.wireguard_ip
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn update(&self, pool: &DbPool) -> Result<(), SqlxError> {
        query!(
            r#"
        UPDATE wireguard_network_device
        SET wireguard_ip = $3
        WHERE device_id = $1 AND wireguard_network_id = $2
        "#,
            self.device_id,
            self.wireguard_network_id,
            self.wireguard_ip
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn find(
        pool: &DbPool,
        device_id: i64,
        network_id: i64,
    ) -> Result<Option<Self>, SqlxError> {
        let res = query_as!(
            Self,
            "SELECT * FROM
                            wireguard_network_device
                            WHERE device_id = $1 AND wireguard_network_id = $2",
            device_id,
            network_id
        )
        .fetch_optional(pool)
        .await?;
        Ok(res)
    }

    pub async fn findy_by_device(
        pool: &DbPool,
        device_id: i64,
    ) -> Result<Option<Vec<Self>>, SqlxError> {
        let result = query_as!(
            Self,
            "SELECT *
            FROM wireguard_network_device WHERE device_id = $1",
            device_id
        )
        .fetch_all(pool)
        .await?;
        if !result.is_empty() {
            return Ok(Some(result));
        }
        Ok(None)
    }
}

impl Device {
    #[must_use]
    pub fn new(name: String, wireguard_pubkey: String, user_id: i64) -> Self {
        Self {
            id: None,
            name,
            wireguard_pubkey,
            user_id,
            created: Utc::now().naive_utc(),
        }
    }

    pub fn update_from(&mut self, other: ModifyDevice) {
        self.name = other.name;
        self.wireguard_pubkey = other.wireguard_pubkey;
    }
    /// Create wireguard config for device
    #[must_use]
    pub fn create_config(
        &self,
        network: &WireguardNetwork,
        device_network_info: &DeviceNetworkInfo,
    ) -> String {
        let dns = match network.dns {
            Some(dns) => {
                if dns.is_empty() {
                    String::new()
                } else {
                    format!("DNS = {}", dns)
                }
            }
            None => String::new(),
        };
        let allowed_ips = network
            .allowed_ips
            .iter()
            .map(IpNetwork::to_string)
            .collect::<Vec<String>>()
            .join(",");
        format!(
            "[Interface]\n\
            PrivateKey = YOUR_PRIVATE_KEY\n\
            Address = {}\n\
            {}\n\
            \n\
            [Peer]\n\
            PublicKey = {}\n\
            AllowedIPs = {}\n\
            Endpoint = {}:{}\n\
            PersistentKeepalive = 300",
            device_network_info.wireguard_ip,
            dns,
            network.pubkey,
            allowed_ips,
            network.endpoint,
            network.port,
        )
    }

    pub async fn find_by_ip(
        pool: &DbPool,
        ip: &str,
        network_id: i64,
    ) -> Result<Option<Self>, SqlxError> {
        query_as!(
            Self,
            "SELECT d.id \"id?\", d.name, d.wireguard_pubkey, d.user_id, d.created \
            FROM device d 
            JOIN wireguard_network_device wnd
            ON d.id = wnd.device_id
            WHERE wnd.wireguard_ip = $1 AND wnd.wireguard_network_id = $2",
            ip,
            network_id
        )
        .fetch_optional(pool)
        .await
    }

    // find all devices by network id and return with assosieted network information
    pub async fn find_by_network(
        pool: &DbPool,
        network_id: i64,
    ) -> Result<Option<Vec<(Self, DeviceNetworkInfo)>>, SqlxError> {
        let result = query!(
            r#"
            SELECT * FROM wireguard_network_device wnd
            JOIN device d
            ON wnd.device_id = d.id
            WHERE wireguard_network_id = $1
        "#,
            network_id
        )
        .fetch_all(pool)
        .await?;

        if !result.is_empty() {
            let res: Vec<(Self, DeviceNetworkInfo)> = result
                .iter()
                .map(|r| {
                    let device = Self {
                        id: Some(r.id),
                        user_id: r.user_id,
                        created: r.created,
                        name: r.name.clone(),
                        wireguard_pubkey: r.wireguard_pubkey.clone(),
                    };
                    let device_network_info = DeviceNetworkInfo {
                        device_id: r.device_id,
                        wireguard_network_id: r.wireguard_network_id,
                        wireguard_ip: r.wireguard_ip.clone(),
                    };
                    (device, device_network_info)
                })
                .collect();
            return Ok(Some(res));
        };

        Ok(None)
    }

    pub async fn find_by_pubkey(pool: &DbPool, pubkey: &str) -> Result<Option<Self>, SqlxError> {
        query_as!(
            Self,
            "SELECT id \"id?\", name, wireguard_pubkey, user_id, created \
            FROM device WHERE wireguard_pubkey = $1",
            pubkey
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn find_by_id_and_username(
        pool: &DbPool,
        id: i64,
        username: &str,
    ) -> Result<Option<Self>, SqlxError> {
        query_as!(
            Self,
            "SELECT device.id \"id?\", name, wireguard_pubkey, user_id, created \
            FROM device JOIN \"user\" ON device.user_id = \"user\".id \
            WHERE device.id = $1 AND \"user\".username = $2",
            id,
            username
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn find_by_id_and_user_id(
        pool: &DbPool,
        id: i64,
        user_id: i64,
    ) -> Result<Option<Self>, SqlxError> {
        query_as!(
            Self,
            "SELECT device.id \"id?\", name, wireguard_pubkey, user_id, created \
            FROM device JOIN \"user\" ON device.user_id = \"user\".id \
            WHERE device.id = $1 AND \"user\".id = $2",
            id,
            user_id
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn get_ip(
        &self,
        pool: &DbPool,
        network_id: i64,
    ) -> Result<Option<String>, SqlxError> {
        if let Some(device_id) = self.id {
            let result = query!(
                r#"
                SELECT wireguard_ip
                FROM wireguard_network_device
                WHERE device_id = $1 AND wireguard_network_id = $2
            "#,
                device_id,
                network_id
            )
            .fetch_one(pool)
            .await?;
            return Ok(Some(result.wireguard_ip));
        }

        Ok(None)
    }

    pub async fn all_for_username(pool: &DbPool, username: &str) -> Result<Vec<Self>, SqlxError> {
        query_as!(
            Self,
            "SELECT device.id \"id?\", name, wireguard_pubkey, user_id, created \
            FROM device JOIN \"user\" ON device.user_id = \"user\".id \
            WHERE \"user\".username = $1",
            username
        )
        .fetch_all(pool)
        .await
    }
    /// Creates new device and assign IP in given network
    pub async fn new_with_ip(
        pool: &DbPool,
        user_id: i64,
        name: String,
        pubkey: String,
        network: &WireguardNetwork,
    ) -> Result<(Self, DeviceNetworkInfo), ModelError> {
        let network_id = match network.id {
            Some(id) => id,
            None => {
                return Err(ModelError::CannotCreate);
            }
        };
        let net_ip = network.address.ip();
        let net_network = network.address.network();
        let net_broadcast = network.address.broadcast();
        for ip in network.address.iter() {
            if ip == net_ip || ip == net_network || ip == net_broadcast {
                continue;
            }
            // Break loop if IP is unassigned and return device
            match Self::find_by_ip(pool, &ip.to_string(), network_id).await? {
                Some(_) => (),
                None => {
                    let mut device = Self::new(name.clone(), pubkey, user_id);
                    device.save(pool).await?;
                    info!("Created device: {}", device.name);
                    debug!("For user: {}", device.user_id);
                    let device_network_info =
                        DeviceNetworkInfo::new(network_id, device.id.unwrap(), ip.to_string());
                    device_network_info.insert(pool).await?;
                    info!(
                        "Assigned IP: {} for device: {} in network: {}",
                        ip, name, network_id
                    );
                    return Ok((device, device_network_info));
                }
            }
        }
        Err(ModelError::CannotCreate)
    }

    // Assign IP to the device in given network
    pub async fn assign_ip(
        &self,
        pool: &DbPool,
        network: &WireguardNetwork,
    ) -> Result<DeviceNetworkInfo, ModelError> {
        let network_id = match network.id {
            Some(id) => id,
            None => {
                return Err(ModelError::CannotCreate);
            }
        };
        let net_ip = network.address.ip();
        let net_network = network.address.network();
        let net_broadcast = network.address.broadcast();
        for ip in network.address.iter() {
            if ip == net_ip || ip == net_network || ip == net_broadcast {
                continue;
            }
            // Break loop if IP is unassigned and return device
            match Self::find_by_ip(pool, &ip.to_string(), network_id).await? {
                Some(_) => (),
                None => {
                    info!("Created IP: {} for device: {}", ip, self.name);
                    let device_network_info =
                        DeviceNetworkInfo::new(network_id, self.id.unwrap(), ip.to_string());
                    device_network_info.insert(pool).await?;
                    return Ok(device_network_info);
                }
            }
        }
        Err(ModelError::CannotCreate)
    }

    pub fn validate_pubkey(pubkey: &str) -> Result<(), String> {
        lazy_static! {
            static ref RE: Regex = Regex::new("^[A-Za-z0-9+/]{42}[AEIMQUYcgkosw480]=$").unwrap();
        }
        if RE.is_match(pubkey) {
            Ok(())
        } else {
            Err(format!("{} is not a valid pubkey", pubkey))
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::db::User;
    use claims::{assert_err, assert_ok};

    #[sqlx::test]
    async fn test_assign_device_ip(pool: DbPool) {
        let mut network = WireguardNetwork::default();
        network.try_set_address("10.1.1.1/30").unwrap();

        let mut user = User::new(
            "testuser".to_string(),
            "hunter2",
            "Tester".to_string(),
            "Test".to_string(),
            "test@test.com".to_string(),
            None,
        );
        user.save(&pool).await.unwrap();
        let (device, device_network_info) = Device::new_with_ip(
            &pool,
            user.id.unwrap(),
            "dev1".into(),
            "key1".into(),
            &network,
        )
        .await
        .unwrap();
        assert_eq!(device_network_info.wireguard_ip, "10.1.1.2");

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
