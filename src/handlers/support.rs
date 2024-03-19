use axum::{extract::State, http::StatusCode};

use super::{ApiResponse, ApiResult};
use crate::{
    auth::{AdminRole, SessionInfo},
    error::WebError,
    server_config,
    support::dump_config,
    AppState,
};

pub async fn configuration(
    _admin: AdminRole,
    State(appstate): State<AppState>,
    session: SessionInfo,
) -> ApiResult {
    debug!("User {} dumping app configuration", session.user.username);
    let config = dump_config(&appstate.pool).await;
    info!("User {} dumped app configuration", session.user.username);
    Ok(ApiResponse {
        json: config,
        status: StatusCode::OK,
    })
}

pub async fn logs(_admin: AdminRole, session: SessionInfo) -> Result<String, WebError> {
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
        Ok("Log file not configured".to_string())
    }
}
