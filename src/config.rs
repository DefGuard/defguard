use clap::Parser;
use openidconnect::{core::CoreRsaPrivateSigningKey, JsonWebKeyId};
use reqwest::Url;
use rsa::{pkcs1::EncodeRsaPrivateKey, pkcs8::DecodePrivateKey, PublicKeyParts, RsaPrivateKey};

#[derive(Clone, Parser)]
pub struct DefGuardConfig {
    #[clap(long, env = "DEFGUARD_LOG_LEVEL", default_value = "info")]
    pub log_level: String,

    #[clap(long, env = "DEFGUARD_DB_HOST", default_value = "localhost")]
    pub database_host: String,

    #[clap(long, env = "DEFGUARD_DB_PORT", default_value_t = 5432)]
    pub database_port: u16,

    #[clap(long, env = "DEFGUARD_DB_NAME", default_value = "defguard")]
    pub database_name: String,

    #[clap(long, env = "DEFGUARD_DB_USER", default_value = "defguard")]
    pub database_user: String,

    #[clap(long, env = "DEFGUARD_DB_PASSWORD", default_value = "")]
    pub database_password: String,

    #[clap(long, env = "DEFGUARD_HTTP_PORT", default_value_t = 8000)]
    pub http_port: u16,

    #[clap(long, env = "DEFGUARD_GRPC_PORT", default_value_t = 50055)]
    pub grpc_port: u16,

    #[clap(long, env = "DEFGUARD_GRPC_CERT")]
    pub grpc_cert: Option<String>,

    #[clap(long, env = "DEFGUARD_GRPC_KEY")]
    pub grpc_key: Option<String>,

    #[clap(long, env = "DEFGUARD_ADMIN_GROUPNAME", default_value = "admin")]
    pub admin_groupname: String,

    #[clap(long, env = "DEFGUARD_OPENID_KEY", value_parser = Self::parse_openid_key)]
    pub openid_signing_key: Option<RsaPrivateKey>,

    // relying party id and relying party origin for WebAuthn
    #[clap(long, env = "DEFGUARD_WEBAUTHN_RP_ID", default_value = "localhost")]
    pub webauthn_rp_id: String,
    #[clap(long, env = "DEFGUARD_URL", value_parser = Url::parse, default_value = "http://localhost:8000")]
    pub url: Url,

    #[clap(
        long,
        env = "DEFGUARD_LDAP_URL",
        default_value = "ldap://localhost:389"
    )]
    pub ldap_url: String,

    #[clap(
        long,
        env = "DEFGUARD_LDAP_BIND_USERNAME",
        default_value = "cn=admin,dc=example,dc=org"
    )]
    pub ldap_bind_username: String,

    #[clap(long, env = "DEFGUARD_LDAP_BIND_PASSWORD", default_value = "")]
    pub ldap_bind_password: String,

    #[clap(
        long,
        env = "DEFGUARD_LDAP_USER_SEARCH_BASE",
        default_value = "ou=users,dc=example,dc=org"
    )]
    pub ldap_user_search_base: String,

    #[clap(
        long,
        env = "DEFGUARD_LDAP_GROUP_SEARCH_BASE",
        default_value = "ou=groups,dc=example,dc=org"
    )]
    pub ldap_group_search_base: String,

    #[clap(
        long,
        env = "DEFGUARD_LDAP_USER_OBJ_CLASS",
        default_value = "inetOrgPerson"
    )]
    pub ldap_user_obj_class: String,

    #[clap(
        long,
        env = "DEFGUARD_LDAP_GROUP_OBJ_CLASS",
        default_value = "groupOfUniqueNames"
    )]
    pub ldap_group_obj_class: String,

    #[clap(long, env = "DEFGUARD_LDAP_USERNAME_ATTR", default_value = "cn")]
    pub ldap_username_attr: String,

    #[clap(long, env = "DEFGUARD_LDAP_GROUPNAME_ATTR", default_value = "cn")]
    pub ldap_groupname_attr: String,

    #[clap(long, env = "DEFGUARD_LDAP_MEMBER_ATTR", default_value = "memberOf")]
    pub ldap_member_attr: String,

    #[clap(long, env = "DEFGUARD_LICENSE", default_value = "")]
    pub license: String,

    #[clap(
        long,
        env = "DEFGUARD_LDAP_GROUP_MEMBER_ATTR",
        default_value = "uniqueMember"
    )]
    pub ldap_group_member_attr: String,

    #[clap(subcommand)]
    pub cmd: Option<Command>,
}

#[derive(Clone, Parser)]
pub enum Command {
    #[clap(
        about = "Initialize development environment. Inserts test network and device into database."
    )]
    InitDevEnv,
}

impl DefGuardConfig {
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
