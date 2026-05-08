use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use defguard_common::db::{Id, NoId};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    appstate::AppState,
    auth::{AdminRole, SessionInfo},
    enterprise::{db::models::device_posture::DevicePosture, handlers::EnterpriseLicenseInfo},
    error::WebError,
    events::{ApiEvent, ApiEventType, ApiRequestContext},
    handlers::{
        ApiResponse, ApiResult,
        pagination::{PaginatedApiResponse, PaginatedApiResult, PaginationParams},
    },
};

/// Minimum defguard desktop client versions available for posture rules.
/// FIXME: 2.0 does not actually exist, remove before release
/// TODO: also consider if this is the best way to store possible options
pub static CLIENT_VERSIONS: &[&str] = &["1.6", "2.0"];

/// API response type for a device posture check policy.
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ApiDevicePosture {
    pub id: Id,
    pub name: String,
    pub description: Option<String>,
    pub min_client_version: Option<String>,
    pub allow_prerelease_client: bool,
    /// IDs of VPN locations this policy is assigned to.
    pub locations: Vec<Id>,
}

impl From<DevicePosture<Id>> for ApiDevicePosture {
    fn from(p: DevicePosture<Id>) -> Self {
        Self {
            id: p.id,
            name: p.name,
            description: p.description,
            min_client_version: p.min_client_version,
            allow_prerelease_client: p.allow_prerelease_client,
            locations: vec![],
        }
    }
}

/// Request body for creating or updating a device posture check policy.
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct EditDevicePosture {
    pub name: String,
    pub description: Option<String>,
    pub min_client_version: Option<String>,
    pub allow_prerelease_client: bool,
}

/// Validates the base fields of an [`EditDevicePosture`] request.
///
/// Returns `Err(WebError::BadRequest(...))` if `min_client_version` is set to
/// a value not present in [`CLIENT_VERSIONS`].
fn validate_device_posture_base(data: &EditDevicePosture) -> Result<(), WebError> {
    if let Some(ref version) = data.min_client_version {
        if !CLIENT_VERSIONS.contains(&version.as_str()) {
            return Err(WebError::BadRequest(format!(
                "Unknown client version '{version}'. Valid values: {}",
                CLIENT_VERSIONS.join(", ")
            )));
        }
    }
    Ok(())
}

#[utoipa::path(
    post,
    path = "/api/v1/posture",
    tag = "DevicePosture",
    request_body = EditDevicePosture,
    responses(
        (status = 201, description = "Posture check created successfully", body = ApiDevicePosture),
        (status = 400, description = "Bad request - invalid field value"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - enterprise license required"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub async fn create_device_posture(
    _license: EnterpriseLicenseInfo,
    _admin: AdminRole,
    session: SessionInfo,
    context: ApiRequestContext,
    State(appstate): State<AppState>,
    Json(data): Json<EditDevicePosture>,
) -> ApiResult {
    debug!(
        "User {} creating device posture check {:?}",
        session.user.username, data.name
    );

    validate_device_posture_base(&data)?;

    let posture = DevicePosture {
        id: NoId,
        name: data.name,
        description: data.description,
        min_client_version: data.min_client_version,
        allow_prerelease_client: data.allow_prerelease_client,
    }
    .save(&appstate.pool)
    .await?;

    debug!("Created posture check {}", posture.id);

    appstate.emit_event(ApiEvent {
        context,
        event: Box::new(ApiEventType::DevicePostureCreated {
            posture: posture.clone(),
        }),
    })?;

    Ok(ApiResponse::json(
        ApiDevicePosture::from(posture),
        StatusCode::CREATED,
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/device-posture",
    tag = "DevicePosture",
    params(
        ("page" = Option<u32>, Query, description = "Page number (default: 1)"),
        ("per_page" = Option<u32>, Query, description = "Items per page (default: 10)"),
    ),
    responses(
        (status = 200, description = "Paginated list of device posture check policies", body = [ApiDevicePosture]),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - enterprise license required"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub async fn list_device_postures(
    _license: EnterpriseLicenseInfo,
    _admin: AdminRole,
    session: SessionInfo,
    pagination: Query<PaginationParams>,
    State(appstate): State<AppState>,
) -> PaginatedApiResult<ApiDevicePosture> {
    let pagination = pagination.0;
    debug!(
        "User {} listing device posture checks",
        session.user.username
    );

    let mut conn = appstate.pool.acquire().await?;
    let device_postures = DevicePosture::all_paginated(
        &mut *conn,
        i64::from(pagination.per_page()),
        i64::from(pagination.offset()),
    )
    .await?;
    let count = DevicePosture::count(&mut *conn).await?;

    Ok(PaginatedApiResponse::new(
        device_postures
            .into_iter()
            .map(ApiDevicePosture::from)
            .collect(),
        pagination,
        count as u32,
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/device-posture/{id}",
    tag = "DevicePosture",
    params(
        ("id" = Id, Path, description = "Device posture check policy ID")
    ),
    responses(
        (status = 200, description = "Device posture check policy", body = ApiDevicePosture),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - enterprise license required"),
        (status = 404, description = "Not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub async fn get_device_posture(
    _license: EnterpriseLicenseInfo,
    _admin: AdminRole,
    session: SessionInfo,
    Path(id): Path<Id>,
    State(appstate): State<AppState>,
) -> ApiResult {
    debug!(
        "User {} fetching device posture check {id}",
        session.user.username
    );

    let device_posture = DevicePosture::find_by_id(&appstate.pool, id)
        .await?
        .ok_or_else(|| WebError::ObjectNotFound(format!("Device posture check {id} not found")))?;

    Ok(ApiResponse::json(
        ApiDevicePosture::from(device_posture),
        StatusCode::OK,
    ))
}
