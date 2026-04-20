use std::{collections::HashMap, time::Duration};

use chrono::{Datelike, NaiveDateTime, Utc};
use defguard_common::{
    VERSION,
    db::models::{Session, Settings, user::MFAMethod},
    types::UrlParseError,
};
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
    // Convert content to HTML.
    let parser = pulldown_cmark::Parser::new(content);
    let mut html_output = String::new();
    pulldown_cmark::html::push_html(&mut html_output, parser);

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

/// Certificate is about to expire.
pub async fn certificate_expiration_mail(
    to: &str,
    conn: &mut PgConnection,
    certificate_type: &str,
    expiration: NaiveDateTime,
) -> Result<(), TemplateError> {
    let (mut tera, mut context) = get_base_tera_mjml(Context::new(), None, None, None)?;

    context.insert("cert_type", certificate_type);
    context.insert(
        "exp_date",
        &expiration.format(MAIL_DATETIME_FORMAT).to_string(),
    );

    let message = MailMessage::CertificateExpiration;
    message.fill_context(conn, &mut context).await?;
    message.mail(&mut tera, &context, to)?.send_and_forget();

    Ok(())
}

/// Certificate has expired.
pub async fn certificate_expired_mail(
    to: &str,
    conn: &mut PgConnection,
    certificate_type: &str,
    expiration: NaiveDateTime,
) -> Result<(), TemplateError> {
    let (mut tera, mut context) = get_base_tera_mjml(Context::new(), None, None, None)?;

    context.insert("cert_type", certificate_type);
    context.insert(
        "exp_date",
        &expiration.format(MAIL_DATETIME_FORMAT).to_string(),
    );

    let message = MailMessage::CertificateExpired;
    message.fill_context(conn, &mut context).await?;
    message.mail(&mut tera, &context, to)?.send_and_forget();

    Ok(())
}
