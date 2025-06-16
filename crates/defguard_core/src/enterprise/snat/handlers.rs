use axum::extract::{Path, State};
use reqwest::StatusCode;
use serde_json::json;
use sqlx::query_as;
use std::net::IpAddr;

use crate::{
    appstate::AppState,
    auth::AdminRole,
    db::Id,
    enterprise::snat::UserSnatBinding,
    handlers::{ApiResponse, ApiResult},
};

pub async fn list_snat_bindings(
    _role: AdminRole,
    Path(location_id): Path<Id>,
    State(appstate): State<AppState>,
) -> ApiResult {
    debug!("Listing SNAT bindings for WireGuard location {location_id}");

    let bindings = query_as!(
        UserSnatBinding::<Id>,
        "SELECT id, user_id, location_id, \"public_ip\" \"public_ip: IpAddr\" FROM user_snat_binding WHERE location_id = $1",
        location_id
    )
    .fetch_all(&appstate.pool)
    .await?;

    Ok(ApiResponse {
        json: json!(bindings),
        status: StatusCode::OK,
    })
}
