use std::{
    fmt,
    time::{Duration, SystemTime},
};

use argon2::{
    Argon2,
    password_hash::{
        PasswordHash, PasswordHasher, PasswordVerifier, SaltString, errors::Error as HashError,
        rand_core::OsRng,
    },
};
use model_derive::Model;
use rand::{
    Rng,
    distributions::{Alphanumeric, DistString, Standard},
    prelude::Distribution,
};
use serde::{Deserialize, Serialize};
use sqlx::{
    Error as SqlxError, FromRow, PgConnection, PgExecutor, PgPool, Type, query, query_as,
    query_scalar,
};
use thiserror::Error;
use totp_lite::{Sha1, totp_custom};
use tracing::{debug, error, info, warn};
use utoipa::ToSchema;

use super::{
    device::{Device, DeviceType, UserDevice},
    group::Group,
};
use crate::{
    db::{
        Id, NoId,
        models::{MFAInfo, Session, Settings, WebAuthn, group::Permission},
    },
    random::{gen_alphanumeric, gen_totp_secret},
    types::user_info::OAuth2AuthorizedAppInfo,
};

const RECOVERY_CODES_COUNT: usize = 8;
pub const TOTP_CODE_VALIDITY_PERIOD: u64 = 30;
pub const EMAIL_CODE_DIGITS: u32 = 6;
pub const TOTP_CODE_DIGITS: u32 = 6;

#[derive(Debug, Error)]
pub enum UserError {
    #[error("Invalid MFA state for user {username}")]
    InvalidMfaState { username: String },
    #[error(transparent)]
    DbError(#[from] SqlxError),
    #[error("{0}")]
    EmailMfaError(String),
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Hash, ToSchema, Type)]
#[sqlx(type_name = "mfa_method", rename_all = "snake_case")]
pub enum MFAMethod {
    None,
    OneTimePassword,
    Webauthn,
    Email,
}

// Web MFA methods
impl fmt::Display for MFAMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                MFAMethod::None => "None",
                MFAMethod::OneTimePassword => "TOTP",
                MFAMethod::Webauthn => "WebAuthn",
                MFAMethod::Email => "Email",
            }
        )
    }
}

/// Only `id` and `name` from [`WebAuthn`].
#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct SecurityKey {
    pub id: Id,
    pub name: String,
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

#[derive(Clone, Model, PartialEq, Eq, Hash, Serialize, FromRow)]
pub struct User<I = NoId> {
    pub id: I,
    pub username: String,
    pub password_hash: Option<String>,
    pub last_name: String,
    pub first_name: String,
    pub email: String,
    pub phone: Option<String>,
    pub mfa_enabled: bool,
    pub is_active: bool,
    /// Indicates whether the user has been created via the LDAP integration.
    pub from_ldap: bool,
    /// Indicates whether a user has a random password set in LDAP, if so, the user
    /// will be prompted to change it on their profile page.
    ///
    /// The random password is set if we are creating a new user in LDAP from a Defguard user
    /// and we don't have access to the plain text password, e.g. during Defguard -> LDAP user import.
    pub ldap_pass_randomized: bool,
    /// The user's LDAP RDN value. This is the first part of the DN.
    /// For example, if the DN is `cn=John Doe,ou=users,dc=example,dc=com`,
    /// the RDN is `cn=John Doe`.
    /// This is used to identify the user in LDAP as we sometimes can't use the Defguard's username
    /// since the RDN may contain spaces or other special characters and the username may not.
    pub ldap_rdn: Option<String>,
    /// Rest of the user's DN
    pub ldap_user_path: Option<String>,
    /// The user's sub claim returned by the OpenID provider. Also indicates whether the user has
    /// used OpenID to log in.
    // FIXME: must be unique
    pub openid_sub: Option<String>,
    // secret has been verified and TOTP can be used
    pub totp_enabled: bool,
    pub email_mfa_enabled: bool,
    pub totp_secret: Option<Vec<u8>>,
    pub email_mfa_secret: Option<Vec<u8>>,
    #[model(enum)]
    pub mfa_method: MFAMethod,
    #[model(ref)]
    pub recovery_codes: Vec<String>,
    /// Indicates that an administrator has requested an enrollment token for this user.
    /// Uninitialized clients should then guide the user through enrollment process.
    /// Related issue: https://github.com/DefGuard/client/issues/647.
    pub enrollment_pending: bool,
}

