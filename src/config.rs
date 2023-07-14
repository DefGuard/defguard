use clap::Parser;
use openidconnect::{core::CoreRsaPrivateSigningKey, JsonWebKeyId};
use reqwest::Url;
use rsa::{pkcs1::EncodeRsaPrivateKey, pkcs8::DecodePrivateKey, PublicKeyParts, RsaPrivateKey};

#[derive(Clone, Parser)]
#[command(version)]
pub struct DefGuardConfig {
    #[arg(long, env = "DEFGUARD_LOG_LEVEL", default_value = "info")]
    pub log_level: String,

    #[arg(long, env = "DEFGUARD_SESSION_LIFETIME")]
    pub session_lifetime: Option<i64>,

    #[arg(long, env = "DEFGUARD_SECRET_KEY", value_parser = validate_secret_key)]
    pub secret_key: String,

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
    pub default_admin_password: String,

    #[arg(long, env = "DEFGUARD_OPENID_KEY", value_parser = Self::parse_openid_key)]
    pub openid_signing_key: Option<RsaPrivateKey>,

    // relying party id and relying party origin for WebAuthn
    #[arg(long, env = "DEFGUARD_WEBAUTHN_RP_ID")]
    pub webauthn_rp_id: Option<String>,
    #[arg(long, env = "DEFGUARD_URL", value_parser = Url::parse, default_value = "http://localhost:8000")]
    pub url: Url,

    #[arg(long, env = "DEFGUARD_GRPC_URL", value_parser = Url::parse, default_value = "http://localhost:50055")]
    pub grpc_url: Url,

    #[arg(
        long,
        env = "DEFGUARD_LDAP_URL",
        default_value = "ldap://localhost:389"
    )]
    pub ldap_url: String,

    #[arg(
        long,
        env = "DEFGUARD_LDAP_BIND_USERNAME",
        default_value = "cn=admin,dc=example,dc=org"
    )]
    pub ldap_bind_username: String,

    #[arg(long, env = "DEFGUARD_LDAP_BIND_PASSWORD", default_value = "")]
    pub ldap_bind_password: String,

    #[arg(
        long,
        env = "DEFGUARD_LDAP_USER_SEARCH_BASE",
        default_value = "ou=users,dc=example,dc=org"
    )]
    pub ldap_user_search_base: String,

    #[arg(
        long,
        env = "DEFGUARD_LDAP_GROUP_SEARCH_BASE",
        default_value = "ou=groups,dc=example,dc=org"
    )]
    pub ldap_group_search_base: String,

    #[arg(
        long,
        env = "DEFGUARD_LDAP_USER_OBJ_CLASS",
        default_value = "inetOrgPerson"
    )]
    pub ldap_user_obj_class: String,

    #[arg(
        long,
        env = "DEFGUARD_LDAP_GROUP_OBJ_CLASS",
        default_value = "groupOfUniqueNames"
    )]
    pub ldap_group_obj_class: String,

    #[arg(long, env = "DEFGUARD_LDAP_USERNAME_ATTR", default_value = "cn")]
    pub ldap_username_attr: String,

    #[arg(long, env = "DEFGUARD_LDAP_GROUPNAME_ATTR", default_value = "cn")]
    pub ldap_groupname_attr: String,

    #[arg(long, env = "DEFGUARD_LDAP_MEMBER_ATTR", default_value = "memberOf")]
    pub ldap_member_attr: String,

    #[arg(long, env = "DEFGUARD_LICENSE", default_value = "")]
    pub license: String,

    #[arg(
        long,
        env = "DEFGUARD_LDAP_GROUP_MEMBER_ATTR",
        default_value = "uniqueMember"
    )]
    pub ldap_group_member_attr: String,

    #[command(subcommand)]
    pub cmd: Option<Command>,
}

#[derive(Clone, Parser)]
pub enum Command {
    #[command(
        about = "Initialize development environment. Inserts test network and device into database."
    )]
    InitDevEnv,
}

fn validate_secret_key(secret_key: &str) -> Result<String, String> {
    if secret_key.trim().len() != secret_key.len() {
        return Err(String::from(
            "SECRET_KEY cannot have leading and trailing space",
        ));
    }

    if secret_key.len() < 64 {
        return Err(format!(
            "SECRET_KEY must be at least 64 characters long, provided value has {} characters",
            secret_key.len()
        ));
    }

    Ok(secret_key.into())
}

impl DefGuardConfig {
    pub fn new() -> Self {
        let mut config = Self::parse();
        config.validate_rp_id();
        config
    }

    // this is an ugly workaround to avoid `cargo test` args being captured by `clap`
    pub fn new_test_config() -> Self {
        let mut config = Self::parse_from::<[_; 0], String>([]);
        config.validate_rp_id();
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
            )
        }
    }

    /// Constructs user distinguished name.
    #[must_use]
    pub fn user_dn(&self, username: &str) -> String {
        format!(
            "{}={},{}",
            &self.ldap_username_attr, username, &self.ldap_user_search_base
        )
    }

    /// Constructs group distinguished name.
    #[must_use]
    pub fn group_dn(&self, groupname: &str) -> String {
        format!(
            "{}={},{}",
            &self.ldap_groupname_attr, groupname, &self.ldap_group_search_base
        )
    }

    fn parse_openid_key(path: &str) -> Result<RsaPrivateKey, rsa::pkcs8::Error> {
        RsaPrivateKey::read_pkcs8_pem_file(path)
    }

    pub fn openid_key(&self) -> Option<CoreRsaPrivateSigningKey> {
        let key = self.openid_signing_key.as_ref()?;
        if let Ok(pem) = key.to_pkcs1_pem(rsa::pkcs8::LineEnding::default()) {
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
    use super::*;
    use std::env;

    #[test]
    fn verify_cli() {
        use clap::CommandFactory;
        DefGuardConfig::command().debug_assert()
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
}
