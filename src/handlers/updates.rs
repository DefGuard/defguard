use axum::http::StatusCode;
use serde_json::json;

use super::{ApiResponse, ApiResult};
use crate::{
    auth::{AdminRole, SessionInfo},
    updates::get_update,
};

pub async fn check_new_version(_admin: AdminRole, session: SessionInfo) -> ApiResult {
    debug!(
        "User {} is checking if there is a new version available",
        session.user.username
    );
    let update = get_update();
    if let Some(update) = update.as_ref() {
        debug!("A new version is available, returning the update information");
        Ok(ApiResponse {
            json: json!(update),
            status: StatusCode::OK,
        })
    } else {
        debug!("No new version available");
        Ok(ApiResponse {
            json: serde_json::json!({ "message": "No updates available" }),
            status: StatusCode::NOT_FOUND,
        })
    }
}
