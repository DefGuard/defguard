use rocket::{http::Status, serde::json::json, State};

use crate::{appstate::AppState, auth::SessionInfo, db::WireguardNetwork};

use super::{ApiResult, VERSION};

// Additional information about core state
#[derive(Debug, Serialize, Deserialize)]
pub struct AppInfo {
    version: String,
    network_present: bool,
}

#[get("/info", format = "json")]
pub async fn get_app_info(appstate: &State<AppState>, _session: SessionInfo) -> ApiResult {
    let networks = WireguardNetwork::all(&appstate.pool).await?;
    let res = AppInfo {
        network_present: !networks.is_empty(),
        version: VERSION.into(),
    };

    Ok(super::ApiResponse {
        json: json!(res),
        status: Status::Ok,
    })
}
