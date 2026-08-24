use std::collections::HashSet;

use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
};
use defguard_common::{
    csv::AsCsv,
    db::{
        Id,
        models::{
            Device, DeviceConfig, DeviceType, User, WireguardNetwork,
            device::{AddDevice, DeviceInfo, ModifyDevice, WireguardNetworkDevice},
            mfa_flow::{LocationMfaFlowAssignment, MfaFlow, MfaFlowStep},
            wireguard::{MappedDevice, ServiceLocationMode},
        },
    },
    utils::parse_network_address_list,
};
use ipnetwork::IpNetwork;
use serde_json::{Value, json};
use sqlx::{PgConnection, PgPool};
use thiserror::Error;
use utoipa::ToSchema;

use super::{
    ApiErrorResponse, ApiResponse, ApiResult, WebError, device_for_admin_or_self,
    user_for_admin_or_self,
};
use crate::{
    appstate::AppState,
    auth::{AdminRole, SessionInfo},
    device_access::{build_device_config, join_device_to_all_networks},
    enterprise::{
        db::models::{
            device_posture::DevicePostureLocation, enterprise_settings::EnterpriseSettings,
        },
        firewall::try_get_location_firewall_config,
        handlers::CanManageDevices,
        has_enterprise_access, is_business_license_active,
        license::{LicenseFeature, get_cached_license},
        limits::{get_counts, update_counts},
    },
    events::{ApiEvent, ApiEventType, ApiRequestContext},
    grpc::GatewayCommand,
    handlers::{
        gateway::GatewayInfo,
        mfa_flow::{assignment_error_response, license_error_response},
        network_devices::DeviceWireGuardConfig,
    },
    location_management::{
        allowed_peers::get_location_allowed_peers, handle_imported_devices, handle_mapped_devices,
        sync_location_allowed_devices,
    },
    mail::templates::{TemplateLocation, new_device_added_mail},
    wg_config::{ImportedDevice, parse_wireguard_config},
};

#[derive(Serialize, ToSchema)]
pub(crate) struct WireguardNetworkInfo {
    #[serde(flatten)]
    network: WireguardNetwork<Id>,
    gateways: Vec<GatewayInfo>,
    allowed_groups: Vec<String>,
    has_devices: bool,
    posture_checks: Vec<Id>,
}

#[derive(Serialize, ToSchema)]
pub(crate) struct LocationsCount {
    count: usize,
}

#[derive(Clone, Deserialize, Serialize, ToSchema)]
pub struct WireguardNetworkData {
    pub name: String,
    pub address: String, // comma-separated list of addresses
    pub endpoint: String,
    pub port: i32,
    pub allowed_ips: Option<String>,
    pub dns: Option<String>,
    pub mtu: i32,
    pub fwmark: i64,
    pub allow_all_groups: bool,
    pub allowed_groups: Vec<String>,
    pub keepalive_interval: i32,
    pub peer_disconnect_threshold: i32,
    pub acl_enabled: bool,
    pub acl_default_allow: bool,
    #[serde(default)]
    pub allowed_ips_from_acl: bool,
    pub mfa_enabled: bool,
    pub service_location_mode: ServiceLocationMode,
    pub posture_checks: Vec<Id>,
    pub mfa_flows: Vec<LocationMfaFlowAssignment>,
}

const MIN_PEER_DISCONNECT_THRESHOLD_WITH_MFA: i32 = 120;

/// Build the structured `400` response for the `mfa_enabled` precondition: a location cannot be
/// MFA-enabled while no MFA flow exists to assign to it.
///
/// The body is a real structured `validation_failed` payload, not a string inside `msg`, so the
/// frontend parses it in one step like every other validation path in this feature.
#[must_use]
pub fn no_flows_exist_response() -> ApiResponse {
    ApiResponse::new(
        json!({
            "error": "validation_failed",
            "fields": [{"field": "mfa_enabled", "code": "no_flows_exist"}]
        }),
        StatusCode::BAD_REQUEST,
    )
}

/// Build the structured `400` response for the `mfa_enabled` precondition when the location has no
/// default flow assigned: MFA cannot be enabled until a policy exists to enforce.
#[must_use]
pub fn no_flows_assigned_response() -> ApiResponse {
    ApiResponse::new(
        json!({
            "error": "validation_failed",
            "fields": [{"field": "mfa_enabled", "code": "no_flows_assigned"}]
        }),
        StatusCode::BAD_REQUEST,
    )
}

/// Validate enabling MFA on an already-persisted location.
///
/// Used by the auto-adoption wizard after it creates a location. The normal location create and
/// update handlers validate assignments inside their transactions. Returns a structured `400`
/// response (not a `WebError`) when no MFA flow or no default assignment exists.
pub async fn validate_mfa_flows_exist<'e, E: sqlx::PgExecutor<'e> + Copy>(
    executor: E,
    mfa_enabled: bool,
    location_id: Option<Id>,
) -> Result<Option<ApiResponse>, WebError> {
    if !mfa_enabled {
        return Ok(None);
    }

    if !MfaFlow::any_exist(executor).await? {
        error!("Unable to enable MFA for location: no MFA flows are configured");
        return Ok(Some(no_flows_exist_response()));
    }

    let has_default = match location_id {
        Some(id) => MfaFlow::has_default_assignment(executor, id).await?,
        None => false,
    };
    if !has_default {
        error!("Unable to enable MFA for location: no default MFA flow is assigned");
        return Ok(Some(no_flows_assigned_response()));
    }

    Ok(None)
}

impl WireguardNetworkData {
    pub(crate) fn parse_allowed_ips(&self) -> Vec<IpNetwork> {
        self.allowed_ips
            .as_ref()
            .map_or(Vec::new(), |ips| parse_network_address_list(ips))
    }

    pub(crate) fn validate_peer_disconnect_threshold(&self) -> Result<(), WebError> {
        if !self.mfa_enabled {
            return Ok(());
        }

        if self.peer_disconnect_threshold >= MIN_PEER_DISCONNECT_THRESHOLD_WITH_MFA {
            return Ok(());
        }

        Err(WebError::BadRequest(format!(
            "peer_disconnect_threshold must be at least {MIN_PEER_DISCONNECT_THRESHOLD_WITH_MFA} when location MFA is enabled"
        )))
    }

    /// Rejects service-location mode combined with location MFA: core cannot serve it and the
    /// client cannot represent it (`Location::is_service_location()` requires MFA disabled).
    pub(crate) fn validate_service_location_mfa(&self) -> Result<(), WebError> {
        if self.service_location_mode == ServiceLocationMode::Disabled || !self.mfa_enabled {
            return Ok(());
        }

        Err(WebError::BadRequest(
            "Service location mode cannot be combined with location MFA".into(),
        ))
    }

