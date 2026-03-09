use axum::{Extension, http::StatusCode};
use sqlx::FromRow;

use super::{ApiResponse, ApiResult};
use crate::auth::AdminRole;

#[derive(Serialize, FromRow, Debug)]
pub struct ResourceDisplay {
    pub id: i64,
    pub display: String,
}

pub async fn get_locations_display(
    _admin: AdminRole,
    Extension(pool): Extension<sqlx::PgPool>,
) -> ApiResult {
    let resources =
        sqlx::query_as::<_, ResourceDisplay>("SELECT id, name AS display FROM wireguard_network")
            .fetch_all(&pool)
            .await?;

    Ok(ApiResponse::json(resources, StatusCode::OK))
}
