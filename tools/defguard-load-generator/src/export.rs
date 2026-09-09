use std::{
    fs::{File, OpenOptions},
    io::{BufWriter, Write},
    os::unix::fs::OpenOptionsExt,
};

use anyhow::{Context, bail};
use secrecy::ExposeSecret;
use serde::Serialize;
use sqlx::{
    FromRow,
    postgres::{PgConnectOptions, PgPoolOptions},
    query_as,
};

use crate::config::ConfigPollingExportArgs;

const USERNAME_PREFIX: &str = "load-test-user-%";

#[derive(FromRow, Serialize)]
struct PollingActor {
    user_id: i64,
    device_id: i64,
    polling_token: String,
}

pub async fn export_config_polling(args: ConfigPollingExportArgs) -> anyhow::Result<()> {
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
    let actors: Vec<PollingActor> = query_as(
        "SELECT DISTINCT ON (d.id) u.id AS user_id, d.id AS device_id, p.token AS polling_token \
         FROM pollingtoken p \
         JOIN device d ON d.id = p.device_id \
         JOIN \"user\" u ON u.id = d.user_id \
         JOIN wireguard_network_device wnd ON wnd.device_id = d.id \
         WHERE wnd.wireguard_network_id = $1 AND u.username LIKE $2 \
         ORDER BY d.id, p.created_at DESC",
    )
    .bind(args.network_id)
    .bind(USERNAME_PREFIX)
    .fetch_all(&pool)
    .await?;

    if actors.is_empty() {
        bail!(
            "no seeded polling actors found for network {}",
            args.network_id
        );
    }

    let file = create_output(&args.output)?;
    let mut writer = BufWriter::new(file);
    for actor in &actors {
        serde_json::to_writer(&mut writer, actor)?;
        writer.write_all(b"\n")?;
    }
    writer.flush()?;

    tracing::info!(
        actors = actors.len(),
        output = %args.output.display(),
        "exported config-polling actors"
    );
    Ok(())
}

fn create_output(path: &std::path::Path) -> anyhow::Result<File> {
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .with_context(|| format!("failed to create output file {}", path.display()))
}

#[cfg(test)]
mod tests {
    use std::{fs, os::unix::fs::PermissionsExt};

    use super::create_output;

    #[test]
    fn creates_owner_only_output_file() {
        let path = std::env::temp_dir().join(format!(
            "defguard-load-generator-export-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_file(&path);

        create_output(&path).unwrap();
        let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;

        assert_eq!(mode, 0o600);
        fs::remove_file(path).unwrap();
    }
}
