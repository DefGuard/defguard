use axum::{Extension, http::StatusCode};
use serde::Serialize;

use super::{ApiErrorResponse, ApiResponse, ApiResult};
use crate::auth::AdminRole;

#[derive(Serialize, Debug)]
pub struct ResourceDisplay {
    pub id: i64,
    pub display: String,
}

/// List locations reduced to ID and name, for use in pickers.
#[utoipa::path(
    get,
    path = "/api/v1/network/display",
    tag = "network",
    responses(
        (status = 200, description = "Locations with their IDs and names.", body = Object),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 500, description = "Unable to list locations.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub async fn get_locations_display(
    _admin: AdminRole,
    Extension(pool): Extension<sqlx::PgPool>,
) -> ApiResult {
    let resources = sqlx::query_as!(
        ResourceDisplay,
        "SELECT id, name AS display FROM wireguard_network ORDER BY id"
    )
    .fetch_all(&pool)
    .await?;

    Ok(ApiResponse::json(resources, StatusCode::OK))
}
