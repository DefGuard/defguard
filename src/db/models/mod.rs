pub mod device;
pub mod error;
pub mod group;
pub mod session;
pub mod settings;
pub mod user;
pub mod wallet;
pub mod webauthn;
pub mod webhook;
pub mod wireguard;

use super::DbPool;
use crate::enterprise::db::openid::AuthorizedApp;
use device::Device;
use sqlx::Error as SqlxError;
use user::{MFAMethod, User};

#[derive(Deserialize, Serialize)]
pub struct WalletInfo {
    pub address: String,
    pub name: String,
    pub chain_id: i64,
    pub use_for_mfa: bool,
}

/// Only `id` and `name` from [`WebAuthn`].
#[derive(Deserialize, Serialize)]
pub struct SecurityKey {
    pub id: i64,
    pub name: String,
}

// FIXME: [`UserInfo`] does not belong here.
#[derive(Deserialize, Serialize)]
pub struct UserInfo {
    pub username: String,
    pub last_name: String,
    pub first_name: String,
    pub email: String,
    pub phone: Option<String>,
    pub ssh_key: Option<String>,
    pub pgp_key: Option<String>,
    pub pgp_cert_id: Option<String>,
    #[serde(default)]
    pub groups: Vec<String>,
    #[serde(default)]
    pub devices: Vec<Device>,
    #[serde(default)]
    pub authorized_apps: Vec<AuthorizedApp>,
    #[serde(default)]
    pub wallets: Vec<WalletInfo>,
    #[serde(default)]
    pub security_keys: Vec<SecurityKey>,
    pub mfa_method: MFAMethod,
}

impl UserInfo {
    pub async fn from_user(pool: &DbPool, user: User) -> Result<Self, SqlxError> {
        let groups = user.member_of(pool).await?;
        let devices = user.devices(pool).await?;
        let authorized_apps = AuthorizedApp::all_for_user(pool, user.id.unwrap()).await?;
        let wallets = user.wallets(pool).await?;
        let security_keys = user.security_keys(pool).await?;
        Ok(Self {
            username: user.username,
            last_name: user.last_name,
            first_name: user.first_name,
            email: user.email,
            phone: user.phone,
            ssh_key: user.ssh_key,
            pgp_key: user.pgp_key,
            pgp_cert_id: user.pgp_cert_id,
            groups,
            devices,
            authorized_apps,
            wallets,
            security_keys,
            mfa_method: user.mfa_method,
        })
    }

    pub fn into_user(self, user: &mut User) {
        user.username = self.username;
        user.last_name = self.last_name;
        user.first_name = self.first_name;
        user.email = self.email;
        user.phone = self.phone;
        user.ssh_key = self.ssh_key;
        user.pgp_key = self.pgp_key;
        user.pgp_cert_id = self.pgp_cert_id;
        user.mfa_method = self.mfa_method;
    }
}
