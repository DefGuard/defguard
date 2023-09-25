use super::{settings::Settings, DbPool, User};
use crate::{
    mail::Mail,
    random::gen_alphanumeric,
    templates::{self, TemplateError},
    SERVER_CONFIG, VERSION,
};
use chrono::{Duration, NaiveDateTime, Utc};
use reqwest::Url;
use sqlx::{query, query_as, Error as SqlxError, PgConnection};
use tera::{Context, Tera};
use thiserror::Error;
use tokio::sync::mpsc::UnboundedSender;
use tonic::{Code, Status};

const ENROLLMENT_START_MAIL_SUBJECT: &str = "Defguard user enrollment";
const DESKTOP_START_MAIL_SUBJECT: &str = "Defguard desktop client configuration";

#[derive(Error, Debug)]
pub enum EnrollmentError {
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

impl From<EnrollmentError> for Status {
    fn from(err: EnrollmentError) -> Self {
        error!("{}", err);
        let (code, msg) = match err {
            EnrollmentError::DbError(_)
            | EnrollmentError::AdminNotFound
            | EnrollmentError::UserNotFound
            | EnrollmentError::NotificationError(_)
            | EnrollmentError::WelcomeMsgNotConfigured
            | EnrollmentError::WelcomeEmailNotConfigured
            | EnrollmentError::TemplateError(_)
            | EnrollmentError::TemplateErrorInternal(_) => (Code::Internal, "unexpected error"),
            EnrollmentError::NotFound
            | EnrollmentError::TokenExpired
            | EnrollmentError::SessionExpired
            | EnrollmentError::TokenUsed => (Code::Unauthenticated, "invalid token"),
            EnrollmentError::AlreadyActive => (Code::InvalidArgument, "already active"),
        };
        Status::new(code, msg)
    }
}

// Representation of a user enrollment session
#[derive(Clone)]
pub struct Enrollment {
    pub id: String,
    pub user_id: i64,
    pub admin_id: i64,
    pub email: Option<String>,
    pub created_at: NaiveDateTime,
    pub expires_at: NaiveDateTime,
    pub used_at: Option<NaiveDateTime>,
}

impl Enrollment {
    #[must_use]
    pub fn new(
        user_id: i64,
        admin_id: i64,
        email: Option<String>,
        token_timeout_seconds: u64,
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
        }
    }

    pub async fn save(&self, transaction: &mut PgConnection) -> Result<(), EnrollmentError> {
        query!(
            "INSERT INTO enrollment (id, user_id, admin_id, email, created_at, expires_at, used_at) \
            VALUES ($1, $2, $3, $4, $5, $6, $7)",
            self.id,
            self.user_id,
            self.admin_id,
            self.email,
            self.created_at,
            self.expires_at,
            self.used_at,
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
    ) -> Result<NaiveDateTime, EnrollmentError> {
        // check if token can be used
        if self.is_expired() {
            return Err(EnrollmentError::TokenExpired);
        }
        if self.is_used() {
            return Err(EnrollmentError::TokenUsed);
        }

        let now = Utc::now().naive_utc();
        query!(
            "UPDATE enrollment SET used_at = $1 WHERE id = $2",
            now,
            self.id
        )
        .execute(transaction)
        .await?;
        self.used_at = Some(now);

        Ok(now + Duration::seconds(session_timeout_seconds as i64))
    }

    pub async fn find_by_id(pool: &DbPool, id: &str) -> Result<Self, EnrollmentError> {
        match query_as!(
            Self,
            "SELECT id, user_id, admin_id, email, created_at, expires_at, used_at \
            FROM enrollment WHERE id = $1",
            id
        )
        .fetch_optional(pool)
        .await?
        {
            Some(enrollment) => Ok(enrollment),
            None => Err(EnrollmentError::NotFound),
        }
    }

    pub async fn fetch_all(pool: &DbPool) -> Result<Vec<Self>, EnrollmentError> {
        let enrollments = query_as!(
            Self,
            "SELECT id, user_id, admin_id, email, created_at, expires_at, used_at \
            FROM enrollment",
        )
        .fetch_all(pool)
        .await?;
        Ok(enrollments)
    }

    pub async fn fetch_user<'e, E>(&self, executor: E) -> Result<User, EnrollmentError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        debug!("Fetching user for enrollment");
        let Some(user) = User::find_by_id(executor, self.user_id).await? else {
            error!("User not found for enrollment token {}", self.id);
            return Err(EnrollmentError::UserNotFound);
        };
        Ok(user)
    }

