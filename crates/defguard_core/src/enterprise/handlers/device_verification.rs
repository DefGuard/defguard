use axum::{
    extract::{Path, State},
    http::StatusCode,
};
use chrono::NaiveDateTime;
use defguard_common::db::{
    Id,
    models::{
        Device, DeviceType,
        device_verification::{DEVICE_VERIFICATION_CODE_PERIOD, DeviceVerification},
    },
};
use serde::Serialize;
use sqlx::PgConnection;

use super::EnterpriseTier;
use crate::{
    appstate::AppState,
    auth::{AdminRole, SessionInfo},
    error::WebError,
    events::{ApiEvent, ApiEventType, ApiRequestContext},
    handlers::{ApiResponse, ApiResult},
};

#[derive(Debug, Serialize)]
pub(crate) struct DeviceVerificationResponse {
    pub device_id: Id,
    pub code: String,
    pub code_expires_in: u64,
    pub code_period: u64,
    pub created_at: NaiveDateTime,
    pub expires_at: NaiveDateTime,
}

async fn find_user_device(conn: &mut PgConnection, device_id: Id) -> Result<Device<Id>, WebError> {
    let device = Device::find_by_id(&mut *conn, device_id)
        .await?
        .ok_or_else(|| WebError::ObjectNotFound(format!("device id {device_id} not found")))?;
    if device.device_type != DeviceType::User {
        return Err(WebError::BadRequest(format!(
            "device id {device_id} is not a user device"
        )));
    }
    Ok(device)
}

/// Returns the active verification of the device, starting a new one if there is none.
pub(crate) async fn start_device_verification(
    _role: AdminRole,
    _license: EnterpriseTier,
    session: SessionInfo,
    context: ApiRequestContext,
    Path(device_id): Path<Id>,
    State(appstate): State<AppState>,
) -> ApiResult {
    let username = &session.user.username;
    debug!("User {username} requested verification of device {device_id}");
    let mut conn = appstate.pool.acquire().await?;
    let device = find_user_device(&mut conn, device_id).await?;
    let (verification, created) = DeviceVerification::new(device.id, Some(session.user.id))
        .start_or_get(&mut conn)
        .await?;

    if created {
        let owner = device.get_owner(&mut *conn).await?;
        info!(
            "User {username} started verification of device {}({}) owned by user {}",
            device.name, device.id, owner.username
        );
        appstate.emit_event(ApiEvent {
            context,
            event: Box::new(ApiEventType::UserDeviceVerificationStarted { owner, device }),
        })?;
    }

    let code = verification.current_code();
    Ok(ApiResponse::json(
        DeviceVerificationResponse {
            device_id: verification.device_id,
            code: code.code,
            code_expires_in: code.expires_in,
            code_period: DEVICE_VERIFICATION_CODE_PERIOD,
            created_at: verification.created_at,
            expires_at: verification.expires_at,
        },
        StatusCode::OK,
    ))
}

pub(crate) async fn cancel_device_verification(
    _role: AdminRole,
    _license: EnterpriseTier,
    session: SessionInfo,
    context: ApiRequestContext,
    Path(device_id): Path<Id>,
    State(appstate): State<AppState>,
) -> ApiResult {
    let username = &session.user.username;
    debug!("User {username} cancelling verification of device {device_id}");
    let mut conn = appstate.pool.acquire().await?;
    let device = find_user_device(&mut conn, device_id).await?;
    if DeviceVerification::delete_active_for_device(&mut *conn, device.id)
        .await?
        .is_none()
    {
        return Err(WebError::ObjectNotFound(format!(
            "no active verification for device id {device_id}"
        )));
    }

    let owner = device.get_owner(&mut *conn).await?;
    info!(
        "User {username} cancelled verification of device {}({}) owned by user {}",
        device.name, device.id, owner.username
    );
    appstate.emit_event(ApiEvent {
        context,
        event: Box::new(ApiEventType::UserDeviceVerificationCancelled { owner, device }),
    })?;

    Ok(ApiResponse::default())
}
