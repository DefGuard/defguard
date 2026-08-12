use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use defguard_common::db::{
    Id,
    models::{
        Settings,
        mfa_flow::{
            LocationMfaFlowAssignment, LocationMfaFlowItem, MfaFlow, MfaFlowAssignmentError,
            MfaFlowDeleteError, MfaFlowSnapshot, MfaFlowStep, MfaFlowValidationField,
            MfaFlowWithStepCount, validate_flow_input,
        },
        vpn_client_session::VpnClientMfaMethod,
    },
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use utoipa::ToSchema;

use crate::{
    appstate::AppState,
    auth::{AdminRole, SessionInfo},
    enterprise::{
        db::models::openid_provider::OpenIdProvider, has_enterprise_access,
        is_business_license_active,
    },
    error::WebError,
    events::{ApiEvent, ApiEventType, ApiRequestContext},
    handlers::{ApiErrorResponse, ApiResponse, ApiResult},
};

/// Enriched list item returned by `GET /mfa-flow`.
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct MfaFlowListItemResponse {
    pub id: Id,
    pub title: String,
    pub step_count: i64,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<MfaFlowWithStepCount> for MfaFlowListItemResponse {
    fn from(f: MfaFlowWithStepCount) -> Self {
        Self {
            id: f.id,
            title: f.title,
            step_count: f.step_count,
            created_at: f.created_at,
            updated_at: f.updated_at,
        }
    }
}

/// Full flow detail returned by `GET /mfa-flow/{id}`, `POST`, and `PUT`.
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct MfaFlowDetailResponse {
    pub id: Id,
    pub title: String,
    pub steps: Vec<MfaFlowStepResponse>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// A single step in a flow detail response.
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct MfaFlowStepResponse {
    pub id: Id,
    pub position: i32,
    pub methods: Vec<VpnClientMfaMethod>,
}

impl From<MfaFlowStep<Id>> for MfaFlowStepResponse {
    fn from(s: MfaFlowStep<Id>) -> Self {
        Self {
            id: s.id,
            position: s.position,
            methods: s.methods,
        }
    }
}

/// Request body for creating an MFA flow.
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateMfaFlowRequest {
    pub title: String,
    pub steps: Vec<CreateMfaFlowStep>,
}

/// A step within a create request: the server derives contiguous 0-based
/// positions from array order, so `position` is accepted but ignored.
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateMfaFlowStep {
    #[serde(default)]
    pub position: i32,
    pub methods: Vec<VpnClientMfaMethod>,
}

/// Request body for updating an MFA flow.
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct UpdateMfaFlowRequest {
    pub title: String,
    pub steps: Vec<UpdateMfaFlowStep>,
}

/// A step within an update request: existing steps carry `id` for
/// reconciliation; new steps omit `id` and are INSERTed.
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct UpdateMfaFlowStep {
    #[serde(default)]
    pub id: Option<Id>,
    #[serde(default)]
    pub position: i32,
    pub methods: Vec<VpnClientMfaMethod>,
}

/// Request body for assigning flows to a location.
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct AssignMfaFlowsRequest {
    pub assignments: Vec<AssignMfaFlowEntry>,
}

/// A single entry in an assignment list.
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct AssignMfaFlowEntry {
    pub flow_id: Id,
    pub is_default: bool,
    #[serde(default)]
    pub group_ids: Vec<Id>,
}

/// Assignment item returned by `GET /location/{id}/mfa-flows`.
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct LocationMfaFlowResponse {
    pub id: Id,
    pub title: String,
    pub step_count: i64,
    pub group_names: Vec<String>,
    pub position: i32,
    pub is_default: bool,
}

impl From<LocationMfaFlowItem> for LocationMfaFlowResponse {
    fn from(item: LocationMfaFlowItem) -> Self {
        Self {
            id: item.id,
            title: item.title,
            step_count: item.step_count,
            group_names: item.group_names,
            position: item.position,
            is_default: item.is_default,
        }
    }
}

// Helpers

/// Build a `403` licence-refusal response carrying the same `fields[]` contract as validation
/// errors, so the editor can attach the message to the offending row.
///
/// The status stays `403` rather than the `400` the impl spec tabulates: a licence refusal is not
/// a malformed request, and the rest of the codebase answers licence gates with `403`. The
/// top-level `error` discriminator distinguishes it from `validation_failed`.
fn license_error_response(field: String, code: &str) -> ApiResponse {
    ApiResponse::new(
        json!({
            "error": "license_required",
            "fields": [{"field": field, "code": code}]
        }),
        StatusCode::FORBIDDEN,
    )
}

/// Check licence gates for flow create/update, returning a refusal response when the request
/// would produce over-tier state.
///
/// Gates compose additively on top of the per-method checks: multi-step needs Business, and OIDC
/// needs Business plus a configured provider.
#[must_use]
fn check_flow_license_gates(
    step_methods: &[Vec<VpnClientMfaMethod>],
    oidc_configured: bool,
) -> Option<ApiResponse> {
    if step_methods.len() > 1 && !is_business_license_active() {
        return Some(license_error_response(
            "steps".into(),
            "business_license_required",
        ));
    }

    let oidc_step = step_methods
        .iter()
        .position(|methods| methods.contains(&VpnClientMfaMethod::Oidc));
    if let Some(index) = oidc_step {
        let field = format!("steps[{index}].methods");
        if !is_business_license_active() {
            return Some(license_error_response(field, "business_license_required"));
        }
        if !oidc_configured {
            return Some(validation_error_response(vec![MfaFlowValidationField {
                field,
                code: "oidc_provider_missing".into(),
            }]));
        }
    }

    None
}

/// Check licence gates for flow assignment: group scoping requires Enterprise.
#[must_use]
fn check_assignment_license_gates(assignments: &[AssignMfaFlowEntry]) -> Option<ApiResponse> {
    let scoped = assignments.iter().position(|a| !a.group_ids.is_empty());
    if let Some(index) = scoped
        && !has_enterprise_access(None)
    {
        return Some(license_error_response(
            format!("assignments[{index}].group_ids"),
            "enterprise_license_required",
        ));
    }
    None
}

/// Build a `400` response with structured `fields[]` errors.
fn validation_error_response(errors: Vec<MfaFlowValidationField>) -> ApiResponse {
    let fields: Vec<Value> = errors
        .iter()
        .map(|e| json!({"field": e.field, "code": e.code}))
        .collect();
    ApiResponse::new(
        json!({"error": "validation_failed", "fields": fields}),
        StatusCode::BAD_REQUEST,
    )
}

/// Extract step methods from a create request.
///
/// Array order is authoritative: the server derives contiguous 0-based positions from it and
/// ignores any client-supplied `position`, which makes gaps and duplicate positions
/// unrepresentable.
fn extract_create_step_methods(steps: &[CreateMfaFlowStep]) -> Vec<Vec<VpnClientMfaMethod>> {
    steps.iter().map(|s| s.methods.clone()).collect()
}

/// Extract step updates from an update request.
///
/// Array order is authoritative, as for create. `id` is carried through so the model can
/// reconcile existing steps.
fn extract_update_step_updates(
    steps: &[UpdateMfaFlowStep],
) -> Vec<(Option<Id>, Vec<VpnClientMfaMethod>)> {
    steps.iter().map(|s| (s.id, s.methods.clone())).collect()
}

// Handlers

/// List all MFA flows
#[utoipa::path(
    get,
    path = "/api/v1/mfa-flow",
    tag = "mfa flow",
    responses(
        (status = 200, description = "List of MFA flows.", body = [MfaFlowListItemResponse]),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "access denied"})),
        (status = 500, description = "Unable to list MFA flows.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub async fn list_mfa_flows(
    _admin: AdminRole,
    session: SessionInfo,
    State(appstate): State<AppState>,
) -> ApiResult {
    debug!("User {} listing MFA flows", session.user.username);

    let items = MfaFlow::list_with_step_count(&appstate.pool).await?;
    let response: Vec<MfaFlowListItemResponse> = items.into_iter().map(Into::into).collect();

    Ok(ApiResponse::json(response, StatusCode::OK))
}

/// Create an MFA flow
#[utoipa::path(
    post,
    path = "/api/v1/mfa-flow",
    tag = "mfa flow",
    request_body = CreateMfaFlowRequest,
    responses(
        (status = 201, description = "MFA flow created.", body = MfaFlowDetailResponse),
        (status = 400, description = "Invalid request data.", body = ApiErrorResponse),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "access denied"})),
        (status = 500, description = "Unable to create MFA flow.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub async fn create_mfa_flow(
    _admin: AdminRole,
    session: SessionInfo,
    context: ApiRequestContext,
    State(appstate): State<AppState>,
    Json(data): Json<CreateMfaFlowRequest>,
) -> ApiResult {
    debug!(
        "User {} creating MFA flow {:?}",
        session.user.username, data.title
    );

    let step_methods = extract_create_step_methods(&data.steps);

    if let Some(resp) = check_flow_license_gates(
        &step_methods,
        OpenIdProvider::get_current(&appstate.pool).await?.is_some(),
    ) {
        return Ok(resp);
    }

    let errors = validate_flow_input(&data.title, &step_methods);
    if !errors.is_empty() {
        return Ok(validation_error_response(errors));
    }

    let mut tx = appstate.pool.begin().await?;
    let (flow, steps) = MfaFlow::create(&mut tx, data.title, step_methods).await?;
    tx.commit().await?;

    debug!("Created MFA flow {}", flow.id);

    appstate.emit_event(ApiEvent {
        context,
        event: Box::new(ApiEventType::MfaFlowCreated {
            snapshot: MfaFlowSnapshot {
                flow: flow.clone(),
                steps: steps.clone(),
            },
        }),
    })?;

    let response = MfaFlowDetailResponse {
        id: flow.id,
        title: flow.title,
        steps: steps.into_iter().map(Into::into).collect(),
        created_at: flow.created_at,
        updated_at: flow.updated_at,
    };

    Ok(ApiResponse::json(response, StatusCode::CREATED))
}

