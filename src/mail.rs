use lettre::{
    address::AddressError,
    message::{header::ContentType, Mailbox},
    transport::smtp::authentication::Credentials,
    Address, AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};
use thiserror::Error;
use tokio::sync::mpsc::UnboundedReceiver;

#[derive(Error, Debug)]
pub enum MailError {
    #[error(transparent)]
    LettreError(#[from] lettre::error::Error),

    #[error(transparent)]
    AddressError(#[from] lettre::address::AddressError),

    #[error(transparent)]
    SmtpError(#[from] lettre::transport::smtp::Error),
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
    mailer: AsyncSmtpTransport<Tokio1Executor>,
}

impl MailHandler {
    pub fn new(
        rx: UnboundedReceiver<Mail>,
        server: &str,
        username: &str,
        password: &str,
    ) -> Result<Self, MailError> {
        let mailer = AsyncSmtpTransport::<Tokio1Executor>::relay(server)?
            .credentials(Credentials::new(username.into(), password.into()))
            .build();
        Ok(Self { rx, mailer })
    }

    pub async fn run(mut self) {
        while let Some(mail) = self.rx.recv().await {
            debug!("Sending mail: {mail:?}");

            let message: Message = match mail.clone().try_into() {
                Ok(message) => message,
                Err(err) => {
                    error!("Failed to build message: {mail:?}, {err}");
                    continue;
                }
            };

            match self.mailer.send(message).await {
                Ok(response) => info!("Mail sent successfully: {mail:?}, {response:?}"),
                Err(err) => error!("Mail sending failed: {mail:?}, {err}"),
            }
        }
    }
}

pub async fn run_mail_handler(
    rx: UnboundedReceiver<Mail>,
    server: &str,
    username: &str,
    password: &str,
) -> Result<(), MailError> {
    MailHandler::new(rx, server, username, password)?
        .run()
        .await;
    Ok(())
}
