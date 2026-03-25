use std::{str::FromStr, time::Duration};

use defguard_common::db::models::{
    Settings,
    settings::{SmtpEncryption, defaults::WELCOME_EMAIL_SUBJECT},
};
use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
    message::{Body, Mailbox, MultiPart, SinglePart, header::ContentType},
    transport::smtp::authentication::Credentials,
};
use serde::Serialize;
use sqlx::PgConnection;
use tera::{Context, Tera, Value};
use thiserror::Error;
use tracing::{debug, error, info, warn};

use super::SmtpSettings;
use crate::{
    mail_context::MailContext,
    qr::qr_png,
    templates::{DEFAULT_LANG, TemplateError},
};

#[derive(Debug)]
pub struct Attachment {
    filename: String,
    content: Vec<u8>,
}

impl Attachment {
    /// Create new [`Attachement`].
    #[must_use]
    pub fn new(filename: String, content: Vec<u8>) -> Self {
        Self { filename, content }
    }
}

impl From<Attachment> for SinglePart {
    fn from(attachment: Attachment) -> Self {
        lettre::message::Attachment::new(attachment.filename)
            .body(attachment.content, ContentType::TEXT_PLAIN)
    }
}

const SMTP_TIMEOUT: Duration = Duration::from_secs(15);
// Template images.
static DEFGUARD_LOGO: &[u8] = include_bytes!("../assets/defguard.png");
static GITHUB_LOGO: &[u8] = include_bytes!("../assets/github.png");
static MASTODON_LOGO: &[u8] = include_bytes!("../assets/mastodon.png");
static X_LOGO: &[u8] = include_bytes!("../assets/x.png");
// MFA code
static DATE_ICON: &[u8] = include_bytes!("../assets/date.png");
static OTP_ICON: &[u8] = include_bytes!("../assets/otp.png");
// New account
static NEW_ACCOUNT_1: &[u8] = include_bytes!("../assets/new_account_1.png");
static NEW_ACCOUNT_2: &[u8] = include_bytes!("../assets/new_account_2.png");
static GOOGLE_PLAY: &[u8] = include_bytes!("../assets/google_play.png");
static APPLE: &[u8] = include_bytes!("../assets/apple.png");

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

/// Mail message
#[derive(Debug)]
pub struct Mail {
    pub(crate) to: String,
    pub(crate) subject: String,
    // HTML version of the message.
    html: String,
    // Plain text version of the message.
    text: String,
    context: Context,
    attachments: Vec<Attachment>,   // text/plain
    images: Vec<(String, Vec<u8>)>, // image/png
}