/// Get a single MFA flow
#[utoipa::path(
    get,
    path = "/api/v1/mfa-flow/{id}",
    tag = "mfa flow",
    params(
        ("id" = i64, Path, description = "ID of the MFA flow.")
    ),
    responses(
        (status = 200, description = "MFA flow details.", body = MfaFlowDetailResponse),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "access denied"})),
        (status = 404, description = "MFA flow not found.", body = ApiErrorResponse, example = json!({"msg": "MFA flow 1 not found"})),
        (status = 500, description = "Unable to get MFA flow.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub async fn get_mfa_flow(
    _admin: AdminRole,
    session: SessionInfo,
    Path(id): Path<Id>,
    State(appstate): State<AppState>,
) -> ApiResult {
    debug!("User {} fetching MFA flow {id}", session.user.username);

    let flow = MfaFlow::find_by_id(&appstate.pool, id)
        .await?
        .ok_or_else(|| WebError::ObjectNotFound(format!("MFA flow {id} not found")))?;
    let steps = MfaFlowStep::find_by_flow(&appstate.pool, id).await?;

    let response = MfaFlowDetailResponse {
        id: flow.id,
        title: flow.title,
        steps: steps.into_iter().map(Into::into).collect(),
        created_at: flow.created_at,
        updated_at: flow.updated_at,
    };

    Ok(ApiResponse::json(response, StatusCode::OK))
}

