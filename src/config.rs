use clap::{Args, Parser, Subcommand};
use humantime::{parse_duration, DurationError};
use ipnetwork::IpNetwork;
use reqwest::Url;
use secrecy::{ExposeSecret, Secret};
use serde::{Deserialize, Deserializer};
use std::{fs, time::Duration};

#[derive(Clone, Parser, Serialize, Deserialize, Debug)]
#[command(version)]
pub struct DefGuardConfig {
    #[arg(long, env = "DEFGUARD_LOG_LEVEL", default_value = "info")]
    pub log_level: String,

    #[arg(long, env = "DEFGUARD_LOG_FILE")]
    pub log_file: Option<String>,

    #[arg(long, env = "DEFGUARD_AUTH_SESSION_LIFETIME")]
    pub session_auth_lifetime: Option<i64>,

    #[arg(long, env = "DEFGUARD_SECRET_KEY")]
    #[serde(skip_serializing)]
    pub secret_key: Secret<String>,

    #[arg(long, env = "DEFGUARD_DB_HOST", default_value = "localhost")]
    pub database_host: String,

    #[arg(long, env = "DEFGUARD_DB_PORT", default_value_t = 5432)]
    pub database_port: u16,

    #[arg(long, env = "DEFGUARD_DB_NAME", default_value = "defguard")]
    pub database_name: String,

    #[arg(long, env = "DEFGUARD_DB_USER", default_value = "defguard")]
    pub database_user: String,

    #[arg(long, env = "DEFGUARD_DB_PASSWORD", default_value = "")]
    #[serde(skip_serializing)]
    pub database_password: Secret<String>,

    #[arg(long, env = "DEFGUARD_HTTP_PORT", default_value_t = 8000)]
    pub http_port: u16,

    #[arg(long, env = "DEFGUARD_GRPC_PORT", default_value_t = 50055)]
    pub grpc_port: u16,

    #[arg(long, env = "DEFGUARD_GRPC_CERT")]
    pub grpc_cert: Option<String>,

    #[arg(long, env = "DEFGUARD_GRPC_KEY")]
    pub grpc_key: Option<String>,

    #[arg(long, env = "DEFGUARD_ADMIN_GROUPNAME", default_value = "admin")]
    pub admin_groupname: String,

