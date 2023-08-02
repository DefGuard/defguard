use handlebars::Handlebars;
use reqwest::Url;
use serde_json::json;
use thiserror::Error;

static MAIL_BASE: &str = include_str!("../templates/mail_base.tpl");
static MAIL_TEST: &str = include_str!("../templates/mail_test.tpl");
static MAIL_ENROLLMENT_START: &str = include_str!("../templates/mail_enrollment_start.tpl");
static MAIL_ENROLLMENT_WELCOME: &str = include_str!("../templates/mail_enrollment_welcome.tpl");

#[derive(Error, Debug)]
pub enum TemplateError {
    #[error(transparent)]
    RenderError(#[from] handlebars::RenderError),

    #[error(transparent)]
    TemplateError(#[from] handlebars::TemplateError),
}

pub fn test_mail() -> Result<String, TemplateError> {
    let mut bars = Handlebars::new();
    bars.register_template_string("mail_base", MAIL_BASE)?;
    bars.register_template_string("mail_test", MAIL_TEST)?;

    Ok(bars.render("mail_test", &json!({"parent": "mail_base"}))?)
}

// mail with link to enrollment service
pub fn enrollment_start_mail(
    mut enrollment_service_url: Url,
    enrollment_token: &str,
) -> Result<String, TemplateError> {
    // prepare enrollment service URL
    enrollment_service_url
        .query_pairs_mut()
        .append_pair("token", enrollment_token);

    let mut bars = Handlebars::new();
    bars.register_template_string("mail_base", MAIL_BASE)?;
    bars.register_template_string("mail_enrollment_start", MAIL_ENROLLMENT_START)?;

    Ok(bars.render(
        "mail_enrollment_start",
        &json!({"parent": "mail_base", "url": enrollment_service_url.to_string()}),
    )?)
}

// welcome message sent when activating an account through enrollment
pub fn enrollment_welcome_mail(content: &str) -> Result<String, TemplateError> {
    let mut bars = Handlebars::new();
    bars.register_template_string("mail_base", MAIL_BASE)?;
    bars.register_template_string("mail_enrollment_welcome", MAIL_ENROLLMENT_WELCOME)?;

    Ok(bars.render(
        "mail_enrollment_welcome",
        &json!({"parent": "mail_base", "content": content}),
    )?)
}
