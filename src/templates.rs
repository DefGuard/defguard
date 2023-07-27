static MAIL_HEADER: &str = include_str!("../templates/mail_header.tpl");
static MAIL_FOOTER: &str = include_str!("../templates/mail_footer.tpl");
static MAIL_TEST: &str = include_str!("../templates/mail_test.tpl");

pub fn test_mail() -> String {
    format!("{MAIL_HEADER}\n{MAIL_TEST}\n{MAIL_FOOTER}")
}
