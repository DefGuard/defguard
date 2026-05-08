use std::collections::HashSet;

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use defguard_common::db::{Id, NoId, models::WireguardNetwork};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    appstate::AppState,
    auth::{AdminRole, SessionInfo},
    enterprise::{
        db::models::device_posture::{
            DevicePosture, DevicePostureLocation, DevicePostureOsRule, DevicePostureSnapshot,
            OsType,
        },
        handlers::EnterpriseLicenseInfo,
    },
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

/// Valid Linux kernel version families for posture rules.
pub static KERNEL_VERSIONS: &[&str] = &["5.x", "6.x"];

/// Returns the list of valid `min_os_version` values for a given OS type.
/// TODO: consider a better format for storing versions
pub fn valid_os_versions(os_type: &OsType) -> &'static [&'static str] {
    match os_type {
        OsType::Windows => &["Windows 10", "Windows 11"],
        OsType::Macos => &[
            "macOS 12 Monterey",
            "macOS 13 Ventura",
            "macOS 14 Sonoma",
            "macOS 15 Sequoia",
        ],
        OsType::Linux => &[],
        OsType::Ios => &["17", "18"],
        OsType::Android => &["13", "14", "15", "16"],
    }
}

/// Per-OS rule included in a posture check policy API.
///
/// Adding this layer on top of the shared DB type allows us
/// to require different fields for specific platforms.
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, PartialEq)]
#[serde(tag = "os_type", rename_all = "lowercase")]
pub enum ApiOsRule {
    Windows {
        min_os_version: Option<String>,
        disk_encryption_required: Option<bool>,
        antivirus_required: Option<bool>,
        ad_domain_joined_required: Option<bool>,
        windows_security_update_current: Option<bool>,
    },
    Macos {
        min_os_version: Option<String>,
        disk_encryption_required: Option<bool>,
        device_integrity_required: Option<bool>,
    },
    Linux {
        min_kernel_version: Option<String>,
        disk_encryption_required: Option<bool>,
    },
    Ios {
        min_os_version: Option<String>,
    },
    Android {
        min_os_version: Option<String>,
        device_integrity_required: Option<bool>,
    },
}

impl ApiOsRule {
    /// Returns the [`OsType`] discriminant for this rule set.
    fn os_type(&self) -> OsType {
        match self {
            Self::Windows { .. } => OsType::Windows,
            Self::Macos { .. } => OsType::Macos,
            Self::Linux { .. } => OsType::Linux,
            Self::Ios { .. } => OsType::Ios,
            Self::Android { .. } => OsType::Android,
        }
    }

    /// Converts this rule set into an unsaved DB row for the given posture ID.
    pub fn into_db_rule(self, posture_id: Id) -> DevicePostureOsRule<NoId> {
        match self {
            Self::Windows {
                min_os_version,
                disk_encryption_required,
                antivirus_required,
                ad_domain_joined_required,
                windows_security_update_current,
            } => DevicePostureOsRule {
                id: NoId,
                posture_id,
                os_type: OsType::Windows,
                min_os_version,
                disk_encryption_required,
                antivirus_required,
                ad_domain_joined_required,
                windows_security_update_current,
                min_kernel_version: None,
                device_integrity_required: None,
            },
            Self::Macos {
                min_os_version,
                disk_encryption_required,
                device_integrity_required,
            } => DevicePostureOsRule {
                id: NoId,
                posture_id,
                os_type: OsType::Macos,
                min_os_version,
                disk_encryption_required,
                antivirus_required: None,
                ad_domain_joined_required: None,
                windows_security_update_current: None,
                min_kernel_version: None,
                device_integrity_required,
            },
            Self::Linux {
                min_kernel_version,
                disk_encryption_required,
            } => DevicePostureOsRule {
                id: NoId,
                posture_id,
                os_type: OsType::Linux,
                min_os_version: None,
                disk_encryption_required,
                antivirus_required: None,
                ad_domain_joined_required: None,
                windows_security_update_current: None,
                min_kernel_version,
                device_integrity_required: None,
            },
            Self::Ios { min_os_version } => DevicePostureOsRule {
                id: NoId,
                posture_id,
                os_type: OsType::Ios,
                min_os_version,
                disk_encryption_required: None,
                antivirus_required: None,
                ad_domain_joined_required: None,
                windows_security_update_current: None,
                min_kernel_version: None,
                device_integrity_required: None,
            },
            Self::Android {
                min_os_version,
                device_integrity_required,
            } => DevicePostureOsRule {
                id: NoId,
                posture_id,
                os_type: OsType::Android,
                min_os_version,
                disk_encryption_required: None,
                antivirus_required: None,
                ad_domain_joined_required: None,
                windows_security_update_current: None,
                min_kernel_version: None,
                device_integrity_required,
            },
        }
    }
}

