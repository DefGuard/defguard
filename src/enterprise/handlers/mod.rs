use crate::{
    auth::{AdminRole, SessionInfo},
    enterprise::get_counts,
    handlers::{ApiResponse, ApiResult},
};

pub mod enterprise_settings;
pub mod openid_login;
pub mod openid_providers;

use axum::{
    extract::{FromRef, FromRequestParts},
    http::{request::Parts, StatusCode},
};

use super::{
    db::models::enterprise_settings::EnterpriseSettings, is_enterprise_enabled,
    license::get_cached_license,
};
use crate::{appstate::AppState, error::WebError};

pub struct LicenseInfo {
    pub valid: bool,
}

/// Used to check if user is allowed to manage his devices.
pub struct CanManageDevices;

impl<S> FromRequestParts<S> for LicenseInfo
where
    S: Send + Sync,
    AppState: FromRef<S>,
{
    type Rejection = WebError;

    async fn from_request_parts(_parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        if is_enterprise_enabled() {
            Ok(LicenseInfo { valid: true })
        } else {
            Err(WebError::Forbidden(
                "Enterprise features are disabled".into(),
            ))
        }
    }
}

/// Gets full information about enterprise status.
pub async fn check_enterprise_info(_admin: AdminRole, _session: SessionInfo) -> ApiResult {
    let license = get_cached_license();
    let license_info = license.as_ref().map(|license| {
        let counts = get_counts();
        serde_json::json!(
            {
                "valid_until": license.valid_until,
                "subscription": license.subscription,
                "expired": license.is_max_overdue(),
                "limits_exceeded": counts.is_over_license_limits(license)
            }
        )
    });
    Ok(ApiResponse {
        json: serde_json::json!(
            {
                "license_info": license_info,
            }
        ),
        status: StatusCode::OK,
    })
}

impl<S> FromRequestParts<S> for CanManageDevices
where
    S: Send + Sync,
    AppState: FromRef<S>,
{
    type Rejection = WebError;

    /// Returns an error if current session user is not allowed to manage devices.
    /// The permission is defined by [`EnterpriseSettings::admin_device_management`] setting.
    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let appstate = AppState::from_ref(state);
        let session = SessionInfo::from_request_parts(parts, state).await?;
        let settings = EnterpriseSettings::get(&appstate.pool).await?;
        if settings.admin_device_management && !session.is_admin {
            Err(WebError::Forbidden(
                "Only admin users can manage devices".into(),
            ))
        } else {
            Ok(Self)
        }
    }
}
