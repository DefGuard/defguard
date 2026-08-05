use crate::{
    auth::{AdminRole, SessionInfo},
    enterprise::get_counts,
    handlers::{ApiErrorResponse, ApiResponse, ApiResult},
};

pub mod acl;
pub mod activity_log_stream;
pub mod api_tokens;
pub mod enterprise_settings;
pub mod openid_login;
pub mod openid_providers;

use axum::{
    extract::{FromRef, FromRequestParts},
    http::{StatusCode, request::Parts},
};
use serde::Serialize;

use super::{
    db::models::enterprise_settings::EnterpriseSettings, is_business_license_active,
    license::get_cached_license,
};
use crate::{appstate::AppState, error::WebError};

pub struct LicenseInfo {
    pub valid: bool,
}

/// Used to check if user is allowed to manage his devices.
pub struct CanManageDevices;

#[derive(Serialize)]
struct LimitInfo {
    current: u32,
    limit: u32,
}

#[derive(Serialize)]
struct LicenseLimitsInfo {
    users: LimitInfo,
    locations: LimitInfo,
    user_devices: Option<LimitInfo>,
    network_devices: Option<LimitInfo>,
    devices: Option<LimitInfo>,
}

impl<S> FromRequestParts<S> for LicenseInfo
where
    S: Send + Sync,
    AppState: FromRef<S>,
{
    type Rejection = WebError;

    async fn from_request_parts(_parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        if is_business_license_active() {
            Ok(LicenseInfo { valid: true })
        } else {
            Err(WebError::Forbidden("Enterprise features are disabled"))
        }
    }
}

/// Get information about the enterprise license
#[utoipa::path(
    get,
    path = "/api/v1/enterprise_info",
    tag = "license",
    responses(
        (status = 200, description = "License information, or `null` in `license_info` when no license is present.", body = Object, example = json!({
            "license_info": {
                "valid_until": "2027-01-01T00:00:00Z",
                "subscription": true,
                "expired": false,
                "limits_exceeded": false,
                "tier": "Enterprise",
                "support_type": "DirectEnterprise",
                "limits": {
                    "users": {"current": 12, "limit": 100},
                    "locations": {"current": 2, "limit": 10},
                    "user_devices": null,
                    "network_devices": null,
                    "devices": {"current": 30, "limit": 500}
                }
            }
        })),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 500, description = "Unable to get license information.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub async fn check_enterprise_info(_admin: AdminRole, _session: SessionInfo) -> ApiResult {
    let license = get_cached_license();
    let license_info = license
        .as_ref()
        .map(|license: &crate::enterprise::license::License| {
            let counts = get_counts();
            let limits_info = license.limits.map(|limits| LicenseLimitsInfo {
                locations: LimitInfo {
                    current: counts.location(),
                    limit: limits.locations,
                },
                users: LimitInfo {
                    current: counts.user(),
                    limit: limits.users,
                },
                devices: limits.network_devices.map_or(
                    Some(LimitInfo {
                        current: counts.user_device() + counts.network_device(),
                        limit: limits.devices,
                    }),
                    |_| None,
                ),
                user_devices: limits.network_devices.map(|_| LimitInfo {
                    current: counts.user_device(),
                    limit: limits.devices,
                }),
                network_devices: limits
                    .network_devices
                    .map(|network_devices_limit| LimitInfo {
                        current: counts.network_device(),
                        limit: network_devices_limit,
                    }),
            });

            serde_json::json!({
                "valid_until": license.valid_until,
                "subscription": license.subscription,
                "expired": license.is_max_overdue(),
                "limits_exceeded": counts.is_over_license_limits(license),
                "tier": license.tier,
                "support_type": license.support_type,
                "limits": limits_info,
            })
        });
    Ok(ApiResponse::json(
        serde_json::json!({"license_info": license_info}),
        StatusCode::OK,
    ))
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
            Err(WebError::Forbidden("Only admin users can manage devices"))
        } else {
            Ok(Self)
        }
    }
}
