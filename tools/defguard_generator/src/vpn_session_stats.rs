use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use anyhow::Result;
use chrono::{Duration, NaiveDateTime, Utc};
use defguard_common::db::{
    Id,
    models::{
        WireguardNetwork,
        device::WireguardNetworkDevice,
        gateway::Gateway,
        vpn_client_session::{VpnClientSession, VpnClientSessionState},
        vpn_session_stats::VpnSessionStats,
    },
};
use rand::{Rng, rngs::ThreadRng, seq::SliceRandom};
use sqlx::{PgConnection, PgPool, QueryBuilder, query};
use tracing::{debug, info};

use crate::{user_devices::prepare_user_devices, users::prepare_users};

const STATS_COLLECTION_INTERVAL: Duration = Duration::seconds(30);
const HANDSHAKE_INTERVAL: Duration = Duration::minutes(2);

#[derive(Debug)]
pub struct VpnSessionGeneratorConfig {
    pub location_id: Option<Id>,
    pub num_users: usize,
    pub devices_per_user: u8,
    pub sessions_per_device: u8,
    pub no_truncate: bool,
    pub stats_batch_size: u16,
}

pub async fn generate_vpn_session_stats(
    pool: PgPool,
    config: VpnSessionGeneratorConfig,
) -> Result<()> {
    info!("Running VPN stats generator with config: {config:#?}");

    // clear sessions & stats tables unless disabled
    if !config.no_truncate {
        info!("Clearing existing sessions & stats");
        truncate_with_restart(&pool).await?;
    }

    let locations = match config.location_id {
        Some(location_id) => {
            let location = WireguardNetwork::find_by_id(&pool, location_id)
                .await?
                .expect("Location not found");
            vec![location]
        }
        None => WireguardNetwork::all(&pool).await?,
    };

    info!("Generating stats for {} VPN location(s)", locations.len());

    for location in locations {
        generate_stats_for_location(&pool, &config, location).await?;
    }

    Ok(())
}

async fn generate_stats_for_location(
    pool: &PgPool,
    config: &VpnSessionGeneratorConfig,
    location: WireguardNetwork<Id>,
) -> Result<()> {
    let mut rng = rand::thread_rng();

    let location_seed = rng.gen_range(0.0..1_000.0);

    info!(
        "Generating VPN stats for location {} ({})",
        location.name, location.id
    );

    // prepare a gateway
    let gateway = prepare_gateway(pool, location.id).await?;

    // prepare requested number of users
    let mut users = prepare_users(pool, &mut rng, config.num_users).await?;
    users.shuffle(&mut rng);
    let user_count = rng.gen_range((config.num_users / 2).max(1)..=config.num_users.max(1));
    users.truncate(user_count);

    // generate sessions for each user
    for (i, user) in users.into_iter().enumerate() {
        info!(
            "[{}/{user_count}] Generating VPN sessions for user {user}",
            i + 1
        );

        // begin DB transaction
        let mut transaction = pool.begin().await?;

        // prepare requested number of devices
        let device_count = rng.gen_range(1..=config.devices_per_user.max(1)) as usize;
        let devices = prepare_user_devices(pool, &mut rng, &user, device_count).await?;

        let mut used_ips = location.all_used_ips_for_network(&mut transaction).await?;
        // assign devices to the network if not already assigned
        for device in &devices {
            if WireguardNetworkDevice::find(&mut *transaction, device.id, location.id)
                .await?
                .is_none()
            {
                info!(
                    "Assigning device {} to network {} with auto-generated IP",
                    device.name, location.name
                );
                let wireguard_network_device = device
                    .assign_next_network_ip(&mut transaction, &location, &used_ips, None, None)
                    .await?;
                used_ips.extend(wireguard_network_device.wireguard_ips);
            } else {
                info!(
                    "Device {} already assigned to network {}",
                    device.name, location.name
                );
            }
        }

        for device in devices {
            info!("Generating sessions for device {device}");
            // generate requested number of sessions for a device
            // we always start with a session that's currently active
            // and generate past ones as needed

            // start with the active session
            let mut session_end = Utc::now().naive_utc();
            let session_count = rng.gen_range(1..=config.sessions_per_device.max(1));

            for i in 0..session_count {
                let session_duration = Duration::minutes(rng.gen_range(10..120));
                let session_start = session_end - session_duration;

                let mut session = VpnClientSession::new(
                    location.id,
                    device.user_id,
                    device.id,
                    Some(session_start),
                    None,
                );

                // mark all but the first session as disconnected
                if i > 0 {
                    session.state = VpnClientSessionState::Disconnected;
                    session.disconnected_at = Some(session_end);
                }

                let session = session.save(&mut *transaction).await?;

                debug!("Created session {session:?}");

                generate_mock_session_stats(
                    &mut transaction,
                    &mut rng,
                    session.id,
                    gateway.id,
                    session_start,
                    session_end,
                    config.stats_batch_size,
                    location_seed,
                )
                .await?;

                debug!("Finished generating mock stats for session {session:?}");

                // update end timestamp for next session
                session_end -= Duration::minutes(rng.gen_range(30..120));
            }
        }
        transaction.commit().await?;
    }

    Ok(())
}