    /// Rejects a zero (or negative) keepalive interval to prevent idle service locations
    /// from disconnecting.
    pub(crate) fn validate_keepalive_interval(&self) -> Result<(), WebError> {
        if self.keepalive_interval >= 1 {
            return Ok(());
        }

        Err(WebError::BadRequest(
            "keepalive_interval must be at least 1".into(),
        ))
    }

    pub(crate) fn validate_allowed_groups(&self) -> Result<(), WebError> {
        if self.allow_all_groups || !self.allowed_groups.is_empty() {
            return Ok(());
        }
        Err(WebError::BadRequest(
            "At least one group must be specified when allow_all_groups is disabled".into(),
        ))
    }
}

// Used in process of importing network from WireGuard config.
#[derive(Deserialize, ToSchema)]
pub(crate) struct MappedDevices {
    devices: Vec<MappedDevice>,
}

#[derive(Deserialize, ToSchema)]
pub(crate) struct ImportNetworkData {
    name: String,
    endpoint: String,
    config: String,
    allow_all_groups: bool,
    allowed_groups: Vec<String>,
}

#[derive(Deserialize, Serialize, ToSchema)]
pub struct ImportedNetworkData {
    pub network: WireguardNetwork<Id>,
    pub devices: Vec<ImportedDevice>,
}

#[derive(Debug, Error)]
enum MfaFlowAssignmentError {
    #[error("MFA flow group assignments require an Enterprise license")]
    GroupAssignmentNotAllowed,
    #[error("Multi-step MFA flows require a Business license")]
    MultipleStepsNotAllowed,
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

/// Validates whether the current license permits the requested MFA flow assignments.
async fn validate_mfa_flow_assignments(
    conn: &mut PgConnection,
    assignments: &[LocationMfaFlowAssignment],
) -> Result<(), MfaFlowAssignmentError> {
    // Enterprise can make all assignments.
    if has_enterprise_access(None) {
        return Ok(());
    }

    // Business and Free can't assign groups.
    if assignments.iter().any(|a| !a.group_ids.is_empty()) {
        return Err(MfaFlowAssignmentError::GroupAssignmentNotAllowed);
    }

    // Business can assign multiple and multi-step flows.
    if is_business_license_active() {
        return Ok(());
    }

    // Free can't assign multi-step flows.
    if let Some(assignment) = assignments.first() {
        let steps = MfaFlowStep::find_by_flow(&mut *conn, assignment.flow_id).await?;
        if steps.len() > 1 {
            return Err(MfaFlowAssignmentError::MultipleStepsNotAllowed);
        }
    }

    Ok(())
}

fn mfa_flow_assignment_validation_error_response(
    error: MfaFlowAssignmentError,
) -> Result<ApiResponse, WebError> {
    let code = match error {
        MfaFlowAssignmentError::GroupAssignmentNotAllowed => "group_assignment_not_allowed",
        MfaFlowAssignmentError::MultipleStepsNotAllowed => "multiple_steps_not_allowed",
        MfaFlowAssignmentError::Database(error) => return Err(WebError::from(error)),
    };
    Ok(license_error_response("mfa_flows".into(), code))
}

/// Create a network
#[utoipa::path(
    post,
    path = "/api/v1/network",
    tag = "network",
    request_body(content = WireguardNetworkData, description = "`address` is a comma-separated list of network addresses.", example = json!({"name": "office", "address": "10.0.0.1/24", "endpoint": "vpn.example.com", "port": 50051, "allowed_ips": "0.0.0.0/0", "dns": "1.1.1.1", "mtu": 1420, "fwmark": 0, "allow_all_groups": true, "allowed_groups": [], "keepalive_interval": 25, "peer_disconnect_threshold": 180, "acl_enabled": false, "acl_default_allow": false, "allowed_ips_from_acl": false, "mfa_enabled": false, "service_location_mode": "disabled"})),
    responses(
        (status = 201, description = "Network created.", body = WireguardNetwork),
        (status = 400, description = "Invalid location settings.", body = ApiErrorResponse, example = json!({"msg": "At least one group must be specified when allow_all_groups is disabled"})),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "access denied"})),
        (status = 500, description = "Unable to create network.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn create_network(
    _role: AdminRole,
    State(appstate): State<AppState>,
    session: SessionInfo,
    context: ApiRequestContext,
    Json(data): Json<WireguardNetworkData>,
) -> ApiResult {
    let network_name = data.name.clone();
    debug!(
        "User {} creating WireGuard network {network_name}",
        session.user.username
    );

    // check if adding new network will go over license limits
    let location_count = get_counts().location();

    if get_cached_license()
        .as_ref()
        .and_then(|l| l.limits.as_ref())
        .is_some_and(|l| location_count >= l.locations)
    {
        error!("Adding location {network_name} blocked! License limit reached.");
        return Ok(WebError::Forbidden("License limit reached").into());
    }

    // check if tries to add service location without active enterprise
    if data.service_location_mode != ServiceLocationMode::Disabled
        && !has_enterprise_access(Some(LicenseFeature::ServiceLocations))
    {
        error!("Adding location {network_name} blocked! Enterprise license required.");
        return Ok(ApiResponse {
            json: json!({
                "msg": "Enterprise license required.",
            }),
            status: StatusCode::FORBIDDEN,
        });
    }

    data.validate_peer_disconnect_threshold()?;
    data.validate_service_location_mfa()?;
    data.validate_keepalive_interval()?;
    data.validate_allowed_groups()?;

    let allowed_ips = data.parse_allowed_ips();
    let mut network = WireguardNetwork::new(
        data.name,
        data.port,
        data.endpoint,
        data.dns,
        allowed_ips,
        data.allow_all_groups,
        data.acl_enabled,
        data.acl_default_allow,
        data.allowed_ips_from_acl,
        data.mfa_enabled,
        data.service_location_mode,
    )
    .try_set_address(&data.address)?;
    network.mtu = data.mtu;
    network.fwmark = data.fwmark;
    network.keepalive_interval = data.keepalive_interval;
    network.peer_disconnect_threshold = data.peer_disconnect_threshold;

    let mut transaction = appstate.pool.begin().await?;
    let network = network.save(&mut *transaction).await?;
    network
        .set_allowed_groups(&mut transaction, &data.allowed_groups)
        .await?;

    // generate IP addresses for existing devices
    network.add_all_allowed_devices(&mut transaction).await?;
    info!("Assigning IPs for existing devices in network {network}");

    debug!(
        "Assigning posture checks {:?} to {network}",
        data.posture_checks
    );
    if !has_enterprise_access(Some(LicenseFeature::DevicePosture))
        && !data.posture_checks.is_empty()
    {
        error!(
            "Cannot assign posture checks to new location {network}: Enterprise license required."
        );
        return Ok(WebError::Forbidden(
            "Cannot assign posture checks to new location: Enterprise license required.",
        )
        .into());
    }
    DevicePostureLocation::set_for_location(&mut transaction, network.id, &data.posture_checks)
        .await?;
    info!(
        "Assigned posture checks {:?} to new location {network}",
        data.posture_checks
    );

    let mfa_assignments: Vec<LocationMfaFlowAssignment> = data.mfa_flows.clone();
    if let Err(error) = validate_mfa_flow_assignments(&mut transaction, &mfa_assignments).await {
        return mfa_flow_assignment_validation_error_response(error);
    }
    if let Err(error) =
        MfaFlow::assign_to_location(&mut transaction, network.id, &mfa_assignments).await
    {
        return assignment_error_response(&data.mfa_flows, error);
    }
    transaction.commit().await?;

    appstate.send_gateway_command(GatewayCommand::NetworkCreated(network.id, network.clone()));

    info!(
        "User {} created WireGuard network {network_name}",
        session.user.username
    );

    if !data.posture_checks.is_empty() {
        appstate.emit_event(ApiEvent {
            context: context.clone(),
            event: Box::new(ApiEventType::LocationPosturesAssigned {
                location: network.clone(),
                posture_ids: data.posture_checks.clone(),
            }),
        })?;
    }
    if !mfa_assignments.is_empty() {
        appstate.emit_event(ApiEvent {
            context: context.clone(),
            event: Box::new(ApiEventType::LocationMfaFlowsAssigned {
                location_id: network.id,
                location_name: network.name.clone(),
                assignments: LocationMfaFlowAssignment::snapshot(&mfa_assignments),
            }),
        })?;
    }
    appstate.emit_event(ApiEvent {
        context,
        event: Box::new(ApiEventType::VpnLocationAdded {
            location: network.clone(),
        }),
    })?;
    update_counts(&appstate.pool).await?;

    Ok(ApiResponse::json(network, StatusCode::CREATED))
}

