use model_derive::Model;

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
    pub smtp_tls: Option<bool>,
    pub smtp_user: Option<String>,
    pub smtp_password: Option<String>,
}