/// Update an MFA flow
#[utoipa::path(
    put,
    path = "/api/v1/mfa-flow/{id}",
    tag = "mfa flow",
    params(
        ("id" = i64, Path, description = "ID of the MFA flow.")
    ),
    request_body = UpdateMfaFlowRequest,
    responses(
        (status = 200, description = "MFA flow updated.", body = MfaFlowDetailResponse),
        (status = 400, description = "Invalid request data.", body = ApiErrorResponse),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "access denied"})),
        (status = 404, description = "MFA flow not found.", body = ApiErrorResponse, example = json!({"msg": "MFA flow 1 not found"})),
        (status = 500, description = "Unable to update MFA flow.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub async fn update_mfa_flow(
    _admin: AdminRole,
    session: SessionInfo,
    context: ApiRequestContext,
    Path(id): Path<Id>,
    State(appstate): State<AppState>,
    Json(data): Json<UpdateMfaFlowRequest>,
) -> ApiResult {
    debug!("User {} updating MFA flow {id}", session.user.username);

    // Ensure the flow exists
    let existing = MfaFlow::find_by_id(&appstate.pool, id)
        .await?
        .ok_or_else(|| WebError::ObjectNotFound(format!("MFA flow {id} not found")))?;
    let before_steps = MfaFlowStep::find_by_flow(&appstate.pool, id).await?;

    let step_updates = extract_update_step_updates(&data.steps);
    let step_methods: Vec<Vec<VpnClientMfaMethod>> =
        data.steps.iter().map(|s| s.methods.clone()).collect();

    if let Some(resp) = check_flow_license_gates(
        &step_methods,
        OpenIdProvider::get_current(&appstate.pool).await?.is_some(),
    ) {
        return Ok(resp);
    }

    let errors = validate_flow_input(&data.title, &step_methods);
    if !errors.is_empty() {
        return Ok(validation_error_response(errors));
    }

    let mut tx = appstate.pool.begin().await?;
    let (flow, steps) =
        MfaFlow::update_with_steps(&mut tx, existing.id, data.title, step_updates).await?;
    tx.commit().await?;

    appstate.emit_event(ApiEvent {
        context,
        event: Box::new(ApiEventType::MfaFlowUpdated {
            before: MfaFlowSnapshot {
                flow: existing,
                steps: before_steps,
            },
            after: MfaFlowSnapshot {
                flow: flow.clone(),
                steps: steps.clone(),
            },
        }),
    })?;

    let response = MfaFlowDetailResponse {
        id: flow.id,
        title: flow.title,
        steps: steps.into_iter().map(Into::into).collect(),
        created_at: flow.created_at,
        updated_at: flow.updated_at,
    };

    Ok(ApiResponse::json(response, StatusCode::OK))
}