impl From<DevicePostureOsRule<Id>> for ApiOsRule {
    fn from(rule: DevicePostureOsRule<Id>) -> Self {
        match rule.os_type {
            OsType::Windows => Self::Windows {
                min_os_version: rule.min_os_version,
                disk_encryption_required: rule.disk_encryption_required,
                antivirus_required: rule.antivirus_required,
                ad_domain_joined_required: rule.ad_domain_joined_required,
                windows_security_update_current: rule.windows_security_update_current,
            },
            OsType::Macos => Self::Macos {
                min_os_version: rule.min_os_version,
                disk_encryption_required: rule.disk_encryption_required,
                device_integrity_required: rule.device_integrity_required,
            },
            OsType::Linux => Self::Linux {
                min_kernel_version: rule.min_kernel_version,
                disk_encryption_required: rule.disk_encryption_required,
            },
            OsType::Ios => Self::Ios {
                min_os_version: rule.min_os_version,
            },
            OsType::Android => Self::Android {
                min_os_version: rule.min_os_version,
                device_integrity_required: rule.device_integrity_required,
            },
        }
    }
}

/// API response type for a device posture check policy.
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ApiDevicePosture {
    pub id: Id,
    pub name: String,
    pub description: Option<String>,
    pub min_client_version: Option<String>,
    pub allow_prerelease_client: bool,
    pub os_rules: Vec<ApiOsRule>,
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
            os_rules: Vec::new(),
            locations: Vec::new(),
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
    #[serde(default)]
    pub os_rules: Vec<ApiOsRule>,
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
    validate_device_posture_os_rules(&data.os_rules)
}

