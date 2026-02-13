use std::{collections::HashMap, time::Duration};

use chrono::{Datelike, NaiveDateTime, Utc};
use defguard_common::{
    VERSION,
    db::{
        Id,
        models::{
            Session, Settings,
            user::{MFAMethod, User},
        },
    },
    types::UrlParseError,
};
use reqwest::Url;
use serde::Serialize;
use serde_json::Value;
use sqlx::PgConnection;
use tera::{Context, Function, Tera};
use thiserror::Error;
use tracing::debug;

use crate::{Mail, mail_context::MailContext};

const DEFAULT_LANG: &str = "en_US";

static BASE_MJML: &str = include_str!("../templates/base.mjml");
static MACROS_MJML: &str = include_str!("../templates/macros.mjml");

static DESKTOP_START_SUBJECT: &str = "Defguard desktop client configuration";
static DESKTOP_START_MJML: &str = include_str!("../templates/desktop-start.mjml");
// static DESKTOP_START_TEXT: &str = include_str!("../templates/desktop-start.text");

static NEW_DEVICE_SUBJECT: &str = "Defguard: new device added to your account";
static NEW_DEVICE_MJML: &str = include_str!("../templates/new-device.mjml");
// static NEW_DEVICE_TEXT: &str = include_str!("../templates/new-device.text");

static MFA_CODE_SUBJECT: &str = "Defguard: Multi-Factor Authentication code for login";
static MFA_CODE_MJML: &str = include_str!("../templates/mfa-code.mjml");
// static MFA_CODE_TEXT: &str = include_str!("../templates/mfa-code.text");

static MAIL_BASE: &str = include_str!("../templates/base.tera");
static MAIL_MACROS: &str = include_str!("../templates/macros.tera");
static MAIL_TEST: &str = include_str!("../templates/mail_test.mjml");
static MAIL_ENROLLMENT_START: &str = include_str!("../templates/mail_enrollment_start.tera");
static MAIL_ENROLLMENT_WELCOME: &str = include_str!("../templates/mail_enrollment_welcome.tera");
static MAIL_ENROLLMENT_ADMIN_NOTIFICATION: &str =
    include_str!("../templates/mail_enrollment_admin_notification.tera");
static MAIL_SUPPORT_DATA: &str = include_str!("../templates/mail_support_data.tera");
static MAIL_GATEWAY_DISCONNECTED: &str =
    include_str!("../templates/mail_gateway_disconnected.tera");
static MAIL_GATEWAY_RECONNECTED: &str = include_str!("../templates/mail_gateway_reconnected.tera");
static MAIL_MFA_CONFIGURED: &str = include_str!("../templates/mail_mfa_configured.tera");
static MAIL_NEW_DEVICE_LOGIN: &str = include_str!("../templates/mail_new_device_login.tera");
static MAIL_NEW_DEVICE_OCID_LOGIN: &str =
    include_str!("../templates/mail_new_device_ocid_login.tera");
static MAIL_EMAIL_MFA_ACTIVATION: &str =
    include_str!("../templates/mail_email_mfa_activation.tera");
static MAIL_PASSWORD_RESET_START: &str =
    include_str!("../templates/mail_password_reset_start.tera");
static MAIL_PASSWORD_RESET_SUCCESS: &str =
    include_str!("../templates/mail_password_reset_success.tera");
static MAIL_DATETIME_FORMAT: &str = "%A, %B %d, %Y at %r";