/// Delete an MFA flow
#[utoipa::path(
    delete,
    path = "/api/v1/mfa-flow/{id}",
    tag = "mfa flow",
    params(
        ("id" = i64, Path, description = "ID of the MFA flow.")
    ),
    responses(
        (status = 200, description = "MFA flow deleted."),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "access denied"})),
        (status = 404, description = "MFA flow not found.", body = ApiErrorResponse, example = json!({"msg": "MFA flow 1 not found"})),
        (status = 409, description = "The flow is still load-bearing for at least one location: `location_requires_flow` when deleting it would leave an MFA-enabled location with no flows, `flow_is_default` when it is a location's designated default.", body = ApiErrorResponse, example = json!({"error": "conflict", "fields": [{"field": "id", "code": "flow_is_default", "locations": ["Warsaw Office"]}]})),
        (status = 500, description = "Unable to delete MFA flow.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub async fn delete_mfa_flow(
    _admin: AdminRole,
    session: SessionInfo,
    context: ApiRequestContext,
    Path(id): Path<Id>,
    State(appstate): State<AppState>,
) -> ApiResult {
    debug!("User {} deleting MFA flow {id}", session.user.username);

    let flow = MfaFlow::find_by_id(&appstate.pool, id)
        .await?
        .ok_or_else(|| WebError::ObjectNotFound(format!("MFA flow {id} not found")))?;

    // Both refusals are 409 Conflict: the request is well formed, but the flow is load-bearing
    // for at least one location. The two codes are deliberately distinct.
    if let Err(e) = MfaFlow::check_deletable(&appstate.pool, id).await {
        let (code, locations) = match e {
            MfaFlowDeleteError::LocationRequiresFlow(locations) => {
                ("location_requires_flow", locations)
            }
            MfaFlowDeleteError::FlowIsDefault(locations) => ("flow_is_default", locations),
            MfaFlowDeleteError::Sqlx(e) => return Err(WebError::from(e)),
        };

        return Ok(ApiResponse::new(
            json!({
                "error": "conflict",
                "fields": [{
                    "field": "id",
                    "code": code,
                    "locations": locations,
                }]
            }),
            StatusCode::CONFLICT,
        ));
    }

    let steps = MfaFlowStep::find_by_flow(&appstate.pool, id).await?;

    let snapshot = MfaFlowSnapshot {
        flow: flow.clone(),
        steps,
    };

    flow.delete(&appstate.pool).await?;

    debug!("Deleted MFA flow {id}");

    appstate.emit_event(ApiEvent {
        context,
        event: Box::new(ApiEventType::MfaFlowDeleted { snapshot }),
    })?;

    Ok(ApiResponse::default())
}

