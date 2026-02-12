use axum::{
    Json,
    extract::{Path, State},
};
use chrono::Utc;
use defguard_common::{
    db::models::proxy::Proxy,
    types::proxy::{ProxyControlMessage, ProxyInfo},
};
use reqwest::StatusCode;
use serde_json::Value;
use utoipa::ToSchema;

use crate::{
    appstate::AppState,
    auth::{AdminRole, SessionInfo},
    events::{ApiEvent, ApiEventType, ApiRequestContext},
    handlers::{ApiResponse, ApiResult},
};

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ProxyUpdateData {
    pub name: String,
}

#[utoipa::path(
    get,
    path = "/api/v1/proxy",
    responses(
        (status = 200, description = "Edge list", body = [ProxyInfo]),
        (status = 401, description = "Unauthorized to get edge list.", body = ApiResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "You don't have permission to get edge list.", body = ApiResponse, example = json!({"msg": "access denied"})),
        (status = 500, description = "Unable to get edge list.", body = ApiResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn proxy_list(
    _role: AdminRole,
    session: SessionInfo,
    State(appstate): State<AppState>,
) -> ApiResult {
    debug!("User {} displaying proxy list", session.user.username);
    let proxies = Proxy::list(&appstate.pool).await?;
    info!("User {} displayed proxy list", session.user.username);

    Ok(ApiResponse::json(proxies, StatusCode::OK))
}

#[utoipa::path(
    get,
    path = "/api/v1/proxy/{proxy_id}",
    responses(
        (status = 200, description = "Edge details", body = Proxy),
        (status = 401, description = "Unauthorized to get edge details.", body = ApiResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "You don't have permission to get edge details.", body = ApiResponse, example = json!({"msg": "access denied"})),
        (status = 404, description = "Edge not found", body = ApiResponse, example = json!({"msg": "network not found"})),
        (status = 500, description = "Unable to get edge details.", body = ApiResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn proxy_details(
    Path(proxy_id): Path<i64>,
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

#[utoipa::path(
    put,
    path = "/api/v1/proxy/{proxy_id}",
    request_body = Proxy,
    responses(
        (status = 200, description = "Successfully modified edge.", body = ProxyUpdateData),
        (status = 401, description = "Unauthorized to modify edge.", body = ApiResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "You don't have permission to modify an edge.", body = ApiResponse, example = json!({"msg": "access denied"})),
        (status = 404, description = "Edge not found", body = ApiResponse, example = json!({"msg": "proxy not found"})),
        (status = 500, description = "Unable to modify edge.", body = ApiResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn update_proxy(
    _role: AdminRole,
    Path(proxy_id): Path<i64>,
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
    proxy.modified_by = session.user.id;
    proxy.modified_at = Utc::now().naive_utc();
    proxy.save(&appstate.pool).await?;

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

#[utoipa::path(
    delete,
    path = "/api/v1/proxy/{proxy_id}",
    request_body = Proxy,
    responses(
        (status = 200, description = "Successfully deleted edge.", body = ApiResponse),
        (status = 401, description = "Unauthorized to delete edge.", body = ApiResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "You don't have permission delete an edge.", body = ApiResponse, example = json!({"msg": "access denied"})),
        (status = 404, description = "Edge not found", body = ApiResponse, example = json!({"msg": "proxy not found"})),
        (status = 500, description = "Unable to delete edge.", body = ApiResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn delete_proxy(
    _role: AdminRole,
    Path(proxy_id): Path<i64>,
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
            "Error shutting down proxy {}, it may be disconnected: {err:?}",
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
