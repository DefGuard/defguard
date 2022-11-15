use clap::Parser;

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

    // relying party id and relying party origin for WebAuthn
    #[clap(long, env = "DEFGUARD_WEBAUTHN_RP_ID", default_value = "localhost")]
    pub webauthn_rp_id: String,
    #[clap(long, env = "DEFGUARD_URL", default_value = "http://localhost:8080")]
    pub url: String,

    #[clap(
        long,
        env = "DEFGUARD_LDAP_URL",
        default_value = "ldap://localhost:389"
    )]
    pub ldap_url: String,

    #[clap(
        long,
        env = "DEFGUARD_BIND_USERNAME",
        default_value = "dc=admin,dc=example,dc=org"
    )]
    pub ldap_bind_username: String,

    #[clap(long, env = "DEFGUARD_BIND_PASSWORD", default_value = "")]
    pub ldap_bind_password: String,

    #[clap(
        long,
        env = "DEFGUARD_LDAP_USER_SEARCH_BASE",
        default_value = "dc=example,dc=org"
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
    pub fn user_dn(&self, username: &str) -> String {
        format!(
            "{}={},{}",
            &self.ldap_username_attr, username, &self.ldap_user_search_base
        )
    }

    /// Constructs group distinguished name.
    pub fn group_dn(&self, groupname: &str) -> String {
        format!(
            "{}={},{}",
            &self.ldap_groupname_attr, groupname, &self.ldap_group_search_base
        )
    }
}
