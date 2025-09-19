use std::fmt;

use serde::{Deserialize, Serialize};
use sqlx::Type;
use utoipa::ToSchema;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Hash, ToSchema, Type)]
#[sqlx(type_name = "mfa_method", rename_all = "snake_case")]
pub enum MFAMethod {
    None,
    OneTimePassword,
    Webauthn,
    Email,
}

// Web MFA methods
impl fmt::Display for MFAMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                MFAMethod::None => "None",
                MFAMethod::OneTimePassword => "TOTP",
                MFAMethod::Webauthn => "WebAuthn",
                MFAMethod::Email => "Email",
            }
        )
    }
}
