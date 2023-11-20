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
use secrecy::{ExposeSecret, Secret};

#[derive(Clone, Parser, Serialize, Debug)]
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
        env = "DEFGUARD_DEFAULT_ADMIN_PASSWORD",
        default_value = "pass123"
    )]
    #[serde(skip_serializing)]
    pub default_admin_password: Secret<String>,

    #[arg(long, env = "DEFGUARD_OPENID_KEY", value_parser = Self::parse_openid_key)]
    #[serde(skip_serializing)]
    pub openid_signing_key: Option<RsaPrivateKey>,

    // relying party id and relying party origin for WebAuthn
    #[arg(long, env = "DEFGUARD_WEBAUTHN_RP_ID")]
    pub webauthn_rp_id: Option<String>,
    #[arg(long, env = "DEFGUARD_URL", value_parser = Url::parse, default_value = "http://localhost:8000")]
    pub url: Url,

    #[arg(long, env = "DEFGUARD_GRPC_URL", value_parser = Url::parse, default_value = "http://localhost:50055")]
    pub grpc_url: Url,

    #[arg(long, env = "DEFGUARD_DISABLE_STATS_PURGE")]
    pub disable_stats_purge: bool,

    #[arg(long, env = "DEFGUARD_STATS_PURGE_FREQUENCY", default_value = "24h")]
    #[serde(skip_serializing)]
    pub stats_purge_frequency: Duration,

    #[arg(long, env = "DEFGUARD_STATS_PURGE_THRESHOLD", default_value = "30d")]
    #[serde(skip_serializing)]
    pub stats_purge_threshold: Duration,

    #[arg(long, env = "DEFGUARD_ENROLLMENT_URL", value_parser = Url::parse, default_value = "http://localhost:8080")]
    pub enrollment_url: Url,

    #[arg(long, env = "DEFGUARD_ENROLLMENT_TOKEN_TIMEOUT", default_value = "24h")]
    #[serde(skip_serializing)]
    pub enrollment_token_timeout: Duration,

    #[arg(
        long,
        env = "DEFGUARD_ENROLLMENT_SESSION_TIMEOUT",
        default_value = "10m"
    )]
    #[serde(skip_serializing)]
    pub enrollment_session_timeout: Duration,

    #[arg(long, env = "DEFGUARD_COOKIE_DOMAIN")]
    pub cookie_domain: Option<String>,

    #[arg(long, env = "DEFGUARD_COOKIE_INSECURE")]
    pub cookie_insecure: bool,

    #[arg(
        long,
        env = "DEFGUARD_GATEWAY_DISCONNECTION_NOTIFICATION_TIMEOUT",
        default_value = "10m"
    )]
    #[serde(skip_serializing)]
    pub gateway_disconnection_notification_timeout: Duration,

    #[command(subcommand)]
    #[serde(skip_serializing)]
    pub cmd: Option<Command>,
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
        let mut config = Self::parse();
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
