use std::{fs::read_to_string, io, net::IpAddr, sync::OnceLock};

use clap::{Args, Parser, Subcommand};
use humantime::Duration;
use ipnetwork::IpNetwork;
use openidconnect::{JsonWebKeyId, core::CoreRsaPrivateSigningKey};
use reqwest::Url;
use rsa::{
    RsaPrivateKey,
    pkcs1::{DecodeRsaPrivateKey, EncodeRsaPrivateKey},
    pkcs8::{DecodePrivateKey, LineEnding},
    traits::PublicKeyParts,
};
use secrecy::{ExposeSecret, SecretString};
use serde::Serialize;
use tonic::transport::{Certificate, ClientTlsConfig, Identity};

use crate::db::models::Settings;

pub static SERVER_CONFIG: OnceLock<DefGuardConfig> = OnceLock::new();

pub fn server_config() -> &'static DefGuardConfig {
    SERVER_CONFIG
        .get()
        .expect("Server configuration not set yet")
}

#[derive(Clone, Parser, Serialize, Debug)]
#[command(version)]
// TODO: find a better workaround for clap not
// working nice with test args
#[cfg_attr(test, command(ignore_errors(true)))]
pub struct DefGuardConfig {
    #[arg(long, env = "DEFGUARD_LOG_LEVEL", default_value = "info")]
    pub log_level: String,

    // TODO: restore file logging, seems to have vanished during the switch to tracing
    #[arg(long, env = "DEFGUARD_LOG_FILE")]
    pub log_file: Option<String>,

    #[arg(long, env = "DEFGUARD_AUTH_COOKIE_TIMEOUT", default_value = "7d")]
    #[serde(skip_serializing)]
    pub auth_cookie_timeout: Duration,

    #[arg(long, env = "DEFGUARD_SECRET_KEY")]
    #[serde(skip_serializing)]
    pub secret_key: SecretString,

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

    // Certificate authority (CA), certificate, and key for gRPC communication over HTTPS.
    #[arg(long, env = "DEFGUARD_GRPC_CA")]
    pub grpc_ca: Option<String>,
    #[arg(long, env = "DEFGUARD_GRPC_CERT")]
    pub grpc_cert: Option<String>,
    #[arg(long, env = "DEFGUARD_GRPC_KEY")]
    pub grpc_key: Option<String>,

