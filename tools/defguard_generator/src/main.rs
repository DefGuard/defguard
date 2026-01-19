use anyhow::{Context, Result};
use chrono::{Duration, NaiveDateTime, Utc};
use clap::{Arg, ArgAction, Command, Parser, Subcommand, arg};
use defguard_common::db::{
    Id, init_db,
    models::{
        WireguardNetwork,
        vpn_client_session::{VpnClientSession, VpnClientSessionState},
        vpn_session_stats::VpnSessionStats,
    },
    setup_pool,
};
use defguard_generator::vpn_session_stats::{
    VpnSessionGeneratorConfig, generate_vpn_session_stats,
};
use rand::Rng;
use sqlx::{PgPool, postgres::PgPoolOptions};
use tracing::{Level, info};

#[derive(Debug)]
struct GeneratorConfig {
    users: usize,
    devices_per_user: usize,
    sessions_per_device: usize,
    network_id: Id,
    database_url: String,
}

#[derive(Parser)]
#[command(about, long_about = None)]
struct Cli {
    #[arg(long, env = "DEFGUARD_DB_HOST", default_value = "localhost")]
    pub database_host: String,

    #[arg(long, env = "DEFGUARD_DB_PORT", default_value_t = 5432)]
    pub database_port: u16,

    #[arg(long, env = "DEFGUARD_DB_NAME", default_value = "defguard")]
    pub database_name: String,

    #[arg(long, env = "DEFGUARD_DB_USER", default_value = "defguard")]
    pub database_user: String,

    #[arg(long, env = "DEFGUARD_DB_PASSWORD", default_value = "")]
    pub database_password: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// generates VPN session stats
    VpnSessionStats {
        #[arg(long)]
        location_id: Id,
        #[arg(long)]
        num_users: u16,
        #[arg(long)]
        devices_per_user: u8,
        #[arg(long)]
        sessions_per_device: u8,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    // parse CLI options
    let cli = Cli::parse();

    // setup DB pool
    let pool = init_db(
        &cli.database_host,
        cli.database_port,
        &cli.database_name,
        &cli.database_user,
        &cli.database_password,
    )
    .await;

    // execute based on the selected subcommand
    match cli.command {
        Commands::VpnSessionStats {
            location_id,
            num_users,
            devices_per_user,
            sessions_per_device,
        } => {
            let config = VpnSessionGeneratorConfig {
                location_id,
                num_users,
                devices_per_user,
                sessions_per_device,
            };

            generate_vpn_session_stats(pool, config).await?;
        }
    };

    Ok(())
}
