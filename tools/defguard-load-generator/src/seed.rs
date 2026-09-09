use std::{collections::HashSet, net::IpAddr};

use anyhow::Context;
use base64::{Engine, engine::general_purpose::STANDARD};
use ipnetwork::IpNetwork;
use rand::{
    Rng,
    distributions::{Alphanumeric, DistString},
    rngs::OsRng,
};
use secrecy::ExposeSecret;
use sqlx::{
    FromRow,
    postgres::{PgConnectOptions, PgPoolOptions},
    query, query_as, query_scalar,
};

use crate::config::SeedArgs;

const USERNAME_PREFIX: &str = "load-test-user-";
const DEVICE_PREFIX: &str = "load-test-device-";

#[derive(FromRow)]
struct Network {
    address: Vec<IpNetwork>,
    allow_all_groups: bool,
}

pub async fn run(args: SeedArgs) -> anyhow::Result<()> {
    let options = PgConnectOptions::new()
        .host(&args.database.database_host)
        .port(args.database.database_port)
        .database(&args.database.database_name)
        .username(&args.database.database_user)
        .password(args.database.database_password.expose_secret());
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await?;
    let mut connection = pool.acquire().await?;
    let network = query_as::<_, Network>(
        "SELECT address, allow_all_groups FROM wireguard_network WHERE id = $1",
    )
    .bind(args.network_id)
    .fetch_optional(&mut *connection)
    .await?
    .with_context(|| format!("network {} does not exist", args.network_id))?;
    let assigned_ips: Vec<IpNetwork> = query_scalar(
        "SELECT unnest(wireguard_ips) FROM wireguard_network_device WHERE wireguard_network_id = $1",
    )
    .bind(args.network_id)
    .fetch_all(&mut *connection)
    .await?;
    let mut used_ips = assigned_ips.into_iter().map(|ip| ip.ip()).collect();

    tracing::info!(
        users = args.users.get(),
        network_id = args.network_id,
        "seeding users and devices"
    );

    for index in 1..=args.users.get() {
        let user_id: i64 = query_scalar(
            "INSERT INTO \"user\" (username, last_name, first_name, email, ldap_rdn) \
             VALUES ($1, $2, $3, $4, $1) RETURNING id",
        )
        .bind(user_name(index))
        .bind("Load test")
        .bind("User")
        .bind(user_email(index))
        .fetch_one(&mut *connection)
        .await?;

        if !network.allow_all_groups {
            query(
                "INSERT INTO group_user (group_id, user_id) \
                 SELECT group_id, $1 FROM wireguard_network_allowed_group WHERE network_id = $2 \
                 ON CONFLICT DO NOTHING",
            )
            .bind(user_id)
            .bind(args.network_id)
            .execute(&mut *connection)
            .await?;
        }

        let device_id: i64 = query_scalar(
            "INSERT INTO device (name, wireguard_pubkey, user_id, created, device_type, configured) \
             VALUES ($1, $2, $3, NOW(), 'user'::device_type, true) RETURNING id",
        )
        .bind(device_name(index))
        .bind(wireguard_public_key())
        .bind(user_id)
        .fetch_one(&mut *connection)
        .await?;

        let device_ips = allocate_ips(&network.address, &used_ips)?;
        query(
            "INSERT INTO wireguard_network_device (device_id, wireguard_network_id, wireguard_ips) \
             VALUES ($1, $2, $3)",
        )
        .bind(device_id)
        .bind(args.network_id)
        .bind(&device_ips)
        .execute(&mut *connection)
        .await?;
        used_ips.extend(device_ips.iter().map(IpNetwork::ip));

        query("INSERT INTO pollingtoken (token, device_id) VALUES ($1, $2)")
            .bind(polling_token())
            .bind(device_id)
            .execute(&mut *connection)
            .await?;

        if index == 1 || index % 100 == 0 || index == args.users.get() {
            tracing::info!(seeded_users = index, "seed progress");
        }
    }

    tracing::info!(seeded_users = args.users.get(), "seeding complete");
    Ok(())
}

fn allocate_ips(
    addresses: &[IpNetwork],
    used_ips: &HashSet<IpAddr>,
) -> anyhow::Result<Vec<IpNetwork>> {
    addresses
        .iter()
        .map(|address| {
            address
                .iter()
                .find(|ip| {
                    *ip != address.ip()
                        && *ip != address.network()
                        && *ip != address.broadcast()
                        && !used_ips.contains(ip)
                })
                .map(IpNetwork::from)
                .with_context(|| format!("network address space is exhausted for {address}"))
        })
        .collect()
}

fn user_name(index: usize) -> String {
    format!("{USERNAME_PREFIX}{index:06}")
}

fn user_email(index: usize) -> String {
    format!("{USERNAME_PREFIX}{index:06}@example.invalid")
}

fn device_name(index: usize) -> String {
    format!("{DEVICE_PREFIX}{index:06}")
}

fn wireguard_public_key() -> String {
    let mut key = [0_u8; 32];
    OsRng.fill(&mut key);
    STANDARD.encode(key)
}

fn polling_token() -> String {
    Alphanumeric.sample_string(&mut OsRng, 32)
}

#[cfg(test)]
mod tests {
    use std::{
        collections::HashSet,
        net::{IpAddr, Ipv4Addr},
    };

    use ipnetwork::IpNetwork;

    use super::{allocate_ips, device_name, user_email, user_name};

    #[test]
    fn generates_deterministic_actor_names() {
        assert_eq!(user_name(1), "load-test-user-000001");
        assert_eq!(user_email(42), "load-test-user-000042@example.invalid");
        assert_eq!(device_name(123), "load-test-device-000123");
    }

    #[test]
    fn allocates_the_first_available_host_address() {
        let address = "10.0.0.1/29".parse::<IpNetwork>().unwrap();
        let used_ips = HashSet::from([IpAddr::V4(Ipv4Addr::new(10, 0, 0, 2))]);

        let assigned = allocate_ips(&[address], &used_ips).unwrap();

        assert_eq!(assigned, ["10.0.0.3/32".parse::<IpNetwork>().unwrap()]);
    }
}
