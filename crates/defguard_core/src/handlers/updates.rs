use axum::{extract::State, http::StatusCode};
use serde_json::{Value, json};

use super::{ApiErrorResponse, ApiResponse, ApiResult};
use crate::{
    appstate::AppState,
    auth::{AdminRole, SessionInfo},
    updates::get_update,
    version::IncompatibleComponents,
};

/// Get information about a newer defguard release, if any.
#[utoipa::path(
    get,
    path = "/api/v1/updates",
    tag = "system",
    responses(
        (status = 200, description = "Information about the newest release, or an empty object when up to date.", body = Object),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 500, description = "Unable to check for updates.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn check_new_version(_admin: AdminRole, session: SessionInfo) -> ApiResult {
    debug!(
        "User {} is checking if there is a new version available",
        session.user.username
    );
    let json = if let Some(update) = get_update().as_ref() {
        debug!("A new version is available, returning the update information");
        json!(update)
    } else {
        debug!("No new version available");
        // Front-end expects empty JSON.
        Value::Null
    };
    Ok(ApiResponse::new(json, StatusCode::OK))
}

// FIXME: Switch to SSE and generally make it better.
/// List connected components whose version is incompatible with this Core.
#[utoipa::path(
    get,
    path = "/api/v1/outdated",
    tag = "system",
    responses(
        (status = 200, description = "All incompatible components.", body = Object),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 500, description = "Unable to list outdated components.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn outdated_components(
    _admin: AdminRole,
    State(appstate): State<AppState>,
) -> ApiResult {
    IncompatibleComponents::remove_expired(&appstate.incompatible_components);
    let incompatible_components = (*appstate
        .incompatible_components
        .read()
        .expect("Failed to lock appstate.incompatible_components"))
    .clone();
    Ok(ApiResponse::json(incompatible_components, StatusCode::OK))
}
