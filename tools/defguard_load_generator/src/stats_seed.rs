use anyhow::Context;
use chrono::{Duration as ChronoDuration, Utc};
use secrecy::ExposeSecret;
use sqlx::{
    FromRow, Postgres, QueryBuilder, Transaction,
    postgres::{PgConnectOptions, PgPoolOptions},
    query_as, query_scalar,
};

use crate::config::SeedStatsArgs;

const HORIZON: ChronoDuration = ChronoDuration::days(30);
const SAMPLE_INTERVAL: ChronoDuration = ChronoDuration::seconds(30);
const BATCH_SIZE: usize = 1_000;
const ENDPOINT: &str = "198.18.0.1:51820";

#[derive(Debug, FromRow)]
struct DeviceTarget {
    device_id: i64,
    user_id: i64,
    location_id: i64,
    gateway_id: Option<i64>,
}

struct SeedTarget {
    session_id: i64,
    gateway_id: i64,
    total_upload: i64,
    total_download: i64,
}

pub async fn run(args: SeedStatsArgs) -> anyhow::Result<()> {
    let options = PgConnectOptions::new()
        .host(&args.database.database_host)
        .port(args.database.database_port)
        .database(&args.database.database_name)
        .username(&args.database.database_user)
        .password(args.database.database_password.expose_secret());
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect_with(options)
        .await?;

    let targets = query_as::<_, DeviceTarget>(
        "SELECT DISTINCT ON (d.id, wnd.wireguard_network_id)
             d.id AS device_id,
             d.user_id,
             wnd.wireguard_network_id AS location_id,
             g.id AS gateway_id
         FROM device d
         JOIN wireguard_network_device wnd ON wnd.device_id = d.id
         LEFT JOIN gateway g ON g.location_id = wnd.wireguard_network_id
         ORDER BY d.id, wnd.wireguard_network_id, g.id",
    )
    .fetch_all(&pool)
    .await?;

    let started_at = Utc::now().naive_utc();
    let first_sample = started_at - HORIZON;
    let mut transaction = pool.begin().await?;
    let mut skipped_devices = 0_u64;
    let mut inserted_stats = 0_u64;
    let mut seed_targets = Vec::with_capacity(targets.len());

    tracing::info!(targets = targets.len(), horizon = ?HORIZON, interval = ?SAMPLE_INTERVAL, "seeding VPN statistics");

    // Resolve sessions first. The sample loop below then advances all targets together,
    // keeping one batch in memory instead of a full series for one target.
    for target in targets {
        let existing_session: Option<i64> = query_scalar(
            "SELECT id
             FROM vpn_client_session
             WHERE device_id = $1 AND location_id = $2
             ORDER BY (state = 'connected'::vpn_client_session_state) DESC, created_at DESC
             LIMIT 1",
        )
        .bind(target.device_id)
        .bind(target.location_id)
        .fetch_optional(&mut *transaction)
        .await?;

        let session_id = if let Some(session_id) = existing_session {
            let has_stats: bool = query_scalar(
                "SELECT EXISTS(
                    SELECT 1 FROM vpn_session_stats WHERE session_id = $1
                )",
            )
            .bind(session_id)
            .fetch_one(&mut *transaction)
            .await?;
            if has_stats {
                skipped_devices += 1;
                continue;
            }
            session_id
        } else {
            query_scalar(
                "INSERT INTO vpn_client_session
                    (location_id, user_id, device_id, connected_at, state)
                 VALUES ($1, $2, $3, NOW(), 'connected'::vpn_client_session_state)
                 RETURNING id",
            )
            .bind(target.location_id)
            .bind(target.user_id)
            .bind(target.device_id)
            .fetch_one(&mut *transaction)
            .await
            .with_context(|| format!("failed to create session for device {}", target.device_id))?
        };

        let gateway_id = match target.gateway_id {
            Some(gateway_id) => gateway_id,
            None => ensure_gateway(&mut transaction, target.location_id).await?,
        };
        seed_targets.push(SeedTarget {
            session_id,
            gateway_id,
            total_upload: 0,
            total_download: 0,
        });
    }

    // Calculate the final counters first, so samples can be inserted newest-first
    // while retaining monotonically increasing WireGuard counters over time.
    let mut final_upload = 0_i64;
    let mut final_download = 0_i64;
    let mut timestamp = first_sample;
    while timestamp <= started_at {
        let (upload_diff, download_diff) = sample_diffs(timestamp);
        final_upload += upload_diff;
        final_download += download_diff;
        timestamp += SAMPLE_INTERVAL;
    }
    for target in &mut seed_targets {
        target.total_upload = final_upload;
        target.total_download = final_download;
    }

    let mut batch = Vec::with_capacity(BATCH_SIZE);
    let mut collected_at = started_at;
    while collected_at >= first_sample {
        let (upload_diff, download_diff) = sample_diffs(collected_at);

        for target in &mut seed_targets {
            batch.push((
                target.session_id,
                target.gateway_id,
                collected_at,
                collected_at,
                ENDPOINT,
                target.total_upload,
                target.total_download,
                upload_diff,
                download_diff,
            ));
            target.total_upload -= upload_diff;
            target.total_download -= download_diff;

            if batch.len() == BATCH_SIZE {
                insert_batch(&mut transaction, &batch).await?;
                inserted_stats += batch.len() as u64;
                batch.clear();
            }
        }

        collected_at -= SAMPLE_INTERVAL;
    }

    if !batch.is_empty() {
        insert_batch(&mut transaction, &batch).await?;
        inserted_stats += batch.len() as u64;
    }
    transaction.commit().await?;
    let seeded_devices = seed_targets.len() as u64;

    tracing::info!(
        seeded_devices,
        skipped_devices,
        inserted_stats,
        "VPN statistics seeding complete"
    );
    Ok(())
}

