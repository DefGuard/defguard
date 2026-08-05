use axum::{Json, extract::State, http::StatusCode};
use struct_patch::Patch;

use super::LicenseInfo;
use crate::{
    appstate::AppState,
    auth::{AdminRole, SessionInfo},
    enterprise::db::models::enterprise_settings::{EnterpriseSettings, EnterpriseSettingsPatch},
    handlers::{ApiErrorResponse, ApiResponse, ApiResult},
};

/// Get enterprise settings
///
/// Available to every authenticated user.
#[utoipa::path(
    get,
    path = "/api/v1/settings_enterprise",
    tag = "settings",
    responses(
        (status = 200, description = "Enterprise settings.", body = Object, example = json!({
            "admin_device_management": false,
            "client_traffic_policy": "none",
            "only_client_activation": false
        })),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 500, description = "Unable to get enterprise settings.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub async fn get_enterprise_settings(
    session: SessionInfo,
    State(appstate): State<AppState>,
) -> ApiResult {
    debug!(
        "User {} retrieving enterprise settings",
        session.user.username
    );
    let settings = EnterpriseSettings::get(&appstate.pool).await?;
    debug!(
        "User {} retrieved enterprise settings",
        session.user.username
    );
    Ok(ApiResponse::json(settings, StatusCode::OK))
}

/// Update selected enterprise settings
#[utoipa::path(
    patch,
    path = "/api/v1/settings_enterprise",
    tag = "settings",
    request_body = Object,
    responses(
        (status = 200, description = "Enterprise settings updated."),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges and an active enterprise license.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 500, description = "Unable to update enterprise settings.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub async fn patch_enterprise_settings(
    _license: LicenseInfo,
    _admin: AdminRole,
    State(appstate): State<AppState>,
    session: SessionInfo,
    Json(data): Json<EnterpriseSettingsPatch>,
) -> ApiResult {
    debug!(
        "Admin {} patching enterprise settings.",
        session.user.username,
    );
    let mut settings = EnterpriseSettings::get(&appstate.pool).await?;

    settings.apply(data);
    settings.save(&appstate.pool).await?;
    info!("Admin {} patched settings.", session.user.username);
    Ok(ApiResponse::default())
}
