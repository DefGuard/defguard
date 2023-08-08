use std::time::Duration;

use lettre::{
    address::AddressError,
    message::{header::ContentType, Mailbox, MultiPart, SinglePart},
    transport::smtp::{authentication::Credentials, response::Response},
    Address, AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};
use sqlx::{Pool, Postgres};
use thiserror::Error;
use tokio::sync::{mpsc::UnboundedReceiver, oneshot::Sender};

use crate::db::{models::settings::SmtpEncryption, Settings};

static SMTP_TIMEOUT_SECONDS: u64 = 15;

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

pub struct Mail {
    pub to: String,
    pub subject: String,
    pub content: String,
    pub attachments: Vec<Attachment>,
    pub result_tx: Option<Sender<Result<Response, MailError>>>,
}

pub struct Attachment {
    pub filename: String,
    pub content: Vec<u8>,
    pub content_type: ContentType,
}

impl From<Attachment> for SinglePart {
    fn from(attachment: Attachment) -> Self {
        lettre::message::Attachment::new(attachment.filename)
            .body(attachment.content, attachment.content_type)
    }
}

impl Mail {
    /// Converts Mail to lettre Message
    fn to_message(self, from: &str) -> Result<Message, MailError> {
        let builder = Message::builder()
            .from(Self::mailbox(from)?)
            .to(Self::mailbox(&self.to)?)
            .subject(self.subject.clone());
        match self.attachments {
            attachments if attachments.len() == 0 => Ok(builder
                .header(ContentType::TEXT_HTML)
                .body(self.content.clone())?),
            attachments => {
                let mut multipart =
                    MultiPart::mixed().singlepart(SinglePart::html(self.content.clone()));
                for attachment in attachments {
                    multipart = multipart.singlepart(attachment.into());
                }
                Ok(builder.multipart(multipart)?)
            }
        }
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

    pub fn send_result(
        tx: Option<Sender<Result<Response, MailError>>>,
        result: Result<Response, MailError>,
    ) {
        if let Some(tx) = tx {
            if tx.send(result).is_ok() {
                debug!("SMTP result sent back to caller");
            } else {
                error!("Error sending SMTP result back to caller")
            }
        }
    }

    /// Listens on rx channel for messages and sends them via SMTP.
    pub async fn run(mut self) {
        while let Some(mail) = self.rx.recv().await {
            let (to, subject) = (mail.to.clone(), mail.subject.clone());
            debug!("Sending mail to: {to}, subject: {subject}");
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
            // let result_tx = mail.result_tx;
            let message: Message = match mail.to_message(&settings.sender) {
                Ok(message) => message,
                Err(err) => {
                    error!("Failed to build message to: {to}, subject: {subject}, error: {err}");
                    continue;
                }
            };
            // Build mailer and send the message
            match self.mailer(settings).await {
                Ok(mailer) => match mailer.send(message).await {
                    Ok(response) => {
                        Self::send_result(mail.result_tx, Ok(response.clone()));
                        info!("Mail sent successfully to: {to}, subject: {subject}, response: {response:?}");
                    }
                    Err(err) => {
                        error!("Mail sending failed to: {to}, subject: {subject}, error: {err}");
                        Self::send_result(mail.result_tx, Err(MailError::SmtpError(err)));
                    }
                },
                Err(MailError::SmtpNotConfigured) => {
                    warn!("SMTP not configured, onboarding email sending skipped");
                    Self::send_result(mail.result_tx, Err(MailError::SmtpNotConfigured));
                }
                Err(err) => {
                    error!("Error building mailer: {err}");
                    Self::send_result(mail.result_tx, Err(err))
                }
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
        .port(settings.port)
        .timeout(Some(Duration::from_secs(SMTP_TIMEOUT_SECONDS)));
        Ok(builder
            .credentials(Credentials::new(settings.user, settings.password))
            .build())
    }
}

/// Builds MailHandler and runs it.
pub async fn run_mail_handler(rx: UnboundedReceiver<Mail>, db: Pool<Postgres>) {
    MailHandler::new(rx, db).run().await;
}
