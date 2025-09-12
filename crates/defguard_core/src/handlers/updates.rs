use axum::{extract::State, http::StatusCode};
use serde_json::{Value, json};

use super::{ApiResponse, ApiResult};
use crate::{
    appstate::AppState,
    auth::{AdminRole, SessionInfo},
    updates::get_update, version::IncompatibleComponents,
};

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
    Ok(ApiResponse {
        json,
        status: StatusCode::OK,
    })
}

// FIXME: Switch to SSE and generally make it better.
pub(crate) async fn outdated_components(
    _admin: AdminRole,
    State(appstate): State<AppState>,
) -> ApiResult {
	IncompatibleComponents::remove_old(&appstate.incompatible_components);
    let incompatible_components = (*appstate
        .incompatible_components
        .read()
        .expect("Failed to lock appstate.incompatible_components"))
    .clone();
    Ok(ApiResponse::new(
        json!(incompatible_components),
        StatusCode::OK,
    ))
}
