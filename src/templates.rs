use crate::db::User;
use reqwest::Url;
use tera::{Context, Tera};
use thiserror::Error;

static MAIL_BASE: &str = include_str!("../templates/mail_base.tpl");
static MAIL_TEST: &str = include_str!("../templates/mail_test.tpl");
static MAIL_ENROLLMENT_START: &str = include_str!("../templates/mail_enrollment_start.tpl");
static MAIL_ENROLLMENT_WELCOME: &str = include_str!("../templates/mail_enrollment_welcome.tpl");
static MAIL_ENROLLMENT_ADMIN_NOTIFICATION: &str =
    include_str!("../templates/mail_enrollment_admin_notification.tpl");
static MAIL_SUPPORT_DATA: &str = include_str!("../templates/mail_support_data.tpl");

#[derive(Error, Debug)]
pub enum TemplateError {
    #[error(transparent)]
    TemplateError(#[from] tera::Error),
}

// sends test message when requested during SMTP configuration process
pub fn test_mail() -> Result<String, TemplateError> {
    let mut tera = Tera::default();
    tera.add_raw_template("mail_base", MAIL_BASE)?;
    tera.add_raw_template("mail_test", MAIL_TEST)?;
    Ok(tera.render("mail_test", &Context::new())?)
}

// mail with link to enrollment service
pub fn enrollment_start_mail(
    mut context: Context,
    mut enrollment_service_url: Url,
    enrollment_token: &str,
) -> Result<String, TemplateError> {
    // prepare enrollment service URL
    enrollment_service_url
        .query_pairs_mut()
        .append_pair("token", enrollment_token);

    let mut tera = Tera::default();
    tera.add_raw_template("mail_base", MAIL_BASE)?;
    tera.add_raw_template("mail_enrollment_start", MAIL_ENROLLMENT_START)?;

    context.insert("url", &enrollment_service_url.to_string());

    Ok(tera.render("mail_enrollment_start", &context)?)
}

// welcome message sent when activating an account through enrollment
// content is stored in markdown, so it's parsed into HTML
pub fn enrollment_welcome_mail(content: &str) -> Result<String, TemplateError> {
    let mut tera = Tera::default();
    tera.add_raw_template("mail_base", MAIL_BASE)?;
    tera.add_raw_template("mail_enrollment_welcome", MAIL_ENROLLMENT_WELCOME)?;

    // convert content to HTML
    let parser = pulldown_cmark::Parser::new(content);
    let mut html_output = String::new();
    pulldown_cmark::html::push_html(&mut html_output, parser);

    let mut context = Context::new();
    context.insert("welcome_message_content", &html_output);

    Ok(tera.render("mail_enrollment_welcome", &context)?)
}

// notification sent to admin after user completes enrollment
pub fn enrollment_admin_notification(user: &User, admin: &User) -> Result<String, TemplateError> {
    let mut tera = Tera::default();
    tera.add_raw_template("mail_base", MAIL_BASE)?;
    tera.add_raw_template(
        "mail_enrollment_admin_notification",
        MAIL_ENROLLMENT_ADMIN_NOTIFICATION,
    )?;

    let mut context = Context::new();
    context.insert("first_name", &user.first_name);
    context.insert("last_name", &user.last_name);
    context.insert("admin_first_name", &admin.first_name);
    context.insert("admin_last_name", &admin.last_name);

    Ok(tera.render("mail_enrollment_admin_notification", &context)?)
}

// message with support data
pub fn support_data_mail() -> Result<String, TemplateError> {
    let mut tera = Tera::default();
    tera.add_raw_template("mail_base", MAIL_BASE)?;
    tera.add_raw_template("mail_support_data", MAIL_SUPPORT_DATA)?;

    Ok(tera.render("mail_support_data", &Context::new())?)
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
