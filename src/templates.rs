use handlebars::Handlebars;
use serde_json::json;
use thiserror::Error;

static MAIL_BASE: &str = include_str!("../templates/mail_base.tpl");
static MAIL_TEST: &str = include_str!("../templates/mail_test.tpl");

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