/// Validates the `os_rules` list in an [`EditDevicePosture`] request.
///
/// Returns `Err(WebError::BadRequest(...))` if:
/// - the same `os_type` appears more than once, or
/// - `min_os_version` is not in the known list for its OS type, or
/// - `min_kernel_version` is not in the known Linux kernel version list.
fn validate_device_posture_os_rules(os_rules: &[ApiOsRule]) -> Result<(), WebError> {
    let mut seen = HashSet::new();
    for rule in os_rules {
        let os_type = rule.os_type();
        if !seen.insert(std::mem::discriminant(rule)) {
            return Err(WebError::BadRequest(format!(
                "Duplicate os_type '{os_type:?}' in os_rules"
            )));
        }
        let valid_versions = valid_os_versions(&os_type);
        let min_os_version = match rule {
            ApiOsRule::Windows { min_os_version, .. }
            | ApiOsRule::Macos { min_os_version, .. }
            | ApiOsRule::Ios { min_os_version }
            | ApiOsRule::Android { min_os_version, .. } => min_os_version,
            ApiOsRule::Linux { .. } => &None,
        };
        if let Some(v) = min_os_version {
            if !valid_versions.contains(&v.as_str()) {
                return Err(WebError::BadRequest(format!(
                    "Unknown min_os_version '{v}' for {os_type:?}. Valid values: {}",
                    valid_versions.join(", ")
                )));
            }
        }
        if let ApiOsRule::Linux {
            min_kernel_version: Some(kv),
            ..
        } = rule
        {
            if !KERNEL_VERSIONS.contains(&kv.as_str()) {
                return Err(WebError::BadRequest(format!(
                    "Unknown min_kernel_version '{kv}'. Valid values: {}",
                    KERNEL_VERSIONS.join(", ")
                )));
            }
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

    let EditDevicePosture {
        name,
        description,
        min_client_version,
        allow_prerelease_client,
        os_rules,
    } = data;

    let mut tx = appstate.pool.begin().await?;

    let posture = DevicePosture {
        id: NoId,
        name,
        description,
        min_client_version,
        allow_prerelease_client,
    }
    .save(&mut *tx)
    .await?;

    for rule in &os_rules {
        rule.clone().into_db_rule(posture.id).save(&mut *tx).await?;
    }

    tx.commit().await?;

    debug!("Created device posture check {}", posture.id);

    appstate.emit_event(ApiEvent {
        context,
        event: Box::new(ApiEventType::DevicePostureCreated {
            snapshot: DevicePostureSnapshot {
                device_posture: posture.clone(),
                os_rules: DevicePostureOsRule::find_by_posture(&appstate.pool, posture.id).await?,
                location_ids: Vec::new(),
            },
        }),
    })?;

    let mut response = ApiDevicePosture::from(posture);
    response.os_rules = os_rules;
    Ok(ApiResponse::json(response, StatusCode::CREATED))
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

    let mut api_postures = Vec::with_capacity(device_postures.len());
    for posture in device_postures {
        let db_rules = DevicePostureOsRule::find_by_posture(&mut *conn, posture.id).await?;
        let locations = DevicePostureLocation::find_by_posture(&mut *conn, posture.id).await?;
        let mut api = ApiDevicePosture::from(posture);
        api.os_rules = db_rules.into_iter().map(ApiOsRule::from).collect();
        api.locations = locations;
        api_postures.push(api);
    }

    Ok(PaginatedApiResponse::new(
        api_postures,
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

    let db_rules = DevicePostureOsRule::find_by_posture(&appstate.pool, id).await?;
    let locations = DevicePostureLocation::find_by_posture(&appstate.pool, id).await?;
    let mut response = ApiDevicePosture::from(device_posture);
    response.os_rules = db_rules.into_iter().map(ApiOsRule::from).collect();
    response.locations = locations;

    Ok(ApiResponse::json(response, StatusCode::OK))
}

/// Update an existing device posture check policy
#[utoipa::path(
    put,
    path = "/api/v1/device-posture/{id}",
    tag = "DevicePosture",
    params(
        ("id" = Id, Path, description = "Device posture check policy ID")
    ),
    request_body = EditDevicePosture,
    responses(
        (status = 200, description = "Device posture check policy updated successfully", body = ApiDevicePosture),
        (status = 400, description = "Bad request - invalid field value"),
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
pub async fn update_device_posture(
    _license: EnterpriseLicenseInfo,
    _admin: AdminRole,
    session: SessionInfo,
    context: ApiRequestContext,
    Path(id): Path<Id>,
    State(appstate): State<AppState>,
    Json(data): Json<EditDevicePosture>,
) -> ApiResult {
    debug!(
        "User {} updating device posture check {id}",
        session.user.username
    );

    validate_device_posture_base(&data)?;

    let before_posture = DevicePosture::find_by_id(&appstate.pool, id)
        .await?
        .ok_or_else(|| WebError::ObjectNotFound(format!("Device posture check {id} not found")))?;
    let before_os_rules = DevicePostureOsRule::find_by_posture(&appstate.pool, id).await?;

    let EditDevicePosture {
        name,
        description,
        min_client_version,
        allow_prerelease_client,
        os_rules,
    } = data;

    let after = DevicePosture {
        id,
        name,
        description,
        min_client_version,
        allow_prerelease_client,
    };

    let mut tx = appstate.pool.begin().await?;

    after.save(&mut *tx).await?;
    DevicePostureOsRule::delete_by_posture(&mut *tx, id).await?;
    for rule in &os_rules {
        rule.clone().into_db_rule(id).save(&mut *tx).await?;
    }

    tx.commit().await?;

    let location_ids = DevicePostureLocation::find_by_posture(&appstate.pool, id).await?;

    appstate.emit_event(ApiEvent {
        context,
        event: Box::new(ApiEventType::DevicePostureUpdated {
            before: DevicePostureSnapshot {
                device_posture: before_posture,
                os_rules: before_os_rules,
                location_ids: location_ids.clone(),
            },
            after: DevicePostureSnapshot {
                device_posture: after.clone(),
                os_rules: DevicePostureOsRule::find_by_posture(&appstate.pool, id).await?,
                location_ids,
            },
        }),
    })?;

    let mut response = ApiDevicePosture::from(after);
    response.os_rules = os_rules;
    Ok(ApiResponse::json(response, StatusCode::OK))
}

/// Delete a device posture check policy
#[utoipa::path(
    delete,
    path = "/api/v1/device-posture/{id}",
    tag = "DevicePosture",
    params(
        ("id" = Id, Path, description = "Device posture check policy ID")
    ),
    responses(
        (status = 200, description = "Device posture check policy deleted successfully"),
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
pub async fn delete_device_posture(
    _license: EnterpriseLicenseInfo,
    _admin: AdminRole,
    session: SessionInfo,
    context: ApiRequestContext,
    Path(id): Path<Id>,
    State(appstate): State<AppState>,
) -> ApiResult {
    debug!(
        "User {} deleting device posture check {id}",
        session.user.username
    );

    let device_posture = DevicePosture::find_by_id(&appstate.pool, id)
        .await?
        .ok_or_else(|| WebError::ObjectNotFound(format!("Device posture check {id} not found")))?;
    let os_rules = DevicePostureOsRule::find_by_posture(&appstate.pool, id).await?;
    let location_ids = DevicePostureLocation::find_by_posture(&appstate.pool, id).await?;

    device_posture.clone().delete(&appstate.pool).await?;

    appstate.emit_event(ApiEvent {
        context,
        event: Box::new(ApiEventType::DevicePostureDeleted {
            snapshot: DevicePostureSnapshot {
                device_posture,
                os_rules,
                location_ids,
            },
        }),
    })?;

    Ok(ApiResponse::default())
}

/// Duplicate a device posture check policy
///
/// Creates a copy of the specified policy with the name `"{original} (copy)"`.
#[utoipa::path(
    post,
    path = "/api/v1/device-posture/{id}/duplicate",
    tag = "DevicePosture",
    params(
        ("id" = Id, Path, description = "Device posture check policy ID to duplicate")
    ),
    responses(
        (status = 201, description = "Duplicate created successfully", body = ApiDevicePosture),
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
pub async fn duplicate_device_posture(
    _license: EnterpriseLicenseInfo,
    _admin: AdminRole,
    session: SessionInfo,
    context: ApiRequestContext,
    Path(id): Path<Id>,
    State(appstate): State<AppState>,
) -> ApiResult {
    debug!(
        "User {} duplicating device posture check {id}",
        session.user.username
    );

    let original = DevicePosture::find_by_id(&appstate.pool, id)
        .await?
        .ok_or_else(|| WebError::ObjectNotFound(format!("Device posture check {id} not found")))?;

    let original_rules = DevicePostureOsRule::find_by_posture(&appstate.pool, id).await?;

    let mut tx = appstate.pool.begin().await?;

    let duplicate = DevicePosture {
        id: NoId,
        name: format!("{} (copy)", original.name),
        description: original.description.clone(),
        min_client_version: original.min_client_version.clone(),
        allow_prerelease_client: original.allow_prerelease_client,
    }
    .save(&mut *tx)
    .await?;

    for rule in &original_rules {
        ApiOsRule::from(rule.clone())
            .into_db_rule(duplicate.id)
            .save(&mut *tx)
            .await?;
    }

    let original_location_ids = DevicePostureLocation::find_by_posture(&appstate.pool, id).await?;

    tx.commit().await?;

    let duplicate_rules =
        DevicePostureOsRule::find_by_posture(&appstate.pool, duplicate.id).await?;

    appstate.emit_event(ApiEvent {
        context,
        event: Box::new(ApiEventType::DevicePostureDuplicated {
            original: DevicePostureSnapshot {
                device_posture: original,
                os_rules: original_rules.clone(),
                location_ids: original_location_ids,
            },
            duplicate: DevicePostureSnapshot {
                device_posture: duplicate.clone(),
                os_rules: duplicate_rules,
                location_ids: Vec::new(),
            },
        }),
    })?;

    let mut response = ApiDevicePosture::from(duplicate);
    response.os_rules = original_rules.into_iter().map(ApiOsRule::from).collect();
    Ok(ApiResponse::json(response, StatusCode::CREATED))
}

/// Request body for assigning posture checks to a VPN location.
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct AssignPosturesData {
    pub postures: Vec<Id>,
}

/// Request body for assigning VPN locations to a posture check.
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct AssignLocationsData {
    pub locations: Vec<Id>,
}

/// Assign posture checks to a VPN location (replaces existing assignment)
#[utoipa::path(
    put,
    path = "/api/v1/network/{id}/postures",
    tag = "DevicePosture",
    params(
        ("id" = Id, Path, description = "VPN location ID")
    ),
    request_body = AssignPosturesData,
    responses(
        (status = 200, description = "Postures assigned successfully", body = [Id]),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - enterprise license required"),
        (status = 404, description = "Location not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub async fn set_postures_for_location(
    _license: EnterpriseLicenseInfo,
    _admin: AdminRole,
    session: SessionInfo,
    context: ApiRequestContext,
    Path(location_id): Path<Id>,
    State(appstate): State<AppState>,
    Json(data): Json<AssignPosturesData>,
) -> ApiResult {
    debug!(
        "User {} assigning device posture checks {:?} to location {location_id}",
        session.user.username, data.postures
    );

    let location = WireguardNetwork::find_by_id(&appstate.pool, location_id)
        .await?
        .ok_or_else(|| WebError::ObjectNotFound(format!("Location {location_id} not found")))?;

    let mut tx = appstate.pool.begin().await?;
    let result =
        DevicePostureLocation::set_for_location(&mut tx, location_id, &data.postures).await?;
    tx.commit().await?;

    appstate.emit_event(ApiEvent {
        context,
        event: Box::new(ApiEventType::LocationPosturesAssigned {
            location,
            posture_ids: result.clone(),
        }),
    })?;

    Ok(ApiResponse::json(result, StatusCode::OK))
}

/// Assign VPN locations to a posture check (replaces existing assignment)
#[utoipa::path(
    put,
    path = "/api/v1/device-posture/{id}/locations",
    tag = "DevicePosture",
    params(
        ("id" = Id, Path, description = "Device posture check policy ID")
    ),
    request_body = AssignLocationsData,
    responses(
        (status = 200, description = "Locations assigned successfully", body = [Id]),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - enterprise license required"),
        (status = 404, description = "Posture check not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub async fn set_locations_for_posture(
    _license: EnterpriseLicenseInfo,
    _admin: AdminRole,
    session: SessionInfo,
    context: ApiRequestContext,
    Path(posture_id): Path<Id>,
    State(appstate): State<AppState>,
    Json(data): Json<AssignLocationsData>,
) -> ApiResult {
    debug!(
        "User {} assigning locations {:?} to device posture check {posture_id}",
        session.user.username, data.locations
    );

    let posture = DevicePosture::find_by_id(&appstate.pool, posture_id)
        .await?
        .ok_or_else(|| {
            WebError::ObjectNotFound(format!("Device posture check {posture_id} not found"))
        })?;

    let mut tx = appstate.pool.begin().await?;
    let result =
        DevicePostureLocation::set_for_posture(&mut tx, posture_id, &data.locations).await?;
    tx.commit().await?;

    appstate.emit_event(ApiEvent {
        context,
        event: Box::new(ApiEventType::DevicePostureLocationsAssigned {
            device_posture: posture,
            location_ids: result.clone(),
        }),
    })?;

    Ok(ApiResponse::json(result, StatusCode::OK))
}
///
/// Returns the available `min_os_version` values for each OS, grouped by OS type.
/// The UI should present these as selectable options in the version dropdown.
/// TODO: consider if we actually need this or if we prefer to just manually maintain a frontend list
#[utoipa::path(
    get,
    path = "/api/v1/device-posture/os-versions",
    tag = "DevicePosture",
    responses(
        (status = 200, description = "Valid OS versions grouped by OS type"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - enterprise license required"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub async fn get_device_posture_os_versions(
    _license: EnterpriseLicenseInfo,
    _admin: AdminRole,
) -> ApiResult {
    let versions = serde_json::json!({
        "windows": valid_os_versions(&OsType::Windows),
        "macos":   valid_os_versions(&OsType::Macos),
        "ios":     valid_os_versions(&OsType::Ios),
        "android": valid_os_versions(&OsType::Android),
    });
    Ok(ApiResponse::json(versions, StatusCode::OK))
}

/// List valid defguard client versions for posture rules
///
/// Returns the available `min_client_version` values.
/// The UI should present these as selectable options in the client version dropdown.
/// TODO: consider if we actually need this or if we prefer to just manually maintain a frontend list
#[utoipa::path(
    get,
    path = "/api/v1/device-posture/client-versions",
    tag = "DevicePosture",
    responses(
        (status = 200, description = "Valid client versions", body = [String]),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - enterprise license required"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub async fn get_device_posture_client_versions(
    _license: EnterpriseLicenseInfo,
    _admin: AdminRole,
) -> ApiResult {
    Ok(ApiResponse::json(CLIENT_VERSIONS, StatusCode::OK))
}
