use super::{ApiResponse, ApiResult};
use crate::{
    auth::{AdminRole, SessionInfo},
    db::Settings,
    error::OriWebError,
    AppState,
};
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
    session: SessionInfo,
) -> ApiResult {
    debug!("User {} updating settings", session.user.username);
    data.id = Some(1);
    data.save(&appstate.pool).await?;
    info!("User {} updated settings", session.user.username);
    Ok(ApiResponse::default())
}

#[get("/settings/<id>", format = "json")]
pub async fn set_default_branding(
    _admin: AdminRole,
    appstate: &State<AppState>,
    id: i64,
    session: SessionInfo,
) -> ApiResult {
    debug!(
        "User {} restoring default branding settings",
        session.user.username
    );
    let settings = Settings::find_by_id(&appstate.pool, id).await?;
    match settings {
        Some(mut settings) => {
            settings.instance_name = "Defguard".into();
            settings.nav_logo_url = "/svg/defguard-nav-logo.svg".into();
            settings.main_logo_url = "/svg/logo-defguard-white.svg".into();
            settings.save(&appstate.pool).await?;
            info!(
                "User {} restored default branding settings",
                session.user.username
            );
            Ok(ApiResponse {
                json: json!(settings),
                status: Status::Ok,
            })
        }
        None => Err(OriWebError::DbError("Cannot restore settings".into())),
    }
}
