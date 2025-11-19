use crate::db::{
    Id,
    models::{MFAMethod, user::User},
};
use serde::{Deserialize, Serialize};
use sqlx::{Error as SqlxError, PgPool, query_as};

#[derive(Deserialize, Serialize)]
pub struct MFAInfo {
    pub mfa_method: MFAMethod,
    totp_available: bool,
    webauthn_available: bool,
    email_available: bool,
}

impl MFAInfo {
    pub async fn for_user(pool: &PgPool, user: &User<Id>) -> Result<Option<Self>, SqlxError> {
        query_as!(
            Self,
            "SELECT mfa_method \"mfa_method: _\", totp_enabled totp_available, \
            email_mfa_enabled email_available, \
            (SELECT count(*) > 0 FROM webauthn WHERE user_id = $1) \"webauthn_available!\" \
            FROM \"user\" WHERE \"user\".id = $1",
            user.id
        )
        .fetch_optional(pool)
        .await
    }

    #[must_use]
    pub fn mfa_available(&self) -> bool {
        self.webauthn_available || self.totp_available || self.email_available
    }

    #[must_use]
    pub fn current_mfa_method(&self) -> &MFAMethod {
        &self.mfa_method
    }

    #[must_use]
    pub fn list_available_methods(&self) -> Option<Vec<MFAMethod>> {
        if !self.mfa_available() {
            return None;
        }

        let mut methods = Vec::new();
        if self.webauthn_available {
            methods.push(MFAMethod::Webauthn);
        }
        if self.totp_available {
            methods.push(MFAMethod::OneTimePassword);
        }
        if self.email_available {
            methods.push(MFAMethod::Email);
        }
        Some(methods)
    }
}
