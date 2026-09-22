use std::{
    num::{NonZeroU64, NonZeroUsize},
    time::Duration,
};

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
    /// Seeds users and a group in the dedicated OpenLDAP load-test OU.
    SeedLdap(SeedLdapArgs),
    /// Seeds VPN statistics for existing devices.
    SeedStats(SeedStatsArgs),
    /// Runs a load-test scenario.
    Test(TestArgs),
}

#[derive(Debug, Args)]
pub struct SeedArgs {
    /// Number of users and devices to create.
    #[arg(long)]
    pub users: NonZeroUsize,

    /// ID of the location receiving the devices.
    #[arg(long)]
    pub network_id: i64,

    #[command(flatten)]
    pub database: DatabaseArgs,
}

#[derive(Debug, Args)]
pub struct SeedStatsArgs {
    #[command(flatten)]
    pub database: DatabaseArgs,
}

#[derive(Debug, Args)]
pub struct SeedLdapArgs {
    /// Number of LDAP users to create.
    #[arg(long)]
    pub users: NonZeroUsize,

    #[arg(
        long,
        env = "DEFGUARD_LDAP_URL",
        default_value = "ldap://localhost:389"
    )]
    pub ldap_url: String,

    #[arg(
        long,
        env = "DEFGUARD_LDAP_ADMIN_DN",
        default_value = "cn=admin,dc=example,dc=com"
    )]
    /// LDAP administrator DN.
    pub admin_dn: String,

    #[arg(
        long,
        env = "DEFGUARD_LDAP_ADMIN_PASSWORD",
        default_value = "defguard-ldap"
    )]
    pub admin_password: SecretString,

    #[arg(
        long,
        env = "DEFGUARD_LDAP_USER_PASSWORD",
        default_value = "Password123!1"
    )]
    /// Password assigned to seeded users.
    pub user_password: SecretString,

    /// LDAP search base DN.
    #[arg(long, default_value = "dc=example,dc=com")]
    pub base_dn: String,
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

/// Available load-test scenarios.
#[derive(Debug, Subcommand)]
pub enum TestCommand {
    /// Polls the Edge configuration endpoint.
    ConfigPolling(ConfigPollingArgs),
    /// Runs the desktop-client TOTP MFA flow.
    ClientMfa(ClientMfaArgs),
}

/// MFA test settings.
#[derive(Debug, Args)]
pub struct ClientMfaArgs {
    /// Base URL of the proxy, without the client-mfa path.
    #[arg(long)]
    pub proxy_url: String,

    /// WireGuard network/location with internal MFA enabled.
    #[arg(long)]
    pub network_id: i64,

    #[command(flatten)]
    pub database: DatabaseArgs,

    /// Target logical MFA flow rate. Each flow makes two HTTP requests.
    #[arg(long)]
    pub requests_per_second: NonZeroU64,

    /// Maximum number of MFA flows running concurrently.
    #[arg(long, default_value_t = 1024)]
    pub max_in_flight: usize,

    /// Include synthetic passing Linux posture data in each MFA start request.
    #[arg(long)]
    pub with_posture_checks: bool,

    /// Optional test duration. Without it, the test runs until Ctrl-C.
    #[arg(long, value_parser = humantime::parse_duration)]
    pub duration: Option<Duration>,
}

/// Configuration polling test settings.
#[derive(Debug, Args)]
pub struct ConfigPollingArgs {
    /// Base URL of the proxy, without the polling path.
    #[arg(long)]
    pub proxy_url: String,

    /// WireGuard network whose seeded devices should be polled.
    #[arg(long)]
    pub network_id: i64,

    #[command(flatten)]
    pub database: DatabaseArgs,

    /// Target polling request rate.
    #[arg(long)]
    pub requests_per_second: NonZeroU64,

    /// Maximum number of polling requests running concurrently.
    #[arg(long, default_value_t = 1024)]
    pub max_in_flight: usize,

    /// Optional test duration. Without it, the test runs until Ctrl-C.
    #[arg(long, value_parser = humantime::parse_duration)]
    pub duration: Option<Duration>,
}