impl Mail {
    /// Create new [`Mail`].
    #[must_use]
    pub fn new<T>(to: T, subject: String, html: String, text: String) -> Mail
    where
        T: Into<String>,
    {
        // Append images used in all templates.
        let images = vec![
            (String::from("defguard"), Vec::from(DEFGUARD_LOGO)),
            (String::from("github"), Vec::from(GITHUB_LOGO)),
            (String::from("mastodon"), Vec::from(MASTODON_LOGO)),
            (String::from("x"), Vec::from(X_LOGO)),
        ];

        Self {
            to: to.into(),
            subject,
            html,
            text,
            context: Context::new(),
            attachments: Vec::new(),
            images,
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
    // #[must_use]
    // pub fn content(&self) -> &str {
    //     &self.html
    // }

    /// Add to context.
    pub fn add_to_context<K, V>(&mut self, key: K, value: &V)
    where
        K: Into<String>,
        V: Serialize + ?Sized,
    {
        self.context.insert(key.into(), value);
    }

    /// Setter for `attachments`.
    #[must_use]
    pub fn set_attachments(mut self, attachments: Vec<Attachment>) -> Self {
        self.attachments = attachments;
        self
    }

    pub fn add_png_image<S>(&mut self, name: S, bytes: &[u8])
    where
        S: Into<String>,
    {
        self.images.push((name.into(), Vec::from(bytes)));
    }

    /// Converts Mail to lettre Message.
    /// Message structure should look like this:
    /// - multipart mixed
    ///   - multipart alternative
    ///     - singlepart: plain text
    ///     - multipart related
    ///       - singlepart: HTML version
    ///       - singlepart: image 1
    ///       - singlepart: image 2
    ///   - singlepart: attachments
    pub(crate) fn into_message(self, from: &str) -> Result<Message, MailError> {
        let builder = Message::builder()
            .from(Mailbox::from_str(from)?)
            .to(Mailbox::from_str(&self.to)?)
            .subject(self.subject);

        let plain = SinglePart::plain(self.text);
        let html = SinglePart::html(self.html);
        let image_png = "image/png".parse::<ContentType>().unwrap();
        let mut related = MultiPart::related().singlepart(html);
        for (name, bytes) in self.images {
            related = related.singlepart(
                lettre::message::Attachment::new_inline(name)
                    .body(Body::new(bytes), image_png.clone()),
            );
        }
        let alternative = MultiPart::alternative()
            .singlepart(plain)
            .multipart(related);

        let mut mixed = MultiPart::mixed().multipart(alternative);
        for attachment in self.attachments {
            mixed = mixed.singlepart(attachment.into());
        }

        Ok(builder.multipart(mixed)?)
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

    /// Schedule sending email message.
    pub fn send_and_forget(self) {
        tokio::spawn(self.send());
    }

    /// Builds mailer object with specified configuration.
    fn mailer(settings: SmtpSettings) -> Result<AsyncSmtpTransport<Tokio1Executor>, MailError> {
        type Builder = AsyncSmtpTransport<Tokio1Executor>;

        let builder = match settings.encryption {
            SmtpEncryption::None => Builder::builder_dangerous(&settings.server),
            SmtpEncryption::StartTls => Builder::starttls_relay(&settings.server)?,
            SmtpEncryption::ImplicitTls => Builder::relay(&settings.server)?,
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

/// Email messages.
pub enum MailMessage {
    /// Test email to check if SMTP configuration works correctly.
    Test,
    Welcome,
    /// Information for Defguard support.
    SupportData,
    DesktopStart,
    /// Information after starting an enrollment.
    NewAccount,
    NewDevice,
    NewDeviceLogin,
    NewDeviceOCIDLogin,
    /// Gateway has disconnected.
    GatewayDisconnect,
    /// Gateway has reconnected.
    GatewayReconnect,
    /// MFA activated.
    MFAActivation,
    MFAConfigured,
    /// MFA code.
    MFACode,
    PasswordReset,
    PasswordResetDone,
    UserImportBlocked,
    /// Enrollment notification for admins.
    EnrollmentNotification,
}

impl MailMessage {
    /// Email subject.
    pub(crate) fn subject(&self) -> String {
        // Welcome message's subject should be taken from settings.
        if let Self::Welcome = self {
            let settings = Settings::get_current_settings();
            if let Some(subject) = settings.enrollment_welcome_email_subject {
                return subject;
            }
        }
        match self {
            Self::Test => "Defguard: Test message",
            Self::Welcome => WELCOME_EMAIL_SUBJECT,
            Self::SupportData => "Defguard: Support data",
            Self::DesktopStart => "Defguard: Desktop client configuration",
            Self::NewAccount => "Defguard: User enrollment",
            Self::NewDevice => "Defguard: new device added to your account",
            Self::NewDeviceLogin => "Defguard: New device logged in to your account",
            Self::NewDeviceOCIDLogin => "New login to OCID application",
            Self::GatewayDisconnect => "Defguard: Gateway disconnected",
            Self::GatewayReconnect => "Defguard: Gateway reconnected",
            Self::MFAActivation => "Multi-Factor Authentication activation",
            Self::MFAConfigured => "Multi-Factor Authentication {method} has been activated",
            Self::MFACode => "Defguard: Multi-Factor Authentication code for login",
            Self::PasswordReset => "Defguard: Password reset",
            Self::PasswordResetDone => "Defguard: Password reset success",
            Self::UserImportBlocked => "User import blocked",
            Self::EnrollmentNotification => "Defguard: User enrollment completed",
        }
        .to_string()
    }

    pub(crate) const fn template_name(&self) -> &str {
        match self {
            Self::Test => "test",
            Self::Welcome => "welcome",
            Self::SupportData => "support-data",
            Self::DesktopStart => "desktop-start",
            Self::NewAccount => "new-account",
            Self::NewDevice => "new-device",
            Self::NewDeviceLogin => "new-device-login",
            Self::NewDeviceOCIDLogin => "new-device-ocid-login",
            Self::GatewayDisconnect => "gateway-disconnect",
            Self::GatewayReconnect => "gateway-reconnect",
            Self::MFAActivation => "mfa-activation",
            Self::MFAConfigured => "mfa-configured",
            Self::MFACode => "mfa-code",
            Self::PasswordReset => "password-reset",
            Self::PasswordResetDone => "password-reset-done",
            Self::UserImportBlocked => "user-import-blocked",
            Self::EnrollmentNotification => "enrollment-admin-notification",
        }
    }

    pub(crate) const fn mjml_template(&self) -> &str {
        match self {
            Self::Test => include_str!("../templates/test.mjml"),
            Self::Welcome => include_str!("../templates/enrollment-welcome.mjml"),
            Self::SupportData => include_str!("../templates/support-data.mjml"),
            Self::DesktopStart => include_str!("../templates/desktop-start.mjml"),
            Self::NewAccount => include_str!("../templates/new-account.mjml"),
            Self::NewDevice => include_str!("../templates/new-device.mjml"),
            Self::NewDeviceLogin => include_str!("../templates/new-device-login.mjml"),
            Self::NewDeviceOCIDLogin => include_str!("../templates/new-device-ocid-login.mjml"),
            Self::GatewayDisconnect => include_str!("../templates/gateway-disconnected.mjml"),
            Self::GatewayReconnect => include_str!("../templates/gateway-reconnected.mjml"),
            Self::MFAActivation => include_str!("../templates/mfa-activation.mjml"),
            Self::MFAConfigured => include_str!("../templates/mfa-configured.mjml"),
            Self::MFACode => include_str!("../templates/mfa-code.mjml"),
            Self::PasswordReset => include_str!("../templates/password-reset.mjml"),
            Self::PasswordResetDone => include_str!("../templates/password-reset-done.mjml"),
            Self::UserImportBlocked => include_str!("../templates/plain-notification.mjml"),
            Self::EnrollmentNotification => {
                include_str!("../templates/enrollment-admin-notification.mjml")
            }
        }
    }

    pub(crate) const fn text_template(&self) -> &str {
        match self {
            Self::Test => include_str!("../templates/test.text"),
            Self::Welcome => include_str!("../templates/enrollment-welcome.text"),
            Self::SupportData => include_str!("../templates/support-data.text"),
            Self::DesktopStart => include_str!("../templates/desktop-start.text"),
            Self::NewAccount => include_str!("../templates/new-account.text"),
            Self::NewDevice => include_str!("../templates/new-device.text"),
            Self::NewDeviceLogin => include_str!("../templates/new-device-login.text"),
            Self::NewDeviceOCIDLogin => include_str!("../templates/new-device-ocid-login.text"),
            Self::GatewayDisconnect => include_str!("../templates/gateway-disconnected.text"),
            Self::GatewayReconnect => include_str!("../templates/gateway-reconnected.text"),
            Self::MFAActivation => include_str!("../templates/mfa-activation.text"),
            Self::MFAConfigured => include_str!("../templates/mfa-configured.text"),
            Self::MFACode => include_str!("../templates/mfa-code.text"),
            Self::PasswordReset => include_str!("../templates/password-reset.text"),
            Self::PasswordResetDone => include_str!("../templates/password-reset-done.text"),
            Self::UserImportBlocked => include_str!("../templates/plain-notification.text"),
            Self::EnrollmentNotification => {
                include_str!("../templates/enrollment-admin-notification.text")
            }
        }
    }

    /// Fill `Context` from database.
    pub(crate) async fn fill_context(
        &self,
        conn: &mut PgConnection,
        context: &mut Context,
    ) -> Result<(), sqlx::Error> {
        let db_context =
            MailContext::all_for_template(conn, self.template_name(), DEFAULT_LANG).await?;
        for row in db_context {
            context.insert(row.section, &row.text);
        }

        Ok(())
    }

    /// Build `Mail`.
    pub(crate) fn mail(
        &self,
        tera: &mut Tera,
        context: &Context,
        to: &str,
    ) -> Result<Mail, TemplateError> {
        // Build HTML message.
        tera.add_raw_template(self.template_name(), self.mjml_template())?;
        let processed = tera.render(self.template_name(), context)?;
        let parsed = mrml::parse(processed)?;
        let opts = mrml::prelude::render::RenderOptions::default();
        let html = parsed.element.render(&opts)?;

        // Build plain text message.
        tera.add_raw_template(self.template_name(), self.text_template())?;
        let text = tera.render(self.template_name(), context)?;

        let mut mail = Mail::new(to, self.subject(), html, text);
        // Add PNG images.
        match self {
            Self::NewAccount => {
                mail.add_png_image("new_account_1", NEW_ACCOUNT_1);
                mail.add_png_image("new_account_2", NEW_ACCOUNT_2);
                mail.add_png_image("google_play", GOOGLE_PLAY);
                mail.add_png_image("apple", APPLE);
                if let Some(Value::String(url)) = context.get("url") {
                    if let Ok(qr) = qr_png(url.as_bytes()) {
                        mail.add_png_image("qr", &qr);
                    }
                }
            }
            Self::MFACode | Self::MFAActivation => {
                mail.add_png_image("date", DATE_ICON);
                mail.add_png_image("otp", OTP_ICON);
            }
            _ => (),
        }

        Ok(mail)
    }
}