async fn find_network(id: Id, pool: &PgPool) -> Result<WireguardNetwork<Id>, WebError> {
    WireguardNetwork::find_by_id(pool, id)
        .await?
        .ok_or_else(|| WebError::ObjectNotFound(format!("Network {id} not found")))
}

/// Update a network
#[utoipa::path(
    put,
    path = "/api/v1/network/{network_id}",
    tag = "network",
    params(
        ("network_id" = i64, Path, description = "ID of the network."),
    ),
    request_body = WireguardNetworkData,
    responses(
        (status = 200, description = "Network updated.", body = WireguardNetwork),
        (status = 400, description = "Invalid location settings.", body = ApiErrorResponse, example = json!({"msg": "Enterprise license required."})),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "access denied"})),
        (status = 404, description = "Network not found.", body = ApiErrorResponse, example = json!({"msg": "network not found"})),
        (status = 500, description = "Unable to update network.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn modify_network(
    _role: AdminRole,
    Path(network_id): Path<Id>,
    State(appstate): State<AppState>,
    session: SessionInfo,
    context: ApiRequestContext,
    Json(data): Json<WireguardNetworkData>,
) -> ApiResult {
    debug!(
        "User {} updating WireGuard network {network_id}",
        session.user.username
    );

    // check if tries to configure service location without active enterprise
    if data.service_location_mode != ServiceLocationMode::Disabled
        && !has_enterprise_access(Some(LicenseFeature::ServiceLocations))
    {
        let name = data.name;
        error!("Modification of location {name} blocked! Enterprise license required.");
        return Ok(ApiResponse {
            json: json!({
                "msg": "Enterprise license required.",
            }),
            status: StatusCode::BAD_REQUEST,
        });
    }

    data.validate_peer_disconnect_threshold()?;
    data.validate_service_location_mfa()?;
    data.validate_keepalive_interval()?;
    data.validate_allowed_groups()?;

    let network = find_network(network_id, &appstate.pool).await?;
    // store network before mods
    let before = network.clone();
    let mut network = network.try_set_address(&data.address)?;
    network.allowed_ips = data.parse_allowed_ips();
    network.name = data.name;

    // initialize DB transaction
    let mut transaction = appstate.pool.begin().await?;

    network.endpoint = data.endpoint;
    network.port = data.port;
    network.dns = data.dns;
    network.keepalive_interval = data.keepalive_interval;
    network.mtu = data.mtu;
    network.fwmark = data.fwmark;
    network.peer_disconnect_threshold = data.peer_disconnect_threshold;
    network.allow_all_groups = data.allow_all_groups;
    network.acl_enabled = data.acl_enabled;
    network.acl_default_allow = data.acl_default_allow;
    network.allowed_ips_from_acl = data.allowed_ips_from_acl;
    network.service_location_mode = data.service_location_mode;
    network.mfa_enabled = data.mfa_enabled;

    network.save(&mut *transaction).await?;
    network
        .set_allowed_groups(&mut transaction, &data.allowed_groups)
        .await?;

    // Don't error out on no license - otherwise users won't be able to update other location fields.
    let postures_changed = if has_enterprise_access(Some(LicenseFeature::DevicePosture)) {
        let mut current_postures =
            DevicePostureLocation::find_by_location(&mut *transaction, network.id).await?;
        let mut requested_postures = data.posture_checks.clone();

        current_postures.sort_unstable();
        requested_postures.sort_unstable();

        if current_postures != requested_postures {
            DevicePostureLocation::set_for_location(
                &mut transaction,
                network.id,
                &data.posture_checks,
            )
            .await?;
        }
        current_postures != requested_postures
    } else {
        warn!(
            location_id = network.id,
            "Ignoring posture check assignments because the Enterprise license is inactive"
        );
        false
    };

    let update_mfa_assignments = is_business_license_active();
    let mfa_assignments: Vec<LocationMfaFlowAssignment> = if update_mfa_assignments {
        if let Err(error) =
            MfaFlow::assign_to_location(&mut transaction, network.id, &data.mfa_flows).await
        {
            return assignment_error_response(&data.mfa_flows, error);
        }
        data.mfa_flows.clone()
    } else {
        warn!(
            location_id = network.id,
            "Ignoring MFA flow assignments because the paid license is inactive"
        );
        if let Some(response) =
            validate_mfa_flows_exist(&appstate.pool, data.mfa_enabled, Some(network.id)).await?
        {
            return Ok(response);
        }
        Vec::new()
    };

    let _events = sync_location_allowed_devices(&network, &mut transaction, None).await?;

    let peers = get_location_allowed_peers(&network, &mut transaction).await?;
    let maybe_firewall_config =
        try_get_location_firewall_config(&network, &mut transaction).await?;
    let gateway_command =
        GatewayCommand::NetworkModified(network.id, network.clone(), peers, maybe_firewall_config);

    // commit DB transaction
    transaction.commit().await?;
    appstate.send_gateway_command(gateway_command);

    info!(
        "User {} updated WireGuard network {network_id}",
        session.user.username,
    );
    if postures_changed {
        appstate.emit_event(ApiEvent {
            context: context.clone(),
            event: Box::new(ApiEventType::LocationPosturesAssigned {
                location: network.clone(),
                posture_ids: data.posture_checks.clone(),
            }),
        })?;
    }
    // TODO: also check new-old assignments equality before emitting
    if update_mfa_assignments {
        appstate.emit_event(ApiEvent {
            context: context.clone(),
            event: Box::new(ApiEventType::LocationMfaFlowsAssigned {
                location_id: network.id,
                location_name: network.name.clone(),
                assignments: LocationMfaFlowAssignment::snapshot(&mfa_assignments),
            }),
        })?;
    }
    appstate.emit_event(ApiEvent {
        context,
        event: Box::new(ApiEventType::VpnLocationModified {
            before,
            after: network.clone(),
        }),
    })?;
    Ok(ApiResponse::json(network, StatusCode::OK))
}

