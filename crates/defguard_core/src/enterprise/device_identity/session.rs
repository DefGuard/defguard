use chrono::NaiveDateTime;
use defguard_common::db::{
    Id,
    models::{Device, Settings},
};
use sqlx::PgConnection;
use tonic::Status;

use super::{challenge_data::session_deadline, database_error};
use crate::{
    db::models::enrollment::{ENROLLMENT_TOKEN_TYPE, Token},
    enterprise::db::models::device_identity::DeviceIdentity,
};

struct Authorization {
    user_id: Id,
    used_at: Option<NaiveDateTime>,
    token_type: Option<String>,
    is_active: bool,
}

pub(super) async fn lock_attestation_session(
    connection: &mut PgConnection,
    enrollment: &Token,
    device: &Device<Id>,
) -> Result<NaiveDateTime, Status> {
    // keep token, user and device locks until the challenge or identity write commits
    let authorization = sqlx::query_as!(
        Authorization,
        "SELECT token.user_id, token.used_at, token.token_type, \"user\".is_active \
         FROM token JOIN \"user\" ON \"user\".id = token.user_id WHERE token.id = $1 \
         FOR UPDATE OF token FOR SHARE OF \"user\"",
        &enrollment.id,
    )
    .fetch_optional(&mut *connection)
    .await
    .map_err(database_error)?
    .ok_or_else(|| Status::unauthenticated("enrollment session not found"))?;
    if authorization.token_type.as_deref() != Some(ENROLLMENT_TOKEN_TYPE) {
        return Err(Status::permission_denied("invalid enrollment token type"));
    }
    if authorization.user_id != enrollment.user_id || authorization.used_at != enrollment.used_at {
        return Err(Status::unauthenticated("enrollment session has changed"));
    }
    if !authorization.is_active {
        return Err(Status::permission_denied("user is disabled"));
    }
    let locked = sqlx::query_scalar!("SELECT id FROM device WHERE id = $1 FOR UPDATE", device.id)
        .fetch_optional(&mut *connection)
        .await
        .map_err(database_error)?;
    if locked.is_none() {
        return Err(Status::not_found("device not found"));
    }
    let current = Device::find_by_id(&mut *connection, device.id)
        .await
        .map_err(database_error)?
        .ok_or_else(|| Status::not_found("device not found"))?;
    if current.user_id != enrollment.user_id {
        return Err(Status::permission_denied(
            "enrollment token is not valid for specified device",
        ));
    }
    if current.wireguard_pubkey != device.wireguard_pubkey {
        return Err(Status::failed_precondition(
            "device WireGuard key has changed",
        ));
    }
    if DeviceIdentity::find_by_device_id(&mut *connection, device.id)
        .await
        .map_err(database_error)?
        .is_some()
    {
        return Err(Status::failed_precondition(
            "device already has an attestation identity",
        ));
    }
    // read the wall clock after lock waits, transaction time may already be expired
    let now = sqlx::query_scalar!("SELECT clock_timestamp() AT TIME ZONE 'UTC' AS \"now!\"")
        .fetch_one(&mut *connection)
        .await
        .map_err(database_error)?;
    let deadline = session_deadline(
        enrollment,
        Settings::get_current_settings().enrollment_session_timeout(),
    )?;
    if now >= deadline {
        return Err(Status::unauthenticated("enrollment session has expired"));
    }
    Ok(now)
}
