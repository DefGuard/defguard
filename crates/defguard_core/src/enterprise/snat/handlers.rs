use std::net::IpAddr;

use axum::{
    Json,
    extract::{Path, State},
};
use defguard_common::db::{
    Id,
    models::{User, WireguardNetwork},
};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    appstate::AppState,
    auth::{AdminRole, SessionInfo},
    enterprise::{
        db::models::snat::UserSnatBinding, firewall::try_get_location_firewall_config,
        handlers::LicenseInfo, snat::error::UserSnatBindingError,
    },
    error::WebError,
    events::{ApiEvent, ApiEventType, ApiRequestContext},
    grpc::GatewayEvent,
    handlers::{ApiResponse, ApiResult},
};

/// List all SNAT bindings for a WireGuard location
///
/// # Returns
/// - `Vec<UserSnatBinding<i64>>` object
///
/// - `WebError` if error occurs
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
    Path(location_id): Path<Id>,
    State(appstate): State<AppState>,
) -> ApiResult {
    let current_user = session.user.username;
    //
    // check if target location exists
    let location = WireguardNetwork::find_by_id(&appstate.pool, location_id)
        .await?
        .ok_or_else(|| WebError::ObjectNotFound(format!("Location {location_id} not found")))?;

    debug!("User {current_user} listing SNAT bindings for WireGuard location {location}");

    let bindings = UserSnatBinding::all_for_location(&appstate.pool, location.id).await?;

    Ok(ApiResponse::json(bindings, StatusCode::OK))
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct NewUserSnatBinding {
    /// User ID to bind to the public IP
    pub user_id: Id,
    /// Public IP address for SNAT
    #[schema(value_type = String)]
    pub public_ip: IpAddr,
}

/// Create a new SNAT binding for a user in a WireGuard location
///
/// Create snat binding basing on `NewUserSnatBinding` object.
///
/// # Returns
/// - `UserSnatBinding<i64>` object
///
/// - `WebError` if error occurs
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
    context: ApiRequestContext,
    Path(location_id): Path<Id>,
    State(appstate): State<AppState>,
    Json(data): Json<NewUserSnatBinding>,
) -> ApiResult {
    let current_user = session.user.username;

    // check if target location & user exist
    let location = WireguardNetwork::find_by_id(&appstate.pool, location_id)
        .await?
        .ok_or_else(|| WebError::ObjectNotFound(format!("Location {location_id} not found")))?;
    let snat_user = User::find_by_id(&appstate.pool, data.user_id)
        .await?
        .ok_or_else(|| WebError::ObjectNotFound(format!("User {} not found", data.user_id)))?;

    debug!(
        "User {current_user} creating new SNAT binding for user {snat_user} in WireGuard location {location} with {data:?}"
    );

    let snat_binding = UserSnatBinding::new(data.user_id, location.id, data.public_ip);

    let binding = snat_binding
        .save(&appstate.pool)
        .await
        .map_err(UserSnatBindingError::from)?;

    // emit event
    appstate.emit_event(ApiEvent {
        context,
        event: Box::new(ApiEventType::UserSnatBindingAdded {
            user: snat_user,
            location: location.clone(),
            binding: binding.clone(),
        }),
    })?;

    // trigger firewall config update on relevant gateways
    let mut conn = appstate.pool.acquire().await?;
    if let Some(location) = WireguardNetwork::find_by_id(&appstate.pool, location.id).await? {
        if let Some(firewall_config) =
            try_get_location_firewall_config(&location, &mut conn).await?
        {
            debug!(
                "Sending firewall config update for location {location} affected by adding new SNAT binding"
            );
            appstate.send_wireguard_event(GatewayEvent::FirewallConfigChanged(
                location.id,
                firewall_config,
            ));
        }
    }

    Ok(ApiResponse::json(binding, StatusCode::CREATED))
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct EditUserSnatBinding {
    /// New public IP address for SNAT
    #[schema(value_type = String)]
    pub public_ip: IpAddr,
}

/// Modify an existing SNAT binding for a user in a WireGuard location
///
/// Modify an **existing** SNAT binding basing on `EditUserSnatBinding` object.
///
/// # Returns
/// - `UserSnatBinding` object
///
/// - `WebError` if error occurs
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
    context: ApiRequestContext,
    Path((location_id, user_id)): Path<(Id, Id)>,
    State(appstate): State<AppState>,
    Json(data): Json<EditUserSnatBinding>,
) -> ApiResult {
    let current_user = session.user.username;

    // fetch relevant location & user
    let location = WireguardNetwork::find_by_id(&appstate.pool, location_id)
        .await?
        .ok_or_else(|| WebError::ObjectNotFound(format!("Location {location_id} not found")))?;
    let snat_user = User::find_by_id(&appstate.pool, user_id)
        .await?
        .ok_or_else(|| WebError::ObjectNotFound(format!("User {user_id} not found")))?;

    debug!(
        "User {current_user} updating SNAT binding for user {snat_user} and WireGuard location {location} with {data:?}",
    );

    // fetch existing binding
    let mut snat_binding =
        UserSnatBinding::find_binding(&appstate.pool, location_id, user_id).await?;

    // clone state before modifications
    let before = snat_binding.clone();

    // update public IP
    snat_binding.update_ip(data.public_ip);
    snat_binding.save(&appstate.pool).await?;

    // emit event
    appstate.emit_event(ApiEvent {
        context,
        event: Box::new(ApiEventType::UserSnatBindingModified {
            user: snat_user,
            location: location.clone(),
            before,
            after: snat_binding.clone(),
        }),
    })?;

    // trigger firewall config update on relevant gateways
    let mut conn = appstate.pool.acquire().await?;
    if let Some(location) = WireguardNetwork::find_by_id(&appstate.pool, location_id).await? {
        if let Some(firewall_config) =
            try_get_location_firewall_config(&location, &mut conn).await?
        {
            debug!(
                "Sending firewall config update for location {location} affected by adding new SNAT binding"
            );
            appstate.send_wireguard_event(GatewayEvent::FirewallConfigChanged(
                location_id,
                firewall_config,
            ));
        }
    }

    Ok(ApiResponse::json(snat_binding, StatusCode::OK))
}

