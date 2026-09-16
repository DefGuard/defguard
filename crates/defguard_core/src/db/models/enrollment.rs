use std::{fmt, time::Duration};

use chrono::{NaiveDateTime, TimeDelta, Utc};
use defguard_common::{
    VERSION,
    db::{
        Id,
        models::{Settings, WebAuthn, user::User},
    },
    random::gen_alphanumeric,
    types::UrlParseError,
};
use sqlx::{PgConnection, PgExecutor, PgPool, query, query_as, types::Uuid};
use tera::Context;
use thiserror::Error;
use tonic::{Code, Status};
use webauthn_rs::prelude::{PasskeyRegistration, RegisterPublicKeyCredential};

use crate::mail::templates;

pub static ENROLLMENT_TOKEN_TYPE: &str = "ENROLLMENT";
pub static PASSWORD_RESET_TOKEN_TYPE: &str = "PASSWORD_RESET";
pub static MFA_CONFIG_TOKEN_TYPE: &str = "MFA_CONFIG";
// One window covers both the time to authorize and the time to configure factors.
pub const MFA_CONFIG_SESSION_TIMEOUT: Duration = Duration::from_secs(60 * 60);

#[derive(Error, Debug)]
pub enum TokenError {
    #[error(transparent)]
    DbError(#[from] sqlx::Error),
    #[error("Enrollment token not found")]
    NotFound,
    #[error("Enrollment token expired")]
    TokenExpired,
    #[error("Enrollment session expired")]
    SessionExpired,
    #[error("Enrollment token already used")]
    TokenUsed,
    #[error("Enrollment user not found")]
    UserNotFound,
    #[error("Enrollment user is disabled")]
    UserDisabled,
    #[error("Enrollment admin not found")]
    AdminNotFound,
    #[error("User account is already activated")]
    AlreadyActive,
    #[error("Enrollment welcome message not configured")]
    WelcomeMsgNotConfigured,
    #[error("Enrollment welcome email not configured")]
    WelcomeEmailNotConfigured,
    #[error(transparent)]
    TemplateErrorInternal(#[from] tera::Error),
    #[error(transparent)]
    TemplateError(#[from] templates::TemplateError),
    #[error(transparent)]
    UrlParseError(#[from] UrlParseError),
    #[error("Failed to serialize MFA setup state: {0}")]
    MfaSetupStateSerialization(String),
    #[error("WebAuthn configuration error: {0}")]
    WebauthnConfig(String),
    #[error("WebAuthn registration error: {0}")]
    WebauthnRegistration(String),
}

impl From<TokenError> for Status {
    fn from(err: TokenError) -> Self {
        error!("{err}");
        let unexpected_err_msg = format!("Unexpected error: {err}");
        let (code, msg) = match err {
            TokenError::DbError(_)
            | TokenError::AdminNotFound
            | TokenError::UserNotFound
            | TokenError::UserDisabled
            | TokenError::WelcomeMsgNotConfigured
            | TokenError::WelcomeEmailNotConfigured
            | TokenError::TemplateError(_)
            | TokenError::UrlParseError(_)
            | TokenError::TemplateErrorInternal(_)
            | TokenError::MfaSetupStateSerialization(_)
            | TokenError::WebauthnConfig(_) => (Code::Internal, unexpected_err_msg.as_str()),
            TokenError::NotFound | TokenError::SessionExpired | TokenError::TokenUsed => {
                (Code::Unauthenticated, "invalid token")
            }
            TokenError::AlreadyActive => (Code::InvalidArgument, "already active"),
            TokenError::WebauthnRegistration(ref msg) => (Code::InvalidArgument, msg.as_str()),
            TokenError::TokenExpired => (Code::Unauthenticated, "token expired"),
        };
        Self::new(code, msg)
    }
}

// Representation of a user enrollment session
#[derive(Clone)]
pub struct Token {
    pub id: String,
    pub user_id: Id,
    pub admin_id: Option<Id>,
    pub email: Option<String>,
    pub created_at: NaiveDateTime,
    pub expires_at: NaiveDateTime,
    pub used_at: Option<NaiveDateTime>,
    pub token_type: Option<String>,
    pub device_id: Option<Id>,
    // In-progress WebAuthn PasskeyRegistration (CBOR-serialized) for a FIDO2
    // CodeMfaSetup ceremony; NULL for code-based methods and once a ceremony
    // completes. See `set_passkey_registration` / `get_passkey_registration`.
    pub mfa_setup_state: Option<Vec<u8>>,
}

impl fmt::Debug for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Token")
            .field("id", &"<redacted>")
            .field("user_id", &self.user_id)
            .field("admin_id", &self.admin_id)
            .field("created_at", &self.created_at)
            .field("expires_at", &self.expires_at)
            .field("used_at", &self.used_at)
            .field("token_type", &self.token_type)
            .field("device_id", &self.device_id)
            .finish()
    }
}

impl Token {
    #[must_use]
    pub fn new(
        user_id: Id,
        admin_id: Option<Id>,
        email: Option<String>,
        token_timeout_seconds: u64,
        token_type: Option<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: gen_alphanumeric(32),
            user_id,
            admin_id,
            email,
            created_at: now.naive_utc(),
            expires_at: (now + TimeDelta::seconds(token_timeout_seconds as i64)).naive_utc(),
            used_at: None,
            token_type,
            device_id: None,
            mfa_setup_state: None,
        }
    }

    /// Duration for which the token is valid, i.e. `expires_at - created_at`, clamped to zero.
    #[must_use]
    pub fn validity_duration(&self) -> Duration {
        let seconds = (self.expires_at - self.created_at).num_seconds().max(0);
        Duration::from_secs(u64::try_from(seconds).unwrap_or_default())
    }

    pub async fn save<'e, E>(&self, executor: E) -> Result<(), TokenError>
    where
        E: PgExecutor<'e>,
    {
        query!(
            "INSERT INTO token (id, user_id, admin_id, email, created_at, expires_at, used_at, \
            token_type, device_id) \
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
            self.id,
            self.user_id,
            self.admin_id,
            self.email,
            self.created_at,
            self.expires_at,
            self.used_at,
            self.token_type,
            self.device_id
        )
        .execute(executor)
        .await?;
        Ok(())
    }

    /// Persist the in-progress WebAuthn `PasskeyRegistration` for a FIDO2
    /// CodeMfaSetup ceremony (CBOR-serialized), mirroring
    /// `Session::set_passkey_registration` on the REST path.
    pub async fn set_passkey_registration<'e, E>(
        &mut self,
        executor: E,
        passkey_reg: &PasskeyRegistration,
    ) -> Result<(), TokenError>
    where
        E: PgExecutor<'e>,
    {
        let mfa_setup_state = serde_cbor::to_vec(passkey_reg)
            .map_err(|err| TokenError::MfaSetupStateSerialization(err.to_string()))?;
        query!(
            "UPDATE token SET mfa_setup_state = $1 WHERE id = $2",
            mfa_setup_state,
            self.id
        )
        .execute(executor)
        .await?;
        self.mfa_setup_state = Some(mfa_setup_state);
        Ok(())
    }

