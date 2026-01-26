use anyhow::Result;
use clap::{Parser, Subcommand};
use defguard_common::db::{Id, init_db};
use defguard_generator::vpn_session_stats::{
    VpnSessionGeneratorConfig, generate_vpn_session_stats,
};
use tracing_subscriber::EnvFilter;

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
    /// Generates fake VPN session statistics.
    VpnSessionStats {
        #[arg(long)]
        location_id: Id,
        #[arg(long)]
        num_users: u16,
        #[arg(long)]
        devices_per_user: u8,
        #[arg(long)]
        sessions_per_device: u8,
        /// don't truncate sessions & stats tables before generating stats
        #[arg(long)]
        no_truncate: bool,
        /// insert stats records in batches of specified size
        #[arg(long, default_value_t = 1000)]
        stats_batch_size: u16,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

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
            no_truncate,
            stats_batch_size,
        } => {
            let config = VpnSessionGeneratorConfig {
                location_id,
                num_users,
                devices_per_user,
                sessions_per_device,
                no_truncate,
                stats_batch_size,
            };

            generate_vpn_session_stats(pool, config).await?;
        }
    };

    Ok(())
}
