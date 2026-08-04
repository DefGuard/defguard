use std::str::FromStr;

use axum::extract::{Path, Query, State};
use chrono::{DateTime, NaiveDateTime, TimeDelta, Utc};
use defguard_common::db::{
    Id,
    models::{
        WireguardNetwork,
        wireguard::{
            DateTimeAggregation, LocationConnectedNetworkDevice, LocationConnectedUserStats,
            WireguardNetworkStats, networks_stats,
        },
    },
};
use reqwest::StatusCode;

use crate::{
    appstate::AppState,
    auth::AdminRole,
    error::WebError,
    handlers::{
        ApiErrorResponse, ApiResponse, ApiResult,
        pagination::{PaginatedApiResponse, PaginatedApiResult, PaginationParams},
    },
};

#[derive(Debug, Deserialize)]
pub(crate) struct QueryFrom {
    from: Option<String>,
}

impl QueryFrom {
    /// If `datetime` is Some, parses the date string, otherwise returns `DateTime` one hour ago.
    fn parse_timestamp(&self) -> Result<DateTime<Utc>, StatusCode> {
        Ok(match &self.from {
            Some(from) => DateTime::<Utc>::from_str(from).map_err(|_| StatusCode::BAD_REQUEST)?,
            None => Utc::now() - TimeDelta::hours(1),
        })
    }
}

/// Returns appropriate aggregation level depending on the `from` date param
/// If `from` is >= than 6 hours ago, returns `Hour` aggregation
/// Otherwise returns `Minute` aggregation
fn get_aggregation(from: NaiveDateTime) -> Result<DateTimeAggregation, StatusCode> {
    // Use hourly aggregation for longer periods
    let aggregation = match Utc::now().naive_utc() - from {
        duration if duration >= TimeDelta::hours(6) => Ok(DateTimeAggregation::Hour),
        duration if duration < TimeDelta::zero() => Err(StatusCode::BAD_REQUEST),
        _ => Ok(DateTimeAggregation::Minute),
    }?;
    Ok(aggregation)
}

/// Get traffic statistics for all locations.
#[utoipa::path(
    get,
    path = "/api/v1/network/stats",
    tag = "location stats",
    params(
        ("from" = Option<String>, Query, description = "Start of the reported period as an RFC 3339 timestamp. Defaults to 1 hour ago."),
    ),
    responses(
        (status = 200, description = "Traffic statistics of all locations.", body = Object),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 500, description = "Unable to get location statistics.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn locations_overview_stats(
    _role: AdminRole,
    State(appstate): State<AppState>,
    Query(query_from): Query<QueryFrom>,
) -> ApiResult {
    debug!("Preparing networks overview stats");
    let from = query_from.parse_timestamp()?.naive_utc();
    let aggregation = get_aggregation(from)?;
    let all_networks_stats = networks_stats(&appstate.pool, &from, &aggregation).await?;
    debug!("Finished processing networks overview stats");
    Ok(ApiResponse::json(all_networks_stats, StatusCode::OK))
}

/// Get traffic statistics for a location.
#[utoipa::path(
    get,
    path = "/api/v1/network/{network_id}/stats",
    tag = "location stats",
    params(
        ("network_id" = i64, Path, description = "ID of the network."),
        ("from" = Option<String>, Query, description = "Start of the reported period as an RFC 3339 timestamp. Defaults to 1 hour ago."),
    ),
    responses(
        (status = 200, description = "Traffic statistics of the location.", body = Object),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 404, description = "Network not found.", body = ApiErrorResponse, example = json!({"msg": "network not found"})),
        (status = 500, description = "Unable to get location statistics.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn location_stats(
    _role: AdminRole,
    State(appstate): State<AppState>,
    Path(network_id): Path<Id>,
    Query(query_from): Query<QueryFrom>,
) -> ApiResult {
    debug!("Displaying WireGuard network stats for location {network_id}");
    let Some(location) = WireguardNetwork::find_by_id(&appstate.pool, network_id).await? else {
        return Err(WebError::ObjectNotFound(format!(
            "Requested location ({network_id}) not found"
        )));
    };
    let from = query_from.parse_timestamp()?.naive_utc();
    let aggregation: DateTimeAggregation = get_aggregation(from)?;
    let stats: WireguardNetworkStats = location
        .network_stats(&appstate.pool, &from, &aggregation)
        .await?;
    debug!("Displayed WireGuard network stats for location {network_id}");

    Ok(ApiResponse::json(stats, StatusCode::OK))
}

