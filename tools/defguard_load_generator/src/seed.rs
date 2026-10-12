use std::{collections::HashSet, net::IpAddr};

use anyhow::Context;
use argon2::{Argon2, PasswordHasher};
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
const LOAD_TEST_PASSWORD: &str = "Password123!1";
const TOTP_SECRET_BYTES: usize = 20;

#[derive(FromRow)]
struct Network {
    id: i64,
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
    let selected_networks: Vec<Network> = query_as(
        "SELECT id, address, allow_all_groups FROM wireguard_network \
         WHERE ($1::bigint IS NULL OR id = $1) ORDER BY id",
    )
    .bind(args.network_id)
    .fetch_all(&mut *connection)
    .await?;
    if selected_networks.is_empty() {
        anyhow::bail!("no networks found for selection {:?}", args.network_id);
    }
    let mut networks = Vec::with_capacity(selected_networks.len());
    for network in selected_networks {
        validate_network_capacity(&network, args.users.get())?;
        let assigned_ips: Vec<IpNetwork> = query_scalar(
            "SELECT unnest(wireguard_ips) FROM wireguard_network_device WHERE wireguard_network_id = $1",
        )
        .bind(network.id)
        .fetch_all(&mut *connection)
        .await?;
        let used_ips = assigned_ips.into_iter().map(|ip| ip.ip()).collect();
        networks.push((network, used_ips));
    }

    tracing::info!(
        users = args.users.get(),
        networks = networks.len(),
        "seeding users and devices"
    );
    let password_hash = password_hash()?;

    for index in 1..=args.users.get() {
        let username = user_name(index);
        let user_id: i64 = match query_scalar("SELECT id FROM \"user\" WHERE username = $1")
            .bind(&username)
            .fetch_optional(&mut *connection)
            .await?
        {
            Some(id) => id,
            None => {
                query_scalar(
                    "INSERT INTO \"user\" \
                     (username, password_hash, last_name, first_name, email, ldap_rdn, \
                      mfa_enabled, totp_enabled, totp_secret, mfa_method, is_active, enrollment_pending) \
                     VALUES ($1, $2, $3, $4, $5, $1, TRUE, TRUE, $6, 'one_time_password'::mfa_method, TRUE, FALSE) \
                     RETURNING id",
                )
                .bind(&username)
                .bind(&password_hash)
                .bind("Load test")
                .bind("User")
                .bind(user_email(index))
                .bind(totp_secret())
                .fetch_one(&mut *connection)
                .await?
            }
        };

        let device_id: i64 = match query_scalar(
            "SELECT id FROM device WHERE user_id = $1 AND name = $2 ORDER BY id LIMIT 1",
        )
        .bind(user_id)
        .bind(device_name(index))
        .fetch_optional(&mut *connection)
        .await?
        {
            Some(id) => id,
            None => {
                query_scalar(
                    "INSERT INTO device (name, wireguard_pubkey, user_id, created, device_type, configured) \
                     VALUES ($1, $2, $3, NOW(), 'user'::device_type, true) RETURNING id",
                )
                .bind(device_name(index))
                .bind(wireguard_public_key())
                .bind(user_id)
                .fetch_one(&mut *connection)
                .await?
            }
        };

        let has_token: bool =
            query_scalar("SELECT EXISTS (SELECT 1 FROM pollingtoken WHERE device_id = $1)")
                .bind(device_id)
                .fetch_one(&mut *connection)
                .await?;
        if !has_token {
            query("INSERT INTO pollingtoken (token, device_id) VALUES ($1, $2)")
                .bind(polling_token())
                .bind(device_id)
                .execute(&mut *connection)
                .await?;
        }

        for (network, used_ips) in &mut networks {
            if !network.allow_all_groups {
                query(
                    "INSERT INTO group_user (group_id, user_id) \
                     SELECT group_id, $1 FROM wireguard_network_allowed_group WHERE network_id = $2 \
                     ON CONFLICT DO NOTHING",
                )
                .bind(user_id)
                .bind(network.id)
                .execute(&mut *connection)
                .await?;
            }

            let assigned: bool = query_scalar(
                "SELECT EXISTS (SELECT 1 FROM wireguard_network_device \
                 WHERE device_id = $1 AND wireguard_network_id = $2)",
            )
            .bind(device_id)
            .bind(network.id)
            .fetch_one(&mut *connection)
            .await?;
            if !assigned {
                let device_ips = allocate_ips(&network.address, used_ips)?;
                query(
                    "INSERT INTO wireguard_network_device (device_id, wireguard_network_id, wireguard_ips) \
                     VALUES ($1, $2, $3)",
                )
                .bind(device_id)
                .bind(network.id)
                .bind(&device_ips)
                .execute(&mut *connection)
                .await?;
                used_ips.extend(device_ips.iter().map(IpNetwork::ip));
            }
        }

        if index == 1 || index % 100 == 0 || index == args.users.get() {
            tracing::info!(seeded_users = index, "seed progress");
        }
    }

    tracing::info!(seeded_users = args.users.get(), "seeding complete");
    Ok(())
}

fn validate_network_capacity(network: &Network, users: usize) -> anyhow::Result<()> {
    if network.address.is_empty() {
        anyhow::bail!("network {} has no address ranges", network.id);
    }
    for address in &network.address {
        let capacity = address
            .iter()
            .filter(|ip| is_usable_host(address, *ip))
            .take(users)
            .count();
        if capacity < users {
            anyhow::bail!(
                "network {} address range {} cannot hold {} seeded devices",
                network.id,
                address,
                users
            );
        }
    }
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
                .find(|ip| is_usable_host(address, *ip) && !used_ips.contains(ip))
                .map(IpNetwork::from)
                .with_context(|| format!("network address space is exhausted for {address}"))
        })
        .collect()
}

fn is_usable_host(address: &IpNetwork, ip: IpAddr) -> bool {
    ip != address.ip() && ip != address.network() && ip != address.broadcast()
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

fn password_hash() -> anyhow::Result<String> {
    Ok(Argon2::default()
        .hash_password(LOAD_TEST_PASSWORD.as_bytes())?
        .to_string())
}

fn totp_secret() -> Vec<u8> {
    let mut secret = vec![0_u8; TOTP_SECRET_BYTES];
    OsRng.fill(&mut secret[..]);
    secret
}
