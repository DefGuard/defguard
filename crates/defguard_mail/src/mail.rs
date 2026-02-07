use std::str::FromStr;

use lettre::{
    Message,
    message::{Mailbox, MultiPart, SinglePart, header::ContentType},
};
use thiserror::Error;
use tokio::sync::mpsc::UnboundedSender;

use super::Confirmation;

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
    attachments: Vec<Attachment>,
    pub(crate) result_tx: Option<UnboundedSender<Confirmation>>,
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
}
