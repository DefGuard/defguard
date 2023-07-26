use lettre::{
    address::AddressError,
    message::{header::ContentType, Mailbox},
    transport::smtp::authentication::Credentials,
    Address, AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};
use sqlx::{Pool, Postgres};
use thiserror::Error;
use tokio::sync::mpsc::UnboundedReceiver;

use crate::db::Settings;

const DEFAULT_SMTP_PORT: u16 = 25;
const DEFAULT_SMTP_TLS_PORT: u16 = 587;

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
}

#[derive(Debug, Clone)]
pub struct Mail {
    pub from: String,
    pub to: String,
    pub subject: String,
    pub content: String,
}

/// Builds Mailbox structure from string representing user email address
fn mailbox(address: &str) -> Result<Mailbox, MailError> {
    let mut split = address.split('@');
    let (user, domain) = (
        split.next().ok_or(AddressError::MissingParts)?,
        split.next().ok_or(AddressError::MissingParts)?,
    );
    Ok(Mailbox::new(None, Address::new(user, domain)?))
}

impl TryFrom<Mail> for Message {
    type Error = MailError;

    fn try_from(mail: Mail) -> Result<Self, Self::Error> {
        Ok(Self::builder()
            .from(mailbox(&mail.from)?)
            .to(mailbox(&mail.to)?)
            .subject(mail.subject)
            .header(ContentType::TEXT_HTML)
            .body(mail.content)?)
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

            // Construct lettre Message
            let message: Message = match mail.clone().try_into() {
                Ok(message) => message,
                Err(err) => {
                    error!("Failed to build message: {mail:?}, {err}");
                    continue;
                }
            };

            // Build mailer and send the message
            match self.mailer().await {
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

    /// Builds mailer object using settings from database
    async fn mailer(&self) -> Result<AsyncSmtpTransport<Tokio1Executor>, MailError> {
        let settings = self.get_settings().await?;
        if let (Some(server), Some(tls), Some(user), Some(password)) = (
            settings.smtp_server,
            settings.smtp_tls,
            settings.smtp_user,
            settings.smtp_password,
        ) {
            let port: Option<u16> = settings.smtp_port.and_then(|port| port.try_into().ok());
            let builder = if tls {
                AsyncSmtpTransport::<Tokio1Executor>::relay(&server)?
                    .port(port.unwrap_or(DEFAULT_SMTP_TLS_PORT))
            } else {
                AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&server)
                    .port(port.unwrap_or(DEFAULT_SMTP_PORT))
            };
            Ok(builder
                .credentials(Credentials::new(user, password))
                .build())
        } else {
            Err(MailError::SmtpNotConfigured)
        }
    }

    /// Retrieves settings object from database
    async fn get_settings(&self) -> Result<Settings, MailError> {
        Settings::find_by_id(&self.db, 1)
            .await?
            .ok_or(MailError::EmptySettings)
    }
}

/// Builds MailHandler and runs it.
pub async fn run_mail_handler(rx: UnboundedReceiver<Mail>, db: Pool<Postgres>) {
    MailHandler::new(rx, db).run().await;
}