/// Remove all records from sessions and stats tables.
/// This also resets the auto-incrementing sequences.
async fn truncate_with_restart(pool: &PgPool) -> Result<()> {
    query("TRUNCATE vpn_client_session RESTART IDENTITY CASCADE")
        .execute(pool)
        .await?;

    Ok(())
}

async fn prepare_gateway(pool: &PgPool, location_id: Id) -> Result<Gateway<Id>> {
    // check if a gateway exists already
    let existing_gateways = Gateway::find_by_location_id(pool, location_id).await?;
    match existing_gateways.into_iter().next() {
        Some(gateway) => Ok(gateway),
        None => {
            let gateway = Gateway::new(location_id, "test", "localhost", 50055, "Generator")
                .save(pool)
                .await?;
            Ok(gateway)
        }
    }
}
#[allow(clippy::too_many_arguments)]
async fn generate_mock_session_stats(
    transaction: &mut PgConnection,
    rng: &mut ThreadRng,
    session_id: Id,
    gateway_id: Id,
    session_start: NaiveDateTime,
    session_end: NaiveDateTime,
    batch_size: u16,
    location_seed: f64,
) -> Result<()> {
    let mut latest_handshake = session_start;
    let mut next_handshake = latest_handshake + HANDSHAKE_INTERVAL;
    let mut collected_at = session_start;
    let mut total_upload = 0;
    let mut total_download = 0;

    // assume the IP remains static within a single session
    let endpoint = random_socket_addr(rng).to_string();

    let upload_scale = rng.gen_range(20_000.0..400_000.0);
    let download_scale = rng.gen_range(20_000.0..400_000.0);

    // Vector to accumulate stats before batch insertion
    let mut stats_batch: Vec<VpnSessionStats> = Vec::new();

    while collected_at <= session_end {
        let minutes = collected_at.and_utc().timestamp() as f64 / 60.0;
        let activity = |phase: f64| {
            (0.5 + 0.35 * (minutes * 0.5 + phase + location_seed).sin()).clamp(0.05, 1.0)
        };

        // generate traffic
        let upload_diff = (upload_scale * activity(0.0) * rng.gen_range(0.2..1.8)).max(1.0) as i64;
        total_upload += upload_diff;
        let download_diff =
            (download_scale * activity(2.0) * rng.gen_range(0.2..1.8)).max(1.0) as i64;
        total_download += download_diff;

        let stats = VpnSessionStats::new(
            session_id,
            gateway_id,
            collected_at,
            latest_handshake,
            endpoint.clone(),
            total_upload,
            total_download,
            upload_diff,
            download_diff,
        );

        stats_batch.push(stats);

        // If batch is full, insert all at once
        if stats_batch.len() >= batch_size.into() {
            insert_stats_batch(&mut *transaction, &stats_batch).await?;
            stats_batch.clear();
        }

        // update variables for next sample
        collected_at += STATS_COLLECTION_INTERVAL;

        // update handshake if necessary
        if collected_at > next_handshake {
            latest_handshake = next_handshake;
            next_handshake = latest_handshake + HANDSHAKE_INTERVAL;
        }
    }

    // Insert any remaining stats in the batch
    if !stats_batch.is_empty() {
        insert_stats_batch(&mut *transaction, &stats_batch).await?;
    }

    Ok(())
}

/// Insert multiple VpnSessionStats records in a single query
async fn insert_stats_batch(
    transaction: &mut PgConnection,
    stats_batch: &[VpnSessionStats],
) -> Result<()> {
    if stats_batch.is_empty() {
        return Ok(());
    }

    let mut query_builder = QueryBuilder::new(
        "INSERT INTO vpn_session_stats (session_id, gateway_id, collected_at, latest_handshake, \
        endpoint, total_upload, total_download, upload_diff, download_diff) ",
    );

    query_builder.push_values(stats_batch, |mut b, stats| {
        b.push_bind(stats.session_id)
            .push_bind(stats.gateway_id)
            .push_bind(stats.collected_at)
            .push_bind(stats.latest_handshake)
            .push_bind(&stats.endpoint)
            .push_bind(stats.total_upload)
            .push_bind(stats.total_download)
            .push_bind(stats.upload_diff)
            .push_bind(stats.download_diff);
    });

    let query = query_builder.build();
    query.execute(&mut *transaction).await?;

    Ok(())
}

fn random_socket_addr(rng: &mut ThreadRng) -> SocketAddr {
    let ip = Ipv4Addr::new(rng.r#gen(), rng.r#gen(), rng.r#gen(), rng.r#gen());
    let port = rng.r#gen();
    SocketAddr::new(IpAddr::V4(ip), port)
}
