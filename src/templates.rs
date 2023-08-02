use reqwest::Url;
use tera::{Context, Tera};
use thiserror::Error;

static MAIL_BASE: &str = include_str!("../templates/mail_base.tpl");
static MAIL_TEST: &str = include_str!("../templates/mail_test.tpl");
static MAIL_ENROLLMENT_START: &str = include_str!("../templates/mail_enrollment_start.tpl");
static MAIL_ENROLLMENT_WELCOME: &str = include_str!("../templates/mail_enrollment_welcome.tpl");

#[derive(Error, Debug)]
pub enum TemplateError {
    #[error(transparent)]
    TemplateError(#[from] tera::Error),
}

pub fn test_mail() -> Result<String, TemplateError> {
    let mut tera = Tera::default();
    tera.add_raw_template("mail_base", MAIL_BASE)?;
    tera.add_raw_template("mail_test", MAIL_TEST)?;
    Ok(tera.render("mail_test", &Context::new())?)
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

    let mut tera = Tera::default();
    tera.add_raw_template("mail_base", MAIL_BASE)?;
    tera.add_raw_template("mail_enrollment_start", MAIL_ENROLLMENT_START)?;

    let mut context = Context::new();
    context.insert("url", &enrollment_service_url.to_string());

    Ok(tera.render("mail_enrollment_start", &context)?)
}

// welcome message sent when activating an account through enrollment
pub fn enrollment_welcome_mail(content: &str) -> Result<String, TemplateError> {
    let mut tera = Tera::default();
    tera.add_raw_template("mail_base", MAIL_BASE)?;
    tera.add_raw_template("mail_enrollment_welcome", MAIL_ENROLLMENT_WELCOME)?;

    let mut context = Context::new();
    context.insert("content", content);

    Ok(tera.render("mail_enrollment_welcome", &context)?)
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
            Url::parse("http://localhost:8080"),
            "test_token"
        ));
    }

    #[test]
    fn test_enrollment_welcome_mail() {
        assert_ok!(enrollment_welcome_mail("Hi there! Welcome to DefGuard."));
    }
}
