use std::{str::FromStr, time::Duration};

use defguard_common::db::models::{Settings, settings::SmtpEncryption};
use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
    message::{Mailbox, MultiPart, SinglePart, header::ContentType},
    transport::smtp::authentication::Credentials,
};
use serde::Serialize;
use tera::Context;
use thiserror::Error;
use tracing::{debug, error, info, warn};

use super::SmtpSettings;

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

    #[error("Invalid port: {0}")]
    InvalidPort(i32),
}

#[derive(Debug)]
pub struct Mail {
    pub(crate) to: String,
    pub(crate) subject: String,
    content: String,
    context: Context,
    attachments: Vec<Attachment>,
}

impl Mail {
    /// Create new [`Mail`].
    #[must_use]
    pub fn new<T, S>(to: T, subject: S, content: String) -> Mail
    where
        T: Into<String>,
        S: Into<String>,
    {
        Self {
            to: to.into(),
            subject: subject.into(),
            content,
            context: Context::new(),
            attachments: Vec::new(),
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

    /// Add to context.
    pub fn add_to_context<K, V>(&mut self, key: K, value: &V)
    where
        K: Into<String>,
        V: Serialize + ?Sized,
    {
        self.context.insert(key.into(), value.into());
    }

    /// Setter for `attachments`.
    #[must_use]
    pub fn set_attachments(mut self, attachments: Vec<Attachment>) -> Self {
        self.attachments = attachments;
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
    pub(crate) fn into_message(self, from: &str) -> Result<Message, MailError> {
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

    /// Sends email message using SMTP.
    pub async fn send(self) -> Result<(), MailError> {
        let (to, subject) = (self.to.clone(), self.subject.clone());
        debug!("Sending mail to: {to}, subject: {subject}");

        // fetch SMTP settings
        let settings = Settings::get_current_settings();
        let settings = match SmtpSettings::from_settings(settings) {
            Ok(settings) => settings,
            Err(err @ MailError::SmtpNotConfigured) => {
                warn!("SMTP not configured, email sending skipped");
                return Err(err);
            }
            Err(err) => {
                error!("Error retrieving SMTP settings: {err}");
                return Err(err);
            }
        };

        // Construct lettre Message
        let message = match self.into_message(&settings.sender) {
            Ok(message) => message,
            Err(err) => {
                error!("Failed to build message to: {to}, subject: {subject}, error: {err}");
                return Err(err);
            }
        };
        // Build mailer and send the message
        match Self::mailer(settings) {
            Ok(mailer) => match mailer.send(message).await {
                Ok(response) => {
                    info!("Mail sent to: {to}, subject: {subject}, response: {response:?}");
                    Ok(())
                }
                Err(err) => {
                    error!("Failed to send mail to: {to}, subject: {subject}, error: {err}");
                    Err(err.into())
                }
            },
            Err(err @ MailError::SmtpNotConfigured) => {
                warn!("Unable to send mail to {to}; SMTP not configured");
                Err(err)
            }
            Err(err) => {
                error!("Error building mailer: {err}");
                Err(err)
            }
        }
    }

    pub fn send_and_forget(self) {
        tokio::spawn(self.send());
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