    pub async fn fetch_admin<'e, E>(&self, executor: E) -> Result<User, EnrollmentError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        debug!("Fetching admin for enrollment");
        let Some(user) = User::find_by_id(executor, self.admin_id).await? else {
            error!("Admin not found for enrollment token {}", self.id);
            return Err(EnrollmentError::AdminNotFound);
        };
        Ok(user)
    }

    pub async fn delete_unused_user_tokens(
        transaction: &mut PgConnection,
        user_id: i64,
    ) -> Result<(), EnrollmentError> {
        debug!("Deleting unused enrollment tokens for user {user_id}");
        let result = query!(
            r#"DELETE FROM enrollment
            WHERE user_id = $1
            AND used_at IS NULL"#,
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
    ) -> Result<Context, EnrollmentError> {
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
        context.insert("admin_first_name", &admin.first_name);
        context.insert("admin_last_name", &admin.last_name);
        context.insert("admin_email", &admin.email);
        context.insert("admin_phone", &admin.phone);

        Ok(context)
    }

    // Replace template tags and return markdown content
    // to be displayed on final enrollment page
    pub async fn get_welcome_page_content(
        &self,
        transaction: &mut PgConnection,
    ) -> Result<String, EnrollmentError> {
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
    ) -> Result<String, EnrollmentError> {
        let settings = Settings::get_settings(&mut *transaction).await?;

        // load configured content as template
        let mut tera = Tera::default();
        tera.add_raw_template("welcome_email", &settings.enrollment_welcome_email()?)?;

        let context = self.get_welcome_message_context(&mut *transaction).await?;
        let content = tera.render("welcome_email", &context)?;

        Ok(templates::enrollment_welcome_mail(&content)?)
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
    ) -> Result<String, EnrollmentError> {
        info!(
            "User {} starting enrollment for user {}, notification enabled: {send_user_notification}",
            admin.username, self.username
        );
        if self.has_password() {
            return Err(EnrollmentError::AlreadyActive);
        }

        let user_id = self.id.expect("User without ID");
        let admin_id = admin.id.expect("Admin user without ID");

        self.clear_unused_enrollment_tokens(&mut *transaction)
            .await?;

        let enrollment = Enrollment::new(user_id, admin_id, email.clone(), token_timeout_seconds);
        enrollment.save(&mut *transaction).await?;

        if send_user_notification && email.is_some() {
            let email = email.unwrap();
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
                .map_err(|err| EnrollmentError::NotificationError(err.to_string()))?,
                attachments: Vec::new(),
                result_tx: None,
            };
            match mail_tx.send(mail) {
                Ok(_) => {
                    info!(
                        "Sent enrollment start mail for user {} to {email}",
                        self.username
                    );
                }
                Err(err) => {
                    error!("Error sending mail: {err}");
                    return Err(EnrollmentError::NotificationError(err.to_string()));
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
    ) -> Result<String, EnrollmentError> {
        info!(
            "User {} starting desktop configuration for user {}, notification enabled: {send_user_notification}",
            admin.username, self.username
        );

        let user_id = self.id.expect("User without ID");
        let admin_id = admin.id.expect("Admin user without ID");

        self.clear_unused_enrollment_tokens(&mut *transaction)
            .await?;

        let enrollment = Enrollment::new(user_id, admin_id, email.clone(), token_timeout_seconds);
        enrollment.save(&mut *transaction).await?;

        if send_user_notification && email.is_some() {
            let email = email.unwrap();
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
                    enrollment_service_url,
                    &enrollment.id,
                )
                .map_err(|err| EnrollmentError::NotificationError(err.to_string()))?,
                attachments: Vec::new(),
                result_tx: None,
            };
            match mail_tx.send(mail) {
                Ok(_) => {
                    info!(
                        "Sent desktop configuration start mail for user {} to {email}",
                        self.username
                    );
                }
                Err(err) => {
                    error!("Error sending mail: {err}");
                    return Err(EnrollmentError::NotificationError(err.to_string()));
                }
            }
        }

        Ok(enrollment.id)
    }

    // Remove unused tokens when triggering user enrollment
    async fn clear_unused_enrollment_tokens(
        &self,
        transaction: &mut PgConnection,
    ) -> Result<(), EnrollmentError> {
        info!(
            "Removing unused enrollment tokens for user {}",
            self.username
        );
        Enrollment::delete_unused_user_tokens(transaction, self.id.expect("Missing user ID")).await
    }
}

impl Settings {
    pub fn enrollment_welcome_message(&self) -> Result<String, EnrollmentError> {
        self.enrollment_welcome_message.clone().ok_or_else(|| {
            error!("Enrollment welcome message not configured");
            EnrollmentError::WelcomeMsgNotConfigured
        })
    }

    pub fn enrollment_welcome_email(&self) -> Result<String, EnrollmentError> {
        if self.enrollment_use_welcome_message_as_email {
            return self.enrollment_welcome_message();
        }
        self.enrollment_welcome_email.clone().ok_or_else(|| {
            error!("Enrollment welcome email not configured");
            EnrollmentError::WelcomeEmailNotConfigured
        })
    }
}