    /// Deserialize the stored in-progress `PasskeyRegistration`, if any.
    #[must_use]
    pub fn get_passkey_registration(&self) -> Option<PasskeyRegistration> {
        self.mfa_setup_state
            .as_ref()
            .and_then(|state| serde_cbor::from_slice(state).ok())
    }

    /// Clear the stored ceremony state, e.g. after a successful FIDO2 setup.
    pub async fn clear_mfa_setup_state<'e, E>(&mut self, executor: E) -> Result<(), TokenError>
    where
        E: PgExecutor<'e>,
    {
        query!(
            "UPDATE token SET mfa_setup_state = NULL WHERE id = $1",
            self.id
        )
        .execute(executor)
        .await?;
        self.mfa_setup_state = None;
        Ok(())
    }

    /// Begin a FIDO2 (WebAuthn) registration ceremony for the setup session.
    ///
    /// Builds a creation challenge, stores the in-progress `PasskeyRegistration`
    /// on this token, and returns the `CreationChallengeResponse` as a JSON
    /// string to hand to the client's authenticator. Mirrors the REST
    /// `webauthn_init` handler but persists state on the token, not a session.
    pub async fn start_fido2_setup(
        &mut self,
        pool: &PgPool,
        user: &User<Id>,
    ) -> Result<String, TokenError> {
        let passkeys = WebAuthn::passkeys_for_user(pool, user.id).await?;
        let webauthn = Settings::get_current_settings()
            .build_webauthn()
            .map_err(|err| TokenError::WebauthnConfig(err.to_string()))?;
        let (ccr, passkey_reg) = webauthn
            .start_passkey_registration(
                Uuid::new_v4(),
                &user.username,
                &user.username,
                Some(passkeys.iter().map(|key| key.cred_id().clone()).collect()),
            )
            .map_err(|err| TokenError::WebauthnRegistration(err.to_string()))?;
        self.set_passkey_registration(pool, &passkey_reg).await?;
        serde_json::to_string(&ccr).map_err(|err| TokenError::WebauthnRegistration(err.to_string()))
    }