/// Delete an existing SNAT binding for a user in a WireGuard location
///
/// Delete an existing SNAT binding basing on `location_id` and `user_id`.
///
/// # Returns
/// - empty JSON
///
/// - `WebError` if error occurs
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
    context: ApiRequestContext,
    Path((location_id, user_id)): Path<(Id, Id)>,
    State(appstate): State<AppState>,
) -> ApiResult {
    let current_user = session.user.username;

    // fetch relevant location & user
    let location = WireguardNetwork::find_by_id(&appstate.pool, location_id)
        .await?
        .ok_or_else(|| WebError::ObjectNotFound(format!("Location {location_id} not found")))?;
    let snat_user = User::find_by_id(&appstate.pool, user_id)
        .await?
        .ok_or_else(|| WebError::ObjectNotFound(format!("User {user_id} not found")))?;

    debug!(
        "User {current_user} deleting SNAT binding for user {snat_user} and WireGuard location {location}"
    );

    // fetch existing binding
    let snat_binding = UserSnatBinding::find_binding(&appstate.pool, location_id, user_id).await?;

    // delete binding
    snat_binding.clone().delete(&appstate.pool).await?;

    // emit event
    appstate.emit_event(ApiEvent {
        context,
        event: Box::new(ApiEventType::UserSnatBindingRemoved {
            user: snat_user,
            location: location.clone(),
            binding: snat_binding,
        }),
    })?;

    // trigger firewall config update on relevant gateways
    let mut conn = appstate.pool.acquire().await?;
    if let Some(location) = WireguardNetwork::find_by_id(&appstate.pool, location_id).await? {
        if let Some(firewall_config) =
            try_get_location_firewall_config(&location, &mut conn).await?
        {
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
