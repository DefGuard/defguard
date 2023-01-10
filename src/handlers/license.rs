use crate::{appstate::AppState, auth::SessionInfo, handlers::ApiResponse, license::License};
use rocket::{http::Status, serde::json::serde_json::json, State};

#[get("/license", format = "json")]
pub fn get_license(appstate: &State<AppState>) -> ApiResponse {
    let license = License::decode(&appstate.config.license);
    ApiResponse {
        json: json!(license),
        status: Status::Ok,
    }
}
