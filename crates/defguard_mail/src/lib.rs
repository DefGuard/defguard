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