/// Generates deterministic, non-zero traffic deltas for a sample.
fn sample_diffs(collected_at: chrono::NaiveDateTime) -> (i64, i64) {
    (
        50_000 + (collected_at.and_utc().timestamp().unsigned_abs() % 100_000) as i64,
        75_000 + (collected_at.and_utc().timestamp().unsigned_abs() % 150_000) as i64,
    )
}

/// Reuses a gateway or creates a disabled synthetic one.
async fn ensure_gateway(
    transaction: &mut Transaction<'_, Postgres>,
    location_id: i64,
) -> anyhow::Result<i64> {
    if let Some(gateway_id) = query_scalar::<_, i64>(
        "INSERT INTO gateway (location_id, name, modified_by, enabled)
         SELECT $1, $2, 'stats-seeder', false
         WHERE NOT EXISTS (
             SELECT 1 FROM gateway WHERE location_id = $1
         )
         RETURNING id",
    )
    .bind(location_id)
    .bind(format!("load-test-stats-{location_id}"))
    .fetch_optional(&mut **transaction)
    .await?
    {
        return Ok(gateway_id);
    }

    query_scalar("SELECT id FROM gateway WHERE location_id = $1 ORDER BY id LIMIT 1")
        .bind(location_id)
        .fetch_one(&mut **transaction)
        .await
        .map_err(Into::into)
}

async fn insert_batch(
    transaction: &mut Transaction<'_, Postgres>,
    batch: &[(
        i64,
        i64,
        chrono::NaiveDateTime,
        chrono::NaiveDateTime,
        &str,
        i64,
        i64,
        i64,
        i64,
    )],
) -> anyhow::Result<()> {
    let mut builder = QueryBuilder::new(
        "INSERT INTO vpn_session_stats
            (session_id, gateway_id, collected_at, latest_handshake, endpoint,
             total_upload, total_download, upload_diff, download_diff) ",
    );
    builder.push_values(batch, |mut values, row| {
        values
            .push_bind(row.0)
            .push_bind(row.1)
            .push_bind(row.2)
            .push_bind(row.3)
            .push_bind(row.4)
            .push_bind(row.5)
            .push_bind(row.6)
            .push_bind(row.7)
            .push_bind(row.8);
    });
    builder.build().execute(&mut **transaction).await?;
    Ok(())
}
