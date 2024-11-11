use std::{fmt, time::SystemTime};

use argon2::{
    password_hash::{
        errors::Error as HashError, rand_core::OsRng, PasswordHash, PasswordHasher,
        PasswordVerifier, SaltString,
    },
    Argon2,
};
use axum::http::StatusCode;
use model_derive::Model;
use sqlx::{query, query_as, query_scalar, Error as SqlxError, PgExecutor, PgPool, Type};
use totp_lite::{totp_custom, Sha1};

use super::{
    device::{Device, UserDevice},
    group::Group,
    wallet::Wallet,
    webauthn::WebAuthn,
    MFAInfo, OAuth2AuthorizedAppInfo, SecurityKey, WalletInfo,
};
use crate::{
    auth::{EMAIL_CODE_DIGITS, TOTP_CODE_DIGITS, TOTP_CODE_VALIDITY_PERIOD},
    db::{Id, NoId, Session},
    error::WebError,
    random::{gen_alphanumeric, gen_totp_secret},
    server_config,
};

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

impl fmt::Display for MFAMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                MFAMethod::None => "None",
                MFAMethod::OneTimePassword => "TOTP",
                MFAMethod::Web3 => "Web3",
                MFAMethod::Webauthn => "WebAuthn",
                MFAMethod::Email => "Email",
            }
        )
    }
}

// User information ready to be sent as part of diagnostic data.
#[derive(Serialize)]
pub struct UserDiagnostic {
    pub id: Id,
    pub mfa_enabled: bool,
    pub totp_enabled: bool,
    pub email_mfa_enabled: bool,
    pub mfa_method: MFAMethod,
    pub is_active: bool,
    pub enrolled: bool,
}

#[derive(Clone, Debug, Model, PartialEq, Serialize)]
pub struct User<I = NoId> {
    pub id: I,
    pub username: String,
    pub(crate) password_hash: Option<String>,
    pub last_name: String,
    pub first_name: String,
    pub email: String,
    pub phone: Option<String>,
    pub mfa_enabled: bool,
    pub is_active: bool,
    /// The user's sub claim returned by the OpenID provider. Also indicates whether the user has
    /// used OpenID to log in.
    // FIXME: must be unique
    pub openid_sub: Option<String>,
    // secret has been verified and TOTP can be used
    pub(crate) totp_enabled: bool,
    pub(crate) email_mfa_enabled: bool,
    pub(crate) totp_secret: Option<Vec<u8>>,
    pub(crate) email_mfa_secret: Option<Vec<u8>>,
    #[model(enum)]
    pub(crate) mfa_method: MFAMethod,
    #[model(ref)]
    pub(crate) recovery_codes: Vec<String>,
}

fn hash_password(password: &str) -> Result<String, HashError> {
    let salt = SaltString::generate(&mut OsRng);
    Ok(Argon2::default()
        .hash_password(password.as_bytes(), &salt)?
        .to_string())
}

impl User {
    #[must_use]
    pub fn new<S: Into<String>>(
        username: S,
        password: Option<&str>,
        last_name: S,
        first_name: S,
        email: S,
        phone: Option<String>,
    ) -> Self {
        let password_hash = password.and_then(|password_hash| hash_password(password_hash).ok());
        Self {
            id: NoId,
            username: username.into(),
            password_hash,
            last_name: last_name.into(),
            first_name: first_name.into(),
            email: email.into(),
            phone,
            mfa_enabled: false,
            totp_enabled: false,
            email_mfa_enabled: false,
            totp_secret: None,
            email_mfa_secret: None,
            mfa_method: MFAMethod::None,
            recovery_codes: Vec::new(),
            is_active: true,
            openid_sub: None,
        }
    }
}

impl<I> User<I> {
    pub fn set_password(&mut self, password: &str) {
        self.password_hash = hash_password(password).ok();
    }

