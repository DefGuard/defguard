use std::num::{NonZeroU64, NonZeroUsize};

use clap::{Args, Parser, Subcommand};
use secrecy::SecretString;

#[derive(Debug, Parser)]
#[command(name = "defguard-load-generator")]
pub struct Config {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Seeds one device for every generated user.
    Seed(SeedArgs),
    /// Exports credentials for a load-test scenario.
    Export(ExportArgs),
    /// Runs a load-test scenario.
    Test(TestArgs),
}

#[derive(Debug, Args)]
pub struct SeedArgs {
    /// Number of users and devices to create.
    #[arg(long)]
    pub users: NonZeroUsize,

    #[arg(long)]
    pub network_id: i64,

    #[command(flatten)]
    pub database: DatabaseArgs,
}

#[derive(Debug, Args)]
pub struct ExportArgs {
    #[command(subcommand)]
    pub command: ExportCommand,
}

#[derive(Debug, Subcommand)]
pub enum ExportCommand {
    /// Exports actors for the config-polling scenario.
    ConfigPolling(ConfigPollingExportArgs),
}

#[derive(Debug, Args)]
pub struct ConfigPollingExportArgs {
    #[arg(long)]
    pub network_id: i64,

    #[arg(long)]
    pub output: std::path::PathBuf,

    #[command(flatten)]
    pub database: DatabaseArgs,
}

#[derive(Debug, Args)]
pub struct DatabaseArgs {
    #[arg(long, env = "DEFGUARD_DB_HOST", default_value = "localhost")]
    pub database_host: String,

    #[arg(long, env = "DEFGUARD_DB_PORT", default_value_t = 5432)]
    pub database_port: u16,

    #[arg(long, env = "DEFGUARD_DB_NAME", default_value = "defguard")]
    pub database_name: String,

    #[arg(long, env = "DEFGUARD_DB_USER", default_value = "defguard")]
    pub database_user: String,

    #[arg(long, env = "DEFGUARD_DB_PASSWORD", default_value = "")]
    pub database_password: SecretString,
}

#[derive(Debug, Args)]
pub struct TestArgs {
    #[command(subcommand)]
    pub command: TestCommand,
}

#[derive(Debug, Subcommand)]
pub enum TestCommand {
    /// Polls the Edge configuration endpoint.
    ConfigPolling(ConfigPollingArgs),
}

#[derive(Debug, Args)]
pub struct ConfigPollingArgs {
    /// Base URL of the proxy, without the polling path.
    #[arg(long)]
    pub proxy_url: String,

    /// Path to the JSONL file containing polling actors (devices).
    #[arg(long)]
    pub devices_file: std::path::PathBuf,

    /// Target polling request rate.
    #[arg(long)]
    pub requests_per_second: NonZeroU64,

    /// Maximum number of polling requests running concurrently.
    #[arg(long, default_value_t = 1024)]
    pub max_in_flight: usize,

    /// Optional test duration. Without it, the test runs until Ctrl-C.
    #[arg(long, value_parser = humantime::parse_duration)]
    pub duration: Option<std::time::Duration>,
}
