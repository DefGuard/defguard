use lettre::{
    address::AddressError,
    message::{header::ContentType, Mailbox},
    transport::smtp::authentication::Credentials,
    Address, AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};
use sqlx::{Pool, Postgres};
use thiserror::Error;
use tokio::sync::mpsc::UnboundedReceiver;

use crate::db::{models::settings::SmtpEncryption, Settings};

#[derive(Error, Debug)]
pub enum MailError {
    #[error(transparent)]
    LettreError(#[from] lettre::error::Error),

    #[error(transparent)]
    AddressError(#[from] lettre::address::AddressError),

    #[error(transparent)]
    SmtpError(#[from] lettre::transport::smtp::Error),

    #[error(transparent)]
    SqlxError(#[from] sqlx::Error),

    #[error("SMTP not configured")]
    SmtpNotConfigured,

    #[error("No settings record in database")]
    EmptySettings,

    #[error("Invalid port: {0}")]
    InvalidPort(i32),
}

/// Subset of Settings object representing SMTP configuration
struct SmtpSettings {
    pub server: String,
    pub port: u16,
    pub encryption: SmtpEncryption,
    pub user: String,
    pub password: String,
    pub sender: String,
}

impl SmtpSettings {
    /// Retrieves Settings object from database and builds SmtpSettings
    pub async fn get(db: &Pool<Postgres>) -> Result<Self, MailError> {
        Self::from_settings(Self::get_settings(db).await?).await
    }

    /// Constructs SmtpSettings object from Settings. Returns error if SMTP settings are incomplete.
    pub async fn from_settings(settings: Settings) -> Result<SmtpSettings, MailError> {
        if let (Some(server), Some(port), encryption, Some(user), Some(password), Some(sender)) = (
            settings.smtp_server,
            settings.smtp_port,
            settings.smtp_encryption,
            settings.smtp_user,
            settings.smtp_password,
            settings.smtp_sender,
        ) {
            let port = port.try_into().map_err(|_| MailError::InvalidPort(port))?;
            Ok(Self {
                server,
                port,
                encryption,
                user,
                password,
                sender,
            })
        } else {
            Err(MailError::SmtpNotConfigured)
        }
    }

    /// Retrieves Settings object from database
    async fn get_settings(db: &Pool<Postgres>) -> Result<Settings, MailError> {
        Settings::find_by_id(db, 1)
            .await?
            .ok_or(MailError::EmptySettings)
    }
}

#[derive(Debug, Clone)]
pub struct Mail {
    pub to: String,
    pub subject: String,
    pub content: String,
}

impl Mail {
    /// Converts Mail to lettre Message
    fn to_message(&self, from: &str) -> Result<Message, MailError> {
        Ok(Message::builder()
            .from(Self::mailbox(from)?)
            .to(Self::mailbox(&self.to)?)
            .subject(self.subject.clone())
            .header(ContentType::TEXT_HTML)
            .body(self.content.clone())?)
    }

    /// Builds Mailbox structure from string representing email address
    fn mailbox(address: &str) -> Result<Mailbox, MailError> {
        if let Some((user, domain)) = address.split_once('@') {
            if !(user.is_empty() || domain.is_empty()) {
                return Ok(Mailbox::new(None, Address::new(user, domain)?));
            }
        }
        Err(AddressError::MissingParts)?
    }
}

struct MailHandler {
    rx: UnboundedReceiver<Mail>,
    db: Pool<Postgres>,
}

impl MailHandler {
    pub fn new(rx: UnboundedReceiver<Mail>, db: Pool<Postgres>) -> Self {
        Self { rx, db }
    }

    /// Listens on rx channel for messages and sends them via SMTP.
    pub async fn run(mut self) {
        while let Some(mail) = self.rx.recv().await {
            debug!("Sending mail: {mail:?}");
            let settings = match SmtpSettings::get(&self.db).await {
                Ok(settings) => settings,
                Err(MailError::SmtpNotConfigured) => {
                    warn!("SMTP not configured, email sending skipped");
                    continue;
                }
                Err(err) => {
                    error!("Error retrieving SMTP settings: {err}");
                    continue;
                }
            };

            // Construct lettre Message
            let message: Message = match mail.to_message(&settings.sender) {
                Ok(message) => message,
                Err(err) => {
                    error!("Failed to build message: {mail:?}, {err}");
                    continue;
                }
            };

            // Build mailer and send the message
            match self.mailer(settings).await {
                Ok(mailer) => match mailer.send(message).await {
                    Ok(response) => info!("Mail sent successfully: {mail:?}, {response:?}"),
                    Err(err) => error!("Mail sending failed: {mail:?}, {err}"),
                },
                Err(MailError::SmtpNotConfigured) => {
                    warn!("SMTP not configured, onboarding email sending skipped")
                }
                Err(err) => error!("Error building mailer: {err}"),
            }
        }
    }

    /// Builds mailer object with specified configuration
    async fn mailer(
        &self,
        settings: SmtpSettings,
    ) -> Result<AsyncSmtpTransport<Tokio1Executor>, MailError> {
        let builder = match settings.encryption {
            SmtpEncryption::None => {
                AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(settings.server)
            }
            SmtpEncryption::StartTls => {
                AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&settings.server)?
            }
            SmtpEncryption::ImplicitTls => {
                AsyncSmtpTransport::<Tokio1Executor>::relay(&settings.server)?
            }
        }
        .port(settings.port);
        Ok(builder
            .credentials(Credentials::new(settings.user, settings.password))
            .build())
    }
}

/// Builds MailHandler and runs it.
pub async fn run_mail_handler(rx: UnboundedReceiver<Mail>, db: Pool<Postgres>) {
    MailHandler::new(rx, db).run().await;
}
