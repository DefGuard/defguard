use chrono::{NaiveDateTime, TimeDelta, Utc};
use defguard_common::{
    db::{
        Id,
        models::{Settings, settings::defaults::WELCOME_EMAIL_SUBJECT, user::User},
    },
    random::gen_alphanumeric,
    types::UrlParseError,
};
use defguard_mail::{
    Mail,
    templates::{self, TemplateError, safe_tera},
};
use sqlx::{Error as SqlxError, PgConnection, PgExecutor, PgPool, Transaction, query, query_as};
use tera::Context;
use thiserror::Error;
use tonic::{Code, Status};

pub static ENROLLMENT_TOKEN_TYPE: &str = "ENROLLMENT";
pub static PASSWORD_RESET_TOKEN_TYPE: &str = "PASSWORD_RESET";

#[derive(Error, Debug)]
pub enum TokenError {
    #[error(transparent)]
    DbError(#[from] SqlxError),
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
    #[error("Failed to send enrollment notification: {0}")]
    NotificationError(String),
    #[error("Enrollment welcome message not configured")]
    WelcomeMsgNotConfigured,
    #[error("Enrollment welcome email not configured")]
    WelcomeEmailNotConfigured,
    #[error(transparent)]
    TemplateErrorInternal(#[from] tera::Error),
    #[error(transparent)]
    TemplateError(#[from] TemplateError),
    #[error(transparent)]
    UrlParseError(#[from] UrlParseError),
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
            | TokenError::NotificationError(_)
            | TokenError::WelcomeMsgNotConfigured
            | TokenError::WelcomeEmailNotConfigured
            | TokenError::TemplateError(_)
            | TokenError::UrlParseError(_)
            | TokenError::TemplateErrorInternal(_) => (Code::Internal, unexpected_err_msg.as_str()),
            TokenError::NotFound | TokenError::SessionExpired | TokenError::TokenUsed => {
                (Code::Unauthenticated, "invalid token")
            }
            TokenError::AlreadyActive => (Code::InvalidArgument, "already active"),
            TokenError::TokenExpired => (Code::Unauthenticated, "token expired"),
        };
        Status::new(code, msg)
    }
}

// Representation of a user enrollment session
#[derive(Clone, Debug)]
pub struct Token {
    pub id: String,
    pub user_id: Id,
    pub admin_id: Option<i64>,
    pub email: Option<String>,
    pub created_at: NaiveDateTime,
    pub expires_at: NaiveDateTime,
    pub used_at: Option<NaiveDateTime>,
    pub token_type: Option<String>,
    pub device_id: Option<Id>,
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
        }
    }

    pub async fn save<'e, E>(&self, executor: E) -> Result<(), TokenError>
    where
        E: PgExecutor<'e>,
    {
        query!(
            "INSERT INTO token (id, user_id, admin_id, email, created_at, expires_at, used_at, token_type, device_id) \
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
            "SELECT id, user_id, admin_id, email, created_at, expires_at, used_at, token_type, device_id \
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
            "SELECT id, user_id, admin_id, email, created_at, expires_at, used_at, token_type, device_id \
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
            error!("User not found for enrollment token {}", self.id);
            return Err(TokenError::UserNotFound);
        };
        debug!("Fetched user {user:?}.");

        Ok(user)
    }

    pub async fn fetch_admin<'e, E>(&self, executor: E) -> Result<Option<User<Id>>, TokenError>
    where
        E: PgExecutor<'e>,
    {
        debug!("Fetch admin data.");
        if self.admin_id.is_none() {
            debug!("Admin don't have id. Stop fetching data...");
            return Ok(None);
        }

        let admin_id = self.admin_id.unwrap();
        debug!("Trying to find admin using id {admin_id}");
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
        debug!("Deleting unused tokens for the user.");
        let result = query!(
            "DELETE FROM token \
            WHERE user_id = $1 \
            AND used_at IS NULL",
            user_id
        )
        .execute(executor)
        .await?;
        info!(
            "Deleted {} unused enrollment tokens for the user.",
            result.rows_affected()
        );

        Ok(())
    }

