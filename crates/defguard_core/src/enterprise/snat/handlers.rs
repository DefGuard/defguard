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
    grpc::GatewayCommand,
    handlers::{ApiErrorResponse, ApiResponse, ApiResult},
};

/// List SNAT bindings in a location.
#[utoipa::path(
    get,
    path = "/api/v1/network/{location_id}/snat",
    tag = "SNAT",
    params(
        ("location_id" = i64, Path, description = "ID of the location.")
    ),
    responses(
        (status = 200, description = "All SNAT bindings in the location.", body = [UserSnatBinding]),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges and an active enterprise license.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 404, description = "Location not found.", body = ApiErrorResponse, example = json!({"msg": "Location 1 not found"})),
        (status = 500, description = "Unable to list SNAT bindings.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"}))
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
    /// ID of the user bound to the public IP address.
    pub user_id: Id,
    /// Public IP address used for SNAT.
    #[schema(value_type = String)]
    pub public_ip: IpAddr,
}

/// Create a SNAT binding for a user in a location.
#[utoipa::path(
    post,
    path = "/api/v1/network/{location_id}/snat",
    tag = "SNAT",
    params(
        ("location_id" = i64, Path, description = "ID of the location.")
    ),
    request_body = NewUserSnatBinding,
    responses(
        (status = 201, description = "SNAT binding created.", body = UserSnatBinding),
        (status = 400, description = "Invalid request data.", body = ApiErrorResponse, example = json!({"msg": "Invalid request data"})),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges and an active enterprise license.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 404, description = "Location or user not found.", body = ApiErrorResponse, example = json!({"msg": "Location 1 not found"})),
        (status = 409, description = "A SNAT binding for this user in this location already exists.", body = ApiErrorResponse, example = json!({"msg": "Binding already exists"})),
        (status = 500, description = "Unable to create SNAT binding.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"}))
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
    if let Some(location) = WireguardNetwork::find_by_id(&appstate.pool, location.id).await?
        && let Some(firewall_config) =
            try_get_location_firewall_config(&location, &mut conn).await?
    {
        debug!(
            "Sending firewall config update for location {location} affected by adding new SNAT binding"
        );
        appstate.send_gateway_command(GatewayCommand::FirewallConfigChanged(
            location.id,
            firewall_config,
        ));
    }

    Ok(ApiResponse::json(binding, StatusCode::CREATED))
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct EditUserSnatBinding {
    /// New public IP address used for SNAT.
    #[schema(value_type = String)]
    pub public_ip: IpAddr,
}

/// Update a SNAT binding.
#[utoipa::path(
    put,
    path = "/api/v1/network/{location_id}/snat/{user_id}",
    tag = "SNAT",
    params(
        ("location_id" = i64, Path, description = "ID of the location."),
        ("user_id" = i64, Path, description = "ID of the user.")
    ),
    request_body = EditUserSnatBinding,
    responses(
        (status = 200, description = "SNAT binding updated.", body = UserSnatBinding),
        (status = 400, description = "Invalid request data.", body = ApiErrorResponse, example = json!({"msg": "Invalid request data"})),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges and an active enterprise license.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 404, description = "Location, user or SNAT binding not found.", body = ApiErrorResponse, example = json!({"msg": "Binding not found"})),
        (status = 500, description = "Unable to update SNAT binding.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"}))
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

    snat_binding.public_ip = data.public_ip;
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
    if let Some(location) = WireguardNetwork::find_by_id(&appstate.pool, location_id).await?
        && let Some(firewall_config) =
            try_get_location_firewall_config(&location, &mut conn).await?
    {
        debug!(
            "Sending firewall config update for location {location} affected by adding new SNAT binding"
        );
        appstate.send_gateway_command(GatewayCommand::FirewallConfigChanged(
            location_id,
            firewall_config,
        ));
    }

    Ok(ApiResponse::json(snat_binding, StatusCode::OK))
}

/// Delete a SNAT binding.
#[utoipa::path(
    delete,
    path = "/api/v1/network/{location_id}/snat/{user_id}",
    tag = "SNAT",
    params(
        ("location_id" = i64, Path, description = "ID of the location."),
        ("user_id" = i64, Path, description = "ID of the user.")
    ),
    responses(
        (status = 200, description = "SNAT binding deleted."),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges and an active enterprise license.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 404, description = "Location, user or SNAT binding not found.", body = ApiErrorResponse, example = json!({"msg": "Binding not found"})),
        (status = 500, description = "Unable to delete SNAT binding.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"}))
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
    if let Some(location) = WireguardNetwork::find_by_id(&appstate.pool, location_id).await?
        && let Some(firewall_config) =
            try_get_location_firewall_config(&location, &mut conn).await?
    {
        debug!(
            "Sending firewall config update for location {location} affected by adding new SNAT binding"
        );
        appstate.send_gateway_command(GatewayCommand::FirewallConfigChanged(
            location_id,
            firewall_config,
        ));
    }

    Ok(ApiResponse::default())
}
