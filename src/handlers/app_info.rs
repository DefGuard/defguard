use super::{ApiResponse, ApiResult, VERSION};
use crate::enterprise::license::validate_license;
use crate::{appstate::AppState, auth::SessionInfo, db::WireguardNetwork};

use crate::db::Settings;
use axum::{extract::State, http::StatusCode};
use serde_json::json;

/// Additional information about core state.
#[derive(Serialize)]
pub struct AppInfo {
    version: String,
    network_present: bool,
    smtp_enabled: bool,
    /// Whether the core has an enterprise license.
    enterprise: bool,
}

pub(crate) async fn get_app_info(
    State(appstate): State<AppState>,
    _session: SessionInfo,
) -> ApiResult {
    let networks = WireguardNetwork::all(&appstate.pool).await?;
    let settings = Settings::get_settings(&appstate.pool).await?;
    let license = appstate
        .license
        .lock()
        .expect("Failed to acquire lock on the license.");
    info!("license: {license:?});
    let enterprise = validate_license((*license).as_ref()).is_ok();
    info!("enterprise: {}", enterprise);
    let res = AppInfo {
        network_present: !networks.is_empty(),
        smtp_enabled: settings.smtp_configured(),
        version: VERSION.into(),
        enterprise,
    };

    Ok(ApiResponse::new(json!(res), StatusCode::OK))
}
