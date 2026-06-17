//! Handle email messages.
//!
//! Refer to:
//! - [RFC 2557](https://datatracker.ietf.org/doc/html/rfc2557)
//! - [Meaning of mulitpart](https://www.codestudy.net/blog/mail-multipart-alternative-vs-multipart-mixed/)

pub mod mail;
pub(crate) mod mail_context;
mod qr;
pub mod templates;
#[cfg(test)]
mod tests;

#[derive(Debug, thiserror::Error)]
pub enum MailError {
    #[error(transparent)]
    Lettre(#[from] lettre::error::Error),

    #[error(transparent)]
    Address(#[from] lettre::address::AddressError),

    #[error(transparent)]
    Smtp(#[from] lettre::transport::smtp::Error),

    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),

    #[error("SMTP not configured")]
    SmtpNotConfigured,

    #[error("Invalid port: {0}")]
    InvalidPort(i32),

    #[error(transparent)]
    OAuth2(#[from] crate::enterprise::oauth2::OAuth2Error),
}