/// Delete a network
#[utoipa::path(
    delete,
    path = "/api/v1/network/{network_id}",
    tag = "network",
    params(
        ("network_id" = i64, Path, description = "ID of the network."),
    ),
    responses(
        (status = 200, description = "Network deleted."),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "access denied"})),
        (status = 404, description = "Network not found.", body = ApiErrorResponse, example = json!({"msg": "network not found"})),
        (status = 500, description = "Unable to delete network.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn delete_network(
    _role: AdminRole,
    Path(network_id): Path<Id>,
    State(appstate): State<AppState>,
    session: SessionInfo,
    context: ApiRequestContext,
) -> ApiResult {
    debug!(
        "User {} deleting WireGuard network {network_id}",
        session.user.username,
    );
    let network = find_network(network_id, &appstate.pool).await?;
    let network_name = network.name.clone();
    let mut transaction = appstate.pool.begin().await?;
    let network_devices = network
        .get_devices_by_type(&mut *transaction, DeviceType::Network)
        .await?;
    for device in network_devices {
        device.delete(&mut *transaction).await?;
    }
    network.clone().delete(&mut *transaction).await?;
    transaction.commit().await?;
    appstate.send_gateway_command(GatewayCommand::NetworkDeleted(network_id, network_name));
    info!(
        "User {} deleted WireGuard network {network_id}",
        session.user.username,
    );
    appstate.emit_event(ApiEvent {
        context,
        event: Box::new(ApiEventType::VpnLocationRemoved { location: network }),
    })?;
    update_counts(&appstate.pool).await?;

    Ok(ApiResponse::default())
}

/// List networks
#[utoipa::path(
    get,
    path = "/api/v1/network",
    tag = "network",
    responses(
        (status = 200, description = "All networks.", body = [WireguardNetworkInfo]),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "access denied"})),
        (status = 500, description = "Unable to list networks.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub async fn list_networks(_role: AdminRole, State(appstate): State<AppState>) -> ApiResult {
    debug!("Listing WireGuard networks");
    let mut network_info = Vec::new();
    let networks = WireguardNetwork::all(&appstate.pool).await?;

    for network in networks {
        let allowed_groups = network.fetch_allowed_groups(&appstate.pool).await?;
        let gateways = GatewayInfo::find_by_location_id(&appstate.pool, network.id).await?;
        let has_devices =
            WireguardNetworkDevice::has_devices_in_network(&appstate.pool, network.id).await?;
        let posture_checks =
            DevicePostureLocation::find_by_location(&appstate.pool, network.id).await?;
        network_info.push(WireguardNetworkInfo {
            network,
            gateways,
            allowed_groups,
            has_devices,
            posture_checks,
        });
    }
    network_info.sort_by(|a, b| a.network.name.cmp(&b.network.name));

    debug!("Listed WireGuard networks");

    Ok(ApiResponse::json(network_info, StatusCode::OK))
}

/// Count networks
#[utoipa::path(
    get,
    path = "/api/v1/network/count",
    tag = "network",
    responses(
        (status = 200, description = "Number of networks.", body = LocationsCount),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "access denied"})),
        (status = 500, description = "Unable to count networks.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub async fn count_networks(_role: AdminRole, State(appstate): State<AppState>) -> ApiResult {
    debug!("Counting WireGuard networks");
    let count = WireguardNetwork::count(&appstate.pool).await?;
    Ok(ApiResponse::json(
        LocationsCount {
            count: count as usize,
        },
        StatusCode::OK,
    ))
}

/// Get a network
#[utoipa::path(
    get,
    path = "/api/v1/network/{network_id}",
    tag = "network",
    params(
        ("network_id" = i64, Path, description = "ID of the network."),
    ),
    responses(
        (status = 200, description = "Network details.", body = WireguardNetworkInfo),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "access denied"})),
        (status = 404, description = "Network not found."),
        (status = 500, description = "Unable to get network.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn network_details(
    Path(network_id): Path<Id>,
    _role: AdminRole,
    State(appstate): State<AppState>,
) -> ApiResult {
    debug!("Displaying network details for network {network_id}");
    let network = WireguardNetwork::find_by_id(&appstate.pool, network_id).await?;

    let response = match network {
        Some(network) => {
            let allowed_groups = network.fetch_allowed_groups(&appstate.pool).await?;
            let gateways = GatewayInfo::find_by_location_id(&appstate.pool, network_id).await?;
            let has_devices =
                WireguardNetworkDevice::has_devices_in_network(&appstate.pool, network_id).await?;
            let posture_checks =
                DevicePostureLocation::find_by_location(&appstate.pool, network_id).await?;
            let network_info = WireguardNetworkInfo {
                network,
                gateways,
                allowed_groups,
                has_devices,
                posture_checks,
            };
            ApiResponse::json(network_info, StatusCode::OK)
        }
        None => ApiResponse::new(Value::Null, StatusCode::NOT_FOUND),
    };
    debug!("Displayed network details for network {network_id}");

    Ok(response)
}

/// Get the state of gateways in a location
#[utoipa::path(
    get,
    path = "/api/v1/network/{network_id}/gateways",
    tag = "gateway",
    params(
        ("network_id" = i64, Path, description = "ID of the network."),
    ),
    responses(
        (status = 200, description = "Gateway status in the location.", body = [GatewayInfo]),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 404, description = "Network not found.", body = ApiErrorResponse, example = json!({"msg": "network not found"})),
        (status = 500, description = "Unable to get gateway status.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn gateway_status(
    Path(network_id): Path<Id>,
    _role: AdminRole,
    State(appstate): State<AppState>,
) -> ApiResult {
    debug!("Displaying gateway status for network {network_id}");

    let gateways = GatewayInfo::find_by_location_id(&appstate.pool, network_id).await?;

    debug!("Displayed gateway status for network {network_id}");

    Ok(ApiResponse::json(gateways, StatusCode::OK))
}

