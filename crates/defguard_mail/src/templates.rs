use std::{collections::HashMap, time::Duration};

use chrono::{Datelike, NaiveDateTime, Utc};
use defguard_common::{
    VERSION,
    db::models::{Session, Settings, user::MFAMethod},
    types::UrlParseError,
};
use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd, html};
use reqwest::Url;
use serde::Serialize;
use serde_json::Value;
use sqlx::PgConnection;
use tera::{Context, Function, Tera};
use thiserror::Error;
use tracing::{debug, warn};

use crate::{Attachment, mail::MailMessage};

pub(crate) const DEFAULT_LANG: &str = "en_US";

pub static SUPPORT_EMAIL_ADDRESS: &str = "support@defguard.net";

static BASE_MJML: &str = include_str!("../templates/base.mjml");
static MACROS_MJML: &str = include_str!("../templates/macros.mjml");
static MAIL_DATETIME_FORMAT: &str = "%A, %B %d, %Y at %r";

#[derive(Debug, Error)]
pub enum TemplateError {
    #[error("Failed to generate email MFA code")]
    MfaError,
    #[error(transparent)]
    DatabaseError(#[from] sqlx::Error),
    #[error(transparent)]
    TemplateError(#[from] tera::Error),
    #[error(transparent)]
    UrlParseError(#[from] UrlParseError),
    #[error(transparent)]
    MrmlParserError(#[from] mrml::prelude::parser::Error),
    #[error(transparent)]
    MrmlRenderError(#[from] mrml::prelude::render::Error),
}

struct NoOp(&'static str);

impl Function for NoOp {
    fn call(&self, _args: &HashMap<String, Value>) -> tera::Result<Value> {
        Err(tera::Error::function_not_found(self.0))
    }
}

/// Return a safe instance of Tera, as Tera is vulnerable to `get_env()` function exploit.
/// See: https://github.com/Keats/tera/issues/677
#[must_use]
pub fn safe_tera() -> Tera {
    let mut tera = Tera::default();
    let noop = NoOp("get_env");
    tera.register_function(noop.0, noop);

    tera
}

pub struct SessionContext {
    ip_address: String,
    device_info: Option<String>,
}

impl From<Session> for SessionContext {
    fn from(value: Session) -> Self {
        Self {
            ip_address: value.ip_address,
            device_info: value.device_info,
        }
    }
}

fn get_base_tera_mjml(
    mut context: Context,
    session: Option<&SessionContext>,
    ip_address: Option<&str>,
    device_info: Option<&str>,
) -> Result<(Tera, Context), TemplateError> {
    let mut tera = safe_tera();
    tera.add_raw_template("base.mjml", BASE_MJML)?;
    tera.add_raw_template("macros.mjml", MACROS_MJML)?;
    // Supply context for the base template.
    context.insert("application_version", &VERSION);
    let now = Utc::now();
    context.insert("current_year", &now.year().to_string());
    context.insert("date_now", &now.format(MAIL_DATETIME_FORMAT).to_string());

    if let Some(current_session) = session {
        let device_info = &current_session.device_info;
        context.insert("device_type", &device_info);
        context.insert("ip_address", &current_session.ip_address);
    }

    if let Some(ip) = ip_address {
        context.insert("ip_address", ip);
    }

    if let Some(device_info) = device_info {
        context.insert("device_type", device_info);
    }

    Ok((tera, context))
}

/// Sends test message when requested during SMTP configuration process.
/// Note: this function waits for the result.
pub async fn test_mail(
    to: &str,
    conn: &mut PgConnection,
    session: Option<&SessionContext>,
) -> Result<(), TemplateError> {
    let (mut tera, mut context) = get_base_tera_mjml(Context::new(), session, None, None)?;

    let message = MailMessage::Test;
    message.fill_context(conn, &mut context).await?;
    if let Err(err) = message.mail(&mut tera, &context, to)?.send().await {
        warn!("Failed to send test email: {err}");
    }

    Ok(())
}

pub async fn user_import_blocked_mail(
    to: &str,
    conn: &mut PgConnection,
    context: Context,
) -> Result<(), TemplateError> {
    debug!("Render a plain notification mail template for blocked user import.");
    let (mut tera, mut context) = get_base_tera_mjml(context, None, None, None)?;

    let message = MailMessage::UserImportBlocked;
    message.fill_context(conn, &mut context).await?;
    message.mail(&mut tera, &context, to)?.send_and_forget();

    Ok(())
}

// Mail with link to enrollment service.
pub async fn new_account_mail(
    to: &str,
    conn: &mut PgConnection,
    context: Context,
    mut enrollment_service_url: Url,
    enrollment_token: &str,
) -> Result<(), TemplateError> {
    debug!("Render an enrollment start mail template for the user.");
    let (mut tera, mut context) = get_base_tera_mjml(context, None, None, None)?;

    // add required context
    context.insert("defguard_url", &Settings::url()?);
    context.insert("url", &enrollment_service_url);
    context.insert("token", enrollment_token);

    // Build URL to Proxy's "open desktop" page, with token as query.
    if let Ok(mut url) = enrollment_service_url.path_segments_mut() {
        url.push("open-desktop");
    }
    enrollment_service_url
        .query_pairs_mut()
        .append_pair("token", enrollment_token);
    context.insert("link_url", &enrollment_service_url);

    let message = MailMessage::NewAccount;
    message.fill_context(conn, &mut context).await?;
    message.mail(&mut tera, &context, to)?.send_and_forget();

    Ok(())
}

// Mail with link to enrollment service.
pub async fn desktop_start_mail(
    to: &str,
    conn: &mut PgConnection,
    context: Context,
    enrollment_service_url: &Url,
    enrollment_token: &str,
) -> Result<(), TemplateError> {
    debug!("Render a mail template for desktop activation.");
    let (mut tera, mut context) = get_base_tera_mjml(context, None, None, None)?;

    context.insert("url", &enrollment_service_url);
    context.insert("token", enrollment_token);

    let message = MailMessage::DesktopStart;
    message.fill_context(conn, &mut context).await?;
    message.mail(&mut tera, &context, to)?.send_and_forget();

    Ok(())
}

/// Iterator that enforces the supported subset of CommonMark
/// so that only elements with a corresponding rule in `MARKDOWN_EMAIL_STYLES`
/// are emitted to the HTML renderer.
struct EmailEventFilter<'a, I: Iterator<Item = Event<'a>>> {
    iter: I,
    skip_depth: usize,
}

impl<'a, I: Iterator<Item = Event<'a>>> EmailEventFilter<'a, I> {
    fn new(iter: I) -> Self {
        Self {
            iter,
            skip_depth: 0,
        }
    }
}

impl<'a, I: Iterator<Item = Event<'a>>> Iterator for EmailEventFilter<'a, I> {
    type Item = Event<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let event = self.iter.next()?;

            // Inside a skipped block: track nesting depth and discard.
            if self.skip_depth > 0 {
                match &event {
                    Event::Start(_) => self.skip_depth += 1,
                    Event::End(_) => self.skip_depth -= 1,
                    _ => {}
                }
                continue;
            }

            return Some(match event {
                // block elements without styles: skip entirely
                Event::Start(Tag::BlockQuote(_) | Tag::List(Some(_)) | Tag::CodeBlock(_)) => {
                    self.skip_depth = 1;
                    continue;
                }

                // inline elements without styles: drop the tag, keep text
                Event::Start(Tag::Emphasis | Tag::Strikethrough)
                | Event::End(TagEnd::Emphasis | TagEnd::Strikethrough) => continue,

                // inline code: render as plain text
                Event::Code(text) => Event::Text(text),

                // headings: degrade h3-h6 to h2
                Event::Start(Tag::Heading {
                    level,
                    id,
                    classes,
                    attrs,
                }) if !matches!(level, HeadingLevel::H1 | HeadingLevel::H2) => {
                    Event::Start(Tag::Heading {
                        level: HeadingLevel::H2,
                        id,
                        classes,
                        attrs,
                    })
                }
                Event::End(TagEnd::Heading(level))
                    if !matches!(level, HeadingLevel::H1 | HeadingLevel::H2) =>
                {
                    Event::End(TagEnd::Heading(HeadingLevel::H2))
                }

                // raw HTML and horizontal rules: strip
                Event::Html(_) | Event::InlineHtml(_) | Event::Rule => continue,

                other => other,
            });
        }
    }
}

static MARKDOWN_EMAIL_STYLES: &str = r#"
h1 { font-size: 24px; font-weight: 600; color: #141517; line-height: 32px; font-family: Geist, Arial, sans-serif; margin: 0 0 8px 0; }
h2 { font-size: 16px; font-weight: 400; color: #4A5059; line-height: 24px; font-family: Geist, Arial, sans-serif; margin: 0 0 8px 0; }
p { font-size: 14px; font-weight: 400; color: #4A5059; line-height: 20px; font-family: Geist, Arial, sans-serif; margin: 0 0 12px 0; }
a { color: #3961DB; text-decoration: underline; font-size: 14px; line-height: 20px; }
ul { list-style: disc; margin: 0 0 12px 0; padding: 0; }
li { font-size: 14px; font-weight: 400; color: #4A5059; line-height: 20px; font-family: Geist, Arial, sans-serif; margin-left: 21px; }
strong, b { font-weight: 500; }
"#;

/// Renders a markdown string to an inline-styled HTML fragment.
/// Only elements with a corresponding rule in `MARKDOWN_EMAIL_STYLES` are
/// rendered; everything else is stripped or degraded (see `EmailEventFilter`).
pub fn markdown_to_html(content: &str) -> String {
    let parser = EmailEventFilter::new(Parser::new(content));
    let mut raw_html = String::new();
    html::push_html(&mut raw_html, parser);

    match css_inline::inline_fragment(&raw_html, MARKDOWN_EMAIL_STYLES) {
        Ok(styled) => styled,
        Err(err) => {
            warn!("Failed to apply inline styles to markdown HTML: {err}");
            raw_html
        }
    }
}

/// Welcome message sent when activating an account through enrollment.
/// Its content is stored in markdown, so it's parsed into HTML and plain text.
pub fn enrollment_welcome_mail(
    to: &str,
    content: &str,
    ip_address: Option<&str>,
    device_info: Option<&str>,
) -> Result<(), TemplateError> {
    let (mut tera, mut context) =
        get_base_tera_mjml(Context::new(), None, ip_address, device_info)?;

    debug!("Render welcome mail template for user enrollment");
    let html_output = markdown_to_html(content);

    context.insert("welcome_message_content", &html_output);

    let message = MailMessage::Welcome;
    message.mail(&mut tera, &context, to)?.send_and_forget();

    Ok(())
}

/// Notification for admin after user completes an enrollment.
pub async fn enrollment_admin_notification(
    to: &str,
    conn: &mut PgConnection,
    user_name: &str,
    admin_name: &str,
    ip_address: &str,
    device_info: Option<&str>,
) -> Result<(), TemplateError> {
    debug!("Render an admin notification mail template.");
    let (mut tera, mut context) =
        get_base_tera_mjml(Context::new(), None, Some(ip_address), device_info)?;

    context.insert("username", admin_name);
    context.insert("user_name", user_name);

    let message = MailMessage::EnrollmentNotification;
    message.fill_context(conn, &mut context).await?;
    message.mail(&mut tera, &context, to)?.send_and_forget();

    Ok(())
}

/// Email with support data
pub async fn support_data_mail(
    to: &str,
    conn: &mut PgConnection,
    attachments: Vec<Attachment>,
) -> Result<(), TemplateError> {
    let (mut tera, mut context) = get_base_tera_mjml(Context::new(), None, None, None)?;

    let message = MailMessage::SupportData;
    message.fill_context(conn, &mut context).await?;
    message
        .mail(&mut tera, &context, to)?
        .set_attachments(attachments)
        .send_and_forget();

    Ok(())
}

#[derive(Serialize)]
pub struct TemplateLocation {
    pub name: String,
    pub assigned_ips: String,
}

pub async fn new_device_added_mail(
    to: &str,
    conn: &mut PgConnection,
    device_name: &str,
    public_key: &str,
    template_locations: &[TemplateLocation],
    ip_address: Option<&str>,
    device_info: Option<&str>,
) -> Result<(), TemplateError> {
    debug!("Render a new device added mail template for the user.");
    let (mut tera, mut context) =
        get_base_tera_mjml(Context::new(), None, ip_address, device_info)?;
    context.insert("device_name", device_name);
    context.insert("public_key", public_key);
    context.insert("locations", template_locations);

    let message = MailMessage::NewDevice;
    message.fill_context(conn, &mut context).await?;
    message.mail(&mut tera, &context, to)?.send_and_forget();

    Ok(())
}

pub async fn mfa_configured_mail(
    to: &str,
    conn: &mut PgConnection,
    session: Option<&SessionContext>,
    method: &MFAMethod,
) -> Result<(), TemplateError> {
    let (mut tera, mut context) = get_base_tera_mjml(Context::new(), session, None, None)?;

    context.insert("mfa_method", &method);

    let message = MailMessage::MFAConfigured { method: *method };
    message.fill_context(conn, &mut context).await?;
    message.mail(&mut tera, &context, to)?.send_and_forget();

    Ok(())
}

/// New device login.
pub async fn new_device_login_mail(
    to: &str,
    conn: &mut PgConnection,
    session: Option<&SessionContext>,
    created: NaiveDateTime,
) -> Result<(), TemplateError> {
    let (mut tera, mut context) = get_base_tera_mjml(Context::new(), session, None, None)?;

    context.insert("created", &created.format(MAIL_DATETIME_FORMAT).to_string());

    let message = MailMessage::NewDeviceLogin;
    message.fill_context(conn, &mut context).await?;
    message.mail(&mut tera, &context, to)?.send_and_forget();

    Ok(())
}

/// New device login from OpenID Connect.
pub async fn new_device_oidc_login_mail(
    to: &str,
    conn: &mut PgConnection,
    session: Option<&SessionContext>,
    oauth2client_name: &str,
    username: &str,
) -> Result<(), TemplateError> {
    let (mut tera, mut context) = get_base_tera_mjml(Context::new(), session, None, None)?;

    let url = format!("{}user/{}", Settings::url()?, username);
    context.insert("oauth2client_name", &oauth2client_name);
    context.insert("profile_url", &url);

    let message = MailMessage::NewDeviceOIDCLogin;
    message.fill_context(conn, &mut context).await?;
    message.mail(&mut tera, &context, to)?.send_and_forget();

    Ok(())
}

/// Notification about disconnected Gateway.
pub async fn gateway_disconnected_mail(
    to: &str,
    conn: &mut PgConnection,
    gateway_name: &str,
    gateway_ip_address: &str,
    location_name: &str,
) -> Result<(), TemplateError> {
    let (mut tera, mut context) = get_base_tera_mjml(Context::new(), None, None, None)?;

    context.insert("gateway_name", gateway_name);
    context.insert("ip_address", gateway_ip_address);
    context.insert("location_name", location_name);

    let message = MailMessage::GatewayDisconnect;
    message.fill_context(conn, &mut context).await?;
    message.mail(&mut tera, &context, to)?.send_and_forget();

    Ok(())
}

/// Notification about reconnected Gateway.
pub async fn gateway_reconnected_mail(
    to: &str,
    conn: &mut PgConnection,
    gateway_name: &str,
    gateway_ip_address: &str,
    location_name: &str,
) -> Result<(), TemplateError> {
    let (mut tera, mut context) = get_base_tera_mjml(Context::new(), None, None, None)?;

    context.insert("gateway_name", gateway_name);
    context.insert("ip_address", gateway_ip_address);
    context.insert("location_name", location_name);

    let message = MailMessage::GatewayReconnect;
    message.fill_context(conn, &mut context).await?;
    message.mail(&mut tera, &context, to)?.send_and_forget();

    Ok(())
}

pub async fn mfa_activation_mail(
    to: &str,
    conn: &mut PgConnection,
    first_name: &str,
    code: &str,
    session: Option<&SessionContext>,
) -> Result<(), TemplateError> {
    let (mut tera, mut context) = get_base_tera_mjml(Context::new(), session, None, None)?;
    let settings = Settings::get_current_settings();
    let timeout = humantime::format_duration(Duration::from_secs(
        settings.mfa_code_timeout_seconds as u64,
    ));
    context.insert("code", code);
    context.insert("timeout", &timeout.to_string());
    context.insert("username", first_name);
    context.insert(
        "datetime",
        &Utc::now().format(MAIL_DATETIME_FORMAT).to_string(),
    );

    let message = MailMessage::MFAActivation;
    message.fill_context(conn, &mut context).await?;
    message.mail(&mut tera, &context, to)?.send_and_forget();

    Ok(())
}

pub async fn mfa_code_mail(
    to: &str,
    conn: &mut PgConnection,
    first_name: &str,
    code: &str,
    session: Option<&SessionContext>,
) -> Result<(), TemplateError> {
    let (mut tera, mut context) = get_base_tera_mjml(Context::new(), session, None, None)?;
    let settings = Settings::get_current_settings();
    let timeout = humantime::format_duration(Duration::from_secs(
        settings.mfa_code_timeout_seconds as u64,
    ));
    context.insert("code", code);
    context.insert("timeout", &timeout.to_string());
    context.insert("username", first_name);
    context.insert(
        "datetime",
        &Utc::now().format(MAIL_DATETIME_FORMAT).to_string(),
    );

    let message = MailMessage::MFACode;
    message.fill_context(conn, &mut context).await?;
    message.mail(&mut tera, &context, to)?.send_and_forget();

    Ok(())
}

/// Password reset email.
pub async fn password_reset_mail(
    to: &str,
    conn: &mut PgConnection,
    mut service_url: Url,
    password_reset_token: &str,
    ip_address: Option<&str>,
    device_info: Option<&str>,
) -> Result<(), TemplateError> {
    let (mut tera, mut context) =
        get_base_tera_mjml(Context::new(), None, ip_address, device_info)?;

    context.insert("enrollment_url", &service_url);
    context.insert("defguard_url", &Settings::url()?);
    context.insert("token", password_reset_token);

    service_url.set_path("/password-reset");
    service_url
        .query_pairs_mut()
        .append_pair("token", password_reset_token);

    context.insert("link_url", &service_url);

    let message = MailMessage::PasswordReset;
    message.fill_context(conn, &mut context).await?;
    message.mail(&mut tera, &context, to)?.send_and_forget();

    Ok(())
}

/// Successful password reset email.
pub async fn password_reset_success_mail(
    to: &str,
    conn: &mut PgConnection,
    ip_address: Option<&str>,
    device_info: Option<&str>,
) -> Result<(), TemplateError> {
    let (mut tera, mut context) =
        get_base_tera_mjml(Context::new(), None, ip_address, device_info)?;

    let message = MailMessage::PasswordResetDone;
    message.fill_context(conn, &mut context).await?;
    message.mail(&mut tera, &context, to)?.send_and_forget();

    Ok(())
}
