use std::net::IpAddr;

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use defguard_common::db::Id;
use defguard_static_ip::{DeviceLocationIp, LocationDevices, get_ips_for_device, get_ips_for_user};
use serde::Serialize;

use crate::{
    appstate::AppState,
    auth::{AdminRole, SessionInfo},
    handlers::{ApiResponse, ApiResult},
};

#[derive(Serialize)]
pub struct LocationDevicesResponse {
    pub locations: Vec<LocationDevices>,
}

#[derive(Serialize)]
pub struct DeviceLocationIpsResponse {
    pub locations: Vec<DeviceLocationIp>,
}

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

#[derive(Deserialize)]
pub struct StaticIpAssignment {
    pub device_id: i64,
    pub location_id: Id,
    pub ips: Vec<IpAddr>,
}

pub async fn assign_static_ips(
    _admin_role: AdminRole,
    _session: SessionInfo,
    State(state): State<AppState>,
    Json(payload): Json<Vec<StaticIpAssignment>>,
) -> ApiResult {
    let mut transaction = state.pool.begin().await?;
    for assignment in payload {
        defguard_static_ip::assign_static_ips(
            assignment.device_id,
            assignment.ips,
            assignment.location_id,
            &mut transaction,
        )
        .await?;
    }
    transaction.commit().await?;
    Ok(ApiResponse {
        json: serde_json::json!({"message": "Static IPs assigned successfully"}),
        status: StatusCode::OK,
    })
}

#[derive(Deserialize)]
pub struct ValidateIpAssignmentRequest {
    pub device_id: i64,
    pub ip: IpAddr,
    pub location: Id,
}

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
