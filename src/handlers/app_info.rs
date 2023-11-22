use super::{ApiResponse, ApiResult, VERSION};
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
}

pub(crate) async fn get_app_info(
    State(appstate): State<AppState>,
    _session: SessionInfo,
) -> ApiResult {
    let networks = WireguardNetwork::all(&appstate.pool).await?;
    let settings = Settings::get_settings(&appstate.pool).await?;
    let res = AppInfo {
        network_present: !networks.is_empty(),
        smtp_enabled: settings.smtp_configured(),
        version: VERSION.into(),
    };

    Ok(ApiResponse::new(json!(res), StatusCode::OK))
}
