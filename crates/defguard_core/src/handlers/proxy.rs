use axum::extract::{Path, State};
use reqwest::StatusCode;
use serde_json::{json, Value};
use defguard_common::db::models::proxy::Proxy;

use crate::{appstate::AppState, auth::AdminRole, handlers::{ApiResponse, ApiResult}};

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
        Some(proxy) => {
            ApiResponse {
                json: json!(proxy),
                status: StatusCode::OK,
            }
        }
        None => ApiResponse {
            json: Value::Null,
            status: StatusCode::NOT_FOUND,
        },
    };
    debug!("Displayed details for proxy {proxy_id}");

    Ok(response)
}