    /// Complete a FIDO2 (WebAuthn) registration ceremony for the setup session.
    ///
    /// Verifies the client's attestation against the stored challenge, persists
    /// the new security key, and clears the ceremony state. Returns the saved
    /// [`WebAuthn`] record. Mirrors the REST `webauthn_finish` handler.
    pub async fn finish_fido2_setup(
        &mut self,
        pool: &PgPool,
        user_id: Id,
        name: String,
        attestation_json: &str,
    ) -> Result<WebAuthn<Id>, TokenError> {
        let webauthn = Settings::get_current_settings()
            .build_webauthn()
            .map_err(|err| TokenError::WebauthnConfig(err.to_string()))?;
        let passkey_reg = self.get_passkey_registration().ok_or_else(|| {
            TokenError::WebauthnRegistration("Passkey registration session not found".into())
        })?;
        let rpkc: RegisterPublicKeyCredential =
            serde_json::from_str(attestation_json).map_err(|_| {
                TokenError::WebauthnRegistration("Failed to parse registration attestation".into())
            })?;
        let passkey = webauthn
            .finish_passkey_registration(&rpkc, &passkey_reg)
            .map_err(|err| TokenError::WebauthnRegistration(err.to_string()))?;
        let webauthn_key = WebAuthn::new(user_id, name, &passkey)
            .map_err(|err| TokenError::WebauthnRegistration(err.to_string()))?
            .save(pool)
            .await
            .map_err(|err| TokenError::WebauthnRegistration(err.to_string()))?;
        self.clear_mfa_setup_state(pool).await?;
        Ok(webauthn_key)
    }

    // check if token has already expired
    #[must_use]
    pub fn is_expired(&self) -> bool {
        self.expires_at < Utc::now().naive_utc()
    }

    // check if token has already been used
    #[must_use]
    pub fn is_used(&self) -> bool {
        self.used_at.is_some()
    }

    // check if enrollment session is still valid
    // after using the token user has 10 minutes to complete enrollment
    #[must_use]
    pub fn is_session_valid(&self, session_timeout_seconds: u64) -> bool {
        if let Some(used_at) = self.used_at {
            let now = Utc::now();
            return now.naive_utc()
                < (used_at + TimeDelta::seconds(session_timeout_seconds as i64));
        }
        false
    }

