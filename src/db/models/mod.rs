#[cfg(feature = "openid")]
pub mod auth_code;
pub mod authentication_key;
pub mod device;
pub mod device_login;
pub mod enrollment;
pub mod error;
pub mod group;
#[cfg(feature = "openid")]
pub mod oauth2authorizedapp;
#[cfg(feature = "openid")]
pub mod oauth2client;
#[cfg(feature = "openid")]
pub mod oauth2token;
pub mod session;
pub mod settings;
pub mod user;
pub mod wallet;
pub mod webauthn;
pub mod webhook;
pub mod wireguard;
pub mod yubikey;

use sqlx::{query_as, Error as SqlxError, PgConnection};

use self::{
    device::UserDevice,
    user::{MFAMethod, User},
};
use super::{DbPool, Group};

#[cfg(feature = "openid")]
#[derive(Deserialize, Serialize)]
pub struct NewOpenIDClient {
    pub name: String,
    pub redirect_uri: Vec<String>,
    pub scope: Vec<String>,
    pub enabled: bool,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct WalletInfo {
    pub address: String,
    pub name: String,
    pub chain_id: i64,
    pub use_for_mfa: bool,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct OAuth2AuthorizedAppInfo {
    pub oauth2client_id: i64,
    pub user_id: i64,
    pub oauth2client_name: String,
}

/// Only `id` and `name` from [`WebAuthn`].
#[derive(Deserialize, Serialize, Debug)]
pub struct SecurityKey {
    pub id: i64,
    pub name: String,
}

// Basic user info used in user list, etc.
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct UserInfo {
    pub id: Option<i64>,
    pub username: String,
    pub last_name: String,
    pub first_name: String,
    pub email: String,
    pub phone: Option<String>,
    pub mfa_enabled: bool,
    pub totp_enabled: bool,
    pub email_mfa_enabled: bool,
    pub groups: Vec<String>,
    pub mfa_method: MFAMethod,
    pub authorized_apps: Vec<OAuth2AuthorizedAppInfo>,
    pub is_active: bool,
    pub enrolled: bool,
}

impl UserInfo {
    pub async fn from_user(pool: &DbPool, user: &User) -> Result<Self, SqlxError> {
        let groups = user.member_of_names(pool).await?;
        let authorized_apps = user.oauth2authorizedapps(pool).await?;

        Ok(Self {
            id: user.id,
            username: user.username.clone(),
            last_name: user.last_name.clone(),
            first_name: user.first_name.clone(),
            email: user.email.clone(),
            phone: user.phone.clone(),
            mfa_enabled: user.mfa_enabled,
            totp_enabled: user.totp_enabled,
            email_mfa_enabled: user.email_mfa_enabled,
            groups,
            mfa_method: user.mfa_method.clone(),
            authorized_apps,
            is_active: user.is_active,
            enrolled: user.has_password(),
        })
    }

    /// Copy status to [`User`]. This function should be used by administrators.
    ///
    /// Return `true` if status was changed, `false` otherwise.
    pub(crate) async fn handle_status_change(
        &self,
        transaction: &mut PgConnection,
        user: &mut User,
    ) -> Result<bool, SqlxError> {
        if self.is_active != user.is_active {
            user.is_active = self.is_active;
            user.save(&mut *transaction).await?;

            if !user.is_active {
                user.logout_all_sessions(&mut *transaction).await?;
            }

            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Copy groups to [`User`]. This function should be used by administrators.
    ///
    /// Return `true` if groups were changed, `false` otherwise.
    pub(crate) async fn handle_user_groups(
        &mut self,
        transaction: &mut PgConnection,
        user: &mut User,
    ) -> Result<bool, SqlxError> {
        // initialize return value
        let mut groups_changed = false;

        // handle groups
        let mut present_groups = user.member_of(&mut *transaction).await?;

        // add to groups if not already a member
        for groupname in &self.groups {
            match present_groups
                .iter()
                .position(|group| &group.name == groupname)
            {
                Some(index) => {
                    present_groups.swap_remove(index);
                }
                None => {
                    if let Some(group) = Group::find_by_name(&mut *transaction, groupname).await? {
                        user.add_to_group(&mut *transaction, &group).await?;
                        groups_changed = true;
                    }
                }
            }
        }

        // remove from remaining groups
        for group in present_groups {
            user.remove_from_group(&mut *transaction, &group).await?;
            groups_changed = true;
        }

        Ok(groups_changed)
    }

    /// Copy fields to [`User`]. This function is safe to call by a non-admin user.
    pub fn into_user_safe_fields(self, user: &mut User) -> Result<(), SqlxError> {
        user.phone = self.phone;
        user.mfa_method = self.mfa_method;
        Ok(())
    }

    /// Copy fields to [`User`]. This function should be used by administrators.
    pub fn into_user_all_fields(self, user: &mut User) -> Result<(), SqlxError> {
        user.phone = self.phone;
        user.username = self.username;
        user.last_name = self.last_name;
        user.first_name = self.first_name;
        user.email = self.email;

        Ok(())
    }
}

// Full user info with related objects
#[derive(Deserialize, Serialize, Debug)]
pub struct UserDetails {
    pub user: UserInfo,
    #[serde(default)]
    pub devices: Vec<UserDevice>,
    #[serde(default)]
    pub wallets: Vec<WalletInfo>,
    #[serde(default)]
    pub security_keys: Vec<SecurityKey>,
}

impl UserDetails {
    pub async fn from_user(pool: &DbPool, user: &User) -> Result<Self, SqlxError> {
        let devices = user.devices(pool).await?;
        let wallets = user.wallets(pool).await?;
        let security_keys = user.security_keys(pool).await?;

        Ok(Self {
            user: UserInfo::from_user(pool, user).await?,
            devices,
            wallets,
            security_keys,
        })
    }
}

#[derive(Deserialize, Serialize)]
pub struct MFAInfo {
    mfa_method: MFAMethod,
    totp_available: bool,
    web3_available: bool,
    webauthn_available: bool,
    email_available: bool,
}

impl MFAInfo {
    pub async fn for_user(pool: &DbPool, user: &User) -> Result<Option<Self>, SqlxError> {
        if let Some(id) = user.id {
            query_as!(
                Self,
                "SELECT mfa_method \"mfa_method: _\", totp_enabled totp_available, email_mfa_enabled email_available, \
                (SELECT count(*) > 0 FROM wallet WHERE user_id = $1 AND wallet.use_for_mfa) \"web3_available!\", \
                (SELECT count(*) > 0 FROM webauthn WHERE user_id = $1) \"webauthn_available!\" \
                FROM \"user\" WHERE \"user\".id = $1",
                id
            ).fetch_optional(pool).await
        } else {
            Ok(None)
        }
    }

    #[must_use]
    pub fn mfa_available(&self) -> bool {
        self.webauthn_available
            || self.totp_available
            || self.web3_available
            || self.email_available
    }

    #[must_use]
    pub fn current_mfa_method(&self) -> &MFAMethod {
        &self.mfa_method
    }

    #[must_use]
    pub fn list_available_methods(&self) -> Option<Vec<MFAMethod>> {
        if !self.mfa_available() {
            return None;
        }

        let mut methods = Vec::new();
        if self.webauthn_available {
            methods.push(MFAMethod::Webauthn);
        }
        if self.web3_available {
            methods.push(MFAMethod::Web3);
        }
        if self.totp_available {
            methods.push(MFAMethod::OneTimePassword);
        }
        if self.email_available {
            methods.push(MFAMethod::Email);
        }
        Some(methods)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[sqlx::test]
    async fn test_user_info(pool: DbPool) {
        let mut user = User::new(
            "hpotter",
            Some("pass123"),
            "Potter",
            "Harry",
            "h.potter@hogwart.edu.uk",
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

        let mut user_info = UserInfo::from_user(&pool, &user).await.unwrap();
        assert_eq!(user_info.groups, ["Gryffindor", "Hufflepuff"]);

        user_info.groups = vec!["Gryffindor".into(), "Ravenclaw".into()];
        let mut user = User::find_by_username(&pool, "hpotter")
            .await
            .unwrap()
            .unwrap();

        let mut transaction = pool.begin().await.unwrap();
        user_info
            .handle_user_groups(&mut transaction, &mut user)
            .await
            .unwrap();
        user_info.into_user_all_fields(&mut user).unwrap();
        transaction.commit().await.unwrap();

        assert_eq!(group1.member_usernames(&pool).await.unwrap(), ["hpotter"]);
        assert_eq!(group3.member_usernames(&pool).await.unwrap(), ["hpotter"]);
        assert!(group2.member_usernames(&pool).await.unwrap().is_empty());
        assert!(group4.member_usernames(&pool).await.unwrap().is_empty());
    }
}
