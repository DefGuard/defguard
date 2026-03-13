use std::{net::IpAddr, sync::OnceLock};

use clap::{Args, Parser, Subcommand};
use humantime::Duration;
use ipnetwork::IpNetwork;
use openidconnect::{core::CoreRsaPrivateSigningKey, JsonWebKeyId};
use reqwest::Url;
use rsa::{
    pkcs1::{DecodeRsaPrivateKey, EncodeRsaPrivateKey},
    pkcs8::{DecodePrivateKey, LineEnding},
    traits::PublicKeyParts,
    RsaPrivateKey,
};
use secrecy::{ExposeSecret, SecretString};
use serde::Serialize;

use crate::{db::models::Settings, VERSION};

pub static SERVER_CONFIG: OnceLock<DefGuardConfig> = OnceLock::new();

pub fn server_config() -> &'static DefGuardConfig {
    SERVER_CONFIG
        .get()
        .expect("Server configuration not set yet")
}

#[derive(Clone, Debug, Parser, Serialize)]
#[command(name = "defguard", version = VERSION)]
// TODO: find a better workaround for clap not working nice with test args
#[cfg_attr(test, command(ignore_errors(true)))]
pub struct DefGuardConfig {
    #[arg(long, env = "DEFGUARD_LOG_LEVEL", default_value = "info")]
    pub log_level: String,

    // TODO: restore file logging, seems to have vanished during the switch to tracing
    #[arg(long, env = "DEFGUARD_LOG_FILE")]
    pub log_file: Option<String>,

    #[arg(long, env = "DEFGUARD_SECRET_KEY")]
    #[serde(skip_serializing)]
    #[deprecated(since = "2.0.0", note = "Use Settings.secret_key instead")]
    pub secret_key: Option<SecretString>,

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
    pub database_password: SecretString,

    #[arg(long, env = "DEFGUARD_HTTP_PORT", default_value_t = 8000)]
    pub http_port: u16,

    #[arg(long, env = "DEFGUARD_GRPC_PORT", default_value_t = 50055)]
    pub grpc_port: u16,

    // Certificate and key for gRPC communication over HTTPS.
    // Kept in runtime config for backwards compatibility - workers still use this.
    #[arg(long, env = "DEFGUARD_GRPC_CERT")]
    pub grpc_cert: Option<String>,
    #[arg(long, env = "DEFGUARD_GRPC_KEY")]
    pub grpc_key: Option<String>,

    #[arg(long, env = "DEFGUARD_OPENID_KEY", value_parser = Self::parse_openid_key)]
    #[serde(skip_serializing)]
    pub openid_signing_key: Option<RsaPrivateKey>,

    #[arg(long, env = "DEFGUARD_URL", value_parser = Url::parse, default_value = "http://localhost:8000")]
    #[serde(skip_serializing)]
    #[deprecated(since = "2.0.0", note = "Use Settings.defguard_url instead")]
    pub url: Url,

    #[arg(long, env = "DEFGUARD_DISABLE_STATS_PURGE")]
    #[deprecated(since = "2.0.0", note = "Use Settings.enable_stats_purge instead")]
    pub disable_stats_purge: Option<bool>,

    #[arg(long, env = "DEFGUARD_STATS_PURGE_FREQUENCY")]
    #[serde(skip_serializing)]
    #[deprecated(since = "2.0.0", note = "Use Settings.stats_purge_frequency instead")]
    pub stats_purge_frequency: Option<Duration>,

    #[arg(long, env = "DEFGUARD_STATS_PURGE_THRESHOLD")]
    #[serde(skip_serializing)]
    #[deprecated(since = "2.0.0", note = "Use Settings.stats_purge_threshold instead")]
    pub stats_purge_threshold: Option<Duration>,

    #[arg(long, env = "DEFGUARD_ENROLLMENT_URL", value_parser = Url::parse)]
    #[serde(skip_serializing)]
    #[deprecated(since = "2.0.0", note = "Use Settings.public_proxy_url instead")]
    pub enrollment_url: Option<Url>,

