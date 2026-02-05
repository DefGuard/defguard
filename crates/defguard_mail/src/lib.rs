use std::{str::FromStr, time::Duration};

use defguard_common::db::models::{Settings, settings::SmtpEncryption};
use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
    message::{Mailbox, MultiPart, SinglePart, header::ContentType},
    transport::smtp::{authentication::Credentials, response::Response},
};
use thiserror::Error;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};
use tracing::{debug, error, info, warn};

pub mod templates;

const SMTP_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Debug, Error)]
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
    /// Constructs `SmtpSettings` from `Settings`. Returns error if `SmtpSettings` are incomplete.
    pub fn from_settings(settings: Settings) -> Result<SmtpSettings, MailError> {
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
                password: password.expose_secret().to_string(),
                sender,
            })
        } else {
            Err(MailError::SmtpNotConfigured)
        }
    }
}

type Confirmation = Result<Response, MailError>;

#[derive(Debug)]
pub struct Mail {
    to: String,
    subject: String,
    content: String,
    attachments: Vec<Attachment>,
    result_tx: Option<UnboundedSender<Confirmation>>,
}

impl Mail {
    /// Create new [`Mail`].
    #[must_use]
    pub fn new(to: String, subject: String, content: String) -> Mail {
        Self {
            to,
            subject,
            content,
            attachments: Vec::new(),
            result_tx: None,
        }
    }

    /// Getter for `to`.
    #[must_use]
    pub fn to(&self) -> &str {
        &self.to
    }

    /// Getter for `subject`.
    #[must_use]
    pub fn subject(&self) -> &str {
        &self.subject
    }

    /// Getter for `content`.
    #[must_use]
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Setter for `attachments`.
    #[must_use]
    pub fn set_attachments(mut self, attachments: Vec<Attachment>) -> Self {
        self.attachments = attachments;
        self
    }

    /// Setter for `result_tx`.
    #[must_use]
    pub fn set_result_tx(mut self, result_tx: UnboundedSender<Confirmation>) -> Self {
        self.result_tx = Some(result_tx);
        self
    }
}

#[derive(Debug)]
pub struct Attachment {
    filename: String,
    content: Vec<u8>,
    content_type: ContentType,
}

impl Attachment {
    /// Create new [`Attachement`].
    #[must_use]
    pub fn new(filename: String, content: Vec<u8>) -> Self {
        Self {
            filename,
            content,
            content_type: ContentType::TEXT_PLAIN,
        }
    }
}

impl From<Attachment> for SinglePart {
    fn from(attachment: Attachment) -> Self {
        lettre::message::Attachment::new(attachment.filename)
            .body(attachment.content, attachment.content_type)
    }
}

impl Mail {
    /// Converts Mail to lettre Message
    fn into_message(self, from: &str) -> Result<Message, MailError> {
        let builder = Message::builder()
            .from(Mailbox::from_str(from)?)
            .to(Mailbox::from_str(&self.to)?)
            .subject(self.subject);
        match self.attachments {
            attachments if attachments.is_empty() => Ok(builder
                .header(ContentType::TEXT_HTML)
                .body(self.content.clone())?),
            attachments => {
                let mut multipart = MultiPart::mixed().singlepart(SinglePart::html(self.content));
                for attachment in attachments {
                    multipart = multipart.singlepart(attachment.into());
                }
                Ok(builder.multipart(multipart)?)
            }
        }
    }
}

pub struct MailHandler {
    tx: UnboundedSender<Mail>,
    rx: UnboundedReceiver<Mail>,
}

impl Default for MailHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl MailHandler {
    /// Create new [`MailHandler`].
    #[must_use]
    pub fn new() -> Self {
        let (tx, rx) = unbounded_channel();
        Self { tx, rx }
    }

    /// Return sender's clone.
    #[must_use]
    pub fn tx(&self) -> UnboundedSender<Mail> {
        self.tx.clone()
    }

    fn send_result(tx: Option<UnboundedSender<Confirmation>>, result: Confirmation) {
        if let Some(tx) = tx {
            if tx.send(result).is_ok() {
                debug!("SMTP result sent back to caller");
            } else {
                error!("Error sending SMTP result back to caller");
            }
        }
    }

    /// Listens on the receiver for messages and sends them via SMTP.
    pub async fn run(mut self) {
        while let Some(mail) = self.rx.recv().await {
            let (to, subject) = (mail.to.clone(), mail.subject.clone());
            debug!("Sending mail to: {to}, subject: {subject}");

            // fetch SMTP settings
            let settings = Settings::get_current_settings();
            let settings = match SmtpSettings::from_settings(settings) {
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
            let result_tx = mail.result_tx.clone();
            let message: Message = match mail.into_message(&settings.sender) {
                Ok(message) => message,
                Err(err) => {
                    error!("Failed to build message to: {to}, subject: {subject}, error: {err}");
                    continue;
                }
            };
            // Build mailer and send the message
            match Self::mailer(settings) {
                Ok(mailer) => match mailer.send(message).await {
                    Ok(response) => {
                        Self::send_result(result_tx, Ok(response.clone()));
                        info!(
                            "Mail sent successfully to: {to}, subject: {subject}, response: {response:?}"
                        );
                    }
                    Err(err) => {
                        error!("Mail sending failed to: {to}, subject: {subject}, error: {err}");
                        Self::send_result(result_tx, Err(MailError::SmtpError(err)));
                    }
                },
                Err(MailError::SmtpNotConfigured) => {
                    warn!("SMTP not configured, onboarding email sending skipped");
                    Self::send_result(result_tx, Err(MailError::SmtpNotConfigured));
                }
                Err(err) => {
                    error!("Error building mailer: {err}");
                    Self::send_result(result_tx, Err(err));
                }
            }
        }
    }

    /// Builds mailer object with specified configuration
    fn mailer(settings: SmtpSettings) -> Result<AsyncSmtpTransport<Tokio1Executor>, MailError> {
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
        .timeout(Some(SMTP_TIMEOUT));

        // Skip credentials if any of them is empty
        let builder = if settings.user.is_empty() || settings.password.is_empty() {
            debug!("SMTP credentials were not provided, skipping username/password authentication");
            builder
        } else {
            builder.credentials(Credentials::new(settings.user, settings.password))
        };

        Ok(builder.build())
    }
}