    pub(crate) fn verify_password(&self, password: &str) -> Result<(), HashError> {
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
    pub(crate) fn has_password(&self) -> bool {
        self.password_hash.is_some()
    }

    #[must_use]
    pub(crate) fn name(&self) -> String {
        format!("{} {}", self.first_name, self.last_name)
    }

    /// Check if user is enrolled.
    /// We assume the user is enrolled if they have a password set
    /// or they have logged in using an external OIDC.
    #[must_use]
    pub(crate) fn is_enrolled(&self) -> bool {
        self.password_hash.is_some() || self.openid_sub.is_some()
    }
}

impl User<Id> {
    /// Generate new TOTP secret, save it, then return it as RFC 4648 base32-encoded string.
    pub async fn new_totp_secret<'e, E>(&mut self, executor: E) -> Result<String, SqlxError>
    where
        E: PgExecutor<'e>,
    {
        let secret = gen_totp_secret();
        query!(
            "UPDATE \"user\" SET totp_secret = $1 WHERE id = $2",
            secret,
            self.id
        )
        .execute(executor)
        .await?;

        let secret_base32 = base32::encode(base32::Alphabet::Rfc4648 { padding: false }, &secret);
        self.totp_secret = Some(secret);
        Ok(secret_base32)
    }

    /// Generate new email secret, similar to TOTP secret above, but don't return generated value.
    pub async fn new_email_secret<'e, E>(&mut self, executor: E) -> Result<(), SqlxError>
    where
        E: PgExecutor<'e>,
    {
        let email_secret = gen_totp_secret();
        query!(
            "UPDATE \"user\" SET email_mfa_secret = $1 WHERE id = $2",
            email_secret,
            self.id
        )
        .execute(executor)
        .await?;

        self.email_mfa_secret = Some(email_secret);

        Ok(())
    }

    pub async fn set_mfa_method<'e, E>(
        &mut self,
        executor: E,
        mfa_method: MFAMethod,
    ) -> Result<(), SqlxError>
    where
        E: PgExecutor<'e>,
    {
        info!(
            "Setting MFA method for user {} to {mfa_method:?}",
            self.username
        );
        query!(
            "UPDATE \"user\" SET mfa_method = $2 WHERE id = $1",
            self.id,
            &mfa_method as &MFAMethod
        )
        .execute(executor)
        .await?;
        self.mfa_method = mfa_method;

        Ok(())
    }

    /// Check if any of the multi-factor authentication methods is on.
    /// - TOTP is enabled
    /// - a [`Wallet`] flagged `use_for_mfa`
    /// - a security key for Webauthn
    async fn check_mfa_enabled<'e, E>(&self, executor: E) -> Result<bool, SqlxError>
    where
        E: PgExecutor<'e>,
    {
        // short-cut
        if self.totp_enabled || self.email_mfa_enabled {
            return Ok(true);
        }

        query_scalar!(
            "SELECT totp_enabled OR email_mfa_enabled OR coalesce(bool_or(wallet.use_for_mfa), FALSE) \
            OR count(webauthn.id) > 0 \"bool!\" FROM \"user\" \
            LEFT JOIN wallet ON wallet.user_id = \"user\".id \
            LEFT JOIN webauthn ON webauthn.user_id = \"user\".id \
            WHERE \"user\".id = $1 GROUP BY totp_enabled, email_mfa_enabled;",
            self.id
        )
        .fetch_one(executor)
        .await
    }

