use defguard_common::db::{
    Id,
    models::{ThrottleScope, User},
};
use sqlx::PgExecutor;

use crate::error::WebError;

/// A known account uses its stored username, so that its username and email share one count.
#[must_use]
pub fn login_key(user: Option<&User<Id>>, username_or_email: &str) -> String {
    user.map_or_else(
        || username_or_email.to_lowercase(),
        |user| user.username.clone(),
    )
}

/// Call before the credential is examined, and [`refund_login_attempt`] a correct one.
pub async fn charge_login_attempt<'e, E: PgExecutor<'e>>(
    executor: E,
    key: &str,
) -> Result<(), WebError> {
    if ThrottleScope::WebLogin.hit(executor, key).await? {
        Ok(())
    } else {
        info!("Preventing login for {key}: too many failed attempts");
        Err(WebError::TooManyLoginAttempts)
    }
}

pub async fn refund_login_attempt<'e, E: PgExecutor<'e>>(
    executor: E,
    key: &str,
) -> Result<(), WebError> {
    ThrottleScope::WebLogin.refund(executor, key).await?;
    Ok(())
}
