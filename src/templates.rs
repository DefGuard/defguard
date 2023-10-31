use chrono::{DateLike, Utc};
use reqwest::Url;
use tera::{Context, Tera};
use thiserror::Error;

use crate::{
    db::{Device, User},
    VERSION,
};

static MAIL_BASE: &str = include_str!("../templates/base.tera");
static MAIL_MACROS: &str = include_str!("../templates/macros.tera");
static MAIL_TEST: &str = include_str!("../templates/mail_test.tera");
static MAIL_ENROLLMENT_START: &str = include_str!("../templates/mail_enrollment_start.tera");
static MAIL_DESKTOP_START: &str = include_str!("../templates/mail_desktop_start.tera");
static MAIL_ENROLLMENT_WELCOME: &str = include_str!("../templates/mail_enrollment_welcome.tera");
static MAIL_ENROLLMENT_ADMIN_NOTIFICATION: &str =
    include_str!("../templates/mail_enrollment_admin_notification.tera");
static MAIL_SUPPORT_DATA: &str = include_str!("../templates/mail_support_data.tera");
static MAIL_NEW_DEVICE_ADDED: &str = include_str!("../templates/mail_new_device_added.tpl");
static MAIL_MFA_CONFIGURED: &str = include_str!("../templates/mail_mfa_configured.tpl");

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

pub fn new_device_added_mail(
    device: Device,
    device_network_ips: Vec<String>,
) -> Result<String, TemplateError> {
    let mut tera = Tera::default();
    let mut context = Context::new();
    tera.add_raw_template("mail_base", MAIL_BASE)?;
    tera.add_raw_template("mail_new_device_added", MAIL_NEW_DEVICE_ADDED)?;

    context.insert("version", &VERSION);
    context.insert("device_name", &device.name);
    context.insert("public_key", &device.wireguard_pubkey);
    context.insert("date", &device.created);
    context.insert("ip_addresses", &device_network_ips);

    Ok(tera.render("mail_new_device_added", &context)?)
}

pub fn mfa_configured_mail(mfa_type: String) -> Result<String, TemplateError> {
    let mut tera = Tera::default();
    let mut context = Context::new();

    context.insert("mfa_type", &mfa_type);
    let date = &Utc::now().format("%Y-%m-%dT%H:%M:00Z").to_string();
    context.insert("mfa_date", date);

    tera.add_raw_template("mail_base", MAIL_BASE)?;
    tera.add_raw_template("mail_mfa_configured", MAIL_MFA_CONFIGURED)?;

    Ok(tera.render("mail_mfa_configured", &context)?)
}

pub fn new_device_added_mail(
    device: Device,
    device_network_ips: Vec<String>,
) -> Result<String, TemplateError> {
    let mut tera = Tera::default();
    let mut context = Context::new();
    tera.add_raw_template("mail_base", MAIL_BASE)?;
    tera.add_raw_template("mail_new_device_added", MAIL_NEW_DEVICE_ADDED)?;

    context.insert("version", &VERSION);
    context.insert("device_name", &device.name);
    context.insert("public_key", &device.wireguard_pubkey);
    context.insert("date", &device.created);
    context.insert("ip_addresses", &device_network_ips);

    Ok(tera.render("mail_new_device_added", &context)?)
}

pub fn mfa_configured_mail(mfa_type: String) -> Result<String, TemplateError> {
    let mut tera = Tera::default();
    let mut context = Context::new();

    context.insert("mfa_type", &mfa_type);
    let date = &Utc::now().format("%Y-%m-%dT%H:%M:00Z").to_string();
    context.insert("mfa_date", date);

    tera.add_raw_template("mail_base", MAIL_BASE)?;
    tera.add_raw_template("mail_mfa_configured", MAIL_MFA_CONFIGURED)?;

    Ok(tera.render("mail_mfa_configured", &context)?)
}

#[cfg(test)]
mod test {
    use claims::assert_ok;

    use super::*;
    #[test]
    fn test_test_mail() {
        assert_ok!(test_mail());
    }

    #[test]
    fn test_enrollment_start_mail() {
        assert_ok!(enrollment_start_mail(
            Context::new(),
            Url::parse("http://localhost:8080").unwrap(),
            "test_token"
        ));
    }

    #[test]
    fn test_enrollment_welcome_mail() {
        assert_ok!(enrollment_welcome_mail("Hi there! Welcome to DefGuard."));
    }

    #[test]
    fn test_support_data_mail() {
        assert_ok!(support_data_mail());
    }
}