/// Get the state of gateways in all locations
///
/// Each entry carries the ID of the location the gateway belongs to.
#[utoipa::path(
    get,
    path = "/api/v1/network/gateways",
    tag = "gateway",
    responses(
        (status = 200, description = "Gateway status in all locations.", body = [GatewayInfo]),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 500, description = "Unable to get gateway status.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn all_gateways_status(
    _role: AdminRole,
    State(appstate): State<AppState>,
) -> ApiResult {
    debug!("Displaying gateways status for all networks.");

    let gateways = GatewayInfo::list(&appstate.pool).await?;

    Ok(ApiResponse::json(gateways, StatusCode::OK))
}

/// Import a network from a WireGuard configuration file
///
/// Devices found in the configuration are returned unmapped; use
/// `POST /api/v1/network/{network_id}/devices` to assign them to users.
#[utoipa::path(
    post,
    path = "/api/v1/network/import",
    tag = "network",
    request_body = ImportNetworkData,
    responses(
        (status = 201, description = "Network imported.", body = ImportedNetworkData),
        (status = 400, description = "Invalid WireGuard configuration.", body = ApiErrorResponse, example = json!({"msg": "Invalid config file"})),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 413, description = "Configuration file too large.", body = ApiErrorResponse),
        (status = 500, description = "Unable to import network.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn import_network(
    _role: AdminRole,
    State(appstate): State<AppState>,
    context: ApiRequestContext,
    Json(data): Json<ImportNetworkData>,
) -> ApiResult {
    debug!("Importing network from config file");
    let (mut network, imported_devices) =
        parse_wireguard_config(&data.config).map_err(|error| {
            error!("{error}");
            WebError::Http(StatusCode::UNPROCESSABLE_ENTITY)
        })?;
    network.name = data.name;
    network.endpoint = data.endpoint;
    network.allow_all_groups = data.allow_all_groups;

    let mut transaction = appstate.pool.begin().await?;
    let network = network.save(&mut *transaction).await?;
    network
        .set_allowed_groups(&mut transaction, &data.allowed_groups)
        .await?;

    info!("New network {network} created");
    appstate.send_gateway_command(GatewayCommand::NetworkCreated(network.id, network.clone()));

    let reserved_ips = imported_devices
        .iter()
        .flat_map(|dev| dev.wireguard_ips.clone())
        .collect::<Vec<_>>();
    let (devices, gateway_events) =
        handle_imported_devices(&network, &mut transaction, imported_devices).await?;
    appstate.send_multiple_gateway_commands(gateway_events);

    // assign IPs for other existing devices
    debug!("Assigning IPs in imported network for remaining existing devices");
    let gateway_events =
        sync_location_allowed_devices(&network, &mut transaction, Some(&reserved_ips)).await?;
    appstate.send_multiple_gateway_commands(gateway_events);
    debug!("Assigned IPs in imported network for remaining existing devices");

    transaction.commit().await?;

    info!("Imported network {network} with {} devices", devices.len());
    appstate.emit_event(ApiEvent {
        context,
        event: Box::new(ApiEventType::VpnLocationAdded {
            location: network.clone(),
        }),
    })?;
    update_counts(&appstate.pool).await?;

    Ok(ApiResponse::json(
        ImportedNetworkData { network, devices },
        StatusCode::CREATED,
    ))
}

// This is used exclusively for the wizard to map imported devices to users.
/// Assign imported devices to users
///
/// Used to finish the network import started with `POST /api/v1/network/import`.
#[utoipa::path(
    post,
    path = "/api/v1/network/{network_id}/devices",
    tag = "network",
    request_body = MappedDevices,
    params(
        ("network_id" = i64, Path, description = "ID of the network."),
    ),
    responses(
        (status = 201, description = "Devices assigned to users."),
        (status = 204, description = "Empty device list, nothing was assigned."),
        (status = 400, description = "Invalid device data.", body = ApiErrorResponse, example = json!({"msg": "Public key invalid"})),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 404, description = "Network not found.", body = ApiErrorResponse, example = json!({"msg": "network not found"})),
        (status = 500, description = "Unable to assign devices.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn add_user_devices(
    _role: AdminRole,
    session: SessionInfo,
    State(appstate): State<AppState>,
    Path(network_id): Path<Id>,
    Json(request_data): Json<MappedDevices>,
) -> ApiResult {
    let mapped_devices = request_data.devices;
    let user = session.user;
    let device_count = mapped_devices.len();

    debug!(
        "User {} mapping {device_count} devices for network {network_id}",
        user.username,
    );

    // finish early if no devices were provided in request
    if mapped_devices.is_empty() {
        debug!("No devices provided in request, skipping mapping");
        return Ok(ApiResponse::with_status(StatusCode::NO_CONTENT));
    }

    if let Some(network) = WireguardNetwork::find_by_id(&appstate.pool, network_id).await? {
        // wrap loop in transaction to abort if a device is invalid
        let mut transaction = appstate.pool.begin().await?;
        let events = handle_mapped_devices(&network, &mut transaction, &mapped_devices).await?;
        appstate.send_multiple_gateway_commands(events);
        transaction.commit().await?;

        info!(
            "User {} mapped {device_count} devices for {network_id} network",
            user.username,
        );
        update_counts(&appstate.pool).await?;

        Ok(ApiResponse::with_status(StatusCode::CREATED))
    } else {
        error!("Failed to map devices, network {network_id} not found");
        Err(WebError::ObjectNotFound(format!(
            "Network {network_id} not found"
        )))
    }
}

// assign IPs and generate configs for each network
#[derive(Serialize, ToSchema)]
pub(crate) struct AddDeviceResult {
    configs: Vec<DeviceConfig>,
    device: Device<Id>,
}

