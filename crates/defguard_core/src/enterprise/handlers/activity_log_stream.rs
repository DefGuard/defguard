use axum::{
    Json,
    extract::{Path, State},
};
use defguard_common::db::{Id, NoId};
use reqwest::StatusCode;
use utoipa::ToSchema;

use super::LicenseInfo;
use crate::{
    appstate::AppState,
    auth::{AdminRole, SessionInfo},
    enterprise::db::models::activity_log_stream::{
        ActivityLogStream, ActivityLogStreamConfig, ActivityLogStreamType,
    },
    events::{ApiEvent, ApiEventType, ApiRequestContext},
    handlers::{ApiErrorResponse, ApiResponse, ApiResult},
};

/// List activity log streams.
#[utoipa::path(
    get,
    path = "/api/v1/activity_log_stream/",
    tag = "activity log",
    responses(
        (status = 200, description = "All activity log streams.", body = Object),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 500, description = "Unable to list activity log streams.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub async fn get_activity_log_stream(
    _admin: AdminRole,
    State(appstate): State<AppState>,
    session: SessionInfo,
) -> ApiResult {
    debug!(
        "User {} retrieving activity log streams",
        session.user.username
    );
    let mut conn = appstate.pool.acquire().await?;
    let streams = ActivityLogStream::all(&mut *conn).await?;
    info!(
        "User {} retrieved activity log streams",
        session.user.username
    );
    Ok(ApiResponse::json(streams, StatusCode::OK))
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ActivityLogStreamModificationRequest {
    pub name: String,
    pub stream_type: ActivityLogStreamType,
    pub stream_config: serde_json::Value,
}

/// Create an activity log stream.
#[utoipa::path(
    post,
    path = "/api/v1/activity_log_stream/",
    tag = "activity log",
    request_body = ActivityLogStreamModificationRequest,
    responses(
        (status = 201, description = "Activity log stream created.", body = Object),
        (status = 400, description = "Invalid stream configuration.", body = ApiErrorResponse, example = json!({"msg": "Invalid stream config"})),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges and an active enterprise license.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 500, description = "Unable to create activity log stream.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub async fn create_activity_log_stream(
    _license: LicenseInfo,
    _admin: AdminRole,
    State(appstate): State<AppState>,
    session: SessionInfo,
    context: ApiRequestContext,
    Json(data): Json<ActivityLogStreamModificationRequest>,
) -> ApiResult {
    let session_username = &session.user.username;
    debug!("User {session_username} creates activity log stream");
    // validate config
    let _ = ActivityLogStreamConfig::from_serde_value(&data.stream_type, &data.stream_config)?;
    let stream_model = ActivityLogStream {
        id: NoId,
        name: data.name,
        stream_type: data.stream_type,
        config: data.stream_config,
    };
    let stream = stream_model.save(&appstate.pool).await?;
    info!("User {session_username} created activity log stream");
    appstate.emit_event(ApiEvent {
        context,
        event: Box::new(ApiEventType::ActivityLogStreamCreated { stream }),
    })?;
    debug!("ActivityLogStreamCreated api event sent");
    Ok(ApiResponse::with_status(StatusCode::CREATED))
}

/// Update an activity log stream.
#[utoipa::path(
    put,
    path = "/api/v1/activity_log_stream/{id}",
    tag = "activity log",
    request_body = ActivityLogStreamModificationRequest,
    params(
        ("id" = i64, Path, description = "ID of the activity log stream."),
    ),
    responses(
        (status = 200, description = "Activity log stream updated."),
        (status = 400, description = "Invalid stream configuration.", body = ApiErrorResponse, example = json!({"msg": "Invalid stream config"})),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges and an active enterprise license.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 404, description = "Activity log stream not found.", body = ApiErrorResponse, example = json!({"msg": "stream not found"})),
        (status = 500, description = "Unable to update activity log stream.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub async fn modify_activity_log_stream(
    _license: LicenseInfo,
    _admin: AdminRole,
    State(appstate): State<AppState>,
    session: SessionInfo,
    context: ApiRequestContext,
    Path(id): Path<Id>,
    Json(data): Json<ActivityLogStreamModificationRequest>,
) -> ApiResult {
    let session_username = &session.user.username;
    debug!("User {session_username} modifies activity log stream ");
    if let Some(mut stream) = ActivityLogStream::find_by_id(&appstate.pool, id).await? {
        // store stream before modifications
        let before = stream.clone();
        //validate config
        let _ = ActivityLogStreamConfig::from_serde_value(&data.stream_type, &data.stream_config)?;
        stream.name = data.name;
        stream.config = data.stream_config;
        stream.save(&appstate.pool).await?;
        info!(
            "User {session_username} modified activity log stream {}",
            stream.name
        );
        appstate.emit_event(ApiEvent {
            context,
            event: Box::new(ApiEventType::ActivityLogStreamModified {
                before,
                after: stream,
            }),
        })?;
        debug!("ActivityLogStreamModified api event sent");
        return Ok(ApiResponse::default());
    }
    Err(crate::error::WebError::ObjectNotFound(format!(
        "Activity Log Stream of id {id} not found."
    )))
}

/// Delete an activity log stream.
#[utoipa::path(
    delete,
    path = "/api/v1/activity_log_stream/{id}",
    tag = "activity log",
    params(
        ("id" = i64, Path, description = "ID of the activity log stream."),
    ),
    responses(
        (status = 200, description = "Activity log stream deleted."),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges and an active enterprise license.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 404, description = "Activity log stream not found.", body = ApiErrorResponse, example = json!({"msg": "stream not found"})),
        (status = 500, description = "Unable to delete activity log stream.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub async fn delete_activity_log_stream(
    _license: LicenseInfo,
    _admin: AdminRole,
    State(appstate): State<AppState>,
    session: SessionInfo,
    context: ApiRequestContext,
    Path(id): Path<Id>,
) -> ApiResult {
    let session_username = &session.user.username;
    debug!("User {session_username} deleting Activity Log Stream ({id})");
    if let Some(stream) = ActivityLogStream::find_by_id(&appstate.pool, id).await? {
        stream.clone().delete(&appstate.pool).await?;
        appstate.emit_event(ApiEvent {
            context,
            event: Box::new(ApiEventType::ActivityLogStreamRemoved { stream }),
        })?;
    } else {
        return Err(crate::error::WebError::ObjectNotFound(format!(
            "Activity Log Stream of id {id} not found."
        )));
    }
    info!("User {session_username} deleted Activity Log Stream");
    debug!("ActivityLogStreamRemoved api event sent");
    Ok(ApiResponse::default())
}
