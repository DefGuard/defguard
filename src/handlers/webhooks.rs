use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
};
use serde_json::json;

use super::{ApiResponse, WebHookData};
use crate::{
    appstate::AppState,
    auth::{AdminRole, SessionInfo},
    db::WebHook,
    error::WebError,
};

pub async fn add_webhook(
    _admin: AdminRole,
    session: SessionInfo,
    State(appstate): State<AppState>,
    Json(webhookdata): Json<WebHookData>,
) -> Result<ApiResponse, WebError> {
    let url = webhookdata.url.clone();
    debug!("User {} adding webhook {url}", session.user.username);
    let webhook: WebHook = webhookdata.into();
    let status = match webhook.save(&appstate.pool).await {
        Ok(_) => StatusCode::CREATED,
        Err(_) => StatusCode::BAD_REQUEST,
    };
    info!("User {} added webhook {url}", session.user.username);

    Ok(ApiResponse {
        json: json!({}),
        status,
    })
}

// TODO: paginate
pub async fn list_webhooks(
    _admin: AdminRole,
    State(appstate): State<AppState>,
) -> Result<ApiResponse, WebError> {
    let webhooks = WebHook::all(&appstate.pool).await?;

    Ok(ApiResponse {
        json: json!(webhooks),
        status: StatusCode::OK,
    })
}

pub async fn get_webhook(
    _admin: AdminRole,
    State(appstate): State<AppState>,
    Path(id): Path<i64>,
) -> Result<ApiResponse, WebError> {
    match WebHook::find_by_id(&appstate.pool, id).await? {
        Some(webhook) => Ok(ApiResponse {
            json: json!(webhook),
            status: StatusCode::OK,
        }),
        None => Ok(ApiResponse {
            json: json!({}),
            status: StatusCode::NOT_FOUND,
        }),
    }
}

pub async fn change_webhook(
    _admin: AdminRole,
    session: SessionInfo,
    State(appstate): State<AppState>,
    Path(id): Path<i64>,
    Json(data): Json<WebHookData>,
) -> Result<ApiResponse, WebError> {
    debug!("User {} updating webhook {id}", session.user.username);
    let status = match WebHook::find_by_id(&appstate.pool, id).await? {
        Some(mut webhook) => {
            webhook.url = data.url;
            webhook.description = data.description;
            webhook.token = data.token;
            webhook.enabled = data.enabled;
            webhook.on_user_created = data.on_user_created;
            webhook.on_user_deleted = data.on_user_deleted;
            webhook.on_user_modified = data.on_user_modified;
            webhook.on_hwkey_provision = data.on_hwkey_provision;
            webhook.save(&appstate.pool).await?;
            StatusCode::OK
        }
        None => StatusCode::NOT_FOUND,
    };
    info!("User {} updated webhook {id}", session.user.username);

    Ok(ApiResponse {
        json: json!({}),
        status,
    })
}

pub async fn delete_webhook(
    _admin: AdminRole,
    State(appstate): State<AppState>,
    Path(id): Path<i64>,
    session: SessionInfo,
) -> Result<ApiResponse, WebError> {
    debug!("User {} deleting webhook {id}", session.user.username);
    let status = match WebHook::find_by_id(&appstate.pool, id).await? {
        Some(webhook) => {
            webhook.delete(&appstate.pool).await?;
            StatusCode::OK
        }
        None => StatusCode::NOT_FOUND,
    };
    info!("User {} deleted webhook {id}", session.user.username);
    Ok(ApiResponse {
        json: json!({}),
        status,
    })
}

#[derive(Deserialize)]
pub struct ChangeStateData {
    pub enabled: bool,
}

pub async fn change_enabled(
    _admin: AdminRole,
    session: SessionInfo,
    State(appstate): State<AppState>,
    Path(id): Path<i64>,
    Json(data): Json<ChangeStateData>,
) -> Result<ApiResponse, WebError> {
    debug!(
        "User {} changing webhook {id} enabled state to {}",
        session.user.username, data.enabled
    );
    let status = match WebHook::find_by_id(&appstate.pool, id).await? {
        Some(mut webhook) => {
            webhook.enabled = data.enabled;
            webhook.save(&appstate.pool).await?;
            StatusCode::OK
        }
        None => StatusCode::NOT_FOUND,
    };
    info!(
        "User {} changed webhook {id} enabled state to {}",
        session.user.username, data.enabled
    );
    Ok(ApiResponse {
        json: json!({}),
        status,
    })
}
