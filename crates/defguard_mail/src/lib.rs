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
mod xoauth2;

#[derive(Debug, thiserror::Error)]
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

    #[error(transparent)]
    ReqwestError(#[from] openidconnect::reqwest::Error),

    #[error(transparent)]
    UrlError(#[from] openidconnect::url::ParseError),

    #[error(transparent)]
    OAuth2Error(#[from] openidconnect::ConfigurationError),

    #[error("Open ID discovery")]
    OpenIDDiscovery,

    #[error("Refresh token exchange")]
    RefreshTokenExchange,
}