// TODO: Refactor the user struct to use SecretStringWrapper instead of this
impl<I: std::fmt::Debug> fmt::Debug for User<I> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            id,
            username,
            password_hash: _,
            last_name,
            first_name,
            email,
            phone,
            mfa_enabled,
            is_active,
            from_ldap,
            ldap_pass_randomized,
            ldap_rdn,
            ldap_user_path,
            openid_sub,
            totp_enabled,
            email_mfa_enabled,
            totp_secret: _,
            email_mfa_secret: _,
            mfa_method,
            recovery_codes,
            enrollment_pending,
        } = self;

        f.debug_struct("User")
            .field("id", id)
            .field("username", username)
            .field("last_name", last_name)
            .field("first_name", first_name)
            .field("email", email)
            .field("phone", phone)
            .field("mfa_enabled", mfa_enabled)
            .field("is_active", is_active)
            .field("from_ldap", from_ldap)
            .field("ldap_pass_randomized", ldap_pass_randomized)
            .field("ldap_rdn", ldap_rdn)
            .field("ldap_user_path", ldap_user_path) // sensitive data
            .field("openid_sub", openid_sub)
            .field("totp_enabled", totp_enabled)
            .field("email_mfa_enabled", email_mfa_enabled)
            .field("mfa_method", mfa_method)
            .field(
                "recovery_codes",
                &format_args!("{} items", recovery_codes.len()),
            )
            .field("password_hash", &"***")
            .field("totp_secret", &"***")
            .field("email_mfa_secret", &"***")
            .field("enrollment_pending", enrollment_pending)
            .finish()
    }
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
        let username: String = username.into();
        Self {
            id: NoId,
            username: username.clone(),
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
            from_ldap: false,
            ldap_pass_randomized: false,
            ldap_rdn: Some(username.clone()),
            ldap_user_path: None,
            enrollment_pending: false,
        }
    }
}

impl<I> fmt::Display for User<I> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.username)
    }
}

impl<I> User<I> {
    pub fn set_password(&mut self, password: &str) {
        self.password_hash = hash_password(password).ok();
    }

