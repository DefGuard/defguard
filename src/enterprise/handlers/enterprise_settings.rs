use axum::{extract::State, http::StatusCode, Json};
use serde_json::json;
use struct_patch::Patch;

use super::LicenseInfo;
use crate::{
    appstate::AppState,
    auth::{AdminRole, SessionInfo},
    enterprise::db::models::enterprise_settings::{EnterpriseSettings, EnterpriseSettingsPatch},
    error::WebError,
    handlers::ApiResponse,
};

pub(crate) async fn get_enterprise_settings(
    session: SessionInfo,
    State(appstate): State<AppState>,
) -> Result<ApiResponse, WebError> {
    debug!(
        "User {} retrieving enterprise settings",
        session.user.username
    );
    let settings = EnterpriseSettings::get(&appstate.pool).await?;
    info!(
        "User {} retrieved enterprise settings",
        session.user.username
    );
    Ok(ApiResponse::new(json!(settings), StatusCode::OK))
}

pub(crate) async fn patch_enterprise_settings(
    _license: LicenseInfo,
    _admin: AdminRole,
    State(appstate): State<AppState>,
    session: SessionInfo,
    Json(data): Json<EnterpriseSettingsPatch>,
) -> Result<ApiResponse, WebError> {
    debug!(
        "Admin {} patching enterprise settings.",
        session.user.username,
    );
    let mut settings = EnterpriseSettings::get(&appstate.pool).await?;

    settings.apply(data);
    settings.save(&appstate.pool).await?;
    info!("Admin {} patched settings.", session.user.username);
    Ok(ApiResponse::default())
}