    #[arg(
        long,
        env = "DEFGUARD_USERADMIN_GROUPNAME",
        default_value = "useradmin"
    )]
    pub useradmin_groupname: String,

    #[arg(long, env = "DEFGUARD_VPN_GROUPNAME", default_value = "vpn")]
    pub vpn_groupname: String,

    #[arg(
        long,
        env = "DEFGUARD_DEFAULT_ADMIN_PASSWORD",
        default_value = "pass123"
    )]
    #[serde(skip_serializing)]
    pub default_admin_password: Secret<String>,

    #[arg(long, env = "DEFGUARD_OPENID_KEY")]
    #[serde(skip_serializing)]
    pub openid_signing_key: Option<String>,

    // relying party id and relying party origin for WebAuthn
    #[arg(long, env = "DEFGUARD_WEBAUTHN_RP_ID")]
    pub webauthn_rp_id: Option<String>,
    #[arg(long, env = "DEFGUARD_URL", value_parser = Url::parse, default_value = "http://localhost:8000")]
    pub url: Url,

    #[arg(long, env = "DEFGUARD_GRPC_URL", value_parser = Url::parse, default_value = "http://localhost:50055")]
    pub grpc_url: Url,

    #[arg(long, env = "DEFGUARD_DISABLE_STATS_PURGE")]
    pub disable_stats_purge: bool,

    #[arg(long, env = "DEFGUARD_STATS_PURGE_FREQUENCY", default_value = "24h", value_parser = Self::parse_humantime)]
    #[serde(skip_serializing)]
    #[serde(deserialize_with = "DefGuardConfig::deserialize_humantime")]
    pub stats_purge_frequency: Duration,

    #[arg(long, env = "DEFGUARD_STATS_PURGE_THRESHOLD", default_value = "30d", value_parser = Self::parse_humantime)]
    #[serde(skip_serializing)]
    #[serde(deserialize_with = "DefGuardConfig::deserialize_humantime")]
    pub stats_purge_threshold: Duration,

    #[arg(long, env = "DEFGUARD_ENROLLMENT_URL", value_parser = Url::parse, default_value = "http://localhost:8080")]
    pub enrollment_url: Url,

    #[arg(long, env = "DEFGUARD_ENROLLMENT_TOKEN_TIMEOUT", default_value = "24h", value_parser = Self::parse_humantime)]
    #[serde(skip_serializing)]
    #[serde(deserialize_with = "DefGuardConfig::deserialize_humantime")]
    pub enrollment_token_timeout: Duration,

    #[arg(
        long,
        env = "DEFGUARD_PASSWORD_RESET_TOKEN_TIMEOUT",
        default_value = "24h",
        value_parser = Self::parse_humantime
    )]
    #[serde(skip_serializing)]
    #[serde(deserialize_with = "DefGuardConfig::deserialize_humantime")]
    pub password_reset_token_timeout: Duration,

    #[arg(
        long,
        env = "DEFGUARD_ENROLLMENT_SESSION_TIMEOUT",
        default_value = "10m",
        value_parser = Self::parse_humantime
    )]
    #[serde(skip_serializing)]
    #[serde(deserialize_with = "DefGuardConfig::deserialize_humantime")]
    pub enrollment_session_timeout: Duration,

    #[arg(
        long,
        env = "DEFGUARD_PASSWORD_RESET_SESSION_TIMEOUT",
        default_value = "10m",
        value_parser = Self::parse_humantime
    )]
    #[serde(skip_serializing)]
    #[serde(deserialize_with = "DefGuardConfig::deserialize_humantime")]
    pub password_reset_session_timeout: Duration,

    #[arg(long, env = "DEFGUARD_COOKIE_DOMAIN")]
    pub cookie_domain: Option<String>,

    #[arg(long, env = "DEFGUARD_COOKIE_INSECURE")]
    pub cookie_insecure: bool,

    // TODO: allow multiple values
    #[arg(long, env = "DEFGUARD_PROXY_URL")]
    pub proxy_url: Option<String>,

    // path to certificate `.pem` file used if connecting to proxy over HTTPS
    #[arg(long, env = "DEFGUARD_PROXY_GRPC_CA")]
    pub proxy_grpc_ca: Option<String>,

    #[arg(
        long,
        env = "DEFGUARD_GATEWAY_DISCONNECTION_NOTIFICATION_TIMEOUT",
        default_value = "10m",
        value_parser = Self::parse_humantime
    )]
    #[serde(skip_serializing)]
    #[serde(deserialize_with = "DefGuardConfig::deserialize_humantime")]
    pub gateway_disconnection_notification_timeout: Duration,

    #[command(subcommand)]
    #[serde(skip)]
    pub cmd: Option<Command>,

    /// Configuration file path
    #[arg(long = "config", short)]
    #[serde(skip)]
    config_path: Option<std::path::PathBuf>,
}

#[derive(Clone, Debug, Subcommand)]
pub enum Command {
    #[command(
        about = "Initialize development environment. Inserts test network and device into database."
    )]
    InitDevEnv,
    #[command(
        about = "Add a new VPN location and return a gateway token. Used for automated setup."
    )]
    InitVpnLocation(InitVpnLocationArgs),
}

#[derive(Args, Debug, Clone)]
pub struct InitVpnLocationArgs {
    #[arg(long)]
    pub name: String,
    #[arg(long)]
    pub address: IpNetwork,
    #[arg(long)]
    pub endpoint: String,
    #[arg(long)]
    pub port: i32,
    #[arg(long)]
    pub dns: Option<String>,
    #[arg(long)]
    pub allowed_ips: Vec<IpNetwork>,
}

