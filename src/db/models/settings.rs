use crate::DbPool;
use model_derive::Model;

#[derive(Model, Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct Settings {
    #[serde(skip)]
    pub id: Option<i64>,
    pub web3_enabled: bool,
    pub openid_enabled: bool,
    pub oauth_enabled: bool,
    pub ldap_enabled: bool,
    pub wireguard_enabled: bool,
    pub webhooks_enabled: bool,
    pub worker_enabled: bool,
    pub challenge_template: String,
}