#[derive(Debug, Error)]
pub enum TemplateError {
    #[error("Failed to generate email MFA code")]
    MfaError,
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

pub struct UserContext {
    last_name: String,
    first_name: String,
}

impl From<&User<Id>> for UserContext {
    fn from(user: &User<Id>) -> Self {
        Self {
            last_name: user.last_name.clone(),
            first_name: user.first_name.clone(),
        }
    }
}

fn get_base_tera(
    mut context: Context,
    session: Option<&SessionContext>,
    ip_address: Option<&str>,
    device_info: Option<&str>,
) -> Result<(Tera, Context), TemplateError> {
    let mut tera = safe_tera();
    tera.add_raw_template("base", MAIL_BASE)?;
    tera.add_raw_template("macros", MAIL_MACROS)?;
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

// Sends test message when requested during SMTP configuration process.
pub fn test_mail(session: Option<&SessionContext>) -> Result<String, TemplateError> {
    let (mut tera, context) = get_base_tera_mjml(Context::new(), session, None, None)?;
    tera.add_raw_template("mail_test", MAIL_TEST)?;

    let processed = tera.render("mail_test", &context)?;

    let parsed = mrml::parse(processed)?;
    let opts = mrml::prelude::render::RenderOptions::default();
    let html = parsed.element.render(&opts)?;

    Ok(html)
}

// Mail with link to enrollment service.
pub fn enrollment_start_mail(
    context: Context,
    mut enrollment_service_url: Url,
    enrollment_token: &str,
) -> Result<String, TemplateError> {
    debug!("Render an enrollment start mail template for the user.");
    let (mut tera, mut context) = get_base_tera(context, None, None, None)?;

    // add required context
    context.insert("enrollment_url", &enrollment_service_url);
    context.insert("defguard_url", &Settings::url()?);
    context.insert("token", enrollment_token);

    // prepare enrollment service URL
    enrollment_service_url
        .query_pairs_mut()
        .append_pair("token", enrollment_token);

    context.insert("link_url", &enrollment_service_url);

    tera.add_raw_template("mail_enrollment_start", MAIL_ENROLLMENT_START)?;

    let processed = tera.render("mail_enrollment_start", &context)?;
    Ok(processed)
}

// Mail with link to enrollment service.
pub async fn desktop_start_mail(
    to: &str,
    transaction: &mut PgConnection,
    context: Context,
    enrollment_service_url: &Url,
    enrollment_token: &str,
) -> Result<(), TemplateError> {
    debug!("Render a mail template for desktop activation.");
    let (mut tera, mut context) = get_base_tera_mjml(context, None, None, None)?;

    let template = "desktop-start";
    tera.add_raw_template(template, DESKTOP_START_MJML)?;
    let db_context = MailContext::all_for_template(transaction, template, DEFAULT_LANG)
        .await
        .unwrap();
    for c in db_context {
        context.insert(c.section, &c.text);
    }

    context.insert("url", &enrollment_service_url);
    context.insert("token", enrollment_token);

    // TODO: Move to Mail once every message is converted to MJML.
    let processed = tera.render(template, &context)?;
    let parsed = mrml::parse(processed)?;
    let opts = mrml::prelude::render::RenderOptions::default();
    let html = parsed.element.render(&opts)?;

    Mail::new(to, DESKTOP_START_SUBJECT, html).send_and_forget();

    Ok(())
}

// Welcome message sent when activating an account through enrollment
// content is stored in markdown, so it's parsed into HTML.
pub fn enrollment_welcome_mail(
    content: &str,
    ip_address: Option<&str>,
    device_info: Option<&str>,
) -> Result<String, TemplateError> {
    debug!("Render a welcome mail template for user enrollment.");
    let (mut tera, mut context) = get_base_tera(Context::new(), None, ip_address, device_info)?;
    tera.add_raw_template("mail_enrollment_welcome", MAIL_ENROLLMENT_WELCOME)?;

    // convert content to HTML
    let parser = pulldown_cmark::Parser::new(content);
    let mut html_output = String::new();
    pulldown_cmark::html::push_html(&mut html_output, parser);

    context.insert("welcome_message_content", &html_output);

    Ok(tera.render("mail_enrollment_welcome", &context)?)
}

// Notification for admin after user completes an enrollment.
pub fn enrollment_admin_notification(
    user: &UserContext,
    admin: &UserContext,
    ip_address: &str,
    device_info: Option<&str>,
) -> Result<String, TemplateError> {
    debug!("Render an admin notification mail template.");
    let (mut tera, mut context) =
        get_base_tera(Context::new(), None, Some(ip_address), device_info)?;

    tera.add_raw_template(
        "mail_enrollment_admin_notification",
        MAIL_ENROLLMENT_ADMIN_NOTIFICATION,
    )?;
    context.insert("first_name", &user.first_name);
    context.insert("last_name", &user.last_name);
    context.insert("admin_first_name", &admin.first_name);
    context.insert("admin_last_name", &admin.last_name);

    Ok(tera.render("mail_enrollment_admin_notification", &context)?)
}

// message with support data
pub fn support_data_mail() -> Result<String, TemplateError> {
    let (mut tera, context) = get_base_tera(Context::new(), None, None, None)?;
    tera.add_raw_template("mail_support_data", MAIL_SUPPORT_DATA)?;
    Ok(tera.render("mail_support_data", &context)?)
}

#[derive(Serialize)]
pub struct TemplateLocation {
    pub name: String,
    pub assigned_ips: String,
}

pub async fn new_device_added_mail(
    to: &str,
    transaction: &mut PgConnection,
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

    let template = "new-device";
    tera.add_raw_template(template, NEW_DEVICE_MJML)?;
    let db_context = MailContext::all_for_template(transaction, template, DEFAULT_LANG)
        .await
        .unwrap();
    for c in db_context {
        context.insert(c.section, &c.text);
    }

    // TODO: Move to Mail once every message is converted to MJML.
    let processed = tera.render(template, &context)?;
    let parsed = mrml::parse(processed)?;
    let opts = mrml::prelude::render::RenderOptions::default();
    let html = parsed.element.render(&opts)?;

    Mail::new(to, NEW_DEVICE_SUBJECT, html).send_and_forget();

    Ok(())
}

pub fn mfa_configured_mail(
    session: Option<&SessionContext>,
    method: &MFAMethod,
) -> Result<String, TemplateError> {
    let (mut tera, mut context) = get_base_tera(Context::new(), session, None, None)?;
    context.insert("mfa_method", &method);
    tera.add_raw_template("mail_base", MAIL_BASE)?;
    tera.add_raw_template("mail_mfa_configured", MAIL_MFA_CONFIGURED)?;

    Ok(tera.render("mail_mfa_configured", &context)?)
}

pub fn new_device_login_mail(
    session: &SessionContext,
    created: NaiveDateTime,
) -> Result<String, TemplateError> {
    let (mut tera, mut context) = get_base_tera(Context::new(), Some(session), None, None)?;
    tera.add_raw_template("mail_base", MAIL_BASE)?;
    context.insert(
        "date_now",
        &created.format(MAIL_DATETIME_FORMAT).to_string(),
    );

    tera.add_raw_template("mail_new_device_login", MAIL_NEW_DEVICE_LOGIN)?;
    Ok(tera.render("mail_new_device_login", &context)?)
}

pub fn new_device_ocid_login_mail(
    session: &SessionContext,
    oauth2client_name: &str,
) -> Result<String, TemplateError> {
    let (mut tera, mut context) = get_base_tera(Context::new(), Some(session), None, None)?;
    tera.add_raw_template("mail_base", MAIL_BASE)?;

    let url = format!("{}me", Settings::url()?);

    context.insert("oauth2client_name", &oauth2client_name);
    context.insert("profile_url", &url);

    tera.add_raw_template("mail_new_device_oicd_login", MAIL_NEW_DEVICE_OCID_LOGIN)?;
    Ok(tera.render("mail_new_device_oicd_login", &context)?)
}

pub fn gateway_disconnected_mail(
    gateway_name: &str,
    gateway_ip: &str,
    network_name: &str,
) -> Result<String, TemplateError> {
    let (mut tera, mut context) = get_base_tera(Context::new(), None, None, None)?;
    context.insert("gateway_name", gateway_name);
    context.insert("gateway_ip", gateway_ip);
    context.insert("network_name", network_name);
    tera.add_raw_template("mail_gateway_disconnected", MAIL_GATEWAY_DISCONNECTED)?;
    Ok(tera.render("mail_gateway_disconnected", &context)?)
}

pub fn gateway_reconnected_mail(
    gateway_name: &str,
    gateway_ip: &str,
    network_name: &str,
) -> Result<String, TemplateError> {
    let (mut tera, mut context) = get_base_tera(Context::new(), None, None, None)?;
    context.insert("gateway_name", gateway_name);
    context.insert("gateway_ip", gateway_ip);
    context.insert("network_name", network_name);
    tera.add_raw_template("mail_gateway_reconnected", MAIL_GATEWAY_RECONNECTED)?;
    Ok(tera.render("mail_gateway_reconnected", &context)?)
}

pub fn email_mfa_activation_mail(
    user: &UserContext,
    code: &str,
    session: Option<&SessionContext>,
) -> Result<String, TemplateError> {
    let (mut tera, mut context) = get_base_tera(Context::new(), session, None, None)?;
    let settings = Settings::get_current_settings();
    let timeout = humantime::format_duration(Duration::from_secs(
        settings.mfa_code_timeout_seconds as u64,
    ));
    context.insert("code", code);
    context.insert("timeout", &timeout.to_string());
    context.insert("name", &user.first_name);
    tera.add_raw_template("mail_email_mfa_activation", MAIL_EMAIL_MFA_ACTIVATION)?;

    Ok(tera.render("mail_email_mfa_activation", &context)?)
}

pub async fn mfa_code_mail(
    to: &str,
    transaction: &mut PgConnection,
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

    let template = "mfa-code";
    tera.add_raw_template(template, MFA_CODE_MJML)?;
    let db_context = MailContext::all_for_template(transaction, template, DEFAULT_LANG)
        .await
        .unwrap();
    for c in db_context {
        context.insert(c.section, &c.text);
    }

    // TODO: Move to Mail once every message is converted to MJML.
    let processed = tera.render(template, &context)?;
    let parsed = mrml::parse(processed)?;
    let opts = mrml::prelude::render::RenderOptions::default();
    let html = parsed.element.render(&opts)?;

    Mail::new(to, MFA_CODE_SUBJECT, html).send_and_forget();

    Ok(())
}

pub fn email_password_reset_mail(
    mut service_url: Url,
    password_reset_token: &str,
    ip_address: Option<&str>,
    device_info: Option<&str>,
) -> Result<String, TemplateError> {
    let (mut tera, mut context) = get_base_tera(Context::new(), None, ip_address, device_info)?;

    context.insert("enrollment_url", &service_url);
    context.insert("defguard_url", &Settings::url()?);
    context.insert("token", password_reset_token);

    service_url.set_path("/password-reset");
    service_url
        .query_pairs_mut()
        .append_pair("token", password_reset_token);

    context.insert("link_url", &service_url);

    tera.add_raw_template("mail_passowrd_reset_start", MAIL_PASSWORD_RESET_START)?;

    Ok(tera.render("mail_passowrd_reset_start", &context)?)
}

pub fn email_password_reset_success_mail(
    ip_address: Option<&str>,
    device_info: Option<&str>,
) -> Result<String, TemplateError> {
    let (mut tera, context) = get_base_tera(Context::new(), None, ip_address, device_info)?;

    tera.add_raw_template("mail_passowrd_reset_success", MAIL_PASSWORD_RESET_SUCCESS)?;

    Ok(tera.render("mail_passowrd_reset_success", &context)?)
}

#[cfg(test)]
mod test {
    use claims::assert_ok;
    use defguard_common::{
        config::{DefGuardConfig, SERVER_CONFIG},
        db::{models::settings::initialize_current_settings, setup_pool},
    };
    use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

    use super::*;

    // fn get_welcome_context() -> Context {
    //     let mut context = Context::new();
    //     context.insert("first_name", "test_first");
    //     context.insert("last_name", "test_last");
    //     context.insert("username", "username");
    //     context.insert("defguard_url", "test_url");
    //     context.insert("defguard_version", &VERSION);
    //     context.insert("admin_first_name", "test_first_name");
    //     context.insert("admin_last_name", "test_last_name");
    //     context.insert("admin_email", "test_email");
    //     context.insert("admin_phone", "test_phone");
    //     context
    // }

    async fn init_config(pool: &sqlx::PgPool) {
        let mut config = DefGuardConfig::new_test_config();
        initialize_current_settings(pool)
            .await
            .expect("Could not initialize current settings in the database");
        config.initialize_post_settings();
        let _ = SERVER_CONFIG.set(config.clone());
    }

    #[test]
    fn test_mfa_configured_mail() {
        let mfa_method = MFAMethod::OneTimePassword;
        assert_ok!(mfa_configured_mail(None, &mfa_method));
    }

    #[test]
    fn test_base_mail_no_context() {
        assert_ok!(get_base_tera(Context::new(), None, None, None));
    }

    #[test]
    fn test_test_mail() {
        assert_ok!(test_mail(None));
    }

    #[sqlx::test]
    async fn test_enrollment_start_mail(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;
        init_config(&pool).await;
        assert_ok!(enrollment_start_mail(
            Context::new(),
            Url::parse("http://localhost:8080").unwrap(),
            "test_token"
        ));
    }

    #[sqlx::test]
    async fn test_enrollment_welcome_mail(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;
        init_config(&pool).await;
        assert_ok!(enrollment_welcome_mail(
            "Hi there! Welcome to Defguard.",
            None,
            None
        ));
    }

    // #[sqlx::test]
    // async fn test_desktop_start_mail(_: PgPoolOptions, options: PgConnectOptions) {
    //     let pool = setup_pool(options).await;
    //     init_config(&pool).await;
    //     let context = get_welcome_context();
    //     let url = Url::parse("http://127.0.0.1:8080").unwrap();
    //     let token = "TestToken";
    //     let mut tranaction = pool.begin().await.unwrap();
    //     assert_ok!(desktop_start_mail(&mut tranaction, context, &url, token).await);
    // }

    // #[sqlx::test]
    // async fn test_new_device_added_mail(_: PgPoolOptions, options: PgConnectOptions) {
    //     let pool = setup_pool(options).await;
    //     init_config(&pool).await;
    //     let template_locations: Vec<TemplateLocation> = vec![
    //         TemplateLocation {
    //             name: "Test 01".into(),
    //             assigned_ips: "10.0.0.10".into(),
    //         },
    //         TemplateLocation {
    //             name: "Test 02".into(),
    //             assigned_ips: "10.0.0.10".into(),
    //         },
    //     ];
    //     assert_ok!(new_device_added_mail(
    //         "Test device",
    //         "TestKey",
    //         &template_locations,
    //         Some("1.1.1.1"),
    //         None,
    //     ));
    // }

    #[sqlx::test]
    async fn test_gateway_disconnected(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;
        init_config(&pool).await;
        assert_ok!(gateway_disconnected_mail(
            "Gateway A",
            "127.0.0.1",
            "Location1"
        ));
    }

    #[sqlx::test]
    async fn test_enrollment_admin_notification(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;
        init_config(&pool).await;
        let test_user = UserContext {
            last_name: "test_last".into(),
            first_name: "test_first".into(),
        };

        assert_ok!(enrollment_admin_notification(
            &test_user,
            &test_user,
            "11.11.11.11",
            None
        ));
    }

    #[test]
    fn dg25_8_server_side_template_injection() {
        let mut tera = safe_tera();
        tera.add_raw_template("text", "PATH={{ get_env(name=\"PATH\") }}")
            .unwrap();
        assert!(tera.render("text", &Context::new()).is_err());
    }
}
