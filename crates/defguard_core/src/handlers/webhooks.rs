use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
};
use defguard_common::db::Id;
use utoipa::ToSchema;

use super::{ApiErrorResponse, ApiResponse, ApiResult, WebHookData};
use crate::{
    appstate::AppState,
    auth::{AdminRole, SessionInfo},
    db::WebHook,
    events::{ApiEvent, ApiEventType, ApiRequestContext},
};

/// Create a webhook.
#[utoipa::path(
    post,
    path = "/api/v1/webhook",
    tag = "webhook",
    request_body = WebHookData,
    responses(
        (status = 201, description = "Webhook created.", body = Object, example = json!({})),
        (status = 400, description = "Unable to save the webhook.", body = Object, example = json!({})),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 500, description = "Unable to create webhook.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
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
                event: Box::new(ApiEventType::WebHookAdded { webhook }),
            })?;
            StatusCode::CREATED
        }
        Err(_) => StatusCode::BAD_REQUEST,
    };

    Ok(ApiResponse::with_status(status))
}

// TODO: paginate
/// List webhooks.
#[utoipa::path(
    get,
    path = "/api/v1/webhook",
    tag = "webhook",
    responses(
        (status = 200, description = "All webhooks.", body = [WebHook]),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 500, description = "Unable to list webhooks.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub async fn list_webhooks(_admin: AdminRole, State(appstate): State<AppState>) -> ApiResult {
    let webhooks = WebHook::all(&appstate.pool).await?;

    Ok(ApiResponse::json(webhooks, StatusCode::OK))
}

/// Get a webhook.
#[utoipa::path(
    get,
    path = "/api/v1/webhook/{id}",
    tag = "webhook",
    params(
        ("id" = i64, Path, description = "ID of the webhook."),
    ),
    responses(
        (status = 200, description = "Webhook details.", body = WebHook),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 404, description = "Webhook not found.", body = Object, example = json!({})),
        (status = 500, description = "Unable to get webhook.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub async fn get_webhook(
    _admin: AdminRole,
    State(appstate): State<AppState>,
    Path(id): Path<Id>,
) -> ApiResult {
    match WebHook::find_by_id(&appstate.pool, id).await? {
        Some(webhook) => Ok(ApiResponse::json(webhook, StatusCode::OK)),
        None => Ok(ApiResponse::with_status(StatusCode::NOT_FOUND)),
    }
}

/// Update a webhook.
#[utoipa::path(
    put,
    path = "/api/v1/webhook/{id}",
    tag = "webhook",
    request_body = WebHookData,
    params(
        ("id" = i64, Path, description = "ID of the webhook."),
    ),
    responses(
        (status = 200, description = "Webhook updated.", body = Object, example = json!({})),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 404, description = "Webhook not found.", body = Object, example = json!({})),
        (status = 500, description = "Unable to update webhook.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub async fn change_webhook(
    _admin: AdminRole,
    session: SessionInfo,
    context: ApiRequestContext,
    State(appstate): State<AppState>,
    Path(id): Path<Id>,
    Json(data): Json<WebHookData>,
) -> ApiResult {
    debug!("User {} updating webhook {id}", session.user.username);
    let status = match WebHook::find_by_id(&appstate.pool, id).await? {
        Some(mut webhook) => {
            // store webhook before modifications
            let before = webhook.clone();
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
                event: Box::new(ApiEventType::WebHookModified {
                    before,
                    after: webhook,
                }),
            })?;
            StatusCode::OK
        }
        None => StatusCode::NOT_FOUND,
    };

    Ok(ApiResponse::with_status(status))
}

/// Delete a webhook.
#[utoipa::path(
    delete,
    path = "/api/v1/webhook/{id}",
    tag = "webhook",
    params(
        ("id" = i64, Path, description = "ID of the webhook."),
    ),
    responses(
        (status = 200, description = "Webhook deleted.", body = Object, example = json!({})),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 404, description = "Webhook not found.", body = Object, example = json!({})),
        (status = 500, description = "Unable to delete webhook.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub async fn delete_webhook(
    _admin: AdminRole,
    State(appstate): State<AppState>,
    session: SessionInfo,
    context: ApiRequestContext,
    Path(id): Path<Id>,
) -> ApiResult {
    debug!("User {} deleting webhook {id}", session.user.username);
    let status = match WebHook::find_by_id(&appstate.pool, id).await? {
        Some(webhook) => {
            webhook.clone().delete(&appstate.pool).await?;
            info!("User {} deleted webhook {id}", session.user.username);
            appstate.emit_event(ApiEvent {
                context,
                event: Box::new(ApiEventType::WebHookRemoved { webhook }),
            })?;
            StatusCode::OK
        }
        None => StatusCode::NOT_FOUND,
    };
    Ok(ApiResponse::with_status(status))
}

#[derive(Deserialize, ToSchema)]
pub struct ChangeStateData {
    pub enabled: bool,
}

/// Enable or disable a webhook.
#[utoipa::path(
    post,
    path = "/api/v1/webhook/{id}",
    tag = "webhook",
    request_body = ChangeStateData,
    params(
        ("id" = i64, Path, description = "ID of the webhook."),
    ),
    responses(
        (status = 200, description = "Webhook state changed.", body = Object, example = json!({})),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 404, description = "Webhook not found.", body = Object, example = json!({})),
        (status = 500, description = "Unable to change webhook state.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub async fn change_enabled(
    _admin: AdminRole,
    session: SessionInfo,
    context: ApiRequestContext,
    State(appstate): State<AppState>,
    Path(id): Path<Id>,
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
                event: Box::new(ApiEventType::WebHookStateChanged {
                    enabled: webhook.enabled,
                    webhook,
                }),
            })?;
            StatusCode::OK
        }
        None => StatusCode::NOT_FOUND,
    };
    Ok(ApiResponse::with_status(status))
}
