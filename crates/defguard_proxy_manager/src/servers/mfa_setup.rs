//! Shared tail for enabling an MFA factor, used by both the enrollment/config
//! setup-finish path ([`super::EnrollmentServer::mfa_setup_finish`]) and the
//! MFA-config email fallback ([`super::MfaConfigServer::mfa_config_authorize`]).

use defguard_common::db::{
    Id,
    models::{MFAMethod, User},
};
use defguard_core::{
    events::{ApiEvent, ApiEventType, ApiRequestContext},
    grpc::utils::parse_client_ip_agent,
    mail::templates::mfa_configured_mail,
};
use defguard_proto::proxy::DeviceInfo;
use sqlx::{PgPool, Postgres, Transaction};
use tokio::sync::mpsc::UnboundedSender;
use tonic::Status;

/// Finishes enabling an MFA factor after the caller has run the factor-specific
/// enable inside `transaction`.
///
/// Logs out the user's other sessions, resolves the recovery codes (freshly
/// regenerated when `fresh_recovery_codes` is set), commits, flips `mfa_enabled`
/// on the account, sends the "MFA configured" confirmation email, and emits
/// `event`. Returns the recovery codes to surface to the client (empty when the
/// user already had codes and `fresh_recovery_codes` is false).
#[allow(clippy::too_many_arguments)]
pub(super) async fn finalize_mfa_factor(
    pool: &PgPool,
    event_tx: &UnboundedSender<ApiEvent>,
    mut transaction: Transaction<'_, Postgres>,
    user: &mut User<Id>,
    mfa_method: MFAMethod,
    event: ApiEventType,
    device_info: Option<DeviceInfo>,
    fresh_recovery_codes: bool,
) -> Result<Vec<String>, Status> {
    // Enabling MFA invalidates all existing sessions in the same transaction.
    user.logout_all_sessions(&mut *transaction)
        .await
        .map_err(|err| {
            error!("Failed to log out user sessions: {err}");
            Status::internal("Failed to log out user sessions".to_owned())
        })?;
    // Regenerate recovery codes for a fresh factor; existing users keep theirs.
    if fresh_recovery_codes {
        user.clear_recovery_codes(&mut *transaction)
            .await
            .map_err(|err| {
                error!("Failed to clear recovery codes: {err}");
                Status::internal("Failed to clear recovery codes".to_owned())
            })?;
    }
    // Existing recovery codes were already shown, so return an empty list.
    let recovery_codes = user
        .get_recovery_codes(&mut *transaction)
        .await
        .map_err(|_| Status::internal("Failed to get recovery codes.".to_owned()))?
        .unwrap_or_default();
    transaction.commit().await.map_err(|err| {
        error!("Failed to commit database transaction: {err}");
        Status::internal("Failed to commit database transaction".to_owned())
    })?;

    // Commit before reading the saved factor state or sending the confirmation email.
    user.enable_mfa(pool)
        .await
        .map_err(|_| Status::internal("Enabling MFA on the account failed.".to_owned()))?;
    match pool.acquire().await {
        Ok(mut conn) => {
            if let Err(err) =
                mfa_configured_mail(&user.email, &mut conn, None, &mfa_method, &user.first_name)
                    .await
            {
                error!("Failed to send MFA configured email\nReason: {err}");
            }
        }
        Err(err) => error!("Failed to acquire database connection: {err}"),
    }
    let (ip, user_agent) = parse_client_ip_agent(&device_info).map_err(Status::internal)?;
    let context = ApiRequestContext::new(user.id, user.username.clone(), ip, user_agent);
    event_tx
        .send(ApiEvent {
            context,
            event: Box::new(event),
        })
        .map_err(|err| {
            error!("Failed to send event. Reason: {err}");
            Status::internal("unexpected error")
        })?;
    Ok(recovery_codes)
}
