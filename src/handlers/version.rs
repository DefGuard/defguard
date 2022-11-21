use super::{ApiResponse, ApiResult, VERSION};
use rocket::{http::Status, serde::json::serde_json::json};

#[get("/version", format = "json")]
pub async fn get_version() -> ApiResult {
    Ok(ApiResponse {
        json: json!({ "version": VERSION }),
        status: Status::Ok,
    })
}
