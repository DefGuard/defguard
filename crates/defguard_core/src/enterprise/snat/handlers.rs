use axum::{
    Json,
    extract::{Path, State},
};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::net::IpAddr;
use utoipa::ToSchema;

use crate::{
    appstate::AppState,
    auth::{AdminRole, SessionInfo},
    db::{GatewayEvent, Id, User, WireguardNetwork},
    enterprise::{
        db::models::snat::UserSnatBinding, handlers::LicenseInfo, snat::error::UserSnatBindingError,
    },
    error::WebError,
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
        (status = 404, description = "Not found - location does not exist"),
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
    location: WireguardNetwork<Id>,
    State(appstate): State<AppState>,
) -> ApiResult {
    let current_user = session.user.username;

    debug!("User {current_user} listing SNAT bindings for WireGuard location {location}");

    let bindings = location.get_all_snat_bindings(&appstate.pool).await?;

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
        (status = 404, description = "Not found - location or user does not exist"),
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
    location: WireguardNetwork<Id>,
    State(appstate): State<AppState>,
    Json(data): Json<NewUserSnatBinding>,
) -> ApiResult {
    let current_user = session.user.username;

    // check if target user exists
    let _snat_user = User::find_by_id(&appstate.pool, data.user_id)
        .await?
        .ok_or_else(|| WebError::ObjectNotFound(format!("User {} not found", data.user_id)))?;

    debug!(
        "User {current_user} creating new SNAT binding for WireGuard location {location} with {data:?}"
    );

    let snat_binding = UserSnatBinding::new(data.user_id, location.id, data.public_ip);

    let binding = snat_binding
        .save(&appstate.pool)
        .await
        .map_err(UserSnatBindingError::from)?;

    // trigger firewall config update on relevant gateways
    let mut conn = appstate.pool.acquire().await?;
    if let Some(location) = WireguardNetwork::find_by_id(&appstate.pool, location.id).await? {
        if let Some(firewall_config) = location.try_get_firewall_config(&mut conn).await? {
            debug!(
                "Sending firewall config update for location {location} affected by adding new SNAT binding"
            );
            appstate.send_wireguard_event(GatewayEvent::FirewallConfigChanged(
                location.id,
                firewall_config,
            ));
        }
    }

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
    location: WireguardNetwork<Id>,
    State(appstate): State<AppState>,
    Json(data): Json<EditUserSnatBinding>,
) -> ApiResult {
    let current_user = session.user.username;

    debug!(
        "User {current_user} updating SNAT binding for user {user_id} and WireGuard location {location} with {data:?}"
    );

    // fetch existing binding
    let mut snat_binding =
        UserSnatBinding::find_binding(&appstate.pool, location_id, user_id).await?;

    // update public IP
    snat_binding.update_ip(data.public_ip);
    snat_binding.save(&appstate.pool).await?;

    // trigger firewall config update on relevant gateways
    let mut conn = appstate.pool.acquire().await?;
    if let Some(location) = WireguardNetwork::find_by_id(&appstate.pool, location_id).await? {
        if let Some(firewall_config) = location.try_get_firewall_config(&mut conn).await? {
            debug!(
                "Sending firewall config update for location {location} affected by adding new SNAT binding"
            );
            appstate.send_wireguard_event(GatewayEvent::FirewallConfigChanged(
                location_id,
                firewall_config,
            ));
        }
    }

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
    location: WireguardNetwork<Id>,
    State(appstate): State<AppState>,
) -> ApiResult {
    let current_user = session.user.username;

    debug!(
        "User {current_user} deleting SNAT binding for user {user_id} and WireGuard location {location}"
    );

    // fetch existing binding
    let snat_binding = UserSnatBinding::find_binding(&appstate.pool, location_id, user_id).await?;

    // delete binding
    snat_binding.delete(&appstate.pool).await?;

    // trigger firewall config update on relevant gateways
    let mut conn = appstate.pool.acquire().await?;
    if let Some(location) = WireguardNetwork::find_by_id(&appstate.pool, location_id).await? {
        if let Some(firewall_config) = location.try_get_firewall_config(&mut conn).await? {
            debug!(
                "Sending firewall config update for location {location} affected by adding new SNAT binding"
            );
            appstate.send_wireguard_event(GatewayEvent::FirewallConfigChanged(
                location_id,
                firewall_config,
            ));
        }
    }

    Ok(ApiResponse::default())
}