    /// Verify the state of mfa flags are correct.
    /// Recovers from invalid mfa_method
    /// Use this function after removing any of the authentication factors.
    pub async fn verify_mfa_state(&mut self, pool: &PgPool) -> Result<(), WebError> {
        if let Some(info) = MFAInfo::for_user(pool, self).await? {
            let factors_present = info.mfa_available();
            if self.mfa_enabled != factors_present {
                // store correct value for MFA flag in the DB
                if self.mfa_enabled {
                    // last factor was removed so we have to disable MFA
                    self.disable_mfa(pool).await?;
                } else {
                    // first factor was added so MFA needs to be enabled
                    query!(
                        "UPDATE \"user\" SET mfa_enabled = $2 WHERE id = $1",
                        self.id,
                        factors_present
                    )
                    .execute(pool)
                    .await?;
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
                            "Checking if {:?} in in available methods {methods:?}, {}",
                            info.mfa_method,
                            methods.contains(&info.mfa_method)
                        );
                        if !methods.contains(&info.mfa_method) {
                            // FIXME: do not panic
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
    pub async fn enable_mfa(&mut self, pool: &PgPool) -> Result<(), WebError> {
        if !self.mfa_enabled {
            self.verify_mfa_state(pool).await?;
        }
        Ok(())
    }

    /// Get recovery codes. If recovery codes exist, this function returns `None`.
    /// That way recovery codes are returned only once - when MFA is turned on.
    pub async fn get_recovery_codes<'e, E>(
        &mut self,
        executor: E,
    ) -> Result<Option<Vec<String>>, SqlxError>
    where
        E: PgExecutor<'e>,
    {
        if !self.recovery_codes.is_empty() {
            return Ok(None);
        }

        for _ in 0..RECOVERY_CODES_COUNT {
            let code = gen_alphanumeric(16);
            self.recovery_codes.push(code);
        }
        query!(
            "UPDATE \"user\" SET recovery_codes = $2 WHERE id = $1",
            self.id,
            &self.recovery_codes
        )
        .execute(executor)
        .await?;

        Ok(Some(self.recovery_codes.clone()))
    }

    /// Disable MFA; discard recovery codes, TOTP secret, and security keys.
    pub async fn disable_mfa(&mut self, pool: &PgPool) -> Result<(), SqlxError> {
        query!(
            "UPDATE \"user\" SET mfa_enabled = FALSE, mfa_method = 'none', totp_enabled = FALSE, email_mfa_enabled = FALSE, \
            totp_secret = NULL, email_mfa_secret = NULL, recovery_codes = '{}' WHERE id = $1",
            self.id
        )
        .execute(pool)
        .await?;
        Wallet::disable_mfa_for_user(pool, self.id).await?;
        WebAuthn::delete_all_for_user(pool, self.id).await?;

        self.totp_secret = None;
        self.email_mfa_secret = None;
        self.totp_enabled = false;
        self.email_mfa_enabled = false;
        self.mfa_method = MFAMethod::None;
        self.recovery_codes.clear();

        Ok(())
    }

    /// Enable TOTP
    pub async fn enable_totp<'e, E>(&mut self, executor: E) -> Result<(), SqlxError>
    where
        E: PgExecutor<'e>,
    {
        if !self.totp_enabled {
            query!(
                "UPDATE \"user\" SET totp_enabled = TRUE WHERE id = $1",
                self.id
            )
            .execute(executor)
            .await?;
            self.totp_enabled = true;
        }

        Ok(())
    }

    /// Disable TOTP; discard the secret.
    pub async fn disable_totp(&mut self, pool: &PgPool) -> Result<(), SqlxError> {
        if self.totp_enabled {
            // FIXME: check if this flag is set correctly when TOTP is the only method
            self.mfa_enabled = self.check_mfa_enabled(pool).await?;
            self.totp_enabled = false;
            self.totp_secret = None;

            query!(
                "UPDATE \"user\" SET mfa_enabled = $2, totp_enabled = $3 AND totp_secret = $4 \
                WHERE id = $1",
                self.id,
                self.mfa_enabled,
                self.totp_enabled,
                self.totp_secret,
            )
            .execute(pool)
            .await?;
        }

        Ok(())
    }

    /// Enable email MFA
    pub async fn enable_email_mfa<'e, E>(&mut self, executor: E) -> Result<(), SqlxError>
    where
        E: PgExecutor<'e>,
    {
        if !self.email_mfa_enabled {
            query!(
                "UPDATE \"user\" SET email_mfa_enabled = TRUE WHERE id = $1",
                self.id
            )
            .execute(executor)
            .await?;

            self.email_mfa_enabled = true;
        }

        Ok(())
    }

    /// Disable email MFA; discard the secret.
    pub async fn disable_email_mfa(&mut self, pool: &PgPool) -> Result<(), SqlxError> {
        if self.email_mfa_enabled {
            self.mfa_enabled = self.check_mfa_enabled(pool).await?;
            self.email_mfa_enabled = false;
            self.email_mfa_secret = None;

            query!(
                "UPDATE \"user\" SET mfa_enabled = $2, email_mfa_enabled = $3 AND email_mfa_secret = $4 \
                WHERE id = $1",
                self.id,
                self.mfa_enabled,
                self.email_mfa_enabled,
                self.email_mfa_secret,
            )
            .execute(pool)
            .await?;
        }

        Ok(())
    }

    /// Select all users without sensitive data.
    // FIXME: Remove it when Model macro will support SecretString
    pub async fn all_without_sensitive_data(
        pool: &PgPool,
    ) -> Result<Vec<UserDiagnostic>, SqlxError> {
        let users = query!(
            "SELECT id, mfa_enabled, totp_enabled, email_mfa_enabled, \
                mfa_method \"mfa_method: MFAMethod\", password_hash, is_active, openid_sub \
            FROM \"user\""
        )
        .fetch_all(pool)
        .await?;
        let res: Vec<UserDiagnostic> = users
            .iter()
            .map(|u| UserDiagnostic {
                mfa_method: u.mfa_method.clone(),
                totp_enabled: u.totp_enabled,
                email_mfa_enabled: u.email_mfa_enabled,
                mfa_enabled: u.mfa_enabled,
                id: u.id,
                is_active: u.is_active,
                enrolled: u.password_hash.is_some() || u.openid_sub.is_some(),
            })
            .collect();

        Ok(res)
    }

    /// Return all members of group
    pub async fn find_by_group_name(
        pool: &PgPool,
        group_name: &str,
    ) -> Result<Vec<User<Id>>, SqlxError> {
        let users = query_as!(
            Self,
            "SELECT \"user\".id, username, password_hash, last_name, first_name, email, \
            phone, mfa_enabled, totp_enabled, totp_secret, \
            email_mfa_enabled, email_mfa_secret, \
            mfa_method \"mfa_method: _\", recovery_codes, is_active, openid_sub \
            FROM \"user\" \
            INNER JOIN \"group_user\" ON \"user\".id = \"group_user\".user_id \
            INNER JOIN \"group\" ON \"group_user\".group_id = \"group\".id \
            WHERE \"group\".name = $1",
            group_name
        )
        .fetch_all(pool)
        .await?;

        Ok(users)
    }

    /// Check if TOTP `code` is valid.
    #[must_use]
    pub fn verify_totp_code(&self, code: &str) -> bool {
        if let Some(totp_secret) = &self.totp_secret {
            if let Ok(timestamp) = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
                let expected_code = totp_custom::<Sha1>(
                    TOTP_CODE_VALIDITY_PERIOD,
                    TOTP_CODE_DIGITS,
                    totp_secret,
                    timestamp.as_secs(),
                );
                return code == expected_code;
            }
        }

        false
    }

    /// Generate MFA code for email verification.
    ///
    /// NOTE: This code will be valid for two time frames. See comment for verify_email_mfa_code().
    pub fn generate_email_mfa_code(&self) -> Result<String, WebError> {
        if let Some(email_mfa_secret) = &self.email_mfa_secret {
            let timeout = &server_config().mfa_code_timeout;
            if let Ok(timestamp) = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
                let code = totp_custom::<Sha1>(
                    timeout.as_secs(),
                    EMAIL_CODE_DIGITS,
                    email_mfa_secret,
                    timestamp.as_secs(),
                );
                Ok(code)
            } else {
                Err(WebError::EmailMfa("SystemTime before UNIX epoch".into()))
            }
        } else {
            Err(WebError::EmailMfa(format!(
                "Email MFA secret not configured for user {}",
                self.username
            )))
        }
    }

    /// Check if email MFA `code` is valid.
    ///
    /// IMPORTANT: because current implementation uses TOTP for email verification,
    /// allow the code for the previous time frame. This approach pretends the code is valid
    /// for a certain *period of time* (as opposed to a TOTP code which is valid for a certain time *frame*).
    ///
    /// ```text
    /// |<---- frame #0 ---->|<---- frame #1 ---->|<---- frame #2 ---->|
    /// |................[*]email sent.................................|
    /// |......................[*]email code verified..................|
    /// ```
    #[must_use]
    pub fn verify_email_mfa_code(&self, code: &str) -> bool {
        if let Some(email_mfa_secret) = &self.email_mfa_secret {
            let timeout = server_config().mfa_code_timeout.as_secs();
            if let Ok(timestamp) = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
                let expected_code = totp_custom::<Sha1>(
                    timeout,
                    EMAIL_CODE_DIGITS,
                    email_mfa_secret,
                    timestamp.as_secs(),
                );
                if code == expected_code {
                    return true;
                }

                let previous_code = totp_custom::<Sha1>(
                    timeout,
                    EMAIL_CODE_DIGITS,
                    email_mfa_secret,
                    timestamp.as_secs() - timeout,
                );
                return code == previous_code;
            }
        }
        false
    }