    pub async fn delete_unused_user_password_reset_tokens(
        transaction: &mut PgConnection,
        user_id: Id,
    ) -> Result<(), TokenError> {
        debug!("Deleting unused password reset tokens for user {user_id}");
        let result = query!(
            "DELETE FROM token \
            WHERE user_id = $1 \
            AND token_type = 'PASSWORD_RESET' \
            AND used_at IS NULL",
            user_id
        )
        .execute(transaction)
        .await?;
        debug!(
            "Deleted {} unused password reset tokens for user {user_id}",
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
        transaction: &mut PgConnection,
    ) -> Result<Context, TokenError> {
        debug!(
            "Preparing welcome message context for enrollment token {}",
            self.id
        );

        let user = self.fetch_user(&mut *transaction).await?;
        let admin = self.fetch_admin(&mut *transaction).await?;
        let url = Settings::url()?;
        let mut context = Context::new();
        context.insert("first_name", &user.first_name);
        context.insert("last_name", &user.last_name);
        context.insert("username", &user.username);
        context.insert("defguard_url", &url);

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
        transaction: &mut PgConnection,
    ) -> Result<String, TokenError> {
        let settings = Settings::get_current_settings();

        // load configured content as template
        let mut tera = safe_tera();
        tera.add_raw_template("welcome_page", &enrollment_welcome_message(&settings)?)?;

        let context = self.get_welcome_message_context(&mut *transaction).await?;

        Ok(tera.render("welcome_page", &context)?)
    }

    // Render welcome email content
    pub(crate) async fn get_welcome_email_content(
        &self,
        transaction: &mut PgConnection,
        ip_address: &str,
        device_info: Option<&str>,
    ) -> Result<String, TokenError> {
        let settings = Settings::get_current_settings();

        // load configured content as template
        let mut tera = safe_tera();
        tera.add_raw_template("welcome_email", &enrollment_welcome_email(&settings)?)?;

        let context = self.get_welcome_message_context(&mut *transaction).await?;
        let content = tera.render("welcome_email", &context)?;

        Ok(templates::enrollment_welcome_mail(
            &content,
            Some(ip_address),
            device_info,
        )?)
    }

    // Send configured welcome email to user after finishing enrollment
    pub async fn send_welcome_email(
        &self,
        transaction: &mut Transaction<'_, sqlx::Postgres>,
        user: &User<Id>,
        settings: &Settings,
        ip_address: &str,
        device_info: Option<&str>,
    ) -> Result<(), TokenError> {
        debug!("Sending welcome mail to {}", user.username);
        let mail = Mail::new(
            &user.email,
            settings
                .enrollment_welcome_email_subject
                .as_deref()
                .unwrap_or(WELCOME_EMAIL_SUBJECT),
            self.get_welcome_email_content(&mut *transaction, ip_address, device_info)
                .await?,
        );
        match mail.send().await {
            Ok(()) => {
                info!("Sent enrollment welcome mail to {}", user.username);
                Ok(())
            }
            Err(err) => {
                error!("Error sending welcome mail: {err}");
                Err(TokenError::NotificationError(err.to_string()))
            }
        }
    }

    // Notify admin that a user has completed enrollment
    pub async fn send_admin_notification(
        admin: &User<Id>,
        user: &User<Id>,
        ip_address: &str,
        device_info: Option<&str>,
    ) -> Result<(), TokenError> {
        debug!(
            "Sending enrollment success notification for user {} to {}",
            user.username, admin.username
        );
        let mail = Mail::new(
            &admin.email,
            "[defguard] User enrollment completed",
            templates::enrollment_admin_notification(
                &user.into(),
                &admin.into(),
                ip_address,
                device_info,
            )?,
        );
        match mail.send().await {
            Ok(()) => {
                info!(
                    "Sent enrollment success notification for user {} to {}",
                    user.username, admin.username
                );
                Ok(())
            }
            Err(err) => {
                error!("Error sending welcome mail: {err}");
                Err(TokenError::NotificationError(err.to_string()))
            }
        }
    }
}

pub fn enrollment_welcome_message(settings: &Settings) -> Result<String, TokenError> {
    settings.enrollment_welcome_message.clone().ok_or_else(|| {
        error!("Enrollment welcome message not configured");
        TokenError::WelcomeMsgNotConfigured
    })
}

pub fn enrollment_welcome_email(settings: &Settings) -> Result<String, TokenError> {
    if settings.enrollment_use_welcome_message_as_email {
        return enrollment_welcome_message(settings);
    }
    settings.enrollment_welcome_email.clone().ok_or_else(|| {
        error!("Enrollment welcome email not configured");
        TokenError::WelcomeEmailNotConfigured
    })
}
