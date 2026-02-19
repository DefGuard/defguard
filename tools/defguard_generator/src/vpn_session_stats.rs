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
use rand::{Rng, rngs::ThreadRng};
use sqlx::{PgConnection, PgPool, QueryBuilder};
use tracing::{debug, info};

use crate::{user_devices::prepare_user_devices, users::prepare_users};

const STATS_COLLECTION_INTERVAL: Duration = Duration::seconds(30);
const HANDSHAKE_INTERVAL: Duration = Duration::minutes(2);

#[derive(Debug)]
pub struct VpnSessionGeneratorConfig {
    pub location_id: Id,
    pub num_users: u16,
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
    let mut rng = rand::thread_rng();

    // clear sessions & stats tables unless disabled
    if !config.no_truncate {
        info!("Clearing existing sessions & stats");
        truncate_with_restart(&pool).await?;
    }

    // fetch specified location
    let location = WireguardNetwork::find_by_id(&pool, config.location_id)
        .await?
        .expect("Location not found");

    // prepare a gateway
    let gateway = prepare_gateway(&pool, location.id).await?;

    // prepare requested number of users
    let user_count = config.num_users as usize;
    let users = prepare_users(&pool, &mut rng, user_count).await?;

    // generate sessions for each user
    for (i, user) in users.into_iter().enumerate() {
        info!(
            "[{}/{user_count}] Generating VPN sessions for user {user}",
            i + 1
        );

        // begin DB transaction
        let mut transaction = pool.begin().await?;

        // prepare requested number of devices
        let devices =
            prepare_user_devices(&pool, &mut rng, &user, config.devices_per_user as usize).await?;

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
                device
                    .assign_next_network_ip(&mut transaction, &location, None, None)
                    .await?;
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

            for i in 0..config.sessions_per_device {
                let session_duration = Duration::minutes(rng.gen_range(10..120));
                let session_start = session_end - session_duration;

                let mut session = VpnClientSession::new(
                    location.id,
                    device.user_id,
                    device.id,
                    Some(session_start),
                    None,
                )
                .save(&mut *transaction)
                .await?;

                // mark all but the first session as disconnected
                if i > 0 {
                    session.state = VpnClientSessionState::Disconnected;
                    session.disconnected_at = Some(session_end);
                    session.save(&mut *transaction).await?;
                }

                debug!("Created session {session:?}");

                generate_mock_session_stats(
                    &mut transaction,
                    &mut rng,
                    session.id,
                    gateway.id,
                    session_start,
                    session_end,
                    config.stats_batch_size,
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

/// Remove all records from sessions & stats tables.
/// This also resets the auto-incrementing sequences
async fn truncate_with_restart(pool: &PgPool) -> Result<()> {
    sqlx::query("TRUNCATE TABLE vpn_client_session RESTART IDENTITY CASCADE")
        .execute(pool)
        .await?;

    Ok(())
}

async fn prepare_gateway(pool: &PgPool, location_id: Id) -> Result<Gateway<Id>> {
    // check if a gateway exists already
    let existing_gateways = Gateway::find_by_network_id(pool, location_id).await?;
    match existing_gateways.into_iter().next() {
        Some(gateway) => Ok(gateway),
        None => {
            let gateway = Gateway::new(location_id, "http://localhost:50055", "gateway")
                .save(pool)
                .await?;
            Ok(gateway)
        }
    }
}

async fn generate_mock_session_stats(
    transaction: &mut PgConnection,
    rng: &mut ThreadRng,
    session_id: Id,
    gateway_id: Id,
    session_start: NaiveDateTime,
    session_end: NaiveDateTime,
    batch_size: u16,
) -> Result<()> {
    let mut latest_handshake = session_start;
    let mut next_handshake = latest_handshake + HANDSHAKE_INTERVAL;
    let mut collected_at = session_start;
    let mut total_upload = 0;
    let mut total_download = 0;

    // assume the IP remains static within a single session
    let endpoint = random_socket_addr(rng).to_string();

    // Vector to accumulate stats before batch insertion
    let mut stats_batch: Vec<VpnSessionStats> = Vec::new();

    while collected_at <= session_end {
        // generate traffic
        let upload_diff = rng.gen_range(100..100_000);
        total_upload += upload_diff;
        let download_diff = rng.gen_range(100..100_000);
        total_download += download_diff;

        let stats = VpnSessionStats::new(
            session_id,
            gateway_id,
            collected_at,
            latest_handshake,
            endpoint.clone(),
            total_upload,
            total_download,
            download_diff,
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
        "INSERT INTO vpn_session_stats (session_id, gateway_id, collected_at, latest_handshake, endpoint, total_upload, total_download, upload_diff, download_diff) ",
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
