use axum::{extract::State, http::StatusCode};

use super::{ApiErrorResponse, ApiResponse, ApiResult};
use crate::{
    AppState,
    auth::{AdminRole, SessionInfo},
    error::WebError,
    server_config,
    support::dump_config,
};

/// Get instance configuration for support purposes.
///
/// Secrets are stripped from the returned configuration.
#[utoipa::path(
    get,
    path = "/api/v1/support/configuration",
    tag = "support",
    responses(
        (status = 200, description = "Instance configuration dump.", body = Object),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 500, description = "Unable to get configuration.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn configuration(
    _admin: AdminRole,
    State(appstate): State<AppState>,
    session: SessionInfo,
) -> ApiResult {
    debug!("User {} dumping app configuration", session.user.username);

    let mut conn = appstate.pool.begin().await?;
    Ok(match dump_config(&mut conn).await {
        Ok(config) => {
            info!("User {} dumped app configuration", session.user.username);
            ApiResponse::new(config, StatusCode::OK)
        }
        Err(err) => {
            warn!("Failed to dump app configuration: {err}");
            ApiResponse::json(
                serde_json::json!({"err": err.to_string()}),
                StatusCode::BAD_REQUEST,
            )
        }
    })
}

/// Get recent instance logs for support purposes.
#[utoipa::path(
    get,
    path = "/api/v1/support/logs",
    tag = "support",
    responses(
        (status = 200, description = "Instance logs as plain text.", body = String),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 500, description = "Unable to get logs.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn logs(_admin: AdminRole, session: SessionInfo) -> Result<String, WebError> {
    debug!("User {} dumping app logs", session.user.username);
    if let Some(ref log_file) = server_config().log_file {
        match tokio::fs::read_to_string(log_file).await {
            Ok(logs) => {
                info!("User {} dumped app logs", session.user.username);
                Ok(logs)
            }
            Err(err) => {
                error!(
                    "Error dumping app logs for user {}: {err}",
                    session.user.username
                );
                Ok(err.to_string())
            }
        }
    } else {
        Ok("Log file not configured".to_owned())
    }
}