    #[arg(long, env = "DEFGUARD_ENROLLMENT_TOKEN_TIMEOUT")]
    #[serde(skip_serializing)]
    #[deprecated(
        since = "2.0.0",
        note = "Use Settings.enrollment_token_timeout instead"
    )]
    pub enrollment_token_timeout: Option<Duration>,

    #[arg(long, env = "DEFGUARD_MFA_CODE_TIMEOUT")]
    #[serde(skip_serializing)]
    #[deprecated(
        since = "2.0.0",
        note = "Use Settings.mfa_code_timeout_seconds instead"
    )]
    pub mfa_code_timeout: Option<Duration>,

    #[arg(long, env = "DEFGUARD_SESSION_TIMEOUT")]
    #[serde(skip_serializing)]
    #[deprecated(
        since = "2.0.0",
        note = "Use Settings.authentication_period_days instead"
    )]
    pub session_timeout: Option<Duration>,

    #[arg(long, env = "DEFGUARD_PASSWORD_RESET_TOKEN_TIMEOUT")]
    #[serde(skip_serializing)]
    #[deprecated(
        since = "2.0.0",
        note = "Use Settings.password_reset_token_timeout instead"
    )]
    pub password_reset_token_timeout: Option<Duration>,

    #[arg(long, env = "DEFGUARD_ENROLLMENT_SESSION_TIMEOUT")]
    #[serde(skip_serializing)]
    #[deprecated(
        since = "2.0.0",
        note = "Use Settings.enrollment_session_timeout instead"
    )]
    pub enrollment_session_timeout: Option<Duration>,

    #[arg(long, env = "DEFGUARD_PASSWORD_RESET_SESSION_TIMEOUT")]
    #[serde(skip_serializing)]
    #[deprecated(
        since = "2.0.0",
        note = "Use Settings.password_reset_session_timeout instead"
    )]
    pub password_reset_session_timeout: Option<Duration>,

    #[arg(long, env = "DEFGUARD_COOKIE_DOMAIN")]
    pub cookie_domain: Option<String>,

    #[arg(long, env = "DEFGUARD_COOKIE_INSECURE")]
    pub cookie_insecure: bool,

    #[command(subcommand)]
    #[serde(skip_serializing)]
    pub cmd: Option<Command>,

    #[arg(long, env = "DEFGUARD_CHECK_PERIOD", default_value = "12h")]
    #[serde(skip_serializing)]
    pub check_period: Duration,

    #[arg(long, env = "DEFGUARD_CHECK_PERIOD_NO_LICENSE", default_value = "24h")]
    #[serde(skip_serializing)]
    pub check_period_no_license: Duration,

    #[arg(long, env = "DEFGUARD_CHECK_RENEWAL_WINDOW", default_value = "1h")]
    #[serde(skip_serializing)]
    pub check_period_renewal_window: Duration,

    #[arg(long, env = "DEFGUARD_HTTP_BIND_ADDRESS")]
    pub http_bind_address: Option<IpAddr>,

    #[arg(long, env = "DEFGUARD_GRPC_BIND_ADDRESS")]
    pub grpc_bind_address: Option<IpAddr>,

    #[arg(long, env = "DEFGUARD_ADOPT_GATEWAY")]
    pub adopt_gateway: Option<String>,

    #[arg(long, env = "DEFGUARD_ADOPT_EDGE")]
    pub adopt_edge: Option<String>,
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
    pub mtu: u32,
    #[arg(long)]
    pub fwmark: u32,
    #[arg(long)]
    pub allowed_ips: Vec<IpNetwork>,
    #[arg(long)]
    pub id: Option<i64>,
}

impl DefGuardConfig {
    #[must_use]
    pub fn new() -> Self {
        let config = Self::parse();
        #[allow(deprecated)]
        if let Some(secret_key) = &config.secret_key {
            Settings::validate_secret_key(secret_key.expose_secret())
                .expect("Invalid DEFGUARD_SECRET_KEY");
        }
        config
    }

    // this is an ugly workaround to avoid `cargo test` args being captured by `clap`
    #[must_use]
    pub fn new_test_config() -> Self {
        Self::parse_from::<[_; 0], String>([])
    }

    /// Initialize values that depend on Settings.
    pub fn initialize_post_settings(&mut self) {
        if self.cookie_domain.is_none() {
            let settings = Settings::get_current_settings();
            self.cookie_domain = settings.cookie_domain().ok();
        }
    }

    /// Try PKCS#1 and PKCS#8 PEM formats.
    fn parse_openid_key(path: &str) -> Result<RsaPrivateKey, rsa::pkcs8::Error> {
        if let Ok(key) = RsaPrivateKey::read_pkcs1_pem_file(path) {
            Ok(key)
        } else {
            RsaPrivateKey::read_pkcs8_pem_file(path)
        }
    }

    #[must_use]
    pub fn openid_key(&self) -> Option<CoreRsaPrivateSigningKey> {
        let key = self.openid_signing_key.as_ref()?;
        if let Ok(pem) = key.to_pkcs1_pem(LineEnding::default()) {
            let key_id = JsonWebKeyId::new(key.n().to_str_radix(36));
            CoreRsaPrivateSigningKey::from_pem(pem.as_ref(), Some(key_id)).ok()
        } else {
            None
        }
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
    fn test_cookie_domain_env_override() {
        unsafe {
            env::set_var("DEFGUARD_COOKIE_DOMAIN", "example.com");
        }

        let config = DefGuardConfig::new();

        assert_eq!(config.cookie_domain, Some("example.com".to_string()));
    }
}