    /// Verify recovery code. If it is valid, consume it, so it can't be used again.
    pub async fn verify_recovery_code(
        &mut self,
        pool: &PgPool,
        code: &str,
    ) -> Result<bool, SqlxError> {
        if let Some(index) = self.recovery_codes.iter().position(|c| c == code) {
            // Note: swap_remove() should be faster than remove().
            self.recovery_codes.swap_remove(index);

            query!(
                "UPDATE \"user\" SET recovery_codes = $2 WHERE id = $1",
                self.id,
                &self.recovery_codes
            )
            .execute(pool)
            .await?;

            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub async fn find_by_username<'e, E>(
        executor: E,
        username: &str,
    ) -> Result<Option<Self>, SqlxError>
    where
        E: PgExecutor<'e>,
    {
        query_as!(
            Self,
            "SELECT id, username, password_hash, last_name, first_name, email, \
            phone, mfa_enabled, totp_enabled, email_mfa_enabled, \
            totp_secret, email_mfa_secret, mfa_method \"mfa_method: _\", recovery_codes, is_active, openid_sub \
            FROM \"user\" WHERE username = $1",
            username
        )
        .fetch_optional(executor)
        .await
    }

    pub async fn find_by_email<'e, E>(executor: E, email: &str) -> Result<Option<Self>, SqlxError>
    where
        E: PgExecutor<'e>,
    {
        query_as!(
            Self,
            "SELECT id, username, password_hash, last_name, first_name, email, phone, \
            mfa_enabled, totp_enabled, email_mfa_enabled, totp_secret, email_mfa_secret, \
            mfa_method \"mfa_method: _\", recovery_codes, is_active, openid_sub \
            FROM \"user\" WHERE email = $1",
            email
        )
        .fetch_optional(executor)
        .await
    }

    // FIXME: Remove `LIMIT 1` when `openid_sub` is unique.
    pub async fn find_by_sub<'e, E>(executor: E, sub: &str) -> Result<Option<Self>, SqlxError>
    where
        E: PgExecutor<'e>,
    {
        query_as!(
            Self,
            "SELECT id, username, password_hash, last_name, first_name, email, phone, \
            mfa_enabled, totp_enabled, email_mfa_enabled, totp_secret, email_mfa_secret, \
            mfa_method \"mfa_method: _\", recovery_codes, is_active, openid_sub \
            FROM \"user\" WHERE openid_sub = $1 LIMIT 1",
            sub
        )
        .fetch_optional(executor)
        .await
    }

    pub async fn member_of_names<'e, E>(&self, executor: E) -> Result<Vec<String>, SqlxError>
    where
        E: PgExecutor<'e>,
    {
        query_scalar!(
            "SELECT \"group\".name FROM \"group\" JOIN group_user ON \"group\".id = group_user.group_id \
            WHERE group_user.user_id = $1",
            self.id
        )
        .fetch_all(executor)
        .await
    }

    pub async fn member_of<'e, E>(&self, executor: E) -> Result<Vec<Group<Id>>, SqlxError>
    where
        E: PgExecutor<'e>,
    {
        query_as!(
            Group,
            "SELECT id, name FROM \"group\" JOIN group_user ON \"group\".id = group_user.group_id \
            WHERE group_user.user_id = $1",
            self.id
        )
        .fetch_all(executor)
        .await
    }

    /// Returns a vector of [`UserDevice`]s (hence the name).
    /// [`UserDevice`] is a struct containing additional network info about a device.
    /// If you only need [`Device`]s, use [`User::devices()`] instead.
    pub async fn user_devices(&self, pool: &PgPool) -> Result<Vec<UserDevice>, SqlxError> {
        let devices = self.devices(pool).await?;
        let mut user_devices = Vec::new();
        for device in devices {
            if let Some(user_device) = UserDevice::from_device(pool, device).await? {
                user_devices.push(user_device);
            }
        }

        Ok(user_devices)
    }

    /// Returns a vector of [`Device`]s related to a user. If you want to get [`UserDevice`]s (which contain additional network info),
    /// use [`User::user_devices()`] instead.
    pub async fn devices<'e, E>(&self, executor: E) -> Result<Vec<Device<Id>>, SqlxError>
    where
        E: PgExecutor<'e>,
    {
        query_as!(
            Device,
            "SELECT device.id, name, wireguard_pubkey, user_id, created \
            FROM device WHERE user_id = $1",
            self.id
        )
        .fetch_all(executor)
        .await
    }

    pub async fn wallets<'e, E>(&self, executor: E) -> Result<Vec<WalletInfo>, SqlxError>
    where
        E: PgExecutor<'e>,
    {
        query_as!(
            WalletInfo,
            "SELECT address \"address!\", name, chain_id, use_for_mfa \
            FROM wallet WHERE user_id = $1 AND validation_timestamp IS NOT NULL",
            self.id
        )
        .fetch_all(executor)
        .await
    }

    pub async fn oauth2authorizedapps<'e, E>(
        &self,
        executor: E,
    ) -> Result<Vec<OAuth2AuthorizedAppInfo>, SqlxError>
    where
        E: PgExecutor<'e>,
    {
        query_as!(
            OAuth2AuthorizedAppInfo,
            "SELECT oauth2client.id \"oauth2client_id!\", oauth2client.name \"oauth2client_name\", \
            oauth2authorizedapp.user_id \"user_id\" \
            FROM oauth2authorizedapp \
            JOIN oauth2client ON oauth2client.id = oauth2authorizedapp.oauth2client_id \
            WHERE oauth2authorizedapp.user_id = $1",
            self.id
        )
        .fetch_all(executor)
        .await
    }