    #[arg(
        long,
        env = "DEFGUARD_DEFAULT_ADMIN_PASSWORD",
        default_value = "pass123"
    )]
    #[serde(skip_serializing)]
    // TODO: Deprecate this, since we have initial setup now.
    // We use it in some dev/test scenarios still so the approach will need to be changed there.
    pub default_admin_password: SecretString,

    #[arg(long, env = "DEFGUARD_OPENID_KEY", value_parser = Self::parse_openid_key)]
    #[serde(skip_serializing)]
    pub openid_signing_key: Option<RsaPrivateKey>,

    // relying party id and relying party origin for WebAuthn
    #[arg(long, env = "DEFGUARD_WEBAUTHN_RP_ID")]
    pub webauthn_rp_id: Option<String>,
    #[arg(long, env = "DEFGUARD_URL", value_parser = Url::parse, default_value = "http://localhost:8000")]
    #[deprecated(since = "2.0.0", note = "Use Settings.defguard_url instead")]
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
    #[deprecated(since = "2.0.0", note = "Use Settings.public_proxy_url instead")]
    pub enrollment_url: Url,

    #[arg(long, env = "DEFGUARD_ENROLLMENT_TOKEN_TIMEOUT", default_value = "24h")]
    #[serde(skip_serializing)]
    pub enrollment_token_timeout: Duration,

    #[arg(long, env = "DEFGUARD_MFA_CODE_TIMEOUT", default_value = "60s")]
    #[serde(skip_serializing)]
    #[deprecated(
        since = "2.0.0",
        note = "Use Settings.default_mfa_code_lifetime instead"
    )]
    pub mfa_code_timeout: Duration,

    #[arg(long, env = "DEFGUARD_SESSION_TIMEOUT", default_value = "7d")]
    #[serde(skip_serializing)]
    #[deprecated(since = "2.0.0", note = "Use Settings.default_authentication instead")]
    pub session_timeout: Duration,

    #[arg(
        long,
        env = "DEFGUARD_PASSWORD_RESET_TOKEN_TIMEOUT",
        default_value = "24h"
    )]
    #[serde(skip_serializing)]
    pub password_reset_token_timeout: Duration,

    #[arg(
        long,
        env = "DEFGUARD_ENROLLMENT_SESSION_TIMEOUT",
        default_value = "10m"
    )]
    #[serde(skip_serializing)]
    pub enrollment_session_timeout: Duration,

    #[arg(
        long,
        env = "DEFGUARD_PASSWORD_RESET_SESSION_TIMEOUT",
        default_value = "10m"
    )]
    #[serde(skip_serializing)]
    pub password_reset_session_timeout: Duration,

    #[arg(long, env = "DEFGUARD_COOKIE_DOMAIN")]
    pub cookie_domain: Option<String>,

    #[arg(long, env = "DEFGUARD_COOKIE_INSECURE")]
    pub cookie_insecure: bool,

    // path to certificate `.pem` file used if connecting to proxy over HTTPS
    #[arg(long, env = "DEFGUARD_PROXY_GRPC_CA")]
    pub proxy_grpc_ca: Option<String>,

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
        config.validate_secret_key();
        config
    }

    // this is an ugly workaround to avoid `cargo test` args being captured by `clap`
    #[must_use]
    pub fn new_test_config() -> Self {
        Self::parse_from::<[_; 0], String>([])
    }

    /// Initialize values that depend on Settings.
    pub fn initialize_post_settings(&mut self) {
        let url = Settings::url().expect("Unable to parse Defguard URL.");
        self.initialize_rp_id(&url);
        self.initialize_cookie_domain(&url);
    }

    fn initialize_rp_id(&mut self, url: &Url) {
        if self.webauthn_rp_id.is_none() {
            self.webauthn_rp_id = Some(
                url.domain()
                    .expect("Unable to get domain for server URL.")
                    .to_string(),
            );
        }
    }

    fn initialize_cookie_domain(&mut self, url: &Url) {
        if self.cookie_domain.is_none() {
            self.cookie_domain = Some(
                url.domain()
                    .expect("Unable to get domain for server URL.")
                    .to_string(),
            );
        }
    }

    fn validate_secret_key(&self) {
        let secret_key = self.secret_key.expose_secret();
        assert!(
            secret_key.trim().len() == secret_key.len(),
            "SECRET_KEY cannot have leading and trailing space",
        );

        assert!(
            secret_key.len() >= 64,
            "SECRET_KEY must be at least 64 characters long, provided value has {} characters",
            secret_key.len()
        );
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

    /// Provide [`ClientTlsConfig`] from paths to cerfiticate, key, and cerfiticate authority (CA).
    pub fn grpc_client_tls_config(&self) -> Result<Option<ClientTlsConfig>, io::Error> {
        if self.grpc_ca.is_none() && (self.grpc_cert.is_none() || self.grpc_key.is_none()) {
            return Ok(None);
        }
        let mut tls = ClientTlsConfig::new();
        if let (Some(cert_path), Some(key_path)) = (&self.grpc_cert, &self.grpc_key) {
            let cert = read_to_string(cert_path)?;
            let key = read_to_string(key_path)?;
            tls = tls.identity(Identity::from_pem(cert, key));
        }
        if let Some(ca_path) = &self.grpc_ca {
            let ca = read_to_string(ca_path)?;
            tls = tls.ca_certificate(Certificate::from_pem(ca));
        }

        Ok(Some(tls))
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
        unsafe {
            env::remove_var("DEFGUARD_WEBAUTHN_RP_ID");
        }

        let url = Url::parse("https://defguard.example.com").unwrap();
        let mut config = DefGuardConfig::new();
        config.initialize_rp_id(&url);

        assert_eq!(
            config.webauthn_rp_id,
            Some("defguard.example.com".to_string())
        );

        unsafe {
            env::set_var("DEFGUARD_WEBAUTHN_RP_ID", "example.com");
        }

        let config = DefGuardConfig::new();

        assert_eq!(config.webauthn_rp_id, Some("example.com".to_string()));
    }

    #[test]
    fn test_generate_cookie_domain() {
        unsafe {
            env::remove_var("DEFGUARD_COOKIE_DOMAIN");
        }

        let url = Url::parse("https://defguard.example.com").unwrap();
        let mut config = DefGuardConfig::new();
        config.initialize_cookie_domain(&url);

        assert_eq!(
            config.cookie_domain,
            Some("defguard.example.com".to_string())
        );

        unsafe {
            env::set_var("DEFGUARD_COOKIE_DOMAIN", "example.com");
        }

        let config = DefGuardConfig::new();

        assert_eq!(config.cookie_domain, Some("example.com".to_string()));
    }
}
