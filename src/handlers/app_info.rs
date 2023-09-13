use super::{ApiResponse, ApiResult, VERSION};
use crate::{appstate::AppState, auth::SessionInfo, db::WireguardNetwork, error::WebError};

use axum::{extract::State, http::StatusCode};
use serde_json::json;

// Additional information about core state
#[derive(Serialize, Deserialize)]
pub struct AppInfo {
    version: String,
    network_present: bool,
}

pub(crate) async fn get_app_info(
    State(appstate): State<AppState>,
    _session: SessionInfo,
) -> ApiResult {
    let networks = WireguardNetwork::all(&appstate.pool)
        .await
        .map_err(WebError::from)?;
    let res = AppInfo {
        network_present: !networks.is_empty(),
        version: VERSION.into(),
    };

    Ok(ApiResponse::new(json!(res), StatusCode::OK))
}
