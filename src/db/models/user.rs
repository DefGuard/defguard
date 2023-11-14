use super::{
    device::{Device, UserDevice},
    group::Group,
    wallet::Wallet,
    webauthn::WebAuthn,
    DbPool, MFAInfo, OAuth2AuthorizedAppInfo, SecurityKey, WalletInfo,
};
use crate::{auth::TOTP_CODE_VALIDITY_PERIOD, error::WebError, random::gen_alphanumeric};
use argon2::{
    password_hash::{
        errors::Error as HashError, rand_core::OsRng, PasswordHash, PasswordHasher,
        PasswordVerifier, SaltString,
    },
    Argon2,
};
use axum::http::StatusCode;
use model_derive::Model;
use otpauth::TOTP;
use rand::{thread_rng, Rng};
use sqlx::{query, query_as, query_scalar, Error as SqlxError, Type};
use std::time::SystemTime;

const RECOVERY_CODES_COUNT: usize = 8;

#[derive(Clone, Deserialize, Serialize, PartialEq, Type, Debug)]
#[sqlx(type_name = "mfa_method", rename_all = "snake_case")]
pub enum MFAMethod {
    None,
    OneTimePassword,
    Webauthn,
    Web3,
    Email,
}

impl std::string::ToString for MFAMethod {
    fn to_string(&self) -> String {
        match self {
            MFAMethod::None => "None".into(),
            MFAMethod::OneTimePassword => "TOTP".into(),
            MFAMethod::Web3 => "Web3".into(),
            MFAMethod::Webauthn => "WebAuthn".into(),
            MFAMethod::Email => "Email".into(),
        }
    }
}

// User information ready to be sent as part of diagnostic data.
#[derive(Debug, Serialize)]
pub struct UserDiagnostic {
    pub id: i64,
    pub mfa_enabled: bool,
    pub totp_enabled: bool,
    pub mfa_method: MFAMethod,
    pub is_active: bool,
}

#[derive(Model, PartialEq, Serialize, Clone)]
pub struct User {
    pub id: Option<i64>,
    pub username: String,
    password_hash: Option<String>,
    pub last_name: String,
    pub first_name: String,
    pub email: String,
    pub phone: Option<String>,
    pub ssh_key: Option<String>,
    pub pgp_key: Option<String>,
    pub pgp_cert_id: Option<String>,
    pub mfa_enabled: bool,
    // secret has been verified and TOTP can be used
    pub(crate) totp_enabled: bool,
    totp_secret: Option<Vec<u8>>,
    #[model(enum)]
    pub(crate) mfa_method: MFAMethod,
    #[model(ref)]
    recovery_codes: Vec<String>,
}

impl User {
    fn hash_password(password: &str) -> Result<String, HashError> {
        let salt = SaltString::generate(&mut OsRng);
        Ok(Argon2::default()
            .hash_password(password.as_bytes(), &salt)?
            .to_string())
    }

    #[must_use]
    pub fn new(
        username: String,
        password: Option<&str>,
        last_name: String,
        first_name: String,
        email: String,
        phone: Option<String>,
    ) -> Self {
        let password_hash = password.map(|password_hash| {
            Self::hash_password(password_hash).expect("Failed to hash password")
        });
        Self {
            id: None,
            username,
            password_hash,
            last_name,
            first_name,
            email,
            phone,
            ssh_key: None,
            pgp_key: None,
            pgp_cert_id: None,
            mfa_enabled: false,
            totp_enabled: false,
            totp_secret: None,
            mfa_method: MFAMethod::None,
            recovery_codes: Vec::new(),
        }
    }

    pub fn set_password(&mut self, password: &str) {
        self.password_hash = Some(Self::hash_password(password).unwrap());
    }

    pub fn verify_password(&self, password: &str) -> Result<(), HashError> {
        match &self.password_hash {
            Some(hash) => {
                let parsed_hash = PasswordHash::new(hash)?;
                Argon2::default().verify_password(password.as_bytes(), &parsed_hash)
            }
            None => {
                error!("Password not set for user {}", self.username);
                Err(HashError::Password)
            }
        }
    }

    #[must_use]
    pub fn has_password(&self) -> bool {
        self.password_hash.is_some()
    }

    #[must_use]
    pub fn name(&self) -> String {
        format!("{} {}", self.first_name, self.last_name)
    }

