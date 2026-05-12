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
    user: Option<String>,
    password: Option<String>,
    sender: String,
}

impl SmtpSettings {
    /// Constructs `SmtpSettings` from `Settings`. Returns error if `SmtpSettings` are incomplete.
    pub(crate) fn from_settings(settings: Settings) -> Result<Self, MailError> {
        if let (Some(server), Some(port), Some(sender)) = (
            settings.smtp_server,
            settings.smtp_port,
            settings.smtp_sender,
        ) {
            let port = port.try_into().map_err(|_| MailError::InvalidPort(port))?;
            Ok(Self {
                server,
                port,
                encryption: settings.smtp_encryption,
                user: settings.smtp_user,
                password: settings.smtp_password.map(|p| p.expose_secret().to_owned()),
                sender,
            })
        } else {
            Err(MailError::SmtpNotConfigured)
        }
    }
}
