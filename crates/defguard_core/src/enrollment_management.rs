use defguard_common::db::{Id, models::user::User};
use defguard_mail::templates::{desktop_start_mail, new_account_mail};
use reqwest::Url;
use sqlx::{PgConnection, PgExecutor};

use crate::db::models::enrollment::{ENROLLMENT_TOKEN_TYPE, Token, TokenError};

/// Start user enrollment process
/// This creates a new enrollment token valid for 24h
/// and optionally sends enrollment email notification to user
pub async fn start_user_enrollment(
    user: &mut User<Id>,
    conn: &mut PgConnection,
    admin: &User<Id>,
    email: Option<String>,
    token_timeout_seconds: u64,
    enrollment_service_url: Url,
    send_user_notification: bool,
) -> Result<String, TokenError> {
    info!(
        "User {} started a new enrollment process for user {}.",
        admin.username, user.username
    );
    debug!(
        "Notify user by mail about the enrollment process: {}",
        send_user_notification
    );
    debug!("Check if {} has a password.", user.username);
    if user.has_password() {
        debug!(
            "User {} that you want to start enrollment process for already has a password.",
            user.username
        );
        return Err(TokenError::AlreadyActive);
    }

    debug!("Verify that {} is an active user.", user.username);
    if !user.is_active {
        warn!(
            "Can't create enrollment token for disabled user {}",
            user.username
        );
        return Err(TokenError::UserDisabled);
    }

    clear_unused_enrollment_tokens(user, &mut *conn).await?;

    debug!("Create a new enrollment token for user {}.", user.username);
    let enrollment = Token::new(
        user.id,
        Some(admin.id),
        email.clone(),
        token_timeout_seconds,
        Some(ENROLLMENT_TOKEN_TYPE.to_string()),
    );
    debug!("Saving a new enrollment token...");
    enrollment.save(&mut *conn).await?;
    debug!(
        "Saved a new enrollment token with id {} for user {}.",
        enrollment.id, user.username
    );

    // Mark the user with enrollment-pending flag.
    // https://github.com/DefGuard/client/issues/647
    user.enrollment_pending = true;
    user.save(&mut *conn).await?;

    if send_user_notification {
        if let Some(email) = email {
            debug!(
                "Sending an enrollment mail for user {} to {email}.",
                user.username
            );
            let base_message_context = enrollment.get_welcome_message_context(&mut *conn).await?;
            let result = new_account_mail(
                &email,
                conn,
                base_message_context,
                enrollment_service_url,
                &enrollment.id,
            )
            .await;
            match result {
                Ok(()) => {
                    info!(
                        "Sent enrollment start mail for user {} to {email}",
                        user.username
                    );
                }
                Err(err) => {
                    error!("Error sending mail: {err}");
                    return Err(TokenError::NotificationError(err.to_string()));
                }
            }
        }
    }
    info!(
        "New enrollment token has been generated for {}.",
        user.username
    );

    Ok(enrollment.id)
}

/// Start user remote desktop configuration process
/// This creates a new enrollment token valid for 24h
/// and optionally sends email notification to user
pub async fn start_desktop_configuration(
    user: &User<Id>,
    conn: &mut PgConnection,
    admin: &User<Id>,
    email: Option<String>,
    token_timeout_seconds: u64,
    enrollment_service_url: Url,
    send_user_notification: bool,
    // Whether to attach some device to the token. It allows for a partial initialization of
    // the device before the desktop configuration has taken place.
    device_id: Option<Id>,
) -> Result<String, TokenError> {
    info!(
        "User {} starting a new desktop activation for user {}",
        admin.username, user.username
    );
    debug!(
        "Notify {} by mail about the enrollment process: {}",
        user.username, send_user_notification
    );

    debug!("Verify that {} is an active user.", user.username);
    if !user.is_active {
        warn!(
            "Can't create desktop activation token for disabled user {}.",
            user.username
        );
        return Err(TokenError::UserDisabled);
    }

    clear_unused_enrollment_tokens(user, &mut *conn).await?;
    debug!("Cleared unused tokens for {}.", user.username);

    debug!(
        "Create a new desktop activation token for user {}.",
        user.username
    );
    let mut desktop_configuration = Token::new(
        user.id,
        Some(admin.id),
        email.clone(),
        token_timeout_seconds,
        Some(ENROLLMENT_TOKEN_TYPE.to_string()),
    );
    if let Some(device_id) = device_id {
        desktop_configuration.device_id = Some(device_id);
    }
    debug!("Saving a new desktop configuration token...");
    desktop_configuration.save(&mut *conn).await?;
    debug!(
        "Saved a new desktop activation token with id {} for user {}.",
        desktop_configuration.id, user.username
    );

    if send_user_notification {
        if let Some(email) = email {
            debug!(
                "Sending a desktop configuration mail for user {} to {email}",
                user.username
            );
            let base_message_context = desktop_configuration
                .get_welcome_message_context(&mut *conn)
                .await?;
            let result = desktop_start_mail(
                &email,
                conn,
                base_message_context,
                &enrollment_service_url,
                &desktop_configuration.id,
            )
            .await;
            if let Err(err) = result {
                debug!(
                    "Cannot send an email to the user {} due to the error {err}.",
                    user.username,
                );
            }
        }
    }
    info!(
        "New desktop activation token has been generated for {}.",
        user.username
    );

    Ok(desktop_configuration.id)
}

// Remove unused tokens when triggering user enrollment
pub async fn clear_unused_enrollment_tokens<'e, E>(
    user: &User<Id>,
    executor: E,
) -> Result<(), TokenError>
where
    E: PgExecutor<'e>,
{
    info!("Removing unused tokens for user {}.", user.username);
    Token::delete_unused_user_tokens(executor, user.id).await
}
