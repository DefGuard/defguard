use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
};
use serde_json::json;

use super::{ApiResponse, ApiResult, WebHookData};
use crate::{
    appstate::AppState,
    auth::{AdminRole, SessionInfo},
    db::WebHook,
    events::{ApiEvent, ApiEventType, ApiRequestContext},
};

pub async fn add_webhook(
    _admin: AdminRole,
    session: SessionInfo,
    context: ApiRequestContext,
    State(appstate): State<AppState>,
    Json(webhookdata): Json<WebHookData>,
) -> ApiResult {
    let url = webhookdata.url.clone();
    debug!("User {} adding webhook {url}", session.user.username);
    let webhook: WebHook = webhookdata.into();
    let status = match webhook.save(&appstate.pool).await {
        Ok(webhook) => {
            info!("User {} added webhook {url}", session.user.username);
            appstate.emit_event(ApiEvent {
                context,
                event: ApiEventType::WebHookAdded { webhook },
            })?;
            StatusCode::CREATED
        }
        Err(_) => StatusCode::BAD_REQUEST,
    };

    Ok(ApiResponse {
        json: json!({}),
        status,
    })
}

// TODO: paginate
pub async fn list_webhooks(_admin: AdminRole, State(appstate): State<AppState>) -> ApiResult {
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
) -> ApiResult {
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
    context: ApiRequestContext,
    State(appstate): State<AppState>,
    Path(id): Path<i64>,
    Json(data): Json<WebHookData>,
) -> ApiResult {
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
            info!("User {} updated webhook {id}", session.user.username);
            appstate.emit_event(ApiEvent {
                context,
                event: ApiEventType::WebHookModified { webhook },
            })?;
            StatusCode::OK
        }
        None => StatusCode::NOT_FOUND,
    };

    Ok(ApiResponse {
        json: json!({}),
        status,
    })
}

pub async fn delete_webhook(
    _admin: AdminRole,
    State(appstate): State<AppState>,
    session: SessionInfo,
    context: ApiRequestContext,
    Path(id): Path<i64>,
) -> ApiResult {
    debug!("User {} deleting webhook {id}", session.user.username);
    let status = match WebHook::find_by_id(&appstate.pool, id).await? {
        Some(webhook) => {
            webhook.clone().delete(&appstate.pool).await?;
            info!("User {} deleted webhook {id}", session.user.username);
            appstate.emit_event(ApiEvent {
                context,
                event: ApiEventType::WebHookRemoved { webhook },
            })?;
            StatusCode::OK
        }
        None => StatusCode::NOT_FOUND,
    };
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
    context: ApiRequestContext,
    State(appstate): State<AppState>,
    Path(id): Path<i64>,
    Json(data): Json<ChangeStateData>,
) -> ApiResult {
    debug!(
        "User {} changing webhook {id} enabled state to {}",
        session.user.username, data.enabled
    );
    let status = match WebHook::find_by_id(&appstate.pool, id).await? {
        Some(mut webhook) => {
            webhook.enabled = data.enabled;
            webhook.save(&appstate.pool).await?;
            info!(
                "User {} changed webhook {id} enabled state to {}",
                session.user.username, data.enabled
            );
            appstate.emit_event(ApiEvent {
                context,
                event: ApiEventType::WebHookStateChanged {
                    enabled: webhook.enabled,
                    webhook,
                },
            })?;
            StatusCode::OK
        }
        None => StatusCode::NOT_FOUND,
    };
    Ok(ApiResponse {
        json: json!({}),
        status,
    })
}
