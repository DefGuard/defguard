use axum::{
    extract::{Path, State},
    Json,
};
use reqwest::StatusCode;
use serde_json::json;
use sqlx::query_as;
use std::net::IpAddr;

use crate::{
    appstate::AppState,
    auth::{AdminRole, SessionInfo},
    db::Id,
    enterprise::snat::UserSnatBinding,
    handlers::{ApiResponse, ApiResult},
};

pub async fn list_snat_bindings(
    _role: AdminRole,
    session: SessionInfo,
    Path(location_id): Path<Id>,
    State(appstate): State<AppState>,
) -> ApiResult {
    let current_user = session.user.username;

    debug!("User {current_user} listing SNAT bindings for WireGuard location {location_id}");

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

#[derive(Debug, Deserialize)]
pub struct NewUserSnatBinding {
    user_id: Id,
    public_ip: IpAddr,
}

pub async fn create_snat_binding(
    _role: AdminRole,
    session: SessionInfo,
    Path(location_id): Path<Id>,
    State(appstate): State<AppState>,
    Json(data): Json<NewUserSnatBinding>,
) -> ApiResult {
    let current_user = session.user.username;

    debug!("User {current_user} creating new SNAT binding for WireGuard location {location_id} with {data:?}");

    let snat_binding = UserSnatBinding::new(data.user_id, location_id, data.public_ip);

    let binding = snat_binding.save(&appstate.pool).await?;

    Ok(ApiResponse {
        json: json!(binding),
        status: StatusCode::CREATED,
    })
}

#[derive(Debug, Deserialize)]
pub struct EditUserSnatBinding {
    public_ip: IpAddr,
}

pub async fn modify_snat_binding(
    _role: AdminRole,
    session: SessionInfo,
    Path((location_id, user_id)): Path<(Id, Id)>,
    State(appstate): State<AppState>,
    Json(data): Json<EditUserSnatBinding>,
) -> ApiResult {
    let current_user = session.user.username;

    debug!("User {current_user} updating SNAT binding for user {user_id} and WireGuard location {location_id} with {data:?}");

    // fetch existing binding
    let mut snat_binding =
        UserSnatBinding::find_binding(&appstate.pool, location_id, user_id).await?;

    // update public IP
    snat_binding.update_ip(data.public_ip);
    snat_binding.save(&appstate.pool).await?;

    Ok(ApiResponse {
        json: json!(snat_binding),
        status: StatusCode::OK,
    })
}

pub async fn delete_snat_binding(
    _role: AdminRole,
    session: SessionInfo,
    Path((location_id, user_id)): Path<(Id, Id)>,
    State(appstate): State<AppState>,
) -> ApiResult {
    let current_user = session.user.username;

    debug!("User {current_user} deleting SNAT binding for user {user_id} and WireGuard location {location_id}");

    // fetch existing binding
    let snat_binding = UserSnatBinding::find_binding(&appstate.pool, location_id, user_id).await?;

    // delete binding
    snat_binding.delete(&appstate.pool).await?;

    Ok(ApiResponse::default())
}
