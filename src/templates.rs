use tera::{Context, Tera};
use thiserror::Error;

static MAIL_BASE: &str = include_str!("../templates/mail_base.tpl");
static MAIL_TEST: &str = include_str!("../templates/mail_test.tpl");

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

#[cfg(test)]
mod test {
    use claims::assert_ok;

    use super::*;
    #[test]
    fn test_test_mail() {
        assert_ok!(test_mail());
    }
}
