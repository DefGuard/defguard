use axum::{Json, extract::State, http::StatusCode};
use defguard_common::db::{Id, NoId};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    appstate::AppState,
    auth::{AdminRole, SessionInfo},
    enterprise::{db::models::device_posture::DevicePosture, handlers::EnterpriseLicenseInfo},
    error::WebError,
    events::{ApiEvent, ApiEventType, ApiRequestContext},
    handlers::{ApiResponse, ApiResult},
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
