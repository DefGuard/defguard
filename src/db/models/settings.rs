use model_derive::Model;
use sqlx::Type;

#[derive(Clone, Deserialize, Serialize, PartialEq, Eq, Type, Debug)]
#[sqlx(type_name = "smtp_encryption", rename_all = "lowercase")]
pub enum SmtpEncryption {
    None,
    StartTls,
    ImplicitTls,
}

#[derive(Model, Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct Settings {
    #[serde(skip)]
    pub id: Option<i64>,
    pub openid_enabled: bool,
    pub ldap_enabled: bool,
    pub wireguard_enabled: bool,
    pub webhooks_enabled: bool,
    pub worker_enabled: bool,
    pub challenge_template: String,
    pub instance_name: String,
    pub main_logo_url: String,
    pub nav_logo_url: String,
    pub smtp_server: Option<String>,
    pub smtp_port: Option<i32>,
    #[model(enum)]
    pub smtp_encryption: SmtpEncryption,
    pub smtp_user: Option<String>,
    pub smtp_password: Option<String>,
    pub smtp_sender: Option<String>,
}
