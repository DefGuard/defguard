use axum::{
    extract::{Path, State},
    Json,
};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::query_as;
use std::net::IpAddr;
use utoipa::ToSchema;

use crate::{
    appstate::AppState,
    auth::{AdminRole, SessionInfo},
    db::Id,
    enterprise::{handlers::LicenseInfo, snat::UserSnatBinding},
    handlers::{ApiResponse, ApiResult},
};

/// List all SNAT bindings for a WireGuard location
#[utoipa::path(
    get,
    path = "/api/v1/network/{location_id}/snat",
    tag = "SNAT",
    params(
        ("location_id" = Id, Path, description = "WireGuard location ID")
    ),
    responses(
        (status = 200, description = "List of SNAT bindings", body = Vec<UserSnatBinding>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - Admin role required"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub async fn list_snat_bindings(
    _license: LicenseInfo,
    _admin_role: AdminRole,
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

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct NewUserSnatBinding {
    /// User ID to bind to the public IP
    user_id: Id,
    /// Public IP address for SNAT
    #[schema(value_type = String)]
    public_ip: IpAddr,
}

/// Create a new SNAT binding for a user in a WireGuard location
#[utoipa::path(
    post,
    path = "/api/v1/network/{location_id}/snat",
    tag = "SNAT",
    params(
        ("location_id" = Id, Path, description = "WireGuard location ID")
    ),
    request_body = NewUserSnatBinding,
    responses(
        (status = 201, description = "SNAT binding created successfully", body = UserSnatBinding),
        (status = 400, description = "Bad request - Invalid input data"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - Admin role required"),
        (status = 409, description = "Conflict - Binding already exists"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub async fn create_snat_binding(
    _license: LicenseInfo,
    _admin_role: AdminRole,
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

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct EditUserSnatBinding {
    /// New public IP address for SNAT
    #[schema(value_type = String)]
    public_ip: IpAddr,
}

/// Modify an existing SNAT binding for a user in a WireGuard location
#[utoipa::path(
    put,
    path = "/api/v1/network/{location_id}/snat/{user_id}",
    tag = "SNAT",
    params(
        ("location_id" = Id, Path, description = "WireGuard location ID"),
        ("user_id" = Id, Path, description = "User ID")
    ),
    request_body = EditUserSnatBinding,
    responses(
        (status = 200, description = "SNAT binding updated successfully", body = UserSnatBinding),
        (status = 400, description = "Bad request - Invalid input data"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - Admin role required"),
        (status = 404, description = "Not found - SNAT binding does not exist"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub async fn modify_snat_binding(
    _license: LicenseInfo,
    _admin_role: AdminRole,
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

/// Delete an existing SNAT binding for a user in a WireGuard location
#[utoipa::path(
    delete,
    path = "/api/v1/network/{location_id}/snat/{user_id}",
    tag = "SNAT",
    params(
        ("location_id" = Id, Path, description = "WireGuard location ID"),
        ("user_id" = Id, Path, description = "User ID")
    ),
    responses(
        (status = 200, description = "SNAT binding deleted successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - Admin role required"),
        (status = 404, description = "Not found - SNAT binding does not exist"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub async fn delete_snat_binding(
    _license: LicenseInfo,
    _admin_role: AdminRole,
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