    // check if token can be used to start an enrollment session
    // and set timestamp if token is valid
    // returns session deadline
    pub async fn start_session(
        &mut self,
        transaction: &mut PgConnection,
        session_timeout_seconds: u64,
    ) -> Result<NaiveDateTime, TokenError> {
        // check if token can be used
        debug!("Creating a new session.");
        if self.is_expired() {
            debug!("Token is already expired. Cannot establish a new session.");
            return Err(TokenError::TokenExpired);
        }
        match self.used_at {
            // session started but still valid
            Some(used_at) if self.is_session_valid(session_timeout_seconds) => {
                debug!("Session already exists yet it is still valid.");
                Ok(used_at + TimeDelta::seconds(session_timeout_seconds as i64))
            }
            // session expired
            Some(_) => {
                debug!("Session has expired.");
                Err(TokenError::TokenUsed)
            }
            // session not yet started
            None => {
                let now = Utc::now().naive_utc();
                query!("UPDATE token SET used_at = $1 WHERE id = $2", now, self.id)
                    .execute(transaction)
                    .await?;
                self.used_at = Some(now);

                debug!("Generate a new session successfully.");
                Ok(now + TimeDelta::seconds(session_timeout_seconds as i64))
            }
        }
    }

    pub async fn find_by_id(pool: &PgPool, id: &str) -> Result<Self, TokenError> {
        if let Some(enrollment) = query_as!(
            Self,
            "SELECT id, user_id, admin_id, email, created_at, expires_at, used_at, token_type, device_id, \
            mfa_setup_state \
            FROM token WHERE id = $1",
            id
        )
        .fetch_optional(pool)
        .await?
        {
            debug!("Fetch token {enrollment:?} from database.");
            Ok(enrollment)
        } else {
            debug!("Token with id {id} does not exist in database.");
            Err(TokenError::NotFound)
        }
    }

    pub async fn fetch_all(pool: &PgPool) -> Result<Vec<Self>, TokenError> {
        let tokens = query_as!(
            Self,
            "SELECT id, user_id, admin_id, email, created_at, expires_at, used_at, token_type, device_id, \
            mfa_setup_state \
            FROM token",
        )
        .fetch_all(pool)
        .await?;
        Ok(tokens)
    }

    pub async fn fetch_user<'e, E>(&self, executor: E) -> Result<User<Id>, TokenError>
    where
        E: PgExecutor<'e>,
    {
        debug!("Find user by id {}.", self.user_id);
        let Some(user) = User::find_by_id(executor, self.user_id).await? else {
            error!(
                "User not found for enrollment token of user {}",
                self.user_id
            );
            return Err(TokenError::UserNotFound);
        };
        debug!("Fetched user {user:?}.");

        Ok(user)
    }

    pub async fn fetch_admin<'e, E>(&self, executor: E) -> Result<Option<User<Id>>, TokenError>
    where
        E: PgExecutor<'e>,
    {
        debug!("Fetch admin data");
        if self.admin_id.is_none() {
            debug!("Admin doesn't have ID; stop fetching data");
            return Ok(None);
        }

        let admin_id = self.admin_id.unwrap();
        debug!("Trying to find admin using ID {admin_id}");
        let user = User::find_by_id(executor, admin_id).await?;
        debug!("Fetched admin {user:?}.");

        Ok(user)
    }