/// List connected users in a location.
#[utoipa::path(
    get,
    path = "/api/v1/network/{location_id}/stats/connected_users",
    tag = "location stats",
    params(
        ("location_id" = i64, Path, description = "ID of the location."),
        ("from" = Option<String>, Query, description = "Start of the reported period as an RFC 3339 timestamp. Defaults to 1 hour ago."),
        ("page" = Option<u32>, Query, description = "Page number. Defaults to 1."),
        ("per_page" = Option<u32>, Query, description = "Number of items per page, from 1 to 100. Defaults to 50."),
    ),
    responses(
        (status = 200, description = "Paginated list of connected users.", body = PaginatedApiResponse<LocationConnectedUserStats>),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 404, description = "Network not found.", body = ApiErrorResponse, example = json!({"msg": "network not found"})),
        (status = 500, description = "Unable to get connected users.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn location_connected_users(
    _role: AdminRole,
    State(appstate): State<AppState>,
    Path(location_id): Path<Id>,
    Query(query_from): Query<QueryFrom>,
    pagination: Query<PaginationParams>,
) -> PaginatedApiResult<LocationConnectedUserStats> {
    let pagination = pagination.0;
    debug!(
        "Displaying connected users for location {location_id} with time window {query_from:?} and \
        pagination {pagination}"
    );

    let Some(location) = WireguardNetwork::find_by_id(&appstate.pool, location_id).await? else {
        return Err(WebError::ObjectNotFound(format!(
            "Requested location ({location_id}) not found"
        )));
    };
    let from = query_from.parse_timestamp()?.naive_utc();
    let aggregation = get_aggregation(from)?;

    let (connected_users, total_items) = location
        .connected_users_stats(
            &appstate.pool,
            &from,
            &aggregation,
            pagination.per_page(),
            pagination.offset(),
        )
        .await?;

    Ok(PaginatedApiResponse::new(
        connected_users,
        pagination,
        total_items,
    ))
}

/// List connected network devices in a location.
#[utoipa::path(
    get,
    path = "/api/v1/network/{location_id}/stats/connected_network_devices",
    tag = "location stats",
    params(
        ("location_id" = i64, Path, description = "ID of the location."),
        ("from" = Option<String>, Query, description = "Start of the reported period as an RFC 3339 timestamp. Defaults to 1 hour ago."),
        ("page" = Option<u32>, Query, description = "Page number. Defaults to 1."),
        ("per_page" = Option<u32>, Query, description = "Number of items per page, from 1 to 100. Defaults to 50."),
    ),
    responses(
        (status = 200, description = "Paginated list of connected network devices.", body = PaginatedApiResponse<LocationConnectedNetworkDevice>),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 404, description = "Network not found.", body = ApiErrorResponse, example = json!({"msg": "network not found"})),
        (status = 500, description = "Unable to get connected network devices.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn location_connected_network_devices(
    _role: AdminRole,
    State(appstate): State<AppState>,
    Path(location_id): Path<Id>,
    Query(query_from): Query<QueryFrom>,
    pagination: Query<PaginationParams>,
) -> PaginatedApiResult<LocationConnectedNetworkDevice> {
    let pagination = pagination.0;
    debug!(
        "Displaying connected network devices for location {location_id} with time window \
        {query_from:?} and pagination {pagination}"
    );

    let Some(location) = WireguardNetwork::find_by_id(&appstate.pool, location_id).await? else {
        return Err(WebError::ObjectNotFound(format!(
            "Requested location ({location_id}) not found"
        )));
    };
    let from = query_from.parse_timestamp()?.naive_utc();
    let aggregation = get_aggregation(from)?;

    let (connected_network_devices, total_items) = location
        .connected_network_devices_stats(
            &appstate.pool,
            &from,
            &aggregation,
            pagination.page(),
            pagination.per_page(),
        )
        .await?;

    Ok(PaginatedApiResponse::new(
        connected_network_devices,
        pagination,
        total_items,
    ))
}

#[derive(Deserialize)]
pub(crate) struct ConnectedUserDevicesPath {
    location_id: Id,
    user_id: Id,
}

/// List the connected devices of a user in a location.
#[utoipa::path(
    get,
    path = "/api/v1/network/{location_id}/stats/connected_users/{user_id}/devices",
    tag = "location stats",
    params(
        ("location_id" = i64, Path, description = "ID of the location."),
        ("user_id" = i64, Path, description = "ID of the user."),
        ("from" = Option<String>, Query, description = "Start of the reported period as an RFC 3339 timestamp. Defaults to 1 hour ago."),
    ),
    responses(
        (status = 200, description = "All connected devices of the user.", body = Object),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 404, description = "Network or user not found.", body = ApiErrorResponse, example = json!({"msg": "user not found"})),
        (status = 500, description = "Unable to get connected user devices.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn location_connected_user_devices(
    _role: AdminRole,
    State(appstate): State<AppState>,
    Path(path): Path<ConnectedUserDevicesPath>,
    Query(query_from): Query<QueryFrom>,
) -> ApiResult {
    debug!(
        "Displaying connected devices for user {} at location {} with time window {query_from:?}",
        path.user_id, path.location_id
    );

    let Some(location) = WireguardNetwork::find_by_id(&appstate.pool, path.location_id).await?
    else {
        return Err(WebError::ObjectNotFound(format!(
            "Requested location ({}) not found",
            path.location_id
        )));
    };
    let from = query_from.parse_timestamp()?.naive_utc();
    let aggregation = get_aggregation(from)?;

    let connected_devices = location
        .connected_user_devices_stats(&appstate.pool, path.user_id, &from, &aggregation)
        .await?;

    debug!(
        "Displayed {} connected devices for user {} at location {}",
        connected_devices.len(),
        path.user_id,
        path.location_id
    );

    Ok(ApiResponse::json(connected_devices, StatusCode::OK))
}