    /// Generate new `secret`, save it, then return it as RFC 4648 base32-encoded string.
    pub async fn new_secret(&mut self, pool: &DbPool) -> Result<String, SqlxError> {
        let secret = thread_rng().gen::<[u8; 20]>().to_vec();
        if let Some(id) = self.id {
            query!(
                "UPDATE \"user\" SET totp_secret = $1 WHERE id = $2",
                secret,
                id
            )
            .execute(pool)
            .await?;
        }
        let secret_base32 = TOTP::from_bytes(&secret).base32_secret();
        self.totp_secret = Some(secret);
        Ok(secret_base32)
    }

    pub async fn set_mfa_method(
        &mut self,
        pool: &DbPool,
        mfa_method: MFAMethod,
    ) -> Result<(), SqlxError> {
        info!(
            "Setting MFA method for user {} to {:?}",
            self.username, mfa_method
        );
        if let Some(id) = self.id {
            query!(
                "UPDATE \"user\" SET mfa_method = $2 WHERE id = $1",
                id,
                &mfa_method as &MFAMethod
            )
            .execute(pool)
            .await?;
        }
        self.mfa_method = mfa_method;

        Ok(())
    }

    /// Check if any of the multi-factor authentication methods is on.
    /// - TOTP is enabled
    /// - a [`Wallet`] flagged `use_for_mfa`
    /// - a security key for Webauthn
    async fn check_mfa(&self, pool: &DbPool) -> Result<bool, SqlxError> {
        // short-cut
        if self.totp_enabled {
            return Ok(true);
        }

        if let Some(id) = self.id {
            query_scalar!(
                "SELECT totp_enabled OR coalesce(bool_or(wallet.use_for_mfa), FALSE) \
                OR count(webauthn.id) > 0 \"bool!\" FROM \"user\" \
                LEFT JOIN wallet ON wallet.user_id = \"user\".id \
                LEFT JOIN webauthn ON webauthn.user_id = \"user\".id \
                WHERE \"user\".id = $1 GROUP BY totp_enabled;",
                id
            )
            .fetch_one(pool)
            .await
        } else {
            Ok(false)
        }
    }

    /// Verify the state of mfa flags are correct.
    /// Recovers from invalid mfa_method
    /// Use this function after removing any of the authentication factors.
    pub async fn verify_mfa_state(&mut self, pool: &DbPool) -> Result<(), WebError> {
        if let Some(info) = MFAInfo::for_user(pool, self).await? {
            let factors_present = info.mfa_available();
            if self.mfa_enabled != factors_present {
                // store correct value for MFA flag in the DB
                if self.mfa_enabled {
                    // last factor was removed so we have to disable MFA
                    self.disable_mfa(pool).await?;
                } else {
                    // first factor was added so MFA needs to be enabled
                    if let Some(id) = self.id {
                        query!(
                            "UPDATE \"user\" SET mfa_enabled = $2 WHERE id = $1",
                            id,
                            factors_present
                        )
                        .execute(pool)
                        .await?;
                    }
                };

                if !factors_present && self.mfa_method != MFAMethod::None {
                    debug!(
                        "MFA for user {} disabled, updating MFA method to None",
                        self.username
                    );
                    self.set_mfa_method(pool, MFAMethod::None).await?;
                }

                self.mfa_enabled = factors_present;
            }

            // set correct value for default method
            if factors_present {
                match info.list_available_methods() {
                    None => {
                        error!("Incorrect MFA info state for user {}", self.username);
                        return Err(WebError::Http(StatusCode::INTERNAL_SERVER_ERROR));
                    }
                    Some(methods) => {
                        info!(
                            "Checking if {:?} in in available methods {:?}, {}",
                            info.mfa_method,
                            methods,
                            methods.contains(&info.mfa_method)
                        );
                        if !methods.contains(&info.mfa_method) {
                            self.set_mfa_method(pool, methods.into_iter().next().unwrap())
                                .await?;
                        }
                    }
                }
            }
        };
        Ok(())
    }

    /// Enable MFA. At least one of the authenticator factors must be configured.
    pub async fn enable_mfa(&mut self, pool: &DbPool) -> Result<(), WebError> {
        if !self.mfa_enabled {
            self.verify_mfa_state(pool).await?;
        }
        Ok(())
    }