/// Add a device for a user
///
/// The device is added to every location. `wireguard_pubkey` has to be unique. Devices of
/// disabled users can only be added by an admin.
#[utoipa::path(
    post,
    path = "/api/v1/device/{device_id}",
    tag = "device",
    params(
        ("device_id" = String, description = "Name of the user the device is created for.")
    ),
    request_body(content = AddDevice, example = json!({"name": "work laptop", "wireguard_pubkey": "xTIBA5rboUvnH4htodjb6e697QjLERt1NAB4mZqp8Dg="})),
    responses(
        (status = 201, description = "Device added.", body = AddDeviceResult, example = json!(
            {
                "configs": [
                    {
                        "network_id": 0,
                        "network_name": "network_name",
                        "config": "config",
                        "address": "0.0.0.0:8000",
                        "endpoint": "endpoint",
                        "allowed_ips": ["0.0.0.0:8000"],
                        "pubkey": "pubkey",
                        "dns": "8.8.8.8",
                        "keepalive_interval": 5,
                        "mfa_enabled": false,
                        "service_location_mode": "disabled"
                    }
                ],
                "device": {
                    "id": 1,
                    "name": "work laptop",
                    "wireguard_pubkey": "xTIBA5rboUvnH4htodjb6e697QjLERt1NAB4mZqp8Dg=",
                    "user_id": 1,
                    "created": "2024-07-10T10:25:43.231",
                    "device_type": "user",
                    "description": null,
                    "configured": true
                }
            }
        )),
        (status = 400, description = "No networks are configured, or a device with this public key already exists.", body = ApiErrorResponse, example = json!({"msg": "Failed to add device <name>, identical pubkey (<pubkey>) already exists"})),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges or the request must target your own account.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 404, description = "User not found.", body = ApiErrorResponse, example = json!({"msg": "user <username> not found"})),
        (status = 500, description = "Unable to add device.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn add_device(
    _can_manage_devices: CanManageDevices,
    session: SessionInfo,
    context: ApiRequestContext,
    State(appstate): State<AppState>,
    // Alias, because otherwise `axum` reports conflicting routes.
    Path(username): Path<String>,
    Json(add_device): Json<AddDevice>,
) -> ApiResult {
    let device_name = add_device.name.clone();
    debug!(
        "User {} adding device {device_name} for user {username}",
        session.user.username,
    );

    let user = user_for_admin_or_self(&appstate.pool, &session, &username).await?;

    let settings = EnterpriseSettings::get(&appstate.pool).await?;
    if settings.only_client_activation && !session.is_admin {
        warn!(
            "User {} tried to add a device, but manual device management is disaled",
            session.user.username
        );
        return Err(WebError::Forbidden("Manual device management is disabled"));
    }

    // Disabled users' devices never get network access (see
    // `is_device_allowed_in_network`), and get stripped on the next sync even if
    // briefly assigned.
    if !user.is_active {
        warn!(
            "User {} tried to add a device for a disabled user {username}",
            session.user.username
        );

        return Err(WebError::Forbidden("User is disabled"));
    }

    let networks = WireguardNetwork::all(&appstate.pool).await?;
    if networks.is_empty() {
        error!("Failed to add device {device_name}, no networks found");
        return Ok(ApiResponse::with_status(StatusCode::BAD_REQUEST));
    }

    Device::validate_pubkey(&add_device.wireguard_pubkey).map_err(WebError::PubkeyValidation)?;

    // Make sure there is no device with the same pubkey, such state may lead to unexpected issues
    if Device::find_by_pubkey(&appstate.pool, &add_device.wireguard_pubkey)
        .await?
        .is_some()
    {
        return Err(WebError::PubkeyExists(format!(
            "Failed to add device {device_name}, identical pubkey ({}) already exists",
            add_device.wireguard_pubkey
        )));
    }

    // save the device
    let mut transaction = appstate.pool.begin().await?;
    let device = Device::new(
        add_device.name,
        add_device.wireguard_pubkey,
        user.id,
        DeviceType::User,
        None,
        true,
    )
    .save(&mut *transaction)
    .await?;

    let (network_info, configs) =
        join_device_to_all_networks(&mut transaction, &device, &user).await?;

    // prepare a list of gateway commands to be sent
    let mut events = Vec::new();

    // get all locations affected by device being added
    let mut affected_location_ids = HashSet::new();
    for network_info_item in network_info.clone() {
        affected_location_ids.insert(network_info_item.network_id);
    }

    // send firewall config updates to affected locations
    // if they have ACL enabled & enterprise features are active
    for location_id in affected_location_ids {
        if let Some(location) = WireguardNetwork::find_by_id(&mut *transaction, location_id).await?
            && let Some(firewall_config) =
                try_get_location_firewall_config(&location, &mut transaction).await?
        {
            debug!(
                "Sending firewall config update for location {location} affected by adding new \
                    user {username} devices"
            );
            events.push(GatewayCommand::FirewallConfigChanged(
                location_id,
                firewall_config,
            ));
        }
    }

    // add peer on relevant gateways
    events.push(GatewayCommand::DeviceCreated(DeviceInfo {
        device: device.clone(),
        network_info: network_info.clone(),
    }));

    appstate.send_multiple_gateway_commands(events);

    let template_locations = configs
        .iter()
        .map(|c| TemplateLocation {
            name: c.network_name.clone(),
            assigned_ips: c.address.as_csv(),
        })
        .collect::<Vec<_>>();

    // hide session info if triggered by admin for other user
    let (session_ip, session_device_info) = if session.is_admin && session.user != user {
        (None, None)
    } else {
        (
            Some(session.session.ip_address.as_str()),
            session.session.device_info.clone(),
        )
    };
    new_device_added_mail(
        &user.email,
        &mut transaction,
        &device.name,
        &device.wireguard_pubkey,
        &template_locations,
        session_ip,
        session_device_info.as_deref(),
    )
    .await?;

    transaction.commit().await?;

    info!(
        "User {} added device {device_name} for user {username}",
        session.user.username
    );

    let result = AddDeviceResult {
        configs,
        device: device.clone(),
    };

    update_counts(&appstate.pool).await?;

    appstate.emit_event(ApiEvent {
        context,
        event: Box::new(ApiEventType::UserDeviceAdded {
            device,
            owner: user,
        }),
    })?;

    Ok(ApiResponse::json(result, StatusCode::CREATED))
}

