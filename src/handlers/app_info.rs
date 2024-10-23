use axum::{extract::State, http::StatusCode};
use serde_json::json;

use super::{ApiResponse, VERSION};
use crate::{
    appstate::AppState,
    auth::SessionInfo,
    db::models::{settings::Settings, wireguard::WireguardNetwork},
    enterprise::license::{get_cached_license, validate_license},
    error::WebError,
};

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
) -> Result<ApiResponse, WebError> {
    let networks = WireguardNetwork::all(&appstate.pool).await?;
    let settings = Settings::get_settings(&appstate.pool).await?;
    let license = get_cached_license();
    let enterprise = validate_license((license).as_ref()).is_ok();
    let res = AppInfo {
        network_present: !networks.is_empty(),
        smtp_enabled: settings.smtp_configured(),
        version: VERSION.into(),
        enterprise,
    };

    Ok(ApiResponse::new(json!(res), StatusCode::OK))
}
