use std::collections::HashSet;

use axum::{Json, extract::State};
use axum_extra::extract::Query;
use chrono::NaiveDateTime;
use defguard_common::db::{
    Id,
    models::{
        Device, DeviceType, WireguardNetwork,
        device::{DeviceInfo, DeviceState, DeviceVerificationStatus},
        device_client_info::OsType,
        device_identity::DeviceAttestationStatus,
        device_verification::{DeviceVerification, DeviceVerificationOutcome},
        vpn_session_stats::endpoint_without_port,
    },
};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgConnection, Postgres, QueryBuilder};

use super::{EnterpriseTier, device_verification::broadcast_verification_resolved};
use crate::{
    appstate::AppState,
    auth::{AdminRole, SessionInfo},
    enterprise::{
        firewall::try_get_location_firewall_config, has_enterprise_access, limits::update_counts,
    },
    error::WebError,
    events::{ApiEvent, ApiEventType, ApiRequestContext},
    grpc::GatewayCommand,
    handlers::{
        ApiResponse, ApiResult,
        pagination::{PaginatedApiResponse, PaginatedApiResult, PaginationParams},
        user::SortOrder,
    },
};

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum DevicePlatformFilter {
    Desktop,
    Mobile,
    Unknown,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
enum DeviceSystemFilter {
    Windows,
    Macos,
    Linux,
    Ios,
    Android,
    Unknown,
}

impl DeviceSystemFilter {
    fn os_type(self) -> Option<OsType> {
        match self {
            Self::Windows => Some(OsType::Windows),
            Self::Macos => Some(OsType::Macos),
            Self::Linux => Some(OsType::Linux),
            Self::Ios => Some(OsType::Ios),
            Self::Android => Some(OsType::Android),
            Self::Unknown => None,
        }
    }
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct DeviceOverviewFilterParams {
    search: Option<String>,
    #[serde(default)]
    state: Vec<DeviceState>,
    #[serde(default)]
    verification: Vec<DeviceVerificationStatus>,
    #[serde(default)]
    attestation_status: Vec<DeviceAttestationStatus>,
    #[serde(default)]
    platform: Vec<DevicePlatformFilter>,
    #[serde(default)]
    system: Vec<DeviceSystemFilter>,
    connected: Option<bool>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
enum DeviceOverviewSortKey {
    #[default]
    Name,
    User,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct DeviceOverviewSortParams {
    #[serde(default)]
    sort_by: DeviceOverviewSortKey,
    #[serde(default)]
    sort_order: SortOrder,
}

/// Joins shared by the data and count queries, since filters reference ci, s and u.
const DEVICE_OVERVIEW_FROM: &str = "FROM device d \
    LEFT JOIN device_client_info ci ON ci.device_id = d.id \
    LEFT JOIN LATERAL ( \
        SELECT bool_or(state = 'connected') connected, MAX(connected_at) last_connected_at \
        FROM vpn_client_session \
        WHERE device_id = d.id \
    ) s ON true \
    JOIN \"user\" u ON u.id = d.user_id ";

/// Network devices are managed per location and stay out of the overview.
const DEVICE_OVERVIEW_WHERE: &str = "WHERE d.device_type = 'user' ";

#[derive(Debug, FromRow, Serialize)]
pub(crate) struct DeviceOverviewRow {
    id: Id,
    name: String,
    state: DeviceState,
    /// None below the Enterprise license tier.
    verification: Option<DeviceVerificationStatus>,
    /// None below the Enterprise license tier.
    attestation_status: Option<DeviceAttestationStatus>,
    user_id: Id,
    username: String,
    /// Lowercase DB enum label, None until the client reports its platform.
    os_type: Option<String>,
    os_version: Option<String>,
    client_version: Option<String>,
    connected: bool,
    last_connected_at: Option<NaiveDateTime>,
    /// Public endpoint IP, not the VPN address.
    last_connected_ip: Option<String>,
    last_connected_location: Option<String>,
    /// Whether any assigned location has a config the UI can show: no MFA and no posture checks.
    config_available: bool,
}

/// Internal endpoint for the devices page, intentionally left out of the OpenAPI spec.
pub(crate) async fn list_devices_overview(
    _role: AdminRole,
    State(appstate): State<AppState>,
    pagination: Query<PaginationParams>,
    filters: Query<DeviceOverviewFilterParams>,
    sorting: Query<DeviceOverviewSortParams>,
) -> PaginatedApiResult<DeviceOverviewRow> {
    let pagination = pagination.0;
    let mut filters = filters.0;
    let sorting = sorting.0;
    let enterprise = has_enterprise_access(None);
    if !enterprise {
        filters.verification.clear();
        filters.attestation_status.clear();
    }
    debug!(
        "Listing devices overview with filters {filters:?}, sorting {sorting:?} and {pagination}"
    );

    let mut query_builder = QueryBuilder::<Postgres>::new(
        "SELECT d.id, d.name, d.state, d.verification, d.attestation_status, \
        u.id user_id, u.username, \
        ci.os_type::text os_type, ci.os_version, ci.client_version, \
        COALESCE(s.connected, false) connected, s.last_connected_at, \
        ls.endpoint last_connected_ip, ls.location_name last_connected_location, \
        EXISTS ( \
            SELECT 1 FROM wireguard_network_device wnd \
            JOIN wireguard_network n ON n.id = wnd.wireguard_network_id \
            WHERE wnd.device_id = d.id AND NOT n.mfa_enabled \
            AND n.id NOT IN (SELECT location_id FROM device_posture_location) \
        ) config_available ",
    );
    query_builder.push(DEVICE_OVERVIEW_FROM).push(
        "LEFT JOIN LATERAL ( \
            SELECT n.name location_name, \
            (SELECT endpoint FROM vpn_session_stats \
                WHERE session_id = vcs.id \
                ORDER BY collected_at DESC, id DESC LIMIT 1) endpoint \
            FROM vpn_client_session vcs \
            JOIN wireguard_network n ON n.id = vcs.location_id \
            WHERE vcs.device_id = d.id AND vcs.connected_at IS NOT NULL \
            ORDER BY vcs.connected_at DESC, vcs.id DESC LIMIT 1 \
        ) ls ON true ",
    );
    query_builder.push(DEVICE_OVERVIEW_WHERE);
    apply_filters(&mut query_builder, &filters);
    apply_sorting(&mut query_builder, &sorting);
    query_builder
        .push(" LIMIT ")
        .push_bind(i64::from(pagination.per_page()))
        .push(" OFFSET ")
        .push_bind(i64::from(pagination.offset()));

    let mut devices = query_builder
        .build_query_as::<DeviceOverviewRow>()
        .fetch_all(&appstate.pool)
        .await?;
    for device in &mut devices {
        device.last_connected_ip = device
            .last_connected_ip
            .as_deref()
            .and_then(endpoint_without_port);
        if !enterprise {
            device.verification = None;
            device.attestation_status = None;
        }
    }

    let mut count_query_builder = QueryBuilder::<Postgres>::new("SELECT COUNT(*) ");
    count_query_builder
        .push(DEVICE_OVERVIEW_FROM)
        .push(DEVICE_OVERVIEW_WHERE);
    apply_filters(&mut count_query_builder, &filters);
    let count: i64 = count_query_builder
        .build_query_scalar()
        .fetch_one(&appstate.pool)
        .await?;

    Ok(PaginatedApiResponse::new(devices, pagination, count as u32))
}

fn apply_filters(query_builder: &mut QueryBuilder<Postgres>, filters: &DeviceOverviewFilterParams) {
    if let Some(search) = filters.search.as_deref().filter(|s| !s.is_empty()) {
        query_builder
            .push(" AND CONCAT_WS(' ', d.name, u.username, u.first_name, u.last_name) ILIKE ")
            .push_bind(format!("%{search}%"))
            .push(" ");
    }
    if !filters.state.is_empty() {
        query_builder
            .push(" AND d.state = ANY(")
            .push_bind(filters.state.clone())
            .push(") ");
    }
    if !filters.verification.is_empty() {
        query_builder
            .push(" AND d.verification = ANY(")
            .push_bind(filters.verification.clone())
            .push(") ");
    }
    if !filters.attestation_status.is_empty() {
        query_builder
            .push(" AND d.attestation_status = ANY(")
            .push_bind(filters.attestation_status.clone())
            .push(") ");
    }
    if !filters.platform.is_empty() {
        let conditions: Vec<&str> = filters
            .platform
            .iter()
            .map(|platform| match platform {
                DevicePlatformFilter::Desktop => "ci.os_type NOT IN ('ios', 'android')",
                DevicePlatformFilter::Mobile => "ci.os_type IN ('ios', 'android')",
                DevicePlatformFilter::Unknown => "ci.os_type IS NULL",
            })
            .collect();
        query_builder
            .push(" AND (")
            .push(conditions.join(" OR "))
            .push(") ");
    }
    if !filters.system.is_empty() {
        let os_types: Vec<OsType> = filters
            .system
            .iter()
            .filter_map(|system| system.os_type())
            .collect();
        query_builder
            .push(" AND (ci.os_type = ANY(")
            .push_bind(os_types)
            .push(")");
        if filters.system.contains(&DeviceSystemFilter::Unknown) {
            query_builder.push(" OR ci.os_type IS NULL");
        }
        query_builder.push(") ");
    }
    if let Some(connected) = filters.connected {
        query_builder
            .push(" AND COALESCE(s.connected, false) = ")
            .push_bind(connected)
            .push(" ");
    }
}

fn apply_sorting(query_builder: &mut QueryBuilder<Postgres>, sorting: &DeviceOverviewSortParams) {
    let order = &sorting.sort_order;
    query_builder.push(" ORDER BY ");
    if matches!(sorting.sort_by, DeviceOverviewSortKey::User) {
        query_builder.push(format!("u.first_name {order}, u.last_name {order}, "));
    }
    query_builder.push(format!("d.name {order}, d.id {order}"));
}

#[derive(Debug, Deserialize)]
pub(crate) struct BulkDeviceOperationRequest {
    devices: Vec<Id>,
}

/// Rejects the whole request when any of the given IDs does not exist.
async fn find_devices(
    conn: &mut PgConnection,
    mut ids: Vec<Id>,
) -> Result<Vec<Device<Id>>, WebError> {
    ids.sort_unstable();
    ids.dedup();
    let mut devices = Vec::with_capacity(ids.len());
    for id in ids {
        let device = Device::find_by_id(&mut *conn, id).await?.ok_or_else(|| {
            WebError::BadRequest("Request contained devices that don't exist in db.".into())
        })?;
        devices.push(device);
    }
    Ok(devices)
}

#[derive(Clone, Copy)]
enum BulkDeviceAction {
    Verify,
    RevokeVerification,
    Block,
    Enable,
}

impl BulkDeviceAction {
    fn verification_outcome(self) -> Option<DeviceVerificationOutcome> {
        match self {
            Self::Verify => Some(DeviceVerificationOutcome::Verified),
            Self::Block => Some(DeviceVerificationOutcome::Blocked),
            Self::RevokeVerification | Self::Enable => None,
        }
    }

    /// Returns `false` when the device is already in the target state.
    fn apply(self, device: &mut Device<Id>) -> bool {
        match self {
            Self::Verify => {
                if device.verification == DeviceVerificationStatus::Verified {
                    return false;
                }
                device.verification = DeviceVerificationStatus::Verified;
            }
            Self::RevokeVerification => {
                if device.verification != DeviceVerificationStatus::Verified {
                    return false;
                }
                device.verification = DeviceVerificationStatus::Unverified;
            }
            Self::Block => {
                if device.state == DeviceState::Blocked {
                    return false;
                }
                device.state = DeviceState::Blocked;
            }
            Self::Enable => {
                if device.state != DeviceState::Blocked {
                    return false;
                }
                device.state = DeviceState::Enabled;
            }
        }
        true
    }

    async fn event(
        self,
        conn: &mut PgConnection,
        device: Device<Id>,
    ) -> Result<Option<ApiEventType>, WebError> {
        match device.device_type {
            DeviceType::User => {
                let owner = device.get_owner(&mut *conn).await?;
                Ok(Some(match self {
                    Self::Verify => ApiEventType::UserDeviceVerified { owner, device },
                    Self::RevokeVerification => {
                        ApiEventType::UserDeviceVerificationRevoked { owner, device }
                    }
                    Self::Block => ApiEventType::UserDeviceBlocked { owner, device },
                    Self::Enable => ApiEventType::UserDeviceEnabled { owner, device },
                }))
            }
            DeviceType::Network => {
                let Some(location) =
                    WireguardNetwork::find_network_device_networks(&mut *conn, device.id)
                        .await?
                        .pop()
                else {
                    error!(
                        "Network device {}({}) has no network assigned",
                        device.name, device.id
                    );
                    return Ok(None);
                };
                Ok(Some(match self {
                    Self::Verify => ApiEventType::NetworkDeviceVerified { device, location },
                    Self::RevokeVerification => {
                        ApiEventType::NetworkDeviceVerificationRevoked { device, location }
                    }
                    Self::Block => ApiEventType::NetworkDeviceBlocked { device, location },
                    Self::Enable => ApiEventType::NetworkDeviceEnabled { device, location },
                }))
            }
        }
    }
}

struct BulkModification {
    devices: Vec<Device<Id>>,
    events: Vec<ApiEventType>,
    /// Announced only after the transaction commits, so Edge never hears of a rolled back decision.
    resolved_verifications: Vec<DeviceVerification<Id>>,
}

async fn bulk_modify_devices(
    conn: &mut PgConnection,
    ids: Vec<Id>,
    action: BulkDeviceAction,
) -> Result<BulkModification, WebError> {
    let devices = find_devices(&mut *conn, ids).await?;
    let mut modified = Vec::new();
    let mut events = Vec::new();
    let mut resolved_verifications = Vec::new();
    for mut device in devices {
        if let Some(outcome) = action.verification_outcome()
            && let Some(verification) =
                DeviceVerification::resolve_active_for_device(&mut *conn, device.id, outcome)
                    .await?
        {
            resolved_verifications.push(verification);
        }
        if !action.apply(&mut device) {
            continue;
        }
        device.save(&mut *conn).await?;
        if let Some(event) = action.event(&mut *conn, device.clone()).await? {
            events.push(event);
        }
        modified.push(device);
    }
    Ok(BulkModification {
        devices: modified,
        events,
        resolved_verifications,
    })
}

async fn announce_resolved_verifications(
    appstate: &AppState,
    resolved_verifications: Vec<DeviceVerification<Id>>,
) {
    for verification in resolved_verifications {
        broadcast_verification_resolved(appstate, verification).await;
    }
}

fn emit_events(
    appstate: &AppState,
    context: &ApiRequestContext,
    events: Vec<ApiEventType>,
) -> Result<(), WebError> {
    for event in events {
        appstate.emit_event(ApiEvent {
            context: context.clone(),
            event: Box::new(event),
        })?;
    }
    Ok(())
}

pub(crate) async fn bulk_verify_devices(
    _role: AdminRole,
    _license: EnterpriseTier,
    session: SessionInfo,
    context: ApiRequestContext,
    State(appstate): State<AppState>,
    Json(data): Json<BulkDeviceOperationRequest>,
) -> ApiResult {
    let username = &session.user.username;
    debug!(
        "User {username} manually verifying {} device(s)",
        data.devices.len()
    );
    let mut transaction = appstate.pool.begin().await?;
    let BulkModification {
        devices: verified,
        events,
        resolved_verifications,
    } = bulk_modify_devices(&mut transaction, data.devices, BulkDeviceAction::Verify).await?;
    transaction.commit().await?;
    announce_resolved_verifications(&appstate, resolved_verifications).await;
    info!(
        "User {username} manually verified {} device(s)",
        verified.len()
    );
    emit_events(&appstate, &context, events)?;

    Ok(ApiResponse::default())
}

pub(crate) async fn bulk_revoke_device_verification(
    _role: AdminRole,
    _license: EnterpriseTier,
    session: SessionInfo,
    context: ApiRequestContext,
    State(appstate): State<AppState>,
    Json(data): Json<BulkDeviceOperationRequest>,
) -> ApiResult {
    let username = &session.user.username;
    debug!(
        "User {username} revoking verification of {} device(s)",
        data.devices.len()
    );
    let mut transaction = appstate.pool.begin().await?;
    let BulkModification {
        devices: revoked,
        events,
        ..
    } = bulk_modify_devices(
        &mut transaction,
        data.devices,
        BulkDeviceAction::RevokeVerification,
    )
    .await?;
    transaction.commit().await?;
    info!(
        "User {username} revoked verification of {} device(s)",
        revoked.len()
    );
    emit_events(&appstate, &context, events)?;

    Ok(ApiResponse::default())
}

pub(crate) async fn bulk_block_devices(
    _role: AdminRole,
    session: SessionInfo,
    context: ApiRequestContext,
    State(appstate): State<AppState>,
    Json(data): Json<BulkDeviceOperationRequest>,
) -> ApiResult {
    let username = &session.user.username;
    debug!("User {username} blocking {} device(s)", data.devices.len());
    let mut transaction = appstate.pool.begin().await?;
    let BulkModification {
        devices: blocked,
        events,
        resolved_verifications,
    } = bulk_modify_devices(&mut transaction, data.devices, BulkDeviceAction::Block).await?;
    let blocked_count = blocked.len();
    let mut gateway_commands = Vec::with_capacity(blocked_count);
    for device in blocked {
        let device_info = DeviceInfo::from_device(&mut *transaction, device).await?;
        gateway_commands.push(GatewayCommand::DeviceDeleted(device_info));
    }
    transaction.commit().await?;
    announce_resolved_verifications(&appstate, resolved_verifications).await;

    appstate.send_multiple_gateway_commands(gateway_commands);
    info!("User {username} blocked {blocked_count} device(s)");
    emit_events(&appstate, &context, events)?;

    Ok(ApiResponse::default())
}

pub(crate) async fn bulk_enable_devices(
    _role: AdminRole,
    session: SessionInfo,
    context: ApiRequestContext,
    State(appstate): State<AppState>,
    Json(data): Json<BulkDeviceOperationRequest>,
) -> ApiResult {
    let username = &session.user.username;
    debug!("User {username} enabling {} device(s)", data.devices.len());
    let mut transaction = appstate.pool.begin().await?;
    let BulkModification {
        devices: enabled,
        events,
        ..
    } = bulk_modify_devices(&mut transaction, data.devices, BulkDeviceAction::Enable).await?;
    let enabled_count = enabled.len();
    let mut gateway_commands = Vec::with_capacity(enabled_count);
    for device in enabled {
        let device_info = DeviceInfo::from_device(&mut *transaction, device).await?;
        gateway_commands.push(GatewayCommand::DeviceCreated(device_info));
    }
    transaction.commit().await?;

    appstate.send_multiple_gateway_commands(gateway_commands);
    info!("User {username} enabled {enabled_count} device(s)");
    emit_events(&appstate, &context, events)?;

    Ok(ApiResponse::default())
}

pub(crate) async fn bulk_delete_devices(
    _role: AdminRole,
    session: SessionInfo,
    context: ApiRequestContext,
    State(appstate): State<AppState>,
    Json(data): Json<BulkDeviceOperationRequest>,
) -> ApiResult {
    let username = &session.user.username;
    debug!("User {username} deleting {} device(s)", data.devices.len());

    let mut transaction = appstate.pool.begin().await?;
    let devices = find_devices(&mut transaction, data.devices).await?;
    let deleted = devices.len();

    let mut gateway_commands = Vec::new();
    let mut events = Vec::new();
    let mut affected_location_ids = HashSet::new();
    for device in devices {
        let device_info = DeviceInfo::from_device(&mut *transaction, device.clone()).await?;
        let event = match device.device_type {
            DeviceType::User => {
                let owner = device.get_owner(&mut *transaction).await?;
                Some(ApiEventType::UserDeviceRemoved {
                    owner,
                    device: device.clone(),
                })
            }
            DeviceType::Network => {
                let location =
                    WireguardNetwork::find_network_device_networks(&mut *transaction, device.id)
                        .await?
                        .pop();
                if location.is_none() {
                    error!(
                        "Network device {}({}) has no network assigned",
                        device.name, device.id
                    );
                }
                location.map(|location| ApiEventType::NetworkDeviceRemoved {
                    device: device.clone(),
                    location,
                })
            }
        };
        events.extend(event);
        affected_location_ids.extend(device_info.network_info.iter().map(|info| info.network_id));
        gateway_commands.push(GatewayCommand::DeviceDeleted(device_info));
        device.delete(&mut *transaction).await?;
    }
    update_counts(&mut *transaction).await?;

    // Firewall configs must be generated after all selected devices are deleted.
    for location_id in affected_location_ids {
        if let Some(location) = WireguardNetwork::find_by_id(&mut *transaction, location_id).await?
            && let Some(firewall_config) =
                try_get_location_firewall_config(&location, &mut transaction).await?
        {
            gateway_commands.push(GatewayCommand::FirewallConfigChanged(
                location_id,
                firewall_config,
            ));
        }
    }
    transaction.commit().await?;

    appstate.send_multiple_gateway_commands(gateway_commands);
    info!("User {username} deleted {deleted} device(s)");
    emit_events(&appstate, &context, events)?;

    Ok(ApiResponse::default())
}
