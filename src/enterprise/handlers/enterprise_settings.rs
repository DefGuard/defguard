use axum::{extract::State, Json};

use crate::{
    appstate::AppState,
    auth::{AdminRole, SessionInfo},
    enterprise::db::models::enterprise_settings::{EnterpriseSettings, EnterpriseSettingsPatch},
    handlers::{ApiResponse, ApiResult},
};
use struct_patch::Patch;

use super::LicenseInfo;

pub async fn patch_enterprise_settings(
    _license: LicenseInfo,
    _admin: AdminRole,
    State(appstate): State<AppState>,
    session: SessionInfo,
    Json(data): Json<EnterpriseSettingsPatch>,
) -> ApiResult {
    debug!(
        "Admin {} patching enterprise settings.",
        &session.user.username
    );
    let mut settings = EnterpriseSettings::get(&appstate.pool).await?;

    settings.apply(data);
    settings.save(&appstate.pool).await?;
    info!("Admin {} patched settings.", &session.user.username);
    Ok(ApiResponse::default())
}
