pub mod group_diff;
pub mod proxy;
pub mod user_info;

pub type UrlParseError = url::ParseError;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AuthFlowType {
    Enrollment,
    Mfa,
}