impl DefGuardConfig {
    #[must_use]
    pub fn new() -> Self {
        let cli_config = Self::parse();

        // load config from file if one was specified
        let mut config = if let Some(config_path) = cli_config.config_path {
            println!("Reading configuration from config file: {config_path:?}");
            let config_toml = fs::read_to_string(config_path).expect("Failed to read config file");
            toml::from_str(&config_toml).expect("Failed to parse config file")
        } else {
            cli_config
        };

        config.validate_rp_id();
        config.validate_cookie_domain();
        config.validate_secret_key();
        config
    }

    // this is an ugly workaround to avoid `cargo test` args being captured by `clap`
    #[must_use]
    pub fn new_test_config() -> Self {
        let mut config = Self::parse_from::<[_; 0], String>([]);

        config.validate_rp_id();
        config.validate_cookie_domain();
        config
    }

    // Check if RP ID value was provided.
    // If not generate it based on URL.
    fn validate_rp_id(&mut self) {
        if self.webauthn_rp_id.is_none() {
            self.webauthn_rp_id = Some(
                self.url
                    .domain()
                    .expect("Unable to get domain for server URL.")
                    .to_string(),
            );
        }
    }

    // Check if cookie domain value was provided.
    // If not generate it based on URL.
    fn validate_cookie_domain(&mut self) {
        if self.cookie_domain.is_none() {
            self.cookie_domain = Some(
                self.url
                    .domain()
                    .expect("Unable to get domain for server URL.")
                    .to_string(),
            );
        }
    }

    fn validate_secret_key(&self) {
        let secret_key = self.secret_key.expose_secret();
        if secret_key.trim().len() != secret_key.len() {
            panic!("SECRET_KEY cannot have leading and trailing space",);
        }

        if secret_key.len() < 64 {
            panic!(
                "SECRET_KEY must be at least 64 characters long, provided value has {} characters",
                secret_key.len()
            );
        }
    }

    /// helper to parse time-related `clap` args
    fn parse_humantime(value: &str) -> Result<Duration, DurationError> {
        parse_duration(value)
    }

    /// helper to manually deserialize humantime values in config file
    fn deserialize_humantime<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let buf = String::deserialize(deserializer)?;

        parse_duration(&buf).map_err(serde::de::Error::custom)
    }
}

impl Default for DefGuardConfig {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use std::env;

    use super::*;

    #[test]
    fn verify_cli() {
        use clap::CommandFactory;
        DefGuardConfig::command().debug_assert();
    }

    #[test]
    fn test_generate_rp_id() {
        // unset variables
        env::remove_var("DEFGUARD_URL");
        env::remove_var("DEFGUARD_WEBAUTHN_RP_ID");

        env::set_var("DEFGUARD_URL", "https://defguard.example.com");

        let config = DefGuardConfig::new();

        assert_eq!(
            config.webauthn_rp_id,
            Some("defguard.example.com".to_string())
        );

        env::set_var("DEFGUARD_WEBAUTHN_RP_ID", "example.com");

        let config = DefGuardConfig::new();

        assert_eq!(config.webauthn_rp_id, Some("example.com".to_string()));
    }

    #[test]
    fn test_generate_cookie_domain() {
        // unset variables
        env::remove_var("DEFGUARD_URL");
        env::remove_var("DEFGUARD_COOKIE_DOMAIN");

        env::set_var("DEFGUARD_URL", "https://defguard.example.com");

        let config = DefGuardConfig::new();

        assert_eq!(
            config.cookie_domain,
            Some("defguard.example.com".to_string())
        );

        env::set_var("DEFGUARD_COOKIE_DOMAIN", "example.com");

        let config = DefGuardConfig::new();

        assert_eq!(config.cookie_domain, Some("example.com".to_string()));
    }
}