/// Update a device
///
/// `wireguard_pubkey` has to be different from the public key of the location.
#[utoipa::path(
    put,
    path = "/api/v1/device/{device_id}",
    tag = "device",
    params(
        ("device_id" = i64, description = "ID of the device.")
    ),
    request_body = ModifyDevice,
    responses(
        (status = 200, description = "Device updated.", body = Device, example = json!(
            {
                "id": 1,
                "name": "work laptop",
                "wireguard_pubkey": "xTIBA5rboUvnH4htodjb6e697QjLERt1NAB4mZqp8Dg=",
                "user_id": 1,
                "created": "2024-07-10T10:25:43.231",
                "device_type": "user",
                "description": null,
                "configured": true
            }
        )),
        (status = 400, description = "No networks are configured, or the public key belongs to a location.", body = ApiErrorResponse, example = json!({"msg": "device's pubkey must be different from server's pubkey"})),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges or the request must target your own account.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 404, description = "Device not found.", body = ApiErrorResponse, example = json!({"msg": "device id <id> not found"})),
        (status = 500, description = "Unable to update device.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn modify_device(
    _can_manage_devices: CanManageDevices,
    session: SessionInfo,
    context: ApiRequestContext,
    Path(device_id): Path<Id>,
    State(appstate): State<AppState>,
    Json(data): Json<ModifyDevice>,
) -> ApiResult {
    debug!("User {} updating device {device_id}", session.user.username);

    let settings = EnterpriseSettings::get(&appstate.pool).await?;
    if settings.admin_device_management && !session.is_admin {
        warn!(
            "User {} tried to edit a device, but manual device management is disaled",
            session.user.username
        );
        return Err(WebError::Forbidden("Manual device management is disabled"));
    }

    let mut device = device_for_admin_or_self(&appstate.pool, &session, device_id).await?;
    let before = device.clone();

    if settings.only_client_activation
        && !session.is_admin
        && (data.wireguard_pubkey != before.wireguard_pubkey
            || data.description != before.description)
    {
        warn!(
            "User {} tried to modify fields other than device name for device {device_id}, but \
            only client activation is enabled",
            session.user.username
        );
        return Err(WebError::BadRequest(
            "Only the device name can be edited when only client activation is enabled".into(),
        ));
    }

    let networks = WireguardNetwork::all(&appstate.pool).await?;

    if networks.is_empty() {
        error!("Failed to update device {device_id}, no networks found");
        return Ok(ApiResponse::with_status(StatusCode::BAD_REQUEST));
    }

    // check pubkeys
    for network in &networks {
        if network.pubkey == data.wireguard_pubkey {
            error!(
                "Failed to update device {device_id}, device's pubkey must be different from server's pubkey"
            );
            return Ok(ApiResponse::new(
                json!({"msg": "device's pubkey must be different from server's pubkey"}),
                StatusCode::BAD_REQUEST,
            ));
        }
    }

    // update device info
    device.update_from(data);

    // clone to use later

    device.save(&appstate.pool).await?;

    // send update to gateway's
    let mut network_info = Vec::new();
    for network in &networks {
        let wireguard_network_device =
            WireguardNetworkDevice::find(&appstate.pool, device.id, network.id).await?;
        if let Some(wireguard_network_device) = wireguard_network_device {
            let device_network_info = wireguard_network_device
                .to_device_network_info_runtime(&appstate.pool, network)
                .await?;
            network_info.push(device_network_info);
        }
    }
    appstate.send_gateway_command(GatewayCommand::DeviceModified(DeviceInfo {
        device: device.clone(),
        network_info,
    }));

    info!("User {} updated device {device_id}", session.user.username);

    let owner = device.get_owner(&appstate.pool).await?;
    appstate.emit_event(ApiEvent {
        context,
        event: Box::new(ApiEventType::UserDeviceModified {
            owner,
            before,
            after: device.clone(),
        }),
    })?;

    Ok(ApiResponse::json(device, StatusCode::OK))
}

/// Get a device
#[utoipa::path(
    get,
    path = "/api/v1/device/{device_id}",
    tag = "device",
    params(
        ("device_id" = i64, description = "ID of the device.")
    ),
    responses(
        (status = 200, description = "Device details.", body = Device, example = json!(
            {
                "id": 1,
                "name": "work laptop",
                "wireguard_pubkey": "xTIBA5rboUvnH4htodjb6e697QjLERt1NAB4mZqp8Dg=",
                "user_id": 1,
                "created": "2024-07-10T10:25:43.231",
                "device_type": "user",
                "description": null,
                "configured": true
            }
        )),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 404, description = "Device not found.", body = ApiErrorResponse, example = json!({"msg": "device id <id> not found"})),
        (status = 500, description = "Unable to get device.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn get_device(
    session: SessionInfo,
    Path(device_id): Path<Id>,
    State(appstate): State<AppState>,
) -> ApiResult {
    debug!("Retrieving device with id: {device_id}");
    let device = device_for_admin_or_self(&appstate.pool, &session, device_id).await?;
    debug!("Retrieved device with id: {device_id}");
    Ok(ApiResponse::json(device, StatusCode::OK))
}

/// Delete a device
///
/// The device is removed from every location and the gateways are updated.
#[utoipa::path(
    delete,
    path = "/api/v1/device/{device_id}",
    tag = "device",
    params(
        ("device_id" = i64, description = "ID of the device.")
    ),
    responses(
        (status = 200, description = "Device deleted."),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges or the request must target your own account.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 404, description = "Device not found.", body = ApiErrorResponse, example = json!({"msg": "device id <id> not found"})),
        (status = 500, description = "Unable to delete device.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn delete_device(
    _can_manage_devices: CanManageDevices,
    session: SessionInfo,
    context: ApiRequestContext,
    Path(device_id): Path<Id>,
    State(appstate): State<AppState>,
) -> ApiResult {
    // bind username to a variable for easier reference
    let username = &session.user.username;

    debug!("User {username} deleting device {device_id}");
    let mut transaction = appstate.pool.begin().await?;

    let device = device_for_admin_or_self(&mut *transaction, &session, device_id).await?;

    let mut events = Vec::new();

    // prepare device info
    let device_info = DeviceInfo::from_device(&mut *transaction, device.clone()).await?;

    // delete device before firewall config is generated
    device.clone().delete(&mut *transaction).await?;

    update_counts(&mut *transaction).await?;

    // prepare firewall update for affected networks if ACL & enterprise features are enabled
    for info in &device_info.network_info {
        if let Some(location) =
            WireguardNetwork::find_by_id(&mut *transaction, info.network_id).await?
            && let Some(firewall_config) =
                try_get_location_firewall_config(&location, &mut transaction).await?
        {
            debug!(
                "Sending firewall config update for location {location} affected by deleting user {username} device"
            );
            events.push(GatewayCommand::FirewallConfigChanged(
                location.id,
                firewall_config,
            ));
        }
    }

    let device_id = device_info.device.id;
    events.push(GatewayCommand::DeviceDeleted(device_info.clone()));

    // send generated gateway commands
    appstate.send_multiple_gateway_commands(events);

    // Emit event specific to the device type.
    match device.device_type {
        DeviceType::User => {
            let owner = device_info.device.get_owner(&mut *transaction).await?;
            appstate.emit_event(ApiEvent {
                context,
                event: Box::new(ApiEventType::UserDeviceRemoved { device, owner }),
            })?;
        }
        DeviceType::Network => {
            if let Some(network_info) = device_info.network_info.first() {
                let location =
                    WireguardNetwork::find_by_id(&mut *transaction, network_info.network_id)
                        .await?;
                if let Some(location) = location {
                    appstate.emit_event(ApiEvent {
                        context,
                        event: Box::new(ApiEventType::NetworkDeviceRemoved { device, location }),
                    })?;
                } else {
                    error!(
                        "Network device {}({}) is assigned to non-existent location {}",
                        device.name, device.id, network_info.network_id
                    );
                }
            } else {
                error!(
                    "Network device {}({}) has no network assigned",
                    device.name, device.id
                );
            }
        }
    }
    transaction.commit().await?;
    info!("User {username} deleted device {device_id}");

    Ok(ApiResponse::default())
}

/// List devices
#[utoipa::path(
    get,
    path = "/api/v1/device",
    tag = "device",
    responses(
        (status = 200, description = "All devices.", body = [Device], example = json!([
            {
                "id": 1,
                "name": "work laptop",
                "wireguard_pubkey": "xTIBA5rboUvnH4htodjb6e697QjLERt1NAB4mZqp8Dg=",
                "user_id": 1,
                "created": "2024-07-10T10:25:43.231",
                "device_type": "user",
                "description": null,
                "configured": true
            }
        ])),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 500, description = "Unable to list devices.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn list_devices(_role: AdminRole, State(appstate): State<AppState>) -> ApiResult {
    debug!("Listing devices");
    let devices = Device::all(&appstate.pool).await?;
    info!("Listed {} devices", devices.len());

    Ok(ApiResponse::json(devices, StatusCode::OK))
}

