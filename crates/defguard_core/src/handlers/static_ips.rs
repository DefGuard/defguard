use std::{collections::BTreeSet, net::IpAddr};

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use defguard_common::db::{
    Id,
    models::{Device, WireguardNetwork, device::DeviceInfo},
};
use defguard_static_ip::{DeviceLocationIp, LocationDevices, get_ips_for_device, get_ips_for_user};
use serde::Serialize;
use utoipa::ToSchema;

use crate::{
    appstate::AppState,
    auth::{AdminRole, SessionInfo},
    enterprise::firewall::try_get_location_firewall_config,
    grpc::GatewayCommand,
    handlers::{ApiErrorResponse, ApiResponse, ApiResult},
};

#[derive(Serialize)]
pub struct LocationDevicesResponse {
    pub locations: Vec<LocationDevices>,
}

#[derive(Serialize)]
pub struct DeviceLocationIpsResponse {
    pub locations: Vec<DeviceLocationIp>,
}

/// List the IP addresses of all devices of a user, grouped by location
#[utoipa::path(
    get,
    path = "/api/v1/device/user/{username}/ip",
    tag = "static IP",
    params(
        ("username" = String, Path, description = "Name of the user."),
    ),
    responses(
        (status = 200, description = "IP addresses of all devices of the user, grouped by location.", body = Object, example = json!({
            "locations": [{
                "location_id": 1,
                "location_name": "office",
                "devices": [{"device_id": 5, "device_name": "laptop", "wireguard_ips": [{"network_part": "10.0.0.", "modifiable_part": "15", "network_prefix": "/24", "ip": "10.0.0.15"}]}]
            }]
        })),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 404, description = "User not found.", body = ApiErrorResponse, example = json!({"msg": "user not found"})),
        (status = 500, description = "Unable to get user device IP addresses.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub async fn get_all_user_device_ips(
    _admin_role: AdminRole,
    _session: SessionInfo,
    Path(username): Path<String>,
    State(state): State<AppState>,
) -> ApiResult {
    let locations = get_ips_for_user(&username, &state.pool).await?;
    Ok(ApiResponse::json(
        LocationDevicesResponse { locations },
        StatusCode::OK,
    ))
}

/// List the IP addresses of a user device, grouped by location
#[utoipa::path(
    get,
    path = "/api/v1/device/user/{username}/ip/{device_id}",
    tag = "static IP",
    params(
        ("username" = String, Path, description = "Name of the user."),
        ("device_id" = i64, Path, description = "ID of the device."),
    ),
    responses(
        (status = 200, description = "IP addresses of the device, grouped by location.", body = Object, example = json!({
            "locations": [{"location_id": 1, "location_name": "office", "wireguard_ips": [{"network_part": "10.0.0.", "modifiable_part": "15", "network_prefix": "/24", "ip": "10.0.0.15"}]}]
        })),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 404, description = "User or device not found.", body = ApiErrorResponse, example = json!({"msg": "device not found"})),
        (status = 500, description = "Unable to get device IP addresses.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub async fn get_device_ips(
    _admin_role: AdminRole,
    _session: SessionInfo,
    Path((username, device_id)): Path<(String, Id)>,
    State(state): State<AppState>,
) -> ApiResult {
    let locations = get_ips_for_device(&username, device_id, &state.pool).await?;
    Ok(ApiResponse::json(
        DeviceLocationIpsResponse { locations },
        StatusCode::OK,
    ))
}

#[derive(Deserialize, ToSchema)]
pub struct StaticIpAssignment {
    pub device_id: i64,
    pub location_id: Id,
    #[schema(value_type = Vec<String>)]
    pub ips: Vec<IpAddr>,
}

/// Assign static IP addresses to user devices
#[utoipa::path(
    post,
    path = "/api/v1/device/user/{username}/ip",
    tag = "static IP",
    request_body = Vec<StaticIpAssignment>,
    params(
        ("username" = String, Path, description = "Name of the user."),
    ),
    responses(
        (status = 200, description = "IP addresses assigned.", body = Object, example = json!({"message": "Static IPs assigned successfully"})),
        (status = 400, description = "Invalid IP assignment.", body = ApiErrorResponse, example = json!({"msg": "IP address is already in use"})),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 500, description = "Unable to assign IP addresses.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub async fn assign_static_ips(
    _admin_role: AdminRole,
    _session: SessionInfo,
    State(state): State<AppState>,
    Json(payload): Json<Vec<StaticIpAssignment>>,
) -> ApiResult {
    let mut transaction = state.pool.begin().await?;
    let mut device_ids = BTreeSet::new();
    let mut location_ids = BTreeSet::new();
    for assignment in payload {
        defguard_static_ip::assign_static_ips(
            assignment.device_id,
            assignment.ips,
            assignment.location_id,
            &mut transaction,
        )
        .await?;
        device_ids.insert(assignment.device_id);
        location_ids.insert(assignment.location_id);
    }
    transaction.commit().await?;

    let mut conn = state.pool.acquire().await?;
    for device_id in device_ids {
        if let Some(device) = Device::find_by_id(&mut *conn, device_id).await? {
            let device_info = DeviceInfo::from_device(&mut *conn, device).await?;
            state.send_gateway_command(GatewayCommand::DeviceModified(device_info));
        }
    }
    // ACL rules embed peer addresses, so locations with ACLs need a firewall refresh too.
    for location_id in location_ids {
        if let Some(location) = WireguardNetwork::find_by_id(&mut *conn, location_id).await?
            && let Some(firewall_config) =
                try_get_location_firewall_config(&location, &mut conn).await?
        {
            state.send_gateway_command(GatewayCommand::FirewallConfigChanged(
                location_id,
                firewall_config,
            ));
        }
    }

    Ok(ApiResponse {
        json: serde_json::json!({"message": "Static IPs assigned successfully"}),
        status: StatusCode::OK,
    })
}

#[derive(Deserialize, ToSchema)]
pub struct ValidateIpAssignmentRequest {
    pub device_id: i64,
    #[schema(value_type = String)]
    pub ip: IpAddr,
    pub location: Id,
}

/// Check whether a single static IP assignment would be valid
#[utoipa::path(
    post,
    path = "/api/v1/device/user/{username}/ip/validate",
    tag = "static IP",
    request_body = ValidateIpAssignmentRequest,
    params(
        ("username" = String, Path, description = "Name of the user."),
    ),
    responses(
        (status = 200, description = "Validation result.", body = Object, example = json!({"message": "IP assignment is valid"})),
        (status = 400, description = "Invalid IP assignment.", body = ApiErrorResponse, example = json!({"msg": "IP address is already in use"})),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 500, description = "Unable to validate IP assignment.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub async fn validate_ip_assignment(
    _admin_role: AdminRole,
    _session: SessionInfo,
    State(state): State<AppState>,
    Json(payload): Json<ValidateIpAssignmentRequest>,
) -> ApiResult {
    let mut transaction = state.pool.begin().await?;
    defguard_static_ip::validate_ip(
        payload.device_id,
        payload.ip,
        payload.location,
        &mut transaction,
    )
    .await?;
    transaction.commit().await?;
    Ok(ApiResponse {
        json: serde_json::json!({"message": "IP assignment is valid"}),
        status: StatusCode::OK,
    })
}