/// Get MFA flows assigned to a location
#[utoipa::path(
    get,
    path = "/api/v1/location/{id}/mfa-flows",
    tag = "mfa flow",
    params(
        ("id" = i64, Path, description = "ID of the location.")
    ),
    responses(
        (status = 200, description = "MFA flows assigned to the location.", body = [LocationMfaFlowResponse]),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse),
        (status = 500, description = "Unable to list assigned flows.", body = ApiErrorResponse)
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub async fn get_location_mfa_flows(
    _admin: AdminRole,
    session: SessionInfo,
    Path(id): Path<Id>,
    State(appstate): State<AppState>,
) -> ApiResult {
    debug!(
        "User {} getting MFA flows for location {id}",
        session.user.username
    );

    let items = MfaFlow::for_location(&appstate.pool, id).await?;
    let response: Vec<LocationMfaFlowResponse> = items.into_iter().map(Into::into).collect();

    Ok(ApiResponse::json(response, StatusCode::OK))
}

/// Assign MFA flows to a location (full replace)
#[utoipa::path(
    put,
    path = "/api/v1/location/{id}/mfa-flows",
    tag = "mfa flow",
    params(
        ("id" = i64, Path, description = "ID of the location.")
    ),
    request_body = AssignMfaFlowsRequest,
    responses(
        (status = 200, description = "MFA flows assigned to the location.", body = [LocationMfaFlowResponse]),
        (status = 400, description = "Invalid assignment: `no_default_designated`, `multiple_defaults_designated`, or `default_must_have_no_groups`.", body = ApiErrorResponse, example = json!({"error": "validation_failed", "fields": [{"field": "mfa_flows", "code": "no_default_designated"}]})),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse),
        (status = 403, description = "Requires admin privileges, or group scoping without an enterprise license (`enterprise_license_required`).", body = ApiErrorResponse, example = json!({"error": "license_required", "fields": [{"field": "assignments[0].group_ids", "code": "enterprise_license_required"}]})),
        (status = 500, description = "Unable to assign flows.", body = ApiErrorResponse)
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub async fn set_location_mfa_flows(
    _admin: AdminRole,
    session: SessionInfo,
    context: ApiRequestContext,
    Path(location_id): Path<Id>,
    State(appstate): State<AppState>,
    Json(data): Json<AssignMfaFlowsRequest>,
) -> ApiResult {
    debug!(
        "User {} assigning MFA flows to location {location_id}",
        session.user.username
    );

    let assignments: Vec<LocationMfaFlowAssignment> = data
        .assignments
        .iter()
        .map(|a| LocationMfaFlowAssignment {
            flow_id: a.flow_id,
            is_default: a.is_default,
            group_ids: a.group_ids.clone(),
        })
        .collect();

    if let Some(resp) = check_assignment_license_gates(&data.assignments) {
        return Ok(resp);
    }

    let mut tx = appstate.pool.begin().await?;
    if let Err(e) = MfaFlow::assign_to_location(&mut tx, location_id, &assignments).await {
        let (field, code) = match e {
            MfaFlowAssignmentError::NoDefaultDesignated => ("mfa_flows", "no_default_designated"),
            MfaFlowAssignmentError::MultipleDefaultsDesignated => {
                ("mfa_flows", "multiple_defaults_designated")
            }
            MfaFlowAssignmentError::DefaultHasGroups => {
                ("mfa_flows", "default_must_have_no_groups")
            }
            MfaFlowAssignmentError::Sqlx(e) => return Err(WebError::from(e)),
        };

        return Ok(validation_error_response(vec![MfaFlowValidationField {
            field: field.into(),
            code: code.into(),
        }]));
    }
    tx.commit().await?;

    let items = MfaFlow::for_location(&appstate.pool, location_id).await?;
    let response: Vec<LocationMfaFlowResponse> = items.into_iter().map(Into::into).collect();

    appstate.emit_event(ApiEvent {
        context,
        event: Box::new(ApiEventType::LocationMfaFlowsAssigned {
            location_id,
            location_name: String::new(), // populated by event logger
            assignment_count: response.len() as i64,
        }),
    })?;

    Ok(ApiResponse::json(response, StatusCode::OK))
}

