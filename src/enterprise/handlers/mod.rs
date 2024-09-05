use crate::{
    auth::SessionInfo,
    enterprise::license::validate_license,
    handlers::{ApiResponse, ApiResult},
};

pub mod enterprise_settings;
pub mod openid_login;
pub mod openid_providers;

use axum::{
    async_trait,
    extract::{FromRef, FromRequestParts},
    http::{request::Parts, StatusCode},
};

use super::{db::models::enterprise_settings::EnterpriseSettings, license::get_cached_license};
use crate::{appstate::AppState, error::WebError};

pub struct LicenseInfo {
    pub valid: bool,
}

/// Used to check if user is allowed to manage his devices.
pub struct CanManageDevices;

#[async_trait]
impl<S> FromRequestParts<S> for LicenseInfo
where
    S: Send + Sync,
    AppState: FromRef<S>,
{
    type Rejection = WebError;

    async fn from_request_parts(_parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let license = get_cached_license();

        match validate_license(license.as_ref()) {
            // Useless struct, but may come in handy later
            Ok(()) => Ok(LicenseInfo { valid: true }),
            Err(e) => Err(WebError::Forbidden(e.to_string())),
        }
    }
}

pub async fn check_enterprise_status() -> ApiResult {
    let license = get_cached_license();
    let valid = validate_license((license).as_ref()).is_ok();
    let license_info = license.as_ref().map(|license| {
        serde_json::json!(
            {
                "valid_until": license.valid_until,
                "subscription": license.subscription,
            }
        )
    });
    Ok(ApiResponse {
        json: serde_json::json!({ "enabled": valid,
               "license_info": license_info
        }),
        status: StatusCode::OK,
    })
}

#[async_trait]
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
