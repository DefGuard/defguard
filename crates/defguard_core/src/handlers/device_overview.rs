use std::collections::HashSet;

use axum::{
    Json,
    extract::{Query, State},
};
use chrono::NaiveDateTime;
use defguard_common::db::{
    Id,
    models::{
        Device, DeviceType, WireguardNetwork,
        device::{DeviceInfo, DeviceState, DeviceVerification},
        device_identity::DeviceAttestationStatus,
        vpn_session_stats::endpoint_without_port,
    },
};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgConnection, Postgres, QueryBuilder};

use super::{
    ApiResponse, ApiResult, WebError,
    pagination::{PaginatedApiResponse, PaginatedApiResult, PaginationParams},
};
use crate::{
    appstate::AppState,
    auth::{AdminRole, SessionInfo},
    enterprise::{firewall::try_get_location_firewall_config, limits::update_counts},
    events::{ApiEvent, ApiEventType, ApiRequestContext},
    grpc::GatewayCommand,
};

#[derive(Debug, Default, Deserialize)]
pub(crate) struct DeviceOverviewFilterParams {
    search: Option<String>,
}

#[derive(Debug, FromRow, Serialize)]
pub(crate) struct DeviceOverviewRow {
    id: Id,
    name: String,
    state: DeviceState,
    verification: DeviceVerification,
    attestation_status: DeviceAttestationStatus,
    /// `None` for network devices.
    user_id: Option<Id>,
    /// Lowercase DB enum label, `None` until the client reports its platform.
    os_type: Option<String>,
    os_version: Option<String>,
    client_version: Option<String>,
    connected: bool,
    last_connected_at: Option<NaiveDateTime>,
    /// Public IP of the latest connected session.
    last_connected_ip: Option<String>,
    /// Location of the latest connected session.
    last_connected_location: Option<String>,
}

/// Internal endpoint backing the devices overview page; intentionally not part of the public
/// OpenAPI spec.
pub(crate) async fn list_devices_overview(
    _role: AdminRole,
    State(appstate): State<AppState>,
    pagination: Query<PaginationParams>,
    filters: Query<DeviceOverviewFilterParams>,
) -> PaginatedApiResult<DeviceOverviewRow> {
    let pagination = pagination.0;
    let filters = filters.0;
    debug!("Listing devices overview with filters {filters:?} and {pagination}");

    // Network devices store the creating admin in `user_id`, which is not an owner.
    let mut query_builder = QueryBuilder::<Postgres>::new(
        "SELECT d.id, d.name, d.state, d.verification, d.attestation_status, \
        CASE WHEN d.device_type = 'user' THEN d.user_id END user_id, \
        ci.os_type::text os_type, ci.os_version, ci.client_version, \
        COALESCE(s.connected, false) connected, s.last_connected_at, \
        ls.endpoint last_connected_ip, ls.location_name last_connected_location \
        FROM device d \
        LEFT JOIN device_client_info ci ON ci.device_id = d.id \
        LEFT JOIN LATERAL ( \
            SELECT bool_or(state = 'connected') connected, MAX(connected_at) last_connected_at \
            FROM vpn_client_session \
            WHERE device_id = d.id \
        ) s ON true \
        LEFT JOIN LATERAL ( \
            SELECT n.name location_name, \
            (SELECT endpoint FROM vpn_session_stats \
                WHERE session_id = vcs.id \
                ORDER BY collected_at DESC, id DESC LIMIT 1) endpoint \
            FROM vpn_client_session vcs \
            JOIN wireguard_network n ON n.id = vcs.location_id \
            WHERE vcs.device_id = d.id AND vcs.connected_at IS NOT NULL \
            ORDER BY vcs.connected_at DESC, vcs.id DESC LIMIT 1 \
        ) ls ON true \
        WHERE 1=1 ",
    );
    apply_filters(&mut query_builder, &filters);
    query_builder
        .push(" ORDER BY d.name, d.id LIMIT ")
        .push_bind(i64::from(pagination.per_page()))
        .push(" OFFSET ")
        .push_bind(i64::from(pagination.offset()));

    let mut devices = query_builder
        .build_query_as::<DeviceOverviewRow>()
        .fetch_all(&appstate.pool)
        .await?;
    // The query yields the raw WireGuard endpoint (`ip:port`).
    for device in &mut devices {
        device.last_connected_ip = device
            .last_connected_ip
            .as_deref()
            .and_then(endpoint_without_port);
    }

    let mut count_query_builder =
        QueryBuilder::<Postgres>::new("SELECT COUNT(*) FROM device d WHERE 1=1 ");
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
            .push(" AND d.name ILIKE ")
            .push_bind(format!("%{search}%"))
            .push(" ");
    }
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

async fn device_modified_event(
    conn: &mut PgConnection,
    before: Device<Id>,
    after: Device<Id>,
) -> Result<Option<ApiEventType>, WebError> {
    match after.device_type {
        DeviceType::User => {
            let owner = after.get_owner(&mut *conn).await?;
            Ok(Some(ApiEventType::UserDeviceModified {
                owner,
                before,
                after,
            }))
        }
        DeviceType::Network => {
            let location = WireguardNetwork::find_network_device_networks(&mut *conn, after.id)
                .await?
                .pop();
            if let Some(location) = location {
                Ok(Some(ApiEventType::NetworkDeviceModified {
                    before,
                    after,
                    location,
                }))
            } else {
                error!(
                    "Network device {}({}) has no network assigned",
                    after.name, after.id
                );
                Ok(None)
            }
        }
    }
}

async fn bulk_modify_devices(
    appstate: &AppState,
    context: &ApiRequestContext,
    ids: Vec<Id>,
    modify: impl Fn(&mut Device<Id>) -> bool,
) -> Result<usize, WebError> {
    let mut transaction = appstate.pool.begin().await?;
    let devices = find_devices(&mut transaction, ids).await?;
    let mut events = Vec::new();
    for mut device in devices {
        let before = device.clone();
        if !modify(&mut device) {
            continue;
        }
        device.save(&mut *transaction).await?;
        if let Some(event) = device_modified_event(&mut transaction, before, device).await? {
            events.push(event);
        }
    }
    transaction.commit().await?;

    let modified = events.len();
    for event in events {
        appstate.emit_event(ApiEvent {
            context: context.clone(),
            event: Box::new(event),
        })?;
    }
    Ok(modified)
}

pub(crate) async fn bulk_verify_devices(
    _role: AdminRole,
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
    let verified = bulk_modify_devices(&appstate, &context, data.devices, |device| {
        if device.verification == DeviceVerification::Verified {
            return false;
        }
        device.verification = DeviceVerification::Verified;
        true
    })
    .await?;
    info!("User {username} manually verified {verified} device(s)");

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
    let blocked = bulk_modify_devices(&appstate, &context, data.devices, |device| {
        if device.state == DeviceState::Blocked {
            return false;
        }
        device.state = DeviceState::Blocked;
        true
    })
    .await?;
    info!("User {username} blocked {blocked} device(s)");

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
    for event in events {
        appstate.emit_event(ApiEvent {
            context: context.clone(),
            event: Box::new(event),
        })?;
    }

    Ok(ApiResponse::default())
}
