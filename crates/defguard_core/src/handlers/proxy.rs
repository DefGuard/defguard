use axum::{
    Json,
    extract::{Path, State},
};
use defguard_common::db::models::proxy::Proxy;
use reqwest::StatusCode;
use serde_json::{Value, json};
use utoipa::ToSchema;

use crate::{
    appstate::AppState,
    auth::{AdminRole, SessionInfo},
    events::{ApiEvent, ApiEventType, ApiRequestContext},
    handlers::{ApiResponse, ApiResult},
};

#[derive(Deserialize, ToSchema)]
pub(crate) struct ProxyUpdateData {
    name: String,
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
    State(appstate): State<AppState>,
) -> ApiResult {
    debug!("Displaying details for proxy {proxy_id}");
    let proxy = Proxy::find_by_id(&appstate.pool, proxy_id).await?;
    let response = match proxy {
        Some(proxy) => ApiResponse {
            json: json!(proxy),
            status: StatusCode::OK,
        },
        None => ApiResponse {
            json: Value::Null,
            status: StatusCode::NOT_FOUND,
        },
    };
    debug!("Displayed details for proxy {proxy_id}");

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
        return Ok(ApiResponse {
            json: Value::Null,
            status: StatusCode::NOT_FOUND,
        });
    };
    let before = proxy.clone();

    proxy.name = data.name;
    proxy.save(&appstate.pool).await?;

    info!("User {} updated proxy {proxy_id}", session.user.username);

    appstate.emit_event(ApiEvent {
        context,
        event: Box::new(ApiEventType::ProxyModified {
            before,
            after: proxy.clone(),
        }),
    })?;

    Ok(ApiResponse {
        json: json!(proxy),
        status: StatusCode::OK,
    })
}
