use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
};
use serde_json::json;

use super::{ApiResponse, ApiResult};
use crate::{
    auth::{AdminRole, SessionInfo},
    db::Settings,
    error::WebError,
    AppState,
};

pub async fn get_settings(State(appstate): State<AppState>) -> ApiResult {
    debug!("Retrieving settings");
    let settings = Settings::find_by_id(&appstate.pool, 1).await?;
    info!("Retrieved settings");
    Ok(ApiResponse {
        json: json!(settings),
        status: StatusCode::OK,
    })
}

pub async fn update_settings(
    _admin: AdminRole,
    session: SessionInfo,
    State(appstate): State<AppState>,
    Json(mut data): Json<Settings>,
) -> ApiResult {
    debug!("User {} updating settings", session.user.username);
    data.id = Some(1);
    data.save(&appstate.pool).await?;
    info!("User {} updated settings", session.user.username);
    Ok(ApiResponse::default())
}

pub async fn set_default_branding(
    _admin: AdminRole,
    State(appstate): State<AppState>,
    Path(id): Path<i64>,
    session: SessionInfo,
) -> ApiResult {
    debug!(
        "User {} restoring default branding settings",
        session.user.username
    );
    let settings = Settings::find_by_id(&appstate.pool, id).await?;
    match settings {
        Some(mut settings) => {
            settings.instance_name = "Defguard".into();
            settings.nav_logo_url = "/svg/defguard-nav-logo.svg".into();
            settings.main_logo_url = "/svg/logo-defguard-white.svg".into();
            settings.save(&appstate.pool).await?;
            info!(
                "User {} restored default branding settings",
                session.user.username
            );
            Ok(ApiResponse {
                json: json!(settings),
                status: StatusCode::OK,
            })
        }
        None => Err(WebError::DbError("Cannot restore settings".into())),
    }
}
