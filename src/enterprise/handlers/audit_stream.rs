use axum::{
    extract::{Path, State},
    Json,
};
use reqwest::StatusCode;
use serde_json::json;

use crate::{
    appstate::AppState,
    auth::{AdminRole, SessionInfo},
    db::{Id, NoId},
    enterprise::db::models::audit_stream::{AuditStreamConfig, AuditStreamModel, AuditStreamType},
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

#[derive(Debug, Deserialize)]
pub struct AuditStreamModificationRequest {
    pub name: Option<String>,
    pub stream_type: AuditStreamType,
    pub stream_config: serde_json::Value,
}

pub async fn create_audit_stream(
    _admin: AdminRole,
    State(appstate): State<AppState>,
    session: SessionInfo,
    Json(data): Json<AuditStreamModificationRequest>,
) -> ApiResult {
    // validate config
    let _ = AuditStreamConfig::from_serde_value(&data.stream_type, &data.stream_config)?;
    let stream_model: AuditStreamModel<NoId> = AuditStreamModel {
        id: NoId,
        name: data.name,
        stream_type: data.stream_type,
        config: data.stream_config,
    };
    stream_model.save(&appstate.pool).await?;
    Ok(ApiResponse {
        json: json!({}),
        status: StatusCode::CREATED,
    })
}

pub async fn modify_audit_stream(
    _admin: AdminRole,
    State(appstate): State<AppState>,
    session: SessionInfo,
    Path(id): Path<Id>,
    Json(data): Json<AuditStreamModificationRequest>,
) -> ApiResult {
    if let Some(mut stream) = AuditStreamModel::find_by_id(&appstate.pool, id).await? {
        //validate config
        let _ = AuditStreamConfig::from_serde_value(&data.stream_type, &data.stream_config)?;
        stream.name = data.name;
        stream.config = data.stream_config;
        stream.save(&appstate.pool).await?;
        return Ok(ApiResponse::default());
    }
    Err(crate::error::WebError::ObjectNotFound(format!(
        "Audit Stream of id {id} not found."
    )))
}

pub async fn delete_audit_stream(
    _admin: AdminRole,
    State(appstate): State<AppState>,
    session: SessionInfo,
    Path(id): Path<Id>,
) -> ApiResult {
    let session_username = &session.user.username;
    debug!("User {session_username} deleting Audit stream ({id})");
    if let Some(stream) = AuditStreamModel::find_by_id(&appstate.pool, id).await? {
        stream.delete(&appstate.pool).await?;
    } else {
        return Err(crate::error::WebError::ObjectNotFound(format!(
            "Audit Stream of id {id} not found."
        )));
    }
    info!("User {session_username} deleted Audit stream");
    Ok(ApiResponse::default())
}
