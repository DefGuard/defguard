use axum::{Extension, http::StatusCode};
use serde::Serialize;

use super::{ApiResponse, ApiResult};
use crate::auth::AdminRole;

#[derive(Serialize, Debug)]
pub struct ResourceDisplay {
    pub id: i64,
    pub display: String,
}

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