    /// Get recovery codes. If recovery codes exist, this function returns `None`.
    /// That way recovery codes are returned only once - when MFA is turned on.
    pub async fn get_recovery_codes(
        &mut self,
        pool: &DbPool,
    ) -> Result<Option<Vec<String>>, SqlxError> {
        if !self.recovery_codes.is_empty() {
            return Ok(None);
        }

        for _ in 0..RECOVERY_CODES_COUNT {
            let code = gen_alphanumeric(16);
            self.recovery_codes.push(code);
        }
        if let Some(id) = self.id {
            query!(
                "UPDATE \"user\" SET recovery_codes = $2 WHERE id = $1",
                id,
                &self.recovery_codes
            )
            .execute(pool)
            .await?;
        }

        Ok(Some(self.recovery_codes.clone()))
    }

    /// Disable MFA; discard recovery codes, TOTP secret, and security keys.
    pub async fn disable_mfa(&mut self, pool: &DbPool) -> Result<(), SqlxError> {
        if let Some(id) = self.id {
            query!(
                "UPDATE \"user\" SET mfa_enabled = FALSE, mfa_method = 'none', totp_enabled = FALSE, \
                totp_secret = NULL, recovery_codes = '{}' WHERE id = $1",
                id
            )
            .execute(pool)
            .await?;
            Wallet::disable_mfa_for_user(pool, id).await?;
            WebAuthn::delete_all_for_user(pool, id).await?;
        }
        self.totp_secret = None;
        self.totp_enabled = false;
        self.mfa_method = MFAMethod::None;
        self.recovery_codes.clear();
        Ok(())
    }

    /// Enable TOTP
    pub async fn enable_totp(&mut self, pool: &DbPool) -> Result<(), SqlxError> {
        if !self.totp_enabled {
            if let Some(id) = self.id {
                query!("UPDATE \"user\" SET totp_enabled = TRUE WHERE id = $1", id)
                    .execute(pool)
                    .await?;
            }
            self.totp_enabled = false;
        }
        Ok(())
    }

    /// Disable TOTP; discard the secret.
    pub async fn disable_totp(&mut self, pool: &DbPool) -> Result<(), SqlxError> {
        if self.totp_enabled {
            self.mfa_enabled = self.check_mfa(pool).await?;
            self.totp_enabled = false;
            self.totp_secret = None;
            if let Some(id) = self.id {
                query!(
                    "UPDATE \"user\" SET mfa_enabled = $2, totp_enabled = $3 AND totp_secret = $4 \
                    WHERE id = $1",
                    id,
                    self.mfa_enabled,
                    self.totp_enabled,
                    self.totp_secret,
                )
                .execute(pool)
                .await?;
            }
        }
        Ok(())
    }
    /// Select all users without sensitive data.
    // FIXME: Remove it when Model macro will suport SecretString
    pub async fn all_without_sensitive_data(
        pool: &DbPool,
    ) -> Result<Vec<UserDiagnostic>, SqlxError> {
        let users = query!(
            r#"
            SELECT id, mfa_enabled, totp_enabled, mfa_method as "mfa_method: MFAMethod", password_hash FROM "user"
        "#
        )
        .fetch_all(pool)
        .await?;
        let res: Vec<UserDiagnostic> = users
            .iter()
            .map(|u| UserDiagnostic {
                mfa_method: u.mfa_method.clone(),
                totp_enabled: u.totp_enabled,
                mfa_enabled: u.mfa_enabled,
                id: u.id,
                is_active: u.password_hash.is_some(),
            })
            .collect();
        Ok(res)
    }

