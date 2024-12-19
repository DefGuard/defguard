use chrono::{Duration, NaiveDateTime, Utc};
use reqwest::Url;
use sqlx::{query, query_as, Error as SqlxError, PgConnection, PgExecutor, PgPool};
use tera::{Context, Tera};
use thiserror::Error;
use tokio::sync::mpsc::UnboundedSender;
use tonic::{Code, Status};

use super::{settings::Settings, User};
use crate::{
    db::Id,
    mail::Mail,
    random::gen_alphanumeric,
    server_config,
    templates::{self, TemplateError},
    VERSION,
};

pub static ENROLLMENT_TOKEN_TYPE: &str = "ENROLLMENT";
pub static PASSWORD_RESET_TOKEN_TYPE: &str = "PASSWORD_RESET";

static ENROLLMENT_START_MAIL_SUBJECT: &str = "Defguard user enrollment";
static DESKTOP_START_MAIL_SUBJECT: &str = "Defguard desktop client configuration";

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
}

impl From<TokenError> for Status {
    fn from(err: TokenError) -> Self {
        error!("{err}");
        let (code, msg) = match err {
            TokenError::DbError(_)
            | TokenError::AdminNotFound
            | TokenError::UserNotFound
            | TokenError::UserDisabled
            | TokenError::NotificationError(_)
            | TokenError::WelcomeMsgNotConfigured
            | TokenError::WelcomeEmailNotConfigured
            | TokenError::TemplateError(_)
            | TokenError::TemplateErrorInternal(_) => (Code::Internal, "unexpected error"),
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
            expires_at: (now + Duration::seconds(token_timeout_seconds as i64)).naive_utc(),
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
            return now.naive_utc() < (used_at + Duration::seconds(session_timeout_seconds as i64));
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
                Ok(used_at + Duration::seconds(session_timeout_seconds as i64))
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
                Ok(now + Duration::seconds(session_timeout_seconds as i64))
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
    pub async fn get_welcome_message_context(
        &self,
        transaction: &mut PgConnection,
    ) -> Result<Context, TokenError> {
        debug!(
            "Preparing welcome message context for enrollment token {}",
            self.id
        );

        let user = self.fetch_user(&mut *transaction).await?;
        let admin = self.fetch_admin(&mut *transaction).await?;

        let mut context = Context::new();
        context.insert("first_name", &user.first_name);
        context.insert("last_name", &user.last_name);
        context.insert("username", &user.username);
        context.insert("defguard_url", &server_config().url);
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
        transaction: &mut PgConnection,
    ) -> Result<String, TokenError> {
        let settings = Settings::get_settings(&mut *transaction).await?;

        // load configured content as template
        let mut tera = Tera::default();
        tera.add_raw_template("welcome_page", &settings.enrollment_welcome_message()?)?;

        let context = self.get_welcome_message_context(&mut *transaction).await?;

        Ok(tera.render("welcome_page", &context)?)
    }

    // Render welcome email content
    pub async fn get_welcome_email_content(
        &self,
        transaction: &mut PgConnection,
        ip_address: &str,
        device_info: Option<&str>,
    ) -> Result<String, TokenError> {
        let settings = Settings::get_settings(&mut *transaction).await?;

        // load configured content as template
        let mut tera = Tera::default();
        tera.add_raw_template("welcome_email", &settings.enrollment_welcome_email()?)?;

        let context = self.get_welcome_message_context(&mut *transaction).await?;
        let content = tera.render("welcome_email", &context)?;

        Ok(templates::enrollment_welcome_mail(
            &content,
            Some(ip_address),
            device_info,
        )?)
    }
}

impl User<Id> {
    /// Start user enrollment process
    /// This creates a new enrollment token valid for 24h
    /// and optionally sends enrollment email notification to user
    pub async fn start_enrollment(
        &self,
        transaction: &mut PgConnection,
        admin: &User<Id>,
        email: Option<String>,
        token_timeout_seconds: u64,
        enrollment_service_url: Url,
        send_user_notification: bool,
        mail_tx: UnboundedSender<Mail>,
    ) -> Result<String, TokenError> {
        info!(
            "User {} started a new enrollment process for user {}.",
            admin.username, self.username
        );
        debug!(
            "Notify user by mail about the enrollment process: {}",
            send_user_notification
        );
        debug!("Check if {} has a password.", self.username);
        if self.has_password() {
            debug!(
                "User {} that you want to start enrollment process for already has a password.",
                self.username
            );
            return Err(TokenError::AlreadyActive);
        }

        debug!("Verify that {} is an active user.", self.username);
        if !self.is_active {
            warn!(
                "Can't create enrollment token for disabled user {}",
                self.username
            );
            return Err(TokenError::UserDisabled);
        }

        self.clear_unused_enrollment_tokens(&mut *transaction)
            .await?;

        debug!("Create a new enrollment token for user {}.", self.username);
        let enrollment = Token::new(
            self.id,
            Some(admin.id),
            email.clone(),
            token_timeout_seconds,
            Some(ENROLLMENT_TOKEN_TYPE.to_string()),
        );
        debug!("Saving a new enrollment token...");
        enrollment.save(&mut *transaction).await?;
        debug!(
            "Saved a new enrollment token with id {} for user {}.",
            enrollment.id, self.username
        );

        if send_user_notification {
            if let Some(email) = email {
                debug!(
                    "Sending an enrollment mail for user {} to {email}.",
                    self.username
                );
                let base_message_context = enrollment
                    .get_welcome_message_context(&mut *transaction)
                    .await?;
                let mail = Mail {
                    to: email.clone(),
                    subject: ENROLLMENT_START_MAIL_SUBJECT.to_string(),
                    content: templates::enrollment_start_mail(
                        base_message_context,
                        enrollment_service_url,
                        &enrollment.id,
                    )
                    .map_err(|err| {
                        debug!(
                            "Cannot send an email to the user {} due to the error {}.",
                            self.username,
                            err.to_string()
                        );
                        TokenError::NotificationError(err.to_string())
                    })?,
                    attachments: Vec::new(),
                    result_tx: None,
                };
                match mail_tx.send(mail) {
                    Ok(()) => {
                        info!(
                            "Sent enrollment start mail for user {} to {email}",
                            self.username
                        );
                    }
                    Err(err) => {
                        error!("Error sending mail: {err}");
                        return Err(TokenError::NotificationError(err.to_string()));
                    }
                }
            }
        }
        info!(
            "New enrollment token has been generated for {}.",
            self.username
        );

        Ok(enrollment.id)
    }

    /// Start user remote desktop configuration process
    /// This creates a new enrollment token valid for 24h
    /// and optionally sends email notification to user
    pub async fn start_remote_desktop_configuration(
        &self,
        transaction: &mut PgConnection,
        admin: &User<Id>,
        email: Option<String>,
        token_timeout_seconds: u64,
        enrollment_service_url: Url,
        send_user_notification: bool,
        mail_tx: UnboundedSender<Mail>,
        // Whether to attach some device to the token. It allows for a partial initialization of
        // the device before the desktop configuration has taken place.
        device_id: Option<Id>,
    ) -> Result<String, TokenError> {
        info!(
            "User {} starting a new desktop activation for user {}",
            admin.username, self.username
        );
        debug!(
            "Notify {} by mail about the enrollment process: {}",
            self.username, send_user_notification
        );

        debug!("Verify that {} is an active user.", self.username);
        if !self.is_active {
            warn!(
                "Can't create desktop activation token for disabled user {}.",
                self.username
            );
            return Err(TokenError::UserDisabled);
        }

        self.clear_unused_enrollment_tokens(&mut *transaction)
            .await?;
        debug!("Cleared unused tokens for {}.", self.username);

        debug!(
            "Create a new desktop activation token for user {}.",
            self.username
        );
        let mut desktop_configuration = Token::new(
            self.id,
            Some(admin.id),
            email.clone(),
            token_timeout_seconds,
            Some(ENROLLMENT_TOKEN_TYPE.to_string()),
        );
        if let Some(device_id) = device_id {
            desktop_configuration.device_id = Some(device_id);
        }
        debug!("Saving a new desktop configuration token...");
        desktop_configuration.save(&mut *transaction).await?;
        debug!(
            "Saved a new desktop activation token with id {} for user {}.",
            desktop_configuration.id, self.username
        );

        if send_user_notification {
            if let Some(email) = email {
                debug!(
                    "Sending a desktop configuration mail for user {} to {email}",
                    self.username
                );
                let base_message_context = desktop_configuration
                    .get_welcome_message_context(&mut *transaction)
                    .await?;
                let mail = Mail {
                    to: email.clone(),
                    subject: DESKTOP_START_MAIL_SUBJECT.to_string(),
                    content: templates::desktop_start_mail(
                        base_message_context,
                        &enrollment_service_url,
                        &desktop_configuration.id,
                    )
                    .map_err(|err| {
                        debug!(
                            "Cannot send an email to the user {} due to the error {}.",
                            self.username,
                            err.to_string()
                        );
                        TokenError::NotificationError(err.to_string())
                    })?,
                    attachments: Vec::new(),
                    result_tx: None,
                };
                match mail_tx.send(mail) {
                    Ok(()) => {
                        info!(
                            "Sent desktop configuration start mail for user {} to {email}",
                            self.username
                        );
                    }
                    Err(err) => {
                        error!("Error sending mail: {err}");
                    }
                }
            }
        }
        info!(
            "New desktop activation token has been generated for {}.",
            self.username
        );

        Ok(desktop_configuration.id)
    }

    // Remove unused tokens when triggering user enrollment
    pub(crate) async fn clear_unused_enrollment_tokens<'e, E>(
        &self,
        executor: E,
    ) -> Result<(), TokenError>
    where
        E: PgExecutor<'e>,
    {
        info!("Removing unused tokens for user {}.", self.username);
        Token::delete_unused_user_tokens(executor, self.id).await
    }
}

impl Settings {
    pub fn enrollment_welcome_message(&self) -> Result<String, TokenError> {
        self.enrollment_welcome_message.clone().ok_or_else(|| {
            error!("Enrollment welcome message not configured");
            TokenError::WelcomeMsgNotConfigured
        })
    }

    pub fn enrollment_welcome_email(&self) -> Result<String, TokenError> {
        if self.enrollment_use_welcome_message_as_email {
            return self.enrollment_welcome_message();
        }
        self.enrollment_welcome_email.clone().ok_or_else(|| {
            error!("Enrollment welcome email not configured");
            TokenError::WelcomeEmailNotConfigured
        })
    }
}
