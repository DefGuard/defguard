use crate::{
    appstate::AppState,
    auth::{AdminRole, SessionInfo},
    db::WebHook,
    handlers::{ApiResponse, ApiResult},
};
use rocket::{
    http::Status,
    serde::json::{serde_json::json, Json},
    State,
};

#[post("/", format = "json", data = "<data>")]
pub async fn add_webhook(
    _admin: AdminRole,
    appstate: &State<AppState>,
    data: Json<WebHook>,
    session: SessionInfo,
) -> ApiResult {
    let url = data.url.clone();
    debug!("User {} adding webhook {}", session.user.username, url);
    let mut webhook = data.into_inner();
    let status = match webhook.save(&appstate.pool).await {
        Ok(_) => Status::Created,
        Err(_) => Status::BadRequest,
    };
    info!("User {} added webhook {}", session.user.username, url);
    Ok(ApiResponse {
        json: json!({}),
        status,
    })
}

#[get("/", format = "json")]
// TODO: paginate
pub async fn list_webhooks(_admin: AdminRole, appstate: &State<AppState>) -> ApiResult {
    let webhooks = WebHook::all(&appstate.pool).await?;
    Ok(ApiResponse {
        json: json!(webhooks),
        status: Status::Ok,
    })
}

#[get("/<id>", format = "json")]
pub async fn get_webhook(_admin: AdminRole, appstate: &State<AppState>, id: i64) -> ApiResult {
    match WebHook::find_by_id(&appstate.pool, id).await? {
        Some(webhook) => Ok(ApiResponse {
            json: json!(webhook),
            status: Status::Ok,
        }),
        None => Ok(ApiResponse {
            json: json!({}),
            status: Status::NotFound,
        }),
    }
}

#[derive(Deserialize, Serialize)]
pub struct WebHookData {
    pub url: String,
    pub description: String,
    pub token: String,
    pub enabled: bool,
    pub on_user_created: bool,
    pub on_user_deleted: bool,
    pub on_user_modified: bool,
    pub on_hwkey_provision: bool,
}

#[put("/<id>", format = "json", data = "<data>")]
pub async fn change_webhook(
    _admin: AdminRole,
    appstate: &State<AppState>,
    id: i64,
    data: Json<WebHookData>,
    session: SessionInfo,
) -> ApiResult {
    debug!("User {} updating webhook {}", session.user.username, id);
    let status = match WebHook::find_by_id(&appstate.pool, id).await? {
        Some(mut webhook) => {
            let data = data.into_inner();
            webhook.url = data.url;
            webhook.description = data.description;
            webhook.token = data.token;
            webhook.enabled = data.enabled;
            webhook.on_user_created = data.on_user_created;
            webhook.on_user_deleted = data.on_user_deleted;
            webhook.on_user_modified = data.on_user_modified;
            webhook.on_hwkey_provision = data.on_hwkey_provision;
            webhook.save(&appstate.pool).await?;
            Status::Ok
        }
        None => Status::NotFound,
    };
    info!("User {} updated webhook {}", session.user.username, id);
    Ok(ApiResponse {
        json: json!({}),
        status,
    })
}

#[delete("/<id>")]
pub async fn delete_webhook(
    _admin: AdminRole,
    appstate: &State<AppState>,
    id: i64,
    session: SessionInfo,
) -> ApiResult {
    debug!("User {} deleting webhook {}", session.user.username, id);
    let status = match WebHook::find_by_id(&appstate.pool, id).await? {
        Some(webhook) => {
            webhook.delete(&appstate.pool).await?;
            Status::Ok
        }
        None => Status::NotFound,
    };
    info!("User {} deleted webhook {}", session.user.username, id);
    Ok(ApiResponse {
        json: json!({}),
        status,
    })
}

#[derive(Deserialize)]
pub struct ChangeStateData {
    pub enabled: bool,
}

#[post("/<id>", format = "json", data = "<data>")]
pub async fn change_enabled(
    _admin: AdminRole,
    appstate: &State<AppState>,
    id: i64,
    data: Json<ChangeStateData>,
    session: SessionInfo,
) -> ApiResult {
    debug!(
        "User {} changing webhook {} enabled state to {}",
        session.user.username, id, data.enabled
    );
    let status = match WebHook::find_by_id(&appstate.pool, id).await? {
        Some(mut webhook) => {
            webhook.enabled = data.enabled;
            webhook.save(&appstate.pool).await?;
            Status::Ok
        }
        None => Status::NotFound,
    };
    info!(
        "User {} changed webhook {} enabled state to {}",
        session.user.username, id, data.enabled
    );
    Ok(ApiResponse {
        json: json!({}),
        status,
    })
}
