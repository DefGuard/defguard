//! Handle email messages.
//!
//! Refer to:
//! - [RFC 2557](https://datatracker.ietf.org/doc/html/rfc2557)
//! - [Meaning of mulitpart](https://www.codestudy.net/blog/mail-multipart-alternative-vs-multipart-mixed/)

use defguard_common::db::models::{Settings, settings::SmtpEncryption};

use crate::mail::MailError;
pub use crate::mail::{Attachment, Mail};

pub mod mail;
pub(crate) mod mail_context;
mod qr;
pub mod templates;
#[cfg(test)]
mod tests;

/// Subset of Settings representing SMTP configuration.
pub(crate) struct SmtpSettings {
    server: String,
    port: u16,
    encryption: SmtpEncryption,
    user: String,
    password: String,
    sender: String,
}

impl SmtpSettings {
    /// Constructs `SmtpSettings` from `Settings`. Returns error if `SmtpSettings` are incomplete.
    pub(crate) fn from_settings(settings: Settings) -> Result<SmtpSettings, MailError> {
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
