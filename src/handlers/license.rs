use crate::{
    appstate::AppState,
    auth::SessionInfo,
    handlers::{ApiResponse, ApiResult},
    license::License,
};
use rocket::{http::Status, serde::json::serde_json::json, State};

#[get("/license", format = "json")]
pub fn get_license(_session: SessionInfo, appstate: &State<AppState>) -> ApiResult {
    let license = License::decode(&appstate.config.license);
    Ok(ApiResponse {
        json: json!(license),
        status: Status::Ok,
    })
}
