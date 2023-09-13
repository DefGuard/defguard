use super::{ApiResult, VERSION};
use crate::{appstate::AppState, auth::SessionInfo, db::WireguardNetwork};

use axum::{extract::State, http::StatusCode};
use serde_json::json;

// Additional information about core state
#[derive(Debug, Serialize, Deserialize)]
pub struct AppInfo {
    version: String,
    network_present: bool,
}

// #[get("/info", format = "json")]
pub async fn get_app_info(State(appstate): &State<AppState>, _session: SessionInfo) -> ApiResult {
    let networks = WireguardNetwork::all(&appstate.pool).await?;
    let res = AppInfo {
        network_present: !networks.is_empty(),
        version: VERSION.into(),
    };

    Ok(super::ApiResponse {
        json: json!(res),
        status: StatusCode::OK,
    })
}