    /// Check if TOTP `code` is valid.
    #[must_use]
    pub fn verify_code(&self, code: u32) -> bool {
        if let Some(totp_secret) = &self.totp_secret {
            let totp = TOTP::from_bytes(totp_secret);
            if let Ok(timestamp) = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
                return totp.verify(code, TOTP_CODE_VALIDITY_PERIOD, timestamp.as_secs());
            }
        }
        false
    }

    /// Verify recovery code. If it is valid, consume it, so it can't be used again.
    pub async fn verify_recovery_code(
        &mut self,
        pool: &DbPool,
        code: &str,
    ) -> Result<bool, SqlxError> {
        if let Some(index) = self.recovery_codes.iter().position(|c| c == code) {
            // Note: swap_remove() should be faster than remove().
            self.recovery_codes.swap_remove(index);
            if let Some(id) = self.id {
                query!(
                    "UPDATE \"user\" SET recovery_codes = $2 WHERE id = $1",
                    id,
                    &self.recovery_codes
                )
                .execute(pool)
                .await?;
            }
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub async fn find_by_username(
        pool: &DbPool,
        username: &str,
    ) -> Result<Option<Self>, SqlxError> {
        query_as!(
            Self,
            "SELECT id \"id?\", username, password_hash, last_name, first_name, email, \
            phone, ssh_key, pgp_key, pgp_cert_id, mfa_enabled, totp_enabled, totp_secret, \
            mfa_method \"mfa_method: _\", recovery_codes \
            FROM \"user\" WHERE username = $1",
            username
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn member_of<'e, E>(&self, executor: E) -> Result<Vec<String>, SqlxError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        if let Some(id) = self.id {
            query_scalar!(
                "SELECT \"group\".name FROM \"group\" JOIN group_user ON \"group\".id = group_user.group_id \
                WHERE group_user.user_id = $1",
                id
            )
            .fetch_all(executor)
            .await
        } else {
            Ok(Vec::new())
        }
    }

    pub async fn devices(&self, pool: &DbPool) -> Result<Vec<UserDevice>, SqlxError> {
        if let Some(id) = self.id {
            let devices = query_as!(
                Device,
                r#"
                SELECT device.id "id?", name, wireguard_pubkey, user_id, created
                FROM device WHERE user_id = $1
                "#,
                id
            )
            .fetch_all(pool)
            .await?;

            let mut user_devices = Vec::new();
            for device in devices {
                if let Some(user_device) = UserDevice::from_device(pool, device).await? {
                    user_devices.push(user_device);
                }
            }
            Ok(user_devices)
        } else {
            Ok(Vec::new())
        }
    }

    pub async fn wallets(&self, pool: &DbPool) -> Result<Vec<WalletInfo>, SqlxError> {
        if let Some(id) = self.id {
            query_as!(
                WalletInfo,
                "SELECT address \"address!\", name, chain_id, use_for_mfa \
                FROM wallet WHERE user_id = $1 AND validation_timestamp IS NOT NULL",
                id
            )
            .fetch_all(pool)
            .await
        } else {
            Ok(Vec::new())
        }
    }

    pub async fn oauth2authorizedapps<'e, E>(
        &self,
        executor: E,
    ) -> Result<Vec<OAuth2AuthorizedAppInfo>, SqlxError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        if let Some(id) = self.id {
            query_as!(
                OAuth2AuthorizedAppInfo,
                "SELECT oauth2client.id \"oauth2client_id!\", oauth2client.name \"oauth2client_name\", \
                oauth2authorizedapp.user_id \"user_id\" \
                FROM oauth2authorizedapp \
                JOIN oauth2client ON oauth2client.id = oauth2authorizedapp.oauth2client_id \
                WHERE oauth2authorizedapp.user_id = $1",
                id
            )
            .fetch_all(executor)
            .await
        } else {
            Ok(Vec::new())
        }
    }

    pub async fn security_keys(&self, pool: &DbPool) -> Result<Vec<SecurityKey>, SqlxError> {
        if let Some(id) = self.id {
            query_as!(
                SecurityKey,
                "SELECT id \"id!\", name FROM webauthn WHERE user_id = $1",
                id
            )
            .fetch_all(pool)
            .await
        } else {
            Ok(Vec::new())
        }
    }

    pub async fn add_to_group<'e, E>(&self, executor: E, group: &Group) -> Result<(), SqlxError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        if let (Some(id), Some(group_id)) = (self.id, group.id) {
            query!(
                "INSERT INTO group_user (group_id, user_id) VALUES ($1, $2) \
                ON CONFLICT DO NOTHING",
                group_id,
                id
            )
            .execute(executor)
            .await?;
        }
        Ok(())
    }

    pub async fn remove_from_group<'e, E>(
        &self,
        executor: E,
        group: &Group,
    ) -> Result<(), SqlxError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        if let (Some(id), Some(group_id)) = (self.id, group.id) {
            query!(
                "DELETE FROM group_user WHERE group_id = $1 AND user_id = $2",
                group_id,
                id
            )
            .execute(executor)
            .await?;
        }
        Ok(())
    }

    // Remove authoirzed apps by their client id's from user
    pub async fn remove_oauth2_authorized_apps<'e, E>(
        &self,
        executor: E,
        app_client_ids: &Vec<i64>,
    ) -> Result<(), SqlxError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        if let Some(id) = self.id {
            query!(
                "DELETE FROM oauth2authorizedapp WHERE user_id = $1 AND oauth2client_id = ANY($2)",
                id,
                app_client_ids
            )
            .execute(executor)
            .await?;
        }
        Ok(())
    }

    /// Create admin user if one doesn't exist yet
    pub async fn init_admin_user(
        pool: &DbPool,
        default_admin_pass: &str,
    ) -> Result<(), anyhow::Error> {
        info!("Initializing admin user");
        let password_hash = Self::hash_password(default_admin_pass)?;

        // create admin user
        let result = query_scalar!(
            "INSERT INTO \"user\" (username, password_hash, last_name, first_name, email) \
            VALUES ('admin', $1, 'Administrator', 'DefGuard', 'admin@defguard') \
            ON CONFLICT DO NOTHING \
            RETURNING id",
            password_hash
        )
        .fetch_optional(pool)
        .await?;

        // if new user was created add them to admin group (ID 1)
        if let Some(new_user_id) = result {
            info!("New admin user was created, adding to Admin group...");
            query("INSERT INTO group_user (group_id, user_id) VALUES (1, $1)")
                .bind(new_user_id)
                .execute(pool)
                .await?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[sqlx::test]
    async fn test_user(pool: DbPool) {
        let mut user = User::new(
            "hpotter".into(),
            Some("pass123"),
            "Potter".into(),
            "Harry".into(),
            "h.potter@hogwart.edu.uk".into(),
            None,
        );
        user.save(&pool).await.unwrap();

        let fetched_user = User::find_by_username(&pool, "hpotter").await.unwrap();
        assert!(fetched_user.is_some());
        assert_eq!(fetched_user.unwrap().email, "h.potter@hogwart.edu.uk");

        user.email = "harry.potter@hogwart.edu.uk".into();
        user.save(&pool).await.unwrap();

        let fetched_user = User::find_by_username(&pool, "hpotter").await.unwrap();
        assert!(fetched_user.is_some());
        assert_eq!(fetched_user.unwrap().email, "harry.potter@hogwart.edu.uk");

        assert!(user.verify_password("pass123").is_ok());

        let fetched_user = User::find_by_username(&pool, "rweasley").await.unwrap();
        assert!(fetched_user.is_none());
    }

    #[sqlx::test]
    async fn test_all_users(pool: DbPool) {
        let mut harry = User::new(
            "hpotter".into(),
            Some("pass123"),
            "Potter".into(),
            "Harry".into(),
            "h.potter@hogwart.edu.uk".into(),
            None,
        );
        harry.save(&pool).await.unwrap();

        let mut albus = User::new(
            "adumbledore".into(),
            Some("magic!"),
            "Dumbledore".into(),
            "Albus".into(),
            "a.dumbledore@hogwart.edu.uk".into(),
            None,
        );
        albus.save(&pool).await.unwrap();

        let users = User::all(&pool).await.unwrap();
        assert_eq!(users.len(), 2);

        albus.delete(&pool).await.unwrap();

        let users = User::all(&pool).await.unwrap();
        assert_eq!(users.len(), 1);
    }

    #[sqlx::test]
    async fn test_recovery_codes(pool: DbPool) {
        let mut harry = User::new(
            "hpotter".into(),
            Some("pass123"),
            "Potter".into(),
            "Harry".into(),
            "h.potter@hogwart.edu.uk".into(),
            None,
        );
        harry.get_recovery_codes(&pool).await.unwrap();
        assert_eq!(harry.recovery_codes.len(), RECOVERY_CODES_COUNT);
        harry.save(&pool).await.unwrap();

        let fetched_user = User::find_by_username(&pool, "hpotter").await.unwrap();
        assert!(fetched_user.is_some());

        let mut user = fetched_user.unwrap();
        assert_eq!(user.recovery_codes.len(), RECOVERY_CODES_COUNT);
        assert!(!user
            .verify_recovery_code(&pool, "invalid code")
            .await
            .unwrap());
        let codes = user.recovery_codes.clone();
        for code in &codes {
            assert!(user.verify_recovery_code(&pool, code).await.unwrap());
        }
        assert_eq!(user.recovery_codes.len(), 0);
    }
}
