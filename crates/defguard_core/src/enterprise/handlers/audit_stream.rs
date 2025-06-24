use axum::{
    extract::{Path, State},
    Json,
};
use reqwest::StatusCode;
use serde_json::json;

use super::LicenseInfo;
use crate::{
    appstate::AppState,
    auth::{AdminRole, SessionInfo},
    db::{Id, NoId},
    enterprise::db::models::audit_stream::{AuditStream, AuditStreamConfig, AuditStreamType},
    events::{ApiEvent, ApiEventType, ApiRequestContext},
    handlers::{ApiResponse, ApiResult},
};

pub async fn get_audit_stream(
    _admin: AdminRole,
    State(appstate): State<AppState>,
    session: SessionInfo,
) -> ApiResult {
    debug!("User {} retrieving audit stream's", session.user.username);
    let mut conn = appstate.pool.acquire().await?;
    let streams = AuditStream::all(&mut *conn).await?;
    info!("User {} retrieved audit stream's", session.user.username);
    Ok(ApiResponse {
        json: json!(streams),
        status: StatusCode::OK,
    })
}

#[derive(Debug, Deserialize)]
pub struct AuditStreamModificationRequest {
    pub name: String,
    pub stream_type: AuditStreamType,
    pub stream_config: serde_json::Value,
}

pub async fn create_audit_stream(
    _license: LicenseInfo,
    _admin: AdminRole,
    State(appstate): State<AppState>,
    session: SessionInfo,
    context: ApiRequestContext,
    Json(data): Json<AuditStreamModificationRequest>,
) -> ApiResult {
    let session_username = &session.user.username;
    debug!("User {session_username} creates audit stream");
    // validate config
    let _ = AuditStreamConfig::from_serde_value(&data.stream_type, &data.stream_config)?;
    let stream_model: AuditStream<NoId> = AuditStream {
        id: NoId,
        name: data.name,
        stream_type: data.stream_type,
        config: data.stream_config,
    };
    let stream = stream_model.save(&appstate.pool).await?;
    info!("User {session_username} created audit stream");
    appstate.emit_event(ApiEvent {
        context,
        event: Box::new(ApiEventType::AuditStreamCreated { stream }),
    })?;
    Ok(ApiResponse {
        json: json!({}),
        status: StatusCode::CREATED,
    })
}

pub async fn modify_audit_stream(
    _license: LicenseInfo,
    _admin: AdminRole,
    State(appstate): State<AppState>,
    session: SessionInfo,
    context: ApiRequestContext,
    Path(id): Path<Id>,
    Json(data): Json<AuditStreamModificationRequest>,
) -> ApiResult {
    let session_username = &session.user.username;
    debug!("User {session_username} modifies audit stream ");
    if let Some(mut stream) = AuditStream::find_by_id(&appstate.pool, id).await? {
        // store stream before modifications
        let before = stream.clone();
        //validate config
        let _ = AuditStreamConfig::from_serde_value(&data.stream_type, &data.stream_config)?;
        stream.name = data.name;
        stream.config = data.stream_config;
        stream.save(&appstate.pool).await?;
        info!(
            "User {session_username} modified audit stream {}",
            stream.name
        );
        appstate.emit_event(ApiEvent {
            context,
            event: Box::new(ApiEventType::AuditStreamModified {
                before,
                after: stream,
            }),
        })?;
        debug!("AuditStreamModified api event sent");
        return Ok(ApiResponse::default());
    }
    Err(crate::error::WebError::ObjectNotFound(format!(
        "Audit Stream of id {id} not found."
    )))
}

pub async fn delete_audit_stream(
    _license: LicenseInfo,
    _admin: AdminRole,
    State(appstate): State<AppState>,
    session: SessionInfo,
    context: ApiRequestContext,
    Path(id): Path<Id>,
) -> ApiResult {
    let session_username = &session.user.username;
    debug!("User {session_username} deleting Audit stream ({id})");
    if let Some(stream) = AuditStream::find_by_id(&appstate.pool, id).await? {
        stream.clone().delete(&appstate.pool).await?;
        appstate.emit_event(ApiEvent {
            context,
            event: Box::new(ApiEventType::AuditStreamRemoved { stream }),
        })?;
    } else {
        return Err(crate::error::WebError::ObjectNotFound(format!(
            "Audit Stream of id {id} not found."
        )));
    }
    info!("User {session_username} deleted Audit stream");
    debug!("AuditStreamRemoved api event sent");
    Ok(ApiResponse::default())
}
