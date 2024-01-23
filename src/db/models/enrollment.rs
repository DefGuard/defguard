use chrono::{Duration, NaiveDateTime, Utc};
use reqwest::Url;
use sqlx::{query, query_as, Error as SqlxError, PgConnection, PgExecutor};
use tera::{Context, Tera};
use thiserror::Error;
use tokio::sync::mpsc::UnboundedSender;
use tonic::{Code, Status};

use super::{settings::Settings, DbPool, User};
use crate::{
    mail::Mail,
    random::gen_alphanumeric,
    templates::{self, TemplateError},
    SERVER_CONFIG, VERSION,
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
            | TokenError::NotificationError(_)
            | TokenError::WelcomeMsgNotConfigured
            | TokenError::WelcomeEmailNotConfigured
            | TokenError::TemplateError(_)
            | TokenError::TemplateErrorInternal(_) => (Code::Internal, "unexpected error"),
            TokenError::NotFound
            | TokenError::TokenExpired
            | TokenError::SessionExpired
            | TokenError::TokenUsed => (Code::Unauthenticated, "invalid token"),
            TokenError::AlreadyActive => (Code::InvalidArgument, "already active"),
        };
        Status::new(code, msg)
    }
}

// Representation of a user enrollment session
#[derive(Clone)]
pub struct Token {
    pub id: String,
    pub user_id: i64,
    pub admin_id: Option<i64>,
    pub email: Option<String>,
    pub created_at: NaiveDateTime,
    pub expires_at: NaiveDateTime,
    pub used_at: Option<NaiveDateTime>,
    pub token_type: Option<String>,
}

