use std::time::Duration;

use defguard_common::db::models::{Settings, settings::SmtpEncryption};
use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
    transport::smtp::authentication::Credentials,
};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};
use tracing::{debug, error, info, warn};

use crate::{Confirmation, Mail, SmtpSettings, mail::MailError};

const SMTP_TIMEOUT: Duration = Duration::from_secs(15);

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
            let message = match mail.into_message(&settings.sender) {
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
                        info!("Mail sent to: {to}, subject: {subject}, response: {response:?}");
                    }
                    Err(err) => {
                        error!("Failed to send mail to: {to}, subject: {subject}, error: {err}");
                        Self::send_result(result_tx, Err(MailError::SmtpError(err)));
                    }
                },
                Err(MailError::SmtpNotConfigured) => {
                    warn!("Unable to send mail to {to}; SMTP not configured");
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