    pub async fn delete_unused_user_tokens<'e, E>(
        executor: E,
        user_id: Id,
    ) -> Result<(), TokenError>
    where
        E: PgExecutor<'e>,
    {
        debug!("Deleting unused tokens for the user");
        let result = query!(
            "DELETE FROM token \
            WHERE user_id = $1 \
            AND used_at IS NULL",
            user_id
        )
        .execute(executor)
        .await?;
        info!(
            "Deleted {} unused enrollment tokens for the user",
            result.rows_affected()
        );

        Ok(())
    }

    pub async fn delete_unused_user_password_reset_tokens(
        transaction: &mut PgConnection,
        user_id: Id,
    ) -> Result<(), TokenError> {
        Self::delete_unused_user_tokens_of_type(transaction, user_id, PASSWORD_RESET_TOKEN_TYPE)
            .await
    }

    pub async fn delete_unused_user_tokens_of_type<'e, E>(
        executor: E,
        user_id: Id,
        token_type: &str,
    ) -> Result<(), TokenError>
    where
        E: PgExecutor<'e>,
    {
        debug!("Deleting unused {token_type} tokens for user {user_id}");
        // Plain query: the token type is a runtime parameter and needs no cached query data.
        let result =
            query("DELETE FROM token WHERE user_id = $1 AND token_type = $2 AND used_at IS NULL")
                .bind(user_id)
                .bind(token_type)
                .execute(executor)
                .await?;
        debug!(
            "Deleted {} unused {token_type} tokens for user {user_id}",
            result.rows_affected()
        );

        Ok(())
    }

    /// Prepare context for rendering welcome messages
    /// Available tags include:
    /// - first_name
    /// - last_name
    /// - username
    /// - defguard_url
    /// - defguard_version
    /// - admin_first_name
    /// - admin_last_name
    /// - admin_email
    /// - admin_phone
    pub(crate) async fn get_welcome_message_context(
        &self,
        conn: &mut PgConnection,
    ) -> Result<Context, TokenError> {
        debug!(
            "Preparing welcome message context for enrollment token of user {}",
            self.user_id
        );

        let user = self.fetch_user(&mut *conn).await?;
        let admin = self.fetch_admin(&mut *conn).await?;
        let url = Settings::url()?;
        let mut context = Context::new();
        context.insert("first_name", &user.first_name);
        context.insert("last_name", &user.last_name);
        context.insert("username", &user.username);
        context.insert("defguard_url", &url);
        context.insert("defguard_version", &VERSION);

        if let Some(admin) = admin {
            context.insert("admin_first_name", &admin.first_name);
            context.insert("admin_last_name", &admin.last_name);
            context.insert("admin_email", &admin.email);
            context.insert("admin_phone", &admin.phone);
        }

        Ok(context)
    }

    // Replace template tags and return markdown content
    // to be displayed on final enrollment page
    pub async fn get_welcome_page_content(
        &self,
        conn: &mut PgConnection,
    ) -> Result<String, TokenError> {
        let settings = Settings::get_current_settings();

        // load configured content as template
        let mut tera = templates::safe_tera();
        tera.add_raw_template("welcome_page", &enrollment_welcome_message(&settings)?)?;

        let context = self.get_welcome_message_context(&mut *conn).await?;

        Ok(tera.render("welcome_page", &context)?)
    }

    /// Send configured welcome email to a user after finishing enrollment.
    pub async fn send_welcome_email(
        &self,
        conn: &mut PgConnection,
        user: &User<Id>,
        ip_address: &str,
        device_info: Option<&str>,
    ) -> Result<(), TokenError> {
        debug!("Sending welcome mail to {}", user.username);
        let settings = Settings::get_current_settings();

        // load configured content as template
        let mut tera = templates::safe_tera();
        tera.add_raw_template("welcome_email", &enrollment_welcome_email(&settings)?)?;

        let context = self.get_welcome_message_context(conn).await?;
        let content = tera.render("welcome_email", &context)?;

        templates::enrollment_welcome_mail(&user.email, &content, Some(ip_address), device_info)?;

        Ok(())
    }
}

fn enrollment_welcome_message(settings: &Settings) -> Result<String, TokenError> {
    settings.enrollment_welcome_message.clone().ok_or_else(|| {
        error!("Enrollment welcome message not configured");
        TokenError::WelcomeMsgNotConfigured
    })
}

fn enrollment_welcome_email(settings: &Settings) -> Result<String, TokenError> {
    if settings.enrollment_use_welcome_message_as_email {
        return enrollment_welcome_message(settings);
    }
    settings.enrollment_welcome_email.clone().ok_or_else(|| {
        error!("Enrollment welcome email not configured");
        TokenError::WelcomeEmailNotConfigured
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_debug_masks_id() {
        let token = Token::new(
            7,
            Some(1),
            Some("user@example.com".to_owned()),
            60,
            Some(ENROLLMENT_TOKEN_TYPE.to_owned()),
        );
        let debug = format!("{token:?}");

        assert!(!debug.contains(&token.id));
        assert!(!debug.contains("user@example.com"));
        assert!(debug.contains("user_id: 7"));
        assert!(debug.contains(ENROLLMENT_TOKEN_TYPE));
    }
}
