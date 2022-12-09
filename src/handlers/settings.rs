use super::{ApiResponse, ApiResult};
use crate::{auth::AdminRole, db::Settings, AppState};
use rocket::{
    http::Status,
    serde::json::{serde_json::json, Json},
    State,
};

#[get("/settings", format = "json")]
pub async fn get_settings(appstate: &State<AppState>) -> ApiResult {
    debug!("Retrieving settings");
    let settings = Settings::find_by_id(&appstate.pool, 1).await?;
    info!("Retrieved settings");
    Ok(ApiResponse {
        json: json!(settings),
        status: Status::Ok,
    })
}

#[put("/settings", format = "json", data = "<data>")]
pub async fn update_settings(
    _admin: AdminRole,
    appstate: &State<AppState>,
    mut data: Json<Settings>,
) -> ApiResult {
    debug!("Updating settings");
    data.id = Some(1);
    data.save(&appstate.pool).await?;
    info!("Settings updated");
    Ok(ApiResponse::default())
}