/// Method availability entry returned by the catalogue endpoint.
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct MethodAvailabilityResponse {
    pub method: VpnClientMfaMethod,
    pub available: bool,
    pub reason: MethodAvailabilityReason,
}

/// Reason a method is (un)available.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum MethodAvailabilityReason {
    /// Method is usable.
    Available,
    /// A higher-tier license is required.
    Licensed,
    /// SMTP must be configured first.
    SmtpNotConfigured,
    /// An OpenID provider must be configured first.
    OidcProviderMissing,
}

/// Compute per-method availability for the MFA flow editor.
///
/// Checks license tier, SMTP configuration, and OIDC provider presence to
/// determine which methods are currently usable. All five methods in
/// [`VpnClientMfaMethod`] are always enumerated; unavailable methods carry
/// a `reason` that the UI maps to an appropriate CTA.
fn compute_method_availability(
    smtp_configured: bool,
    oidc_configured: bool,
) -> Vec<MethodAvailabilityResponse> {
    let has_business = is_business_license_active();

    let methods = [
        (
            VpnClientMfaMethod::Totp,
            true,
            MethodAvailabilityReason::Available,
        ),
        (
            VpnClientMfaMethod::Email,
            smtp_configured,
            if smtp_configured {
                MethodAvailabilityReason::Available
            } else {
                MethodAvailabilityReason::SmtpNotConfigured
            },
        ),
        (
            VpnClientMfaMethod::Oidc,
            has_business && oidc_configured,
            if !has_business {
                MethodAvailabilityReason::Licensed
            } else if !oidc_configured {
                MethodAvailabilityReason::OidcProviderMissing
            } else {
                MethodAvailabilityReason::Available
            },
        ),
        (
            VpnClientMfaMethod::Biometric,
            true,
            MethodAvailabilityReason::Available,
        ),
        (
            VpnClientMfaMethod::MobileApprove,
            true,
            MethodAvailabilityReason::Available,
        ),
    ];

    methods
        .into_iter()
        .map(|(method, available, reason)| MethodAvailabilityResponse {
            method,
            available,
            reason,
        })
        .collect()
}

/// Get per-method MFA availability.
#[utoipa::path(
    get,
    path = "/api/v1/mfa-flow/method-availability",
    tag = "mfa flow",
    responses(
        (status = 200, description = "Per-method availability catalogue.", body = [MethodAvailabilityResponse]),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse),
        (status = 500, description = "Unable to compute method availability.", body = ApiErrorResponse)
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub async fn get_method_availability(
    _admin: AdminRole,
    session: SessionInfo,
    State(appstate): State<AppState>,
) -> ApiResult {
    debug!(
        "User {} fetching MFA method availability",
        session.user.username
    );
    let smtp_configured = Settings::get_current_settings().smtp_configured();
    let oidc_configured = OpenIdProvider::get_current(&appstate.pool).await?.is_some();
    let result = compute_method_availability(smtp_configured, oidc_configured);
    Ok(ApiResponse::json(result, StatusCode::OK))
}