impl Token {
    #[must_use]
    pub fn new(
        user_id: i64,
        admin_id: Option<i64>,
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
        }
    }

    pub async fn save(&self, transaction: &mut PgConnection) -> Result<(), TokenError> {
        query!(
            "INSERT INTO token (id, user_id, admin_id, email, created_at, expires_at, used_at, token_type) \
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
            self.id,
            self.user_id,
            self.admin_id,
            self.email,
            self.created_at,
            self.expires_at,
            self.used_at,
            self.token_type,
        )
        .execute(transaction)
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
        if self.is_expired() {
            return Err(TokenError::TokenExpired);
        }
        if self.is_used() {
            return Err(TokenError::TokenUsed);
        }

        let now = Utc::now().naive_utc();
        query!("UPDATE token SET used_at = $1 WHERE id = $2", now, self.id)
            .execute(transaction)
            .await?;
        self.used_at = Some(now);

        Ok(now + Duration::seconds(session_timeout_seconds as i64))
    }

    pub async fn find_by_id(pool: &DbPool, id: &str) -> Result<Self, TokenError> {
        match query_as!(
            Self,
            "SELECT id, user_id, admin_id, email, created_at, expires_at, used_at, token_type \
            FROM token WHERE id = $1",
            id
        )
        .fetch_optional(pool)
        .await?
        {
            Some(enrollment) => Ok(enrollment),
            None => Err(TokenError::NotFound),
        }
    }

    pub async fn fetch_all(pool: &DbPool) -> Result<Vec<Self>, TokenError> {
        let tokens = query_as!(
            Self,
            "SELECT id, user_id, admin_id, email, created_at, expires_at, used_at, token_type \
            FROM token",
        )
        .fetch_all(pool)
        .await?;
        Ok(tokens)
    }

    pub async fn fetch_user<'e, E>(&self, executor: E) -> Result<User, TokenError>
    where
        E: PgExecutor<'e>,
    {
        debug!("Fetching user for enrollment");
        let Some(user) = User::find_by_id(executor, self.user_id).await? else {
            error!("User not found for enrollment token {}", self.id);
            return Err(TokenError::UserNotFound);
        };
        Ok(user)
    }

    pub async fn fetch_admin<'e, E>(&self, executor: E) -> Result<Option<User>, TokenError>
    where
        E: PgExecutor<'e>,
    {
        debug!("Fetching admin for enrollment");
        if self.admin_id.is_none() {
            return Ok(None);
        }

        let admin_id = self.admin_id.unwrap();
        let user = User::find_by_id(executor, admin_id).await?;
        Ok(user)
    }

    pub async fn delete_unused_user_tokens(
        transaction: &mut PgConnection,
        user_id: i64,
    ) -> Result<(), TokenError> {
        debug!("Deleting unused enrollment tokens for user {user_id}");
        let result = query!(
            "DELETE FROM token \
            WHERE user_id = $1 \
            AND used_at IS NULL",
            user_id
        )
        .execute(transaction)
        .await?;
        debug!(
            "Deleted {} unused enrollment tokens for user {user_id}",
            result.rows_affected()
        );

        Ok(())
    }

    pub async fn delete_unused_user_password_reset_tokens(
        transaction: &mut PgConnection,
        user_id: i64,
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
        context.insert("defguard_url", &SERVER_CONFIG.get().unwrap().url);
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

impl User {
    /// Start user enrollment process
    /// This creates a new enrollment token valid for 24h
    /// and optionally sends enrollment email notification to user
    pub async fn start_enrollment(
        &self,
        transaction: &mut PgConnection,
        admin: &User,
        email: Option<String>,
        token_timeout_seconds: u64,
        enrollment_service_url: Url,
        send_user_notification: bool,
        mail_tx: UnboundedSender<Mail>,
    ) -> Result<String, TokenError> {
        info!(
            "User {} starting enrollment for user {}, notification enabled: {send_user_notification}",
            admin.username, self.username
        );
        if self.has_password() {
            return Err(TokenError::AlreadyActive);
        }

        let user_id = self.id.expect("User without ID");
        let admin_id = admin.id.expect("Admin user without ID");

        self.clear_unused_enrollment_tokens(&mut *transaction)
            .await?;

        let enrollment = Token::new(
            user_id,
            Some(admin_id),
            email.clone(),
            token_timeout_seconds,
            Some(ENROLLMENT_TOKEN_TYPE.to_string()),
        );
        enrollment.save(&mut *transaction).await?;

        if send_user_notification {
            if let Some(email) = email {
                debug!(
                    "Sending enrollment start mail for user {} to {email}",
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
                    .map_err(|err| TokenError::NotificationError(err.to_string()))?,
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

        Ok(enrollment.id)
    }
    /// Start user remote desktop configuration process
    /// This creates a new enrollment token valid for 24h
    /// and optionally sends email notification to user
    pub async fn start_remote_desktop_configuration(
        &self,
        transaction: &mut PgConnection,
        admin: &User,
        email: Option<String>,
        token_timeout_seconds: u64,
        enrollment_service_url: Url,
        send_user_notification: bool,
        mail_tx: UnboundedSender<Mail>,
    ) -> Result<String, TokenError> {
        info!(
            "User {} starting desktop configuration for user {}, notification enabled: {send_user_notification}",
            admin.username, self.username
        );

        let user_id = self.id.expect("User without ID");
        let admin_id = admin.id.expect("Admin user without ID");

        self.clear_unused_enrollment_tokens(&mut *transaction)
            .await?;

        let enrollment = Token::new(
            user_id,
            Some(admin_id),
            email.clone(),
            token_timeout_seconds,
            Some(ENROLLMENT_TOKEN_TYPE.to_string()),
        );
        enrollment.save(&mut *transaction).await?;

        if send_user_notification {
            if let Some(email) = email {
                debug!(
                    "Sending desktop configuration start mail for user {} to {email}",
                    self.username
                );
                let base_message_context = enrollment
                    .get_welcome_message_context(&mut *transaction)
                    .await?;
                let mail = Mail {
                    to: email.clone(),
                    subject: DESKTOP_START_MAIL_SUBJECT.to_string(),
                    content: templates::desktop_start_mail(
                        base_message_context,
                        &enrollment_service_url,
                        &enrollment.id,
                    )
                    .map_err(|err| TokenError::NotificationError(err.to_string()))?,
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

        Ok(enrollment.id)
    }

    // Remove unused tokens when triggering user enrollment
    async fn clear_unused_enrollment_tokens(
        &self,
        transaction: &mut PgConnection,
    ) -> Result<(), TokenError> {
        info!(
            "Removing unused enrollment tokens for user {}",
            self.username
        );
        Token::delete_unused_user_tokens(transaction, self.id.expect("Missing user ID")).await
    }

    // pub async fn request_password_reset(
    //     &self,
    //     transaction: &mut PgConnection,
    //     admin: &User,
    //     // email: Option<String>,
    //     token_timeout_seconds: u64,
    //     // enrollment_service_url: Url,
    //     // send_user_notification: bool,
    //     mail_tx: UnboundedSender<Mail>,
    // ) -> Result<String, EnrollmentError> {

    // }
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
