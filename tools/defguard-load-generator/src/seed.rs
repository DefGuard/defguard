use std::{collections::HashSet, net::IpAddr};

use anyhow::Context;
use argon2::{Argon2, PasswordHasher, password_hash::SaltString};
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
    let password_hash = password_hash()?;

    for index in 1..=args.users.get() {
        let totp_secret = totp_secret();
        let user_id: i64 = query_scalar(
            "INSERT INTO \"user\" \
             (username, password_hash, last_name, first_name, email, ldap_rdn, \
              mfa_enabled, totp_enabled, totp_secret, mfa_method, is_active, enrollment_pending) \
             VALUES ($1, $2, $3, $4, $5, $1, TRUE, TRUE, $6, 'one_time_password'::mfa_method, TRUE, FALSE) \
             RETURNING id",
        )
        .bind(user_name(index))
        .bind(&password_hash)
        .bind("Load test")
        .bind("User")
        .bind(user_email(index))
        .bind(&totp_secret)
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

fn password_hash() -> anyhow::Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    Ok(Argon2::default()
        .hash_password(LOAD_TEST_PASSWORD.as_bytes(), &salt)?
        .to_string())
}

fn totp_secret() -> Vec<u8> {
    let mut secret = vec![0_u8; TOTP_SECRET_BYTES];
    OsRng.fill(&mut secret[..]);
    secret
}
