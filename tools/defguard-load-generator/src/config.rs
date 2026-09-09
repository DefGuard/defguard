use std::num::{NonZeroU64, NonZeroUsize};

use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "defguard-load-generator")]
pub struct Config {
    #[command(subcommand)]
    command: Command,
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
    users: NonZeroUsize,
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
