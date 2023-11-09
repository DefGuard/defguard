use chrono::Datelike;
use reqwest::Url;
use tera::{Context, Tera};
use thiserror::Error;

use crate::{
    db::{MFAMethod, User},
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
    context.insert("date_now", &now.format("%A, %B %d, %Y at %r").to_string());

    if !context.contains_key("device_type") {
        context.insert("device_type", "");
    }

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
    device_type: Option<&str>,
) -> Result<String, TemplateError> {
    let (mut tera, mut context) = get_base_tera(None)?;
    context.insert("device_name", device_name);
    context.insert("public_key", public_key);
    context.insert("locations", template_locations);

    if device_type.is_some() {
        context.insert("device_type", &device_type);
    }

    tera.add_raw_template("mail_new_device_added", MAIL_NEW_DEVICE_ADDED)?;
    Ok(tera.render("mail_new_device_added", &context)?)
}

pub fn mfa_configured_mail(method: &MFAMethod) -> Result<String, TemplateError> {
    let (mut tera, mut context) = get_base_tera(None)?;
    context.insert("mfa_method", &method.to_string());
    tera.add_raw_template("mail_base", MAIL_BASE)?;
    tera.add_raw_template("mail_mfa_configured", MAIL_MFA_CONFIGURED)?;

    Ok(tera.render("mail_mfa_configured", &context)?)
}

#[cfg(test)]
mod test {
    use claims::assert_ok;

    use super::*;

    fn get_welcome_context() -> Context {
        let mut context = Context::new();
        context.insert("first_name", "test_first");
        context.insert("last_name", "test_last");
        context.insert("username", "username");
        context.insert("defguard_url", "test_url");
        context.insert("defguard_version", &VERSION);
        context.insert("admin_first_name", "test_first_name");
        context.insert("admin_last_name", "test_last_name");
        context.insert("admin_email", "test_email");
        context.insert("admin_phone", "test_phone");
        context
    }

    #[test]
    fn test_mfa_configured_mail() {
        let mfa_method = MFAMethod::OneTimePassword;
        assert_ok!(mfa_configured_mail(&mfa_method));
    }

    #[test]
    fn test_base_mail_no_context() {
        assert_ok!(get_base_tera(None));
    }

    #[test]
    fn test_base_mail_external_context() {
        let external_context: Context = Context::new();
        assert_ok!(get_base_tera(Some(external_context)));
    }

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
    fn test_desktop_start_mail() {
        let external_context = get_welcome_context();
        let url = Url::parse("http://127.0.0.1:8080").unwrap();
        let token = "TestToken";
        assert_ok!(desktop_start_mail(external_context, url, token));
    }

    #[test]
    fn test_new_device_added_mail() {
        let template_locations: Vec<TemplateLocation> = vec![
            TemplateLocation {
                name: "Test 01".into(),
                assigned_ip: "10.0.0.10".into(),
            },
            TemplateLocation {
                name: "Test 02".into(),
                assigned_ip: "10.0.0.10".into(),
            },
        ];
        assert_ok!(new_device_added_mail(
            "Test device",
            "TestKey",
            &template_locations,
            None
        ));
    }

    #[test]
    fn test_enrollment_admin_notification() {
        let test_user: User = User::new(
            "test".into(),
            "1234".into(),
            "test_last".into(),
            "test_first".into(),
            "test@example.com".into(),
            Some("99999".into()),
        );
        assert_ok!(enrollment_admin_notification(&test_user, &test_user));
    }
}
