use axum::extract::State;
use reqwest::StatusCode;
use serde_json::json;

use crate::{
    appstate::AppState,
    auth::{AdminRole, SessionInfo},
    enterprise::db::models::audit_stream::AuditStreamModel,
    handlers::{ApiResponse, ApiResult},
};

pub async fn get_audit_stream(
    _admin: AdminRole,
    State(appstate): State<AppState>,
    session: SessionInfo,
) -> ApiResult {
    debug!("User {} retrieving audit stream's", session.user.username);
    let mut conn = appstate.pool.acquire().await?;
    let streams = AuditStreamModel::all(&mut *conn).await?;
    info!("User {} retrieved audit stream's", session.user.username);
    Ok(ApiResponse {
        json: json!(streams),
        status: StatusCode::OK,
    })
}
