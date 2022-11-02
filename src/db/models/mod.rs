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

use super::{DbPool, Group};
#[cfg(feature = "openid")]
use crate::enterprise::db::openid::AuthorizedApp;
use device::Device;
use sqlx::{query_as, Error as SqlxError};
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
    pub mfa_enabled: bool,
    pub totp_enabled: bool,
    pub groups: Vec<String>,
    #[serde(default)]
    pub devices: Vec<Device>,
    #[cfg(feature = "openid")]
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
        let wallets = user.wallets(pool).await?;
        let security_keys = user.security_keys(pool).await?;

        #[cfg(feature = "openid")]
        let authorized_apps = AuthorizedApp::all_for_user(pool, &user).await?;

        let mfa_enabled = user.mfa_enabled(pool).await?;

        Ok(Self {
            username: user.username,
            last_name: user.last_name,
            first_name: user.first_name,
            email: user.email,
            phone: user.phone,
            ssh_key: user.ssh_key,
            pgp_key: user.pgp_key,
            pgp_cert_id: user.pgp_cert_id,
            mfa_enabled,
            totp_enabled: user.totp_enabled,
            groups,
            devices,
            #[cfg(feature = "openid")]
            authorized_apps,
            wallets,
            security_keys,
            mfa_method: user.mfa_method,
        })
    }

    /// Copy groups to [`User`]. This function should be used by administrators.
    async fn handle_user_groups(
        &mut self,
        pool: &DbPool,
        user: &mut User,
    ) -> Result<(), SqlxError> {
        // handle groups
        let mut present_groups = user.member_of(pool).await?;

        // add to groups if not already a member
        for groupname in &self.groups {
            match present_groups.iter().position(|name| name == groupname) {
                Some(index) => {
                    present_groups.swap_remove(index);
                }
                None => {
                    if let Some(group) = Group::find_by_name(pool, groupname).await? {
                        user.add_to_group(pool, &group).await?;
                    }
                }
            }
        }

        // remove from remaining groups
        for groupname in present_groups {
            if let Some(group) = Group::find_by_name(pool, &groupname).await? {
                user.remove_from_group(pool, &group).await?;
            }
        }

        Ok(())
    }

    /// Copy authorized apps to [`User`]. This function is safe to call by a non-admin user.
    #[cfg(feature = "openid")]
    async fn handle_user_authorized_apps(
        &mut self,
        pool: &DbPool,
        user: &mut User,
    ) -> Result<(), SqlxError> {
        let mut present_apps = AuthorizedApp::all_for_user(pool, user).await?;

        // create applications that don't already exist
        for mut auth_app in &mut self.authorized_apps {
            match present_apps
                .iter()
                .position(|app| app.client_id == auth_app.client_id)
            {
                Some(index) => {
                    present_apps.swap_remove(index);
                }
                None => {
                    if let Some(id) = user.id {
                        auth_app.id = None;
                        auth_app.user_id = id;
                        auth_app.save(pool).await?;
                    }
                }
            }
        }

        // remove from remaining applications
        for app in present_apps {
            app.delete(pool).await?;
        }

        Ok(())
    }

    /// Copy fields to [`User`]. This function is safe to call by a non-admin user.
    pub async fn into_user_safe_fields(
        mut self,
        pool: &DbPool,
        user: &mut User,
    ) -> Result<(), SqlxError> {
        #[cfg(feature = "openid")]
        self.handle_user_authorized_apps(pool, user).await?;

        user.phone = self.phone;
        user.ssh_key = self.ssh_key;
        user.pgp_key = self.pgp_key;
        user.pgp_cert_id = self.pgp_cert_id;
        user.mfa_method = self.mfa_method;

        Ok(())
    }

    /// Copy fields to [`User`]. This function should be used by administrators.
    pub async fn into_user_all_fields(
        mut self,
        pool: &DbPool,
        user: &mut User,
    ) -> Result<(), SqlxError> {
        self.handle_user_groups(pool, user).await?;
        #[cfg(feature = "openid")]
        self.handle_user_authorized_apps(pool, user).await?;

        user.phone = self.phone;
        user.ssh_key = self.ssh_key;
        user.pgp_key = self.pgp_key;
        user.pgp_cert_id = self.pgp_cert_id;
        user.mfa_method = self.mfa_method;

        user.username = self.username;
        user.last_name = self.last_name;
        user.first_name = self.first_name;
        user.email = self.email;

        Ok(())
    }
}

#[derive(Deserialize, Serialize)]
pub struct MFAInfo {
    mfa_method: MFAMethod,
    totp_available: bool,
    web3_available: bool,
    webauthn_available: bool,
}

impl MFAInfo {
    pub async fn for_user(pool: &DbPool, user: &User) -> Result<Option<Self>, SqlxError> {
        if let Some(id) = user.id {
            query_as!(
                Self,
                "SELECT mfa_method \"mfa_method: _\", totp_enabled totp_available, \
                (SELECT count(*) > 0 FROM wallet WHERE user_id = $1 AND wallet.use_for_mfa) \"web3_available!\", \
                (SELECT count(*) > 0 FROM webauthn WHERE user_id = $1) \"webauthn_available!\" \
                FROM \"user\" WHERE \"user\".id = $1",
                id
            ).fetch_optional(pool).await
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[sqlx::test]
    async fn test_user_info(pool: DbPool) {
        let mut user = User::new(
            "hpotter".into(),
            "pass123",
            "Potter".into(),
            "Harry".into(),
            "h.potter@hogwart.edu.uk".into(),
            None,
        );
        user.save(&pool).await.unwrap();

        let mut group1 = Group::new("Gryffindor");
        group1.save(&pool).await.unwrap();
        let mut group2 = Group::new("Hufflepuff");
        group2.save(&pool).await.unwrap();
        let mut group3 = Group::new("Ravenclaw");
        group3.save(&pool).await.unwrap();
        let mut group4 = Group::new("Slytherin");
        group4.save(&pool).await.unwrap();

        user.add_to_group(&pool, &group1).await.unwrap();
        user.add_to_group(&pool, &group2).await.unwrap();

        let mut user_info = UserInfo::from_user(&pool, user).await.unwrap();
        assert_eq!(user_info.groups, ["Gryffindor", "Hufflepuff"]);

        user_info.groups = vec!["Gryffindor".into(), "Ravenclaw".into()];
        let mut user = User::find_by_username(&pool, "hpotter")
            .await
            .unwrap()
            .unwrap();
        user_info
            .into_user_all_fields(&pool, &mut user)
            .await
            .unwrap();

        assert_eq!(group1.member_usernames(&pool).await.unwrap(), ["hpotter"]);
        assert_eq!(group3.member_usernames(&pool).await.unwrap(), ["hpotter"]);
        assert!(group2.member_usernames(&pool).await.unwrap().is_empty());
        assert!(group4.member_usernames(&pool).await.unwrap().is_empty());
    }
}
