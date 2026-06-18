use axum::{Json, extract::State, http::StatusCode};
use defguard_common::types::proxy::ProxyControlMessage;
use struct_patch::Patch;

use super::LicenseInfo;
use crate::{
    appstate::AppState,
    auth::{AdminRole, SessionInfo},
    enterprise::db::models::enterprise_settings::{EnterpriseSettings, EnterpriseSettingsPatch},
    handlers::{ApiResponse, ApiResult},
};

pub async fn get_enterprise_settings(
    session: SessionInfo,
    State(appstate): State<AppState>,
) -> ApiResult {
    debug!(
        "User {} retrieving enterprise settings",
        session.user.username
    );
    let settings = EnterpriseSettings::get(&appstate.pool).await?;
    debug!(
        "User {} retrieved enterprise settings",
        session.user.username
    );
    Ok(ApiResponse::json(settings, StatusCode::OK))
}

pub async fn patch_enterprise_settings(
    _license: LicenseInfo,
    _admin: AdminRole,
    State(appstate): State<AppState>,
    session: SessionInfo,
    Json(data): Json<EnterpriseSettingsPatch>,
) -> ApiResult {
    debug!(
        "Admin {} patching enterprise settings.",
        session.user.username,
    );
    let mut settings = EnterpriseSettings::get(&appstate.pool).await?;

    // snapshot values for comparison
    let old_display_password_reset = settings.display_password_reset;
    let old_display_download_step = settings.display_download_step;

    settings.apply(data);
    settings.save(&appstate.pool).await?;
    info!("Admin {} patched settings.", session.user.username);

    // Broadcast updated public settings to proxies only if they changed.
    if (settings.display_password_reset != old_display_password_reset
        || settings.display_download_step != old_display_download_step)
        && let Err(err) = appstate
            .proxy_control_tx
            .send(ProxyControlMessage::BroadcastPublicSettings {
                display_password_reset: settings.display_password_reset,
                display_download_step: settings.display_download_step,
            })
            .await
    {
        error!("Failed to broadcast PublicSettings to proxies: {err:?}");
    }

    Ok(ApiResponse::default())
}
