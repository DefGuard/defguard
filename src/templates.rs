use chrono::Datelike;
use reqwest::Url;
use tera::{Context, Tera};
use thiserror::Error;

use crate::{db::User, VERSION};

static MAIL_BASE: &str = include_str!("../templates/base.tera");
static MAIL_MACROS: &str = include_str!("../templates/macros.tera");
static MAIL_TEST: &str = include_str!("../templates/mail_test.tera");
static MAIL_ENROLLMENT_START: &str = include_str!("../templates/mail_enrollment_start.tera");
static MAIL_DESKTOP_START: &str = include_str!("../templates/mail_desktop_start.tera");
static MAIL_ENROLLMENT_WELCOME: &str = include_str!("../templates/mail_enrollment_welcome.tera");
static MAIL_ENROLLMENT_ADMIN_NOTIFICATION: &str =
    include_str!("../templates/mail_enrollment_admin_notification.tera");
static MAIL_SUPPORT_DATA: &str = include_str!("../templates/mail_support_data.tera");
static MAIL_NEW_DEVICE_ADDED: &str = include_str!("../templates/mail_new_device_added.tera");
static MAIL_MFA_CONFIGURED: &str = include_str!("../templates/mail_mfa_configured.tera");

#[allow(dead_code)]
static MAIL_DATE_FORMAT: &str = "%Y-%m-%dT%H:%M:00Z";

#[derive(Error, Debug)]
pub enum TemplateError {
    #[error(transparent)]
    TemplateError(#[from] tera::Error),
}

pub fn get_base_tera(external_context: Option<Context>) -> Result<(Tera, Context), TemplateError> {
    let mut tera = Tera::default();
    let mut context = match external_context {
        Some(external) => external,
        None => Context::new(),
    };
    tera.add_raw_template("base.tera", MAIL_BASE)?;
    tera.add_raw_template("macros.tera", MAIL_MACROS)?;
    // supply context required by base
    context.insert("application_version", &VERSION);
    let now = chrono::Utc::now();
    let current_year = format!("{:04}", &now.year());
    context.insert("current_year", &current_year);
    Ok((tera, context))
}

// sends test message when requested during SMTP configuration process
pub fn test_mail() -> Result<String, TemplateError> {
    let (mut tera, context) = get_base_tera(None)?;
    tera.add_raw_template("mail_test", MAIL_TEST)?;
    Ok(tera.render("mail_test", &context)?)
}

// mail with link to enrollment service
pub fn enrollment_start_mail(
    context: Context,
    mut enrollment_service_url: Url,
    enrollment_token: &str,
) -> Result<String, TemplateError> {
    // prepare enrollment service URL
    enrollment_service_url
        .query_pairs_mut()
        .append_pair("token", enrollment_token);

    let (mut tera, mut context) = get_base_tera(Some(context))?;

    tera.add_raw_template("mail_enrollment_start", MAIL_ENROLLMENT_START)?;

    context.insert("url", &enrollment_service_url.to_string());

    Ok(tera.render("mail_enrollment_start", &context)?)
}
// mail with link to enrollment service
pub fn desktop_start_mail(
    context: Context,
    enrollment_service_url: Url,
    enrollment_token: &str,
) -> Result<String, TemplateError> {
    let (mut tera, mut context) = get_base_tera(Some(context))?;

    tera.add_raw_template("mail_desktop_start", MAIL_DESKTOP_START)?;

    context.insert("url", &enrollment_service_url.to_string());
    context.insert("token", enrollment_token);

    Ok(tera.render("mail_desktop_start", &context)?)
}

// welcome message sent when activating an account through enrollment
// content is stored in markdown, so it's parsed into HTML
pub fn enrollment_welcome_mail(content: &str) -> Result<String, TemplateError> {
    let (mut tera, mut context) = get_base_tera(None)?;
    tera.add_raw_template("mail_enrollment_welcome", MAIL_ENROLLMENT_WELCOME)?;

    // convert content to HTML
    let parser = pulldown_cmark::Parser::new(content);
    let mut html_output = String::new();
    pulldown_cmark::html::push_html(&mut html_output, parser);

    context.insert("welcome_message_content", &html_output);

    Ok(tera.render("mail_enrollment_welcome", &context)?)
}

// notification sent to admin after user completes enrollment
pub fn enrollment_admin_notification(user: &User, admin: &User) -> Result<String, TemplateError> {
    let (mut tera, mut context) = get_base_tera(None)?;

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
    let (mut tera, context) = get_base_tera(None)?;
    tera.add_raw_template("mail_support_data", MAIL_SUPPORT_DATA)?;
    Ok(tera.render("mail_support_data", &context)?)
}

#[derive(Serialize, Debug, Clone)]
pub struct TemplateLocation {
    pub name: String,
    pub assigned_ip: String,
}

pub fn new_device_added_mail(
    device_name: &str,
    public_key: &str,
    template_locations: &Vec<TemplateLocation>,
) -> Result<String, TemplateError> {
    let (mut tera, mut context) = get_base_tera(None)?;
    context.insert("device_name", device_name);
    context.insert("public_key", public_key);
    context.insert("locations", template_locations);
    tera.add_raw_template("mail_new_device_added", MAIL_NEW_DEVICE_ADDED)?;
    Ok(tera.render("mail_new_device_added", &context)?)
}

pub fn mfa_configured_mail(mfa_type: String) -> Result<String, TemplateError> {
    let (mut tera, mut context) = get_base_tera(None)?;
    context.insert("mfa_method", &mfa_type);
    tera.add_raw_template("mail_base", MAIL_BASE)?;
    tera.add_raw_template("mail_mfa_configured", MAIL_MFA_CONFIGURED)?;

    Ok(tera.render("mail_mfa_configured", &context)?)
}