    pub fn verify_password(&self, password: &str) -> Result<(), HashError> {
        debug!("Checking if password matches for user {}", self.username);
        if let Some(hash) = &self.password_hash {
            let parsed_hash = PasswordHash::new(hash)?;
            Argon2::default().verify_password(password.as_bytes(), &parsed_hash)
        } else {
            info!("User {} has no password set", self.username);
            Err(HashError::Password)
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

    /// Determines whether the user is considered enrolled.
    ///
    /// A user is treated as enrolled if:
    /// - The `enrollment_pending` flag is **not** set, i.e. enrollment was not requested by an
    ///   administrator (https://github.com/DefGuard/client/issues/647).
    /// - They either have a password configured, have authenticated via an external OIDC provider
    ///   or were synced from LDAP.
    #[must_use]
    pub fn is_enrolled(&self) -> bool {
        !self.enrollment_pending
            && (self.password_hash.is_some() || self.openid_sub.is_some() || self.from_ldap)
    }

    #[must_use]
    pub fn ldap_rdn_value(&self) -> &str {
        if let Some(ldap_rdn) = &self.ldap_rdn {
            ldap_rdn
        } else {
            warn!(
                "LDAP RDN is not set for user {}. Using username as a fallback.",
                self.username
            );
            &self.username
        }
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
            "SELECT totp_enabled OR email_mfa_enabled \
            OR count(webauthn.id) > 0 \"bool!\" FROM \"user\" \
            LEFT JOIN webauthn ON webauthn.user_id = \"user\".id \
            WHERE \"user\".id = $1 GROUP BY totp_enabled, email_mfa_enabled;",
            self.id
        )
        .fetch_one(executor)
        .await
    }

    /// Verify the state of MFA flags are correct.
    /// Recovers from invalid mfa_method
    /// Use this function after removing any of the authentication factors.
    pub async fn verify_mfa_state(&mut self, pool: &PgPool) -> Result<(), UserError> {
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
                }

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
                        return Err(UserError::InvalidMfaState {
                            username: self.username.clone(),
                        });
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
        }
        Ok(())
    }

    /// Enable MFA. At least one of the authenticator factors must be configured.
    pub async fn enable_mfa(&mut self, pool: &PgPool) -> Result<(), UserError> {
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
                mfa_method \"mfa_method: MFAMethod\", password_hash, is_active, openid_sub, \
                from_ldap, ldap_pass_randomized, ldap_rdn \
            FROM \"user\""
        )
        .fetch_all(pool)
        .await?;
        let res = users
            .iter()
            .map(|u| UserDiagnostic {
                mfa_method: u.mfa_method.clone(),
                totp_enabled: u.totp_enabled,
                email_mfa_enabled: u.email_mfa_enabled,
                mfa_enabled: u.mfa_enabled,
                id: u.id,
                is_active: u.is_active,
                enrolled: u.password_hash.is_some() || u.openid_sub.is_some() || u.from_ldap,
            })
            .collect::<Vec<_>>();

        Ok(res)
    }

    /// Return all active users.
    pub async fn all_active<'e, E>(executor: E) -> Result<Vec<Self>, SqlxError>
    where
        E: PgExecutor<'e>,
    {
        query_as!(
            User,
            "SELECT id, username, password_hash, last_name, first_name, email, phone, mfa_enabled, \
            totp_enabled, totp_secret, email_mfa_enabled, email_mfa_secret, \
            mfa_method \"mfa_method: _\", recovery_codes, is_active, openid_sub, from_ldap, \
            ldap_pass_randomized, ldap_rdn, ldap_user_path, enrollment_pending \
            FROM \"user\" \
            WHERE is_active = true"
        )
        .fetch_all(executor)
        .await
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
            mfa_method \"mfa_method: _\", recovery_codes, is_active, openid_sub, \
            from_ldap, ldap_pass_randomized, ldap_rdn, ldap_user_path, enrollment_pending \
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

    /// Generate MFA code for email verification. The code is zero-padded.
    ///
    /// NOTE: This code will be valid for two time frames. See comment for verify_email_mfa_code().
    pub fn generate_email_mfa_code(&self) -> Result<String, UserError> {
        if let Some(email_mfa_secret) = &self.email_mfa_secret {
            let settings = Settings::get_current_settings();
            let timeout = Duration::from_secs(settings.mfa_code_timeout_seconds as u64);
            if let Ok(timestamp) = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
                let code = totp_custom::<Sha1>(
                    timeout.as_secs(),
                    EMAIL_CODE_DIGITS,
                    email_mfa_secret,
                    timestamp.as_secs(),
                );
                Ok(code)
            } else {
                Err(UserError::EmailMfaError(
                    "SystemTime before UNIX epoch".into(),
                ))
            }
        } else {
            Err(UserError::EmailMfaError(format!(
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
            let settings = Settings::get_current_settings();
            let timeout = settings.mfa_code_timeout_seconds as u64;
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
                debug!(
                    "Email MFA verification TOTP code for user {} doesn't fit current time \
                    frame, checking the previous one. \
                    Expected: {expected_code}, got: {code}",
                    self.username
                );

                let previous_code = totp_custom::<Sha1>(
                    timeout,
                    EMAIL_CODE_DIGITS,
                    email_mfa_secret,
                    timestamp.as_secs() - timeout,
                );

                if code == previous_code {
                    return true;
                }
                debug!(
                    "Email MFA verification TOTP code for user {} doesn't fit previous time frame, \
                    expected: {previous_code}, got: {code}",
                    self.username
                );
                return false;
            }
            debug!(
                "Couldn't calculate current timestamp when verifying email MFA code for user {}",
                self.username
            );
        } else {
            debug!("Email MFA secret not configured for user {}", self.username);
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
            "SELECT id, username, password_hash, last_name, first_name, email, phone, mfa_enabled, \
            totp_enabled, email_mfa_enabled, totp_secret, email_mfa_secret, \
            mfa_method \"mfa_method: _\", recovery_codes, is_active, openid_sub, \
            from_ldap, ldap_pass_randomized, ldap_rdn, ldap_user_path, enrollment_pending \
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
            "SELECT id, username, password_hash, last_name, first_name, email, phone, mfa_enabled, \
            totp_enabled, email_mfa_enabled, totp_secret, email_mfa_secret, \
            mfa_method \"mfa_method: _\", recovery_codes, is_active, openid_sub, from_ldap, \
            ldap_pass_randomized, ldap_rdn, ldap_user_path, enrollment_pending \
            FROM \"user\" WHERE email ILIKE $1",
            email
        )
        .fetch_optional(executor)
        .await
    }

    /// Attempts to find user by username and then by email, if none is initially found.
    pub async fn find_by_username_or_email(
        conn: &mut PgConnection,
        username_or_email: &str,
    ) -> Result<Option<Self>, SqlxError> {
        let maybe_user = Self::find_by_username(&mut *conn, username_or_email).await?;
        if let Some(user) = maybe_user {
            Ok(Some(user))
        } else {
            debug!(
                "Failed to find user by username {username_or_email}. Attempting to find by email"
            );
            Ok(Self::find_by_email(&mut *conn, username_or_email).await?)
        }
    }

    pub async fn find_many_by_emails<'e, E>(
        executor: E,
        emails: &[&str],
    ) -> Result<Vec<Self>, SqlxError>
    where
        E: PgExecutor<'e>,
    {
        query_as(
            "SELECT id, username, password_hash, last_name, first_name, email, phone, \
            mfa_enabled, totp_enabled, email_mfa_enabled, totp_secret, email_mfa_secret, \
            mfa_method, recovery_codes, is_active, openid_sub, from_ldap, ldap_pass_randomized, \
            ldap_rdn, ldap_user_path, enrollment_pending \
            FROM \"user\" WHERE email = ANY($1)",
        )
        .bind(emails)
        .fetch_all(executor)
        .await
    }

    pub async fn find_by_sub<'e, E>(executor: E, sub: &str) -> Result<Option<Self>, SqlxError>
    where
        E: PgExecutor<'e>,
    {
        query_as!(
            Self,
            "SELECT id, username, password_hash, last_name, first_name, email, phone, \
            mfa_enabled, totp_enabled, email_mfa_enabled, totp_secret, email_mfa_secret, \
            mfa_method \"mfa_method: _\", recovery_codes, is_active, openid_sub, \
            from_ldap, ldap_pass_randomized, ldap_rdn, ldap_user_path, enrollment_pending \
            FROM \"user\" WHERE openid_sub = $1",
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
            "SELECT id, name, is_admin FROM \"group\" JOIN group_user ON \"group\".id = group_user.group_id \
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
            "SELECT device.id, name, wireguard_pubkey, user_id, created, description, \
            device_type \"device_type: DeviceType\", configured \
            FROM device WHERE user_id = $1 and device_type = 'user'::device_type \
            ORDER BY id",
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
        debug!("Checking if some admin user already exists and creating one if not...");
        let admins = User::find_admins(pool).await?;
        if admins.is_empty() {
            let admin_groups = Group::find_by_permission(pool, Permission::IsAdmin).await?;
            if admin_groups.is_empty() {
                return Err(anyhow::anyhow!(
                    "No admin group and users found, or they are all disabled. \
                    You'll need to create and assign the admin group manually, \
                    as there must be at least one active admin user."
                ));
            }

            // create admin user
            let password_hash = hash_password(default_admin_pass)?;
            let result = query_scalar!(
                "INSERT INTO \"user\" (username, password_hash, last_name, first_name, email, ldap_rdn) \
                VALUES ('admin', $1, 'Administrator', 'DefGuard', 'admin@defguard', 'admin') \
                ON CONFLICT DO NOTHING \
                RETURNING id",
                password_hash
            )
            .fetch_optional(pool)
            .await?;

            // if new user was created add them to admin group, first one you find
            // the groups are sorted by ID desceding, so it will often be the 1st one = the default admin group
            if let Some(new_user_id) = result {
                let admin_group_id = admin_groups
                    .first()
                    .ok_or(anyhow::anyhow!(
                        "No admin group found, can't create admin user"
                    ))?
                    .id;
                info!("New admin user has been created, adding to Admin group...");
                query("INSERT INTO group_user (group_id, user_id) VALUES ($1, $2)")
                    .bind(admin_group_id)
                    .bind(new_user_id)
                    .execute(pool)
                    .await?;
                info!("Admin user has been created as there was no other admin user");
            } else {
                return Err(anyhow::anyhow!(
                    "A conflict occurred while trying to add a missing admin. \
                    There is already a user with username 'admin' but he is not an admin or he is disabled. \
                    You will need to assign someone the admin group manually or enable this admin user, \
                    as there must be at least one active admin."
                ));
            }
        } else {
            debug!("Admin users already exists, skipping creation of the default admin user");
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
            u.totp_secret, u.email_mfa_secret, u.mfa_method \"mfa_method: _\", u.recovery_codes, \
            u.is_active, u.openid_sub, from_ldap, ldap_pass_randomized, ldap_rdn, ldap_user_path, \
            enrollment_pending \
            FROM \"user\" u \
            JOIN \"device\" d ON u.id = d.user_id \
            WHERE d.id = $1",
            device_id
        )
        .fetch_optional(executor)
        .await
    }

    /// Find users which emails are NOT in `user_emails`.
    pub async fn exclude<'e, E>(executor: E, user_emails: &[&str]) -> Result<Vec<Self>, SqlxError>
    where
        E: PgExecutor<'e>,
    {
        // This can't be a macro since sqlx can't handle an array of slices in a macro.
        query_as(
            "SELECT id, username, password_hash, last_name, first_name, email, phone, \
            mfa_enabled, totp_enabled, email_mfa_enabled, totp_secret, email_mfa_secret, \
            mfa_method, recovery_codes, is_active, openid_sub, from_ldap, ldap_pass_randomized, \
            ldap_rdn, ldap_user_path, enrollment_pending \
            FROM \"user\" WHERE email NOT IN (SELECT * FROM UNNEST($1::TEXT[]))",
        )
        .bind(user_emails)
        .fetch_all(executor)
        .await
    }

    pub async fn is_admin<'e, E>(&self, executor: E) -> Result<bool, SqlxError>
    where
        E: PgExecutor<'e>,
    {
        query_scalar!("SELECT EXISTS (SELECT 1 FROM group_user gu LEFT JOIN \"group\" g ON gu.group_id = g.id \
        WHERE is_admin = true AND user_id = $1) \"bool!\"", self.id)
            .fetch_one(executor)
            .await
    }

    /// Find all users that are admins and are active.
    pub async fn find_admins<'e, E>(executor: E) -> Result<Vec<Self>, SqlxError>
    where
        E: PgExecutor<'e>,
    {
        query_as!(
            Self,
            "
            SELECT u.id, u.username, u.password_hash, u.last_name, u.first_name, u.email, \
            u.phone, u.mfa_enabled, u.totp_enabled, u.email_mfa_enabled, \
            u.totp_secret, u.email_mfa_secret, u.mfa_method \"mfa_method: _\", u.recovery_codes, u.is_active, u.openid_sub, \
            from_ldap, ldap_pass_randomized, ldap_rdn, ldap_user_path, enrollment_pending \
            FROM \"user\" u \
            WHERE EXISTS (SELECT 1 FROM group_user gu LEFT JOIN \"group\" g ON gu.group_id = g.id \
            WHERE is_admin = true AND user_id = u.id) AND u.is_active = true"
        )
        .fetch_all(executor)
        .await
    }
}

impl Distribution<User<Id>> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> User<Id> {
        User {
            id: rng.r#gen(),
            username: Alphanumeric.sample_string(rng, 8),
            password_hash: rng
                .r#gen::<bool>()
                .then_some(Alphanumeric.sample_string(rng, 8)),
            last_name: Alphanumeric.sample_string(rng, 8),
            first_name: Alphanumeric.sample_string(rng, 8),
            email: format!("{}@defguard.net", Alphanumeric.sample_string(rng, 6)),
            // FIXME: generate an actual phone number
            phone: rng
                .r#gen::<bool>()
                .then_some(Alphanumeric.sample_string(rng, 9)),
            mfa_enabled: rng.r#gen(),
            is_active: true,
            openid_sub: rng
                .r#gen::<bool>()
                .then_some(Alphanumeric.sample_string(rng, 8)),
            totp_enabled: rng.r#gen(),
            email_mfa_enabled: rng.r#gen(),
            totp_secret: (0..20).map(|_| rng.r#gen()).collect(),
            email_mfa_secret: (0..20).map(|_| rng.r#gen()).collect(),
            mfa_method: match rng.r#gen_range(0..4) {
                0 => MFAMethod::None,
                1 => MFAMethod::Webauthn,
                2 => MFAMethod::OneTimePassword,
                _ => MFAMethod::Email,
            },
            recovery_codes: (0..3).map(|_| Alphanumeric.sample_string(rng, 6)).collect(),
            from_ldap: false,
            ldap_pass_randomized: false,
            ldap_rdn: None,
            ldap_user_path: None,
            enrollment_pending: false,
        }
    }
}

impl Distribution<User<NoId>> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> User<NoId> {
        User {
            id: NoId,
            username: Alphanumeric.sample_string(rng, 8),
            password_hash: rng
                .r#gen::<bool>()
                .then_some(Alphanumeric.sample_string(rng, 8)),
            last_name: Alphanumeric.sample_string(rng, 8),
            first_name: Alphanumeric.sample_string(rng, 8),
            email: format!("{}@defguard.net", Alphanumeric.sample_string(rng, 6)),
            // FIXME: generate an actual phone number
            phone: rng
                .r#gen::<bool>()
                .then_some(Alphanumeric.sample_string(rng, 9)),
            mfa_enabled: rng.r#gen(),
            is_active: true,
            openid_sub: rng
                .r#gen::<bool>()
                .then_some(Alphanumeric.sample_string(rng, 8)),
            totp_enabled: rng.r#gen(),
            email_mfa_enabled: rng.r#gen(),
            totp_secret: (0..20).map(|_| rng.r#gen()).collect(),
            email_mfa_secret: (0..20).map(|_| rng.r#gen()).collect(),
            mfa_method: match rng.r#gen_range(0..4) {
                0 => MFAMethod::None,
                1 => MFAMethod::Webauthn,
                2 => MFAMethod::OneTimePassword,
                _ => MFAMethod::Email,
            },
            recovery_codes: (0..3).map(|_| Alphanumeric.sample_string(rng, 6)).collect(),
            from_ldap: false,
            ldap_pass_randomized: false,
            ldap_rdn: None,
            ldap_user_path: None,
            enrollment_pending: false,
        }
    }
}

#[cfg(test)]
mod test {
    use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

    use super::*;
    use crate::{
        config::{DefGuardConfig, SERVER_CONFIG},
        db::{models::settings::initialize_current_settings, setup_pool},
    };

    #[sqlx::test]
    async fn test_mfa_code(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;

        let config = DefGuardConfig::new_test_config();
        let _ = SERVER_CONFIG.set(config.clone());
        initialize_current_settings(&pool).await.unwrap();

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
    async fn test_user(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;

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
    async fn test_all_users(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;

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
    async fn test_recovery_codes(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;

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
        assert!(
            !user
                .verify_recovery_code(&pool, "invalid code")
                .await
                .unwrap()
        );
        let codes = user.recovery_codes.clone();
        for code in &codes {
            assert!(user.verify_recovery_code(&pool, code).await.unwrap());
        }
        assert_eq!(user.recovery_codes.len(), 0);
    }

    #[sqlx::test]
    async fn test_email_case_insensitivity(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;

        let harry = User::new(
            "hpotter",
            Some("pass123"),
            "Potter",
            "Harry",
            "h.potter@hogwart.edu.uk",
            None,
        );
        assert!(harry.save(&pool).await.is_ok());

        let henry = User::new(
            "h.potter",
            Some("pass123"),
            "Potter",
            "Henry",
            "h.potter@hogwart.edu.uk",
            None,
        );
        assert!(henry.save(&pool).await.is_err());
    }

    #[sqlx::test]
    async fn test_is_admin(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;

        let config = DefGuardConfig::new_test_config();
        let _ = SERVER_CONFIG.set(config.clone());

        let user = User::new(
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

        let is_admin = user.is_admin(&pool).await.unwrap();

        assert!(!is_admin);

        query!(
            "INSERT INTO group_user (group_id, user_id) VALUES (1, $1)",
            user.id
        )
        .execute(&pool)
        .await
        .unwrap();

        let is_admin = user.is_admin(&pool).await.unwrap();

        assert!(is_admin);
    }

    #[sqlx::test]
    async fn test_find_admins(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;

        let config = DefGuardConfig::new_test_config();
        let _ = SERVER_CONFIG.set(config.clone());

        let user = User::new(
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

        let user2 = User::new(
            "hpotter2",
            Some("pass123"),
            "Potter",
            "Harry",
            "h.potter2@hogwart.edu.uk",
            None,
        )
        .save(&pool)
        .await
        .unwrap();

        User::new(
            "hpotter3",
            Some("pass123"),
            "Potter",
            "Harry",
            "h.potter3@hogwart.edu.uk",
            None,
        )
        .save(&pool)
        .await
        .unwrap();

        query!(
            "INSERT INTO group_user (group_id, user_id) VALUES (1, $1), (1, $2)",
            user.id,
            user2.id,
        )
        .execute(&pool)
        .await
        .unwrap();

        let admins = User::find_admins(&pool).await.unwrap();
        assert_eq!(admins.len(), 2);
        assert!(admins.iter().any(|u| u.id == user.id));
        assert!(admins.iter().any(|u| u.id == user2.id));
    }

    #[sqlx::test]
    async fn test_get_missing(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;

        let user1 = User::new(
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
        let user2 = User::new(
            "hpotter2",
            Some("pass1234"),
            "Potter2",
            "Harry2",
            "h.potter2@hogwart.edu.uk",
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

        let user_emails = vec![user1.email.as_str(), albus.email.as_str()];
        let users = User::exclude(&pool, &user_emails).await.unwrap();
        assert_eq!(users.len(), 1);
        assert_eq!(users[0].id, user2.id);
    }

    #[sqlx::test]
    async fn test_find_many_by_emails(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;

        let user1 = User::new(
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
        User::new(
            "hpotter2",
            Some("pass1234"),
            "Potter2",
            "Harry2",
            "h.potter2@hogwart.edu.uk",
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

        let user_emails = vec![user1.email.as_str(), albus.email.as_str()];
        let users = User::find_many_by_emails(&pool, &user_emails)
            .await
            .unwrap();
        assert_eq!(users.len(), 2);
        assert_eq!(users[0].id, user1.id);
        assert_eq!(users[1].id, albus.id);
    }

    #[sqlx::test]
    async fn test_user_is_enrolled(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;
        let user = User::new(
            "test",
            Some("31071980"),
            "harry",
            "potter",
            "harry@hogwart.edu.uk",
            None,
        );
        let mut user = user.save(&pool).await.unwrap();

        user.enrollment_pending = false;
        user.password_hash = Some(hash_password("31071980").unwrap());
        user.openid_sub = Some("sub".to_string());
        user.from_ldap = true;
        user.save(&pool).await.unwrap();
        assert!(user.is_enrolled());

        user.enrollment_pending = false;
        user.password_hash = None;
        user.openid_sub = Some("sub".to_string());
        user.from_ldap = true;
        user.save(&pool).await.unwrap();
        assert!(user.is_enrolled());

        user.enrollment_pending = false;
        user.password_hash = None;
        user.openid_sub = None;
        user.from_ldap = true;
        user.save(&pool).await.unwrap();
        assert!(user.is_enrolled());

        user.enrollment_pending = false;
        user.password_hash = None;
        user.openid_sub = None;
        user.from_ldap = false;
        user.save(&pool).await.unwrap();
        assert!(!user.is_enrolled());

        user.enrollment_pending = true;
        user.password_hash = None;
        user.openid_sub = None;
        user.from_ldap = false;
        user.save(&pool).await.unwrap();
        assert!(!user.is_enrolled());

        user.enrollment_pending = true;
        user.password_hash = Some(hash_password("31071980").unwrap());
        user.openid_sub = Some("sub".to_string());
        user.from_ldap = true;
        user.save(&pool).await.unwrap();
        assert!(!user.is_enrolled());
    }
}
