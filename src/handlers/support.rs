use super::{ApiResponse, ApiResult};
use crate::{
    auth::{AdminRole, SessionInfo},
    error::WebError,
    support::dump_config,
    AppState,
};
use rocket::{http::Status, State};

#[get("/configuration", format = "json")]
pub async fn configuration(
    _admin: AdminRole,
    appstate: &State<AppState>,
    session: SessionInfo,
) -> ApiResult {
    debug!("User {} dumping app configuration", session.user.username);
    let config = dump_config(&appstate.pool, &appstate.config).await;
    info!("User {} dumped app configuration", session.user.username);
    Ok(ApiResponse {
        json: config,
        status: StatusCode::OK,
    })
}

#[get("/logs", format = "json")]
pub async fn logs(
    _admin: AdminRole,
    appstate: &State<AppState>,
    session: SessionInfo,
) -> Result<String, WebError> {
    debug!("User {} dumping app logs", session.user.username);
    if let Some(ref log_file) = appstate.config.log_file {
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
