use super::{error::ModelError, wireguard::WireguardNetwork, DbPool};
use chrono::{NaiveDateTime, Utc};
use ipnetwork::IpNetwork;
use model_derive::Model;
use sqlx::{query_as, Error as SqlxError};

#[derive(Clone, Deserialize, Model, Serialize, Debug)]
pub struct Device {
    pub id: Option<i64>,
    pub name: String,
    pub wireguard_ip: String,
    pub wireguard_pubkey: String,
    pub user_id: i64,
    pub created: NaiveDateTime,
}

#[derive(Deserialize)]
pub struct AddDevice {
    pub name: String,
    pub wireguard_pubkey: String,
}

impl Device {
    #[must_use]
    pub fn new(name: String, wireguard_ip: String, wireguard_pubkey: String, user_id: i64) -> Self {
        Self {
            id: None,
            name,
            wireguard_ip,
            wireguard_pubkey,
            user_id,
            created: Utc::now().naive_utc(),
        }
    }

    // FIXME: `other` should be a different struct
    pub fn update_from(&mut self, other: Self) {
        self.name = other.name;
        self.wireguard_ip = other.wireguard_ip;
        self.wireguard_pubkey = other.wireguard_pubkey;
    }

    pub fn create_config(&self, network: WireguardNetwork) -> String {
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
            self.wireguard_ip, dns, network.pubkey, allowed_ips, network.endpoint, network.port,
        )
    }

    pub async fn find_by_ip(pool: &DbPool, ip: &str) -> Result<Option<Self>, SqlxError> {
        query_as!(
            Self,
            "SELECT id \"id?\", name, wireguard_ip, wireguard_pubkey, user_id, created \
            FROM device WHERE wireguard_ip = $1",
            ip
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn find_by_pubkey(pool: &DbPool, pubkey: &str) -> Result<Option<Self>, SqlxError> {
        query_as!(
            Self,
            "SELECT id \"id?\", name, wireguard_ip, wireguard_pubkey, user_id, created \
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
            "SELECT device.id \"id?\", name, wireguard_ip, wireguard_pubkey, user_id, created \
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
            "SELECT device.id \"id?\", name, wireguard_ip, wireguard_pubkey, user_id, created \
            FROM device JOIN \"user\" ON device.user_id = \"user\".id \
            WHERE device.id = $1 AND \"user\".id = $2",
            id,
            user_id
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn all_for_username(pool: &DbPool, username: &str) -> Result<Vec<Self>, SqlxError> {
        query_as!(
            Self,
            "SELECT device.id \"id?\", name, wireguard_ip, wireguard_pubkey, user_id, created \
            FROM device JOIN \"user\" ON device.user_id = \"user\".id \
            WHERE \"user\".username = $1",
            username
        )
        .fetch_all(pool)
        .await
    }

    pub async fn assign_device_ip(
        pool: &DbPool,
        user_id: i64,
        name: String,
        pubkey: String,
        network: &WireguardNetwork,
    ) -> Result<Self, ModelError> {
        let net_ip = network.address.ip();
        let net_network = network.address.network();
        let net_broadcast = network.address.broadcast();
        for ip in network.address.iter() {
            if ip == net_ip || ip == net_network || ip == net_broadcast {
                continue;
            }
            // Break loop if IP is unassigned and return device
            match Self::find_by_ip(pool, &ip.to_string()).await? {
                Some(_) => (),
                None => {
                    info!("Created IP: {} for device: {}", ip, name);
                    let device = Self::new(name, ip.to_string(), pubkey, user_id);
                    return Ok(device);
                }
            }
        }
        Err(ModelError::CannotCreate)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[sqlx::test]
    async fn test_assign_device_ip(pool: DbPool) {
        let mut network = WireguardNetwork::default();
        network.try_set_address("10.1.1.1/30").unwrap();

        let mut device = Device::assign_device_ip(&pool, 1, "dev1".into(), "key1".into(), &network)
            .await
            .unwrap();
        assert_eq!(device.wireguard_ip, "10.1.1.2");
        device.save(&pool).await.unwrap();

        let device =
            Device::assign_device_ip(&pool, 1, "dev4".into(), "key4".into(), &network).await;
        assert!(device.is_err());
    }
}
