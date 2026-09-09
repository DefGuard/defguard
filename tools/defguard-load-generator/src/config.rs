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
    command: TestCommand,
}

#[derive(Debug, Subcommand)]
pub enum TestCommand {
    /// Polls the Edge configuration endpoint.
    ConfigPolling(ConfigPollingArgs),
}

#[derive(Debug, Args)]
pub struct ConfigPollingArgs {
    /// Target polling request rate.
    #[arg(long)]
    requests_per_second: NonZeroU64,
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::{Command, Config};

    #[test]
    fn parses_seed_network_and_user_count() {
        let config = Config::parse_from([
            "defguard-load-generator",
            "seed",
            "--users",
            "100",
            "--network-id",
            "1",
        ]);

        let Command::Seed(seed) = config.command else {
            panic!("expected seed command");
        };

        assert_eq!(seed.users.get(), 100);
        assert_eq!(seed.network_id, 1);
    }
}