    pub async fn security_keys(&self, pool: &PgPool) -> Result<Vec<SecurityKey>, SqlxError> {
        query_as!(
            SecurityKey,
            "SELECT id \"id!\", name FROM webauthn WHERE user_id = $1",
            self.id
        )
        .fetch_all(pool)
        .await
    }

    pub async fn add_to_group<'e, E>(&self, executor: E, group: &Group<Id>) -> Result<(), SqlxError>
    where
        E: PgExecutor<'e>,
    {
        query!(
            "INSERT INTO group_user (group_id, user_id) VALUES ($1, $2) \
            ON CONFLICT DO NOTHING",
            group.id,
            self.id
        )
        .execute(executor)
        .await?;

        Ok(())
    }

    pub async fn remove_from_group<'e, E>(
        &self,
        executor: E,
        group: &Group<Id>,
    ) -> Result<(), SqlxError>
    where
        E: PgExecutor<'e>,
    {
        query!(
            "DELETE FROM group_user WHERE group_id = $1 AND user_id = $2",
            group.id,
            self.id
        )
        .execute(executor)
        .await?;

        Ok(())
    }

    /// Remove authorized apps by their client id's from user
    pub async fn remove_oauth2_authorized_apps<'e, E>(
        &self,
        executor: E,
        app_client_ids: &[i64],
    ) -> Result<(), SqlxError>
    where
        E: PgExecutor<'e>,
    {
        query!(
            "DELETE FROM oauth2authorizedapp WHERE user_id = $1 AND oauth2client_id = ANY($2)",
            self.id,
            app_client_ids
        )
        .execute(executor)
        .await?;

        Ok(())
    }

    /// Create admin user if one doesn't exist yet
    pub async fn init_admin_user(
        pool: &PgPool,
        default_admin_pass: &str,
    ) -> Result<(), anyhow::Error> {
        info!("Initializing admin user");
        let password_hash = hash_password(default_admin_pass)?;

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
            info!("New admin user has been created, adding to Admin group...");
            query("INSERT INTO group_user (group_id, user_id) VALUES (1, $1)")
                .bind(new_user_id)
                .execute(pool)
                .await?;
        }

        Ok(())
    }

    pub async fn logout_all_sessions<'e, E>(&self, executor: E) -> Result<(), SqlxError>
    where
        E: PgExecutor<'e>,
    {
        Session::delete_all_for_user(executor, self.id).await?;

        Ok(())
    }

    pub async fn find_by_device_id<'e, E>(
        executor: E,
        device_id: Id,
    ) -> Result<Option<Self>, SqlxError>
    where
        E: PgExecutor<'e>,
    {
        query_as!(
            Self,
            "SELECT u.id, u.username, u.password_hash, u.last_name, u.first_name, u.email, \
            u.phone, u.mfa_enabled, u.totp_enabled, u.email_mfa_enabled, \
            u.totp_secret, u.email_mfa_secret, u.mfa_method \"mfa_method: _\", u.recovery_codes, u.is_active, u.openid_sub \
            FROM \"user\" u \
            JOIN \"device\" d ON u.id = d.user_id \
            WHERE d.id = $1",
            device_id
        )
        .fetch_optional(executor)
        .await
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{config::DefGuardConfig, SERVER_CONFIG};

    #[sqlx::test]
    async fn test_mfa_code(pool: PgPool) {
        let config = DefGuardConfig::new_test_config();
        let _ = SERVER_CONFIG.set(config.clone());

        let mut user = User::new(
            "hpotter",
            Some("pass123"),
            "Potter",
            "Harry",
            "h.potter@hogwart.edu.uk",
            None,
        )
        .save(&pool)
        .await
        .unwrap();
        user.new_email_secret(&pool).await.unwrap();
        assert!(user.email_mfa_secret.is_some());
        let code = user.generate_email_mfa_code().unwrap();
        assert!(
            user.verify_email_mfa_code(&code),
            "code={code}, secret={:?}",
            user.email_mfa_secret.unwrap()
        );
    }

    #[sqlx::test]
    async fn test_user(pool: PgPool) {
        let mut user = User::new(
            "hpotter",
            Some("pass123"),
            "Potter",
            "Harry",
            "h.potter@hogwart.edu.uk",
            None,
        )
        .save(&pool)
        .await
        .unwrap();

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
    async fn test_all_users(pool: PgPool) {
        User::new(
            "hpotter",
            Some("pass123"),
            "Potter",
            "Harry",
            "h.potter@hogwart.edu.uk",
            None,
        )
        .save(&pool)
        .await
        .unwrap();

        let albus = User::new(
            "adumbledore",
            Some("magic!"),
            "Dumbledore",
            "Albus",
            "a.dumbledore@hogwart.edu.uk",
            None,
        )
        .save(&pool)
        .await
        .unwrap();

        let users = User::all(&pool).await.unwrap();
        assert_eq!(users.len(), 2);

        albus.delete(&pool).await.unwrap();

        let users = User::all(&pool).await.unwrap();
        assert_eq!(users.len(), 1);
    }

    #[sqlx::test]
    async fn test_recovery_codes(pool: PgPool) {
        let mut harry = User::new(
            "hpotter",
            Some("pass123"),
            "Potter",
            "Harry",
            "h.potter@hogwart.edu.uk",
            None,
        )
        .save(&pool)
        .await
        .unwrap();
        harry.get_recovery_codes(&pool).await.unwrap();
        assert_eq!(harry.recovery_codes.len(), RECOVERY_CODES_COUNT);

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