/// List the devices of a user
#[utoipa::path(
    get,
    path = "/api/v1/device/user/{username}",
    tag = "device",
    params(
        ("username" = String, description = "Name of the user.")
    ),
    responses(
        (status = 200, description = "All devices of the user.", body = [Device], example = json!([
            {
                "id": 1,
                "name": "work laptop",
                "wireguard_pubkey": "xTIBA5rboUvnH4htodjb6e697QjLERt1NAB4mZqp8Dg=",
                "user_id": 1,
                "created": "2024-07-10T10:25:43.231",
                "device_type": "user",
                "description": null,
                "configured": true
            }
        ])),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges or the request must target your own account.", body = ApiErrorResponse, example = json!({"msg": "Admin access required"})),
        (status = 500, description = "Unable to list user devices.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn list_user_devices(
    session: SessionInfo,
    State(appstate): State<AppState>,
    Path(username): Path<String>,
) -> ApiResult {
    // only allow for admin or user themselves
    if !session.is_admin && session.user.username != username {
        warn!(
            "User {} tried to list devices for user {username}, but is not an admin",
            session.user.username
        );
        return Err(WebError::Forbidden("Admin access required"));
    }
    debug!("Listing devices for user: {username}");
    let devices = Device::all_for_username(&appstate.pool, &username).await?;
    info!("Listed {} devices for user: {username}", devices.len());

    Ok(ApiResponse::json(devices, StatusCode::OK))
}

/// Get the WireGuard configuration of a device in a location
#[utoipa::path(
    get,
    path = "/api/v1/network/{network_id}/device/{device_id}/config",
    tag = "network",
    params(
        ("network_id" = i64, Path, description = "ID of the network."),
        ("device_id" = i64, Path, description = "ID of the device."),
    ),
    responses(
        (status = 200, description = "WireGuard configuration of the device.", body = String, example = json!("[Interface]\nPrivateKey = YOUR_PRIVATE_KEY\nAddress = 10.0.0.12\nDNS = 1.1.1.1\n\n[Peer]\nPublicKey = xTIBA5rboUvnH4htodjb6e697QjLERt1NAB4mZqp8Dg=\nAllowedIPs = 10.0.0.0/24\nEndpoint = vpn.example.com:50051\nPersistentKeepalive = 25")),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges or the request must target your own account.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 404, description = "Network or device not found.", body = ApiErrorResponse, example = json!({"msg": "device not found"})),
        (status = 500, description = "Unable to get device configuration.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn download_config(
    session: SessionInfo,
    State(appstate): State<AppState>,
    Path((network_id, device_id)): Path<(Id, Id)>,
) -> Result<String, WebError> {
    debug!("Creating config for device {device_id} in network {network_id}");

    let settings = EnterpriseSettings::get(&appstate.pool).await?;
    if settings.only_client_activation && !session.is_admin {
        warn!(
            "User {} tried to download device config, but manual device management is disabled",
            session.user.username
        );
        return Err(WebError::Forbidden("Manual device management is disabled"));
    }

    let network = find_network(network_id, &appstate.pool).await?;
    let device = device_for_admin_or_self(&appstate.pool, &session, device_id).await?;
    let user = User::find_by_id(&appstate.pool, device.user_id)
        .await?
        .ok_or(WebError::ObjectNotFound(format!(
            "User {} not found",
            device.user_id
        )))?;
    let wireguard_network_device =
        WireguardNetworkDevice::find(&appstate.pool, device_id, network_id).await?;
    if let Some(wireguard_network_device) = wireguard_network_device {
        info!("Created config for device {}({device_id})", device.name);
        let mut conn = appstate.pool.acquire().await?;
        let device_config =
            build_device_config(&mut conn, &network, &wireguard_network_device, &user).await?;
        Ok(device_config.config)
    } else {
        error!(
            "Failed to create config, no IP address found for device: {}({})",
            device.name, device.id
        );
        Err(WebError::ObjectNotFound(format!(
            "No IP address found for device: {}({})",
            device.name, device.id
        )))
    }
}

/// Get the WireGuard configuration of a user device
///
/// Returns one configuration per location the device is allowed to connect to.
#[utoipa::path(
    get,
    path = "/api/v1/device/{device_id}/config",
    tag = "device",
    params(
        ("device_id" = i64, Path, description = "ID of the device."),
    ),
    responses(
        (status = 200, description = "Device configuration for each location.", body = [Object], example = json!([
            {"network_id": 1, "network_name": "office", "config": "[Interface]\n...", "mfa_enabled": false, "posture_check_required": false}
        ])),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges or the request must target your own account.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 404, description = "Device not found.", body = ApiErrorResponse, example = json!({"msg": "device not found"})),
        (status = 500, description = "Unable to get device configuration.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn user_device_configs(
    session: SessionInfo,
    State(appstate): State<AppState>,
    Path(device_id): Path<Id>,
) -> ApiResult {
    debug!("Creating WireGuard configs for user device {device_id}.");

    let settings = EnterpriseSettings::get(&appstate.pool).await?;
    if settings.only_client_activation && !session.is_admin {
        warn!(
            "User {} tried to download device config, but manual device management is disabled",
            session.user.username
        );
        return Err(WebError::Forbidden("Manual device management is disabled"));
    }

    let device = device_for_admin_or_self(&appstate.pool, &session, device_id).await?;
    let user = User::find_by_id(&appstate.pool, device.user_id)
        .await?
        .ok_or(WebError::ObjectNotFound(format!(
            "User {} not found",
            device.user_id
        )))?;
    let locations = WireguardNetwork::find_user_device_networks(&appstate.pool, device_id).await?;

    let mut result = Vec::new();
    for location in locations {
        let location_device = WireguardNetworkDevice::find(&appstate.pool, device_id, location.id)
            .await?
            .ok_or(WebError::ObjectNotFound(format!(
                "No IP address found for device: {}({})",
                device.name, device.id
            )))?;
        debug!(
            "Created WireGuard config for user device {device_id} in location {}.",
            location.name
        );
        let mut conn = appstate.pool.acquire().await?;
        let device_config =
            build_device_config(&mut conn, &location, &location_device, &user).await?;
        result.push(DeviceWireGuardConfig {
            network_id: device_config.network_id,
            network_name: device_config.network_name,
            config: device_config.config,
            mfa_enabled: device_config.mfa_enabled,
            location_mfa_mode: device_config.location_mfa_mode,
            posture_check_required: device_config.posture_check_required,
        });
    }

    Ok(ApiResponse::json(result, StatusCode::OK))
}
