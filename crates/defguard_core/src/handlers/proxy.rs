use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use chrono::Utc;
use defguard_common::{
    db::{Id, models::proxy::Proxy},
    types::proxy::{ProxyControlMessage, ProxyInfo},
};
use serde_json::Value;
use utoipa::ToSchema;

use crate::{
    appstate::AppState,
    auth::{AdminRole, SessionInfo},
    events::{ApiEvent, ApiEventType, ApiRequestContext},
    handlers::{ApiErrorResponse, ApiResponse, ApiResult},
};

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ProxyUpdateData {
    pub name: String,
    pub enabled: bool,
}

/// List edge instances
#[utoipa::path(
    get,
    path = "/api/v1/proxy",
    tag = "proxy",
    responses(
        (status = 200, description = "All edge instances.", body = [ProxyInfo]),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "access denied"})),
        (status = 500, description = "Unable to list edge instances.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub async fn proxy_list(
    _role: AdminRole,
    session: SessionInfo,
    State(appstate): State<AppState>,
) -> ApiResult {
    debug!("User {} displaying proxy list", session.user.username);
    let proxies = Proxy::list(&appstate.pool).await?;
    let proxies: Vec<ProxyInfo> = proxies.into_iter().map(Into::into).collect();
    info!("User {} displayed proxy list", session.user.username);

    Ok(ApiResponse::json(proxies, StatusCode::OK))
}

/// Get an edge instance
#[utoipa::path(
    get,
    path = "/api/v1/proxy/{proxy_id}",
    tag = "proxy",
    params(
        ("proxy_id" = i64, Path, description = "ID of the edge instance."),
    ),
    responses(
        (status = 200, description = "Edge instance details.", body = Proxy),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "access denied"})),
        (status = 404, description = "Edge instance not found."),
        (status = 500, description = "Unable to get edge instance.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn proxy_details(
    Path(proxy_id): Path<Id>,
    _role: AdminRole,
    session: SessionInfo,
    State(appstate): State<AppState>,
) -> ApiResult {
    debug!(
        "User {} displaying details for proxy {proxy_id}",
        session.user.username
    );
    let proxy = Proxy::find_by_id(&appstate.pool, proxy_id).await?;
    let response = match proxy {
        Some(proxy) => ApiResponse::json(proxy, StatusCode::OK),
        None => ApiResponse::json(Value::Null, StatusCode::NOT_FOUND),
    };
    info!(
        "User {} displayed details for proxy {proxy_id}",
        session.user.username
    );

    Ok(response)
}

/// Rename an edge instance, or enable or disable it
#[utoipa::path(
    put,
    path = "/api/v1/proxy/{proxy_id}",
    tag = "proxy",
    params(
        ("proxy_id" = i64, Path, description = "ID of the edge instance."),
    ),
    request_body = ProxyUpdateData,
    responses(
        (status = 200, description = "Edge instance updated.", body = Proxy),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "access denied"})),
        (status = 404, description = "Edge instance not found."),
        (status = 500, description = "Unable to update edge instance.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn update_proxy(
    _role: AdminRole,
    Path(proxy_id): Path<Id>,
    State(appstate): State<AppState>,
    session: SessionInfo,
    context: ApiRequestContext,
    Json(data): Json<ProxyUpdateData>,
) -> ApiResult {
    debug!("User {} updating proxy {proxy_id}", session.user.username);
    let proxy = Proxy::find_by_id(&appstate.pool, proxy_id).await?;

    let Some(mut proxy) = proxy else {
        warn!("Proxy {proxy_id} not found");
        return Ok(ApiResponse::json(Value::Null, StatusCode::NOT_FOUND));
    };
    let before = proxy.clone();

    proxy.name = data.name;
    proxy.enabled = data.enabled;
    proxy.modified_by = session.user.fullname();
    proxy.modified_at = Utc::now().naive_utc();
    proxy.save(&appstate.pool).await?;

    if before.enabled != proxy.enabled {
        if data.enabled {
            if let Err(err) = appstate
                .proxy_control_tx
                .send(ProxyControlMessage::StartConnection(proxy.id))
                .await
            {
                error!(
                    "Failed to start Proxy {}, it may be disconnected: {err:?}",
                    proxy.id
                );
            }
        } else if let Err(err) = appstate
            .proxy_control_tx
            .send(ProxyControlMessage::ShutdownConnection(proxy.id))
            .await
        {
            error!(
                "Failed to shutdown Proxy {}, it may be disconnected: {err:?}",
                proxy.id
            );
        }
    }

    info!("User {} updated proxy {proxy_id}", session.user.username);

    appstate.emit_event(ApiEvent {
        context,
        event: Box::new(ApiEventType::ProxyModified {
            before,
            after: proxy.clone(),
        }),
    })?;

    Ok(ApiResponse::json(proxy, StatusCode::OK))
}

/// Delete an edge instance
#[utoipa::path(
    delete,
    path = "/api/v1/proxy/{proxy_id}",
    tag = "proxy",
    params(
        ("proxy_id" = i64, Path, description = "ID of the edge instance."),
    ),
    responses(
        (status = 200, description = "Edge instance deleted."),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "access denied"})),
        (status = 404, description = "Edge instance not found."),
        (status = 500, description = "Unable to delete edge instance.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn delete_proxy(
    _role: AdminRole,
    Path(proxy_id): Path<Id>,
    State(appstate): State<AppState>,
    session: SessionInfo,
    context: ApiRequestContext,
) -> ApiResult {
    debug!("User {} deleteing proxy {proxy_id}", session.user.username);
    let proxy = Proxy::find_by_id(&appstate.pool, proxy_id).await?;

    let Some(proxy) = proxy else {
        warn!("Proxy {proxy_id} not found");
        return Ok(ApiResponse::json(Value::Null, StatusCode::NOT_FOUND));
    };

    // Disconnect and purge the proxy
    if let Err(err) = appstate
        .proxy_control_tx
        .send(ProxyControlMessage::Purge(proxy.id))
        .await
    {
        error!(
            "Failed to purge Proxy {}, it may be disconnected: {err:?}",
            proxy.id
        );
    }

    proxy.clone().delete(&appstate.pool).await?;

    info!("User {} deleted proxy {proxy_id}", session.user.username);

    appstate.emit_event(ApiEvent {
        context,
        event: Box::new(ApiEventType::ProxyDeleted { proxy }),
    })?;

    Ok(ApiResponse::default())
}
