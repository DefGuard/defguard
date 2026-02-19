use std::str::FromStr;

use axum::extract::{Path, Query, State};
use chrono::{DateTime, NaiveDateTime, TimeDelta, Utc};
use defguard_common::db::models::{
    WireguardNetwork,
    wireguard::{
        DateTimeAggregation, LocationConnectedNetworkDevice, LocationConnectedUserStats,
        WireguardNetworkStats, networks_stats,
    },
};
use reqwest::StatusCode;

use crate::{
    appstate::AppState,
    auth::AdminRole,
    error::WebError,
    handlers::{
        ApiResponse, ApiResult, DEFAULT_API_PAGE_SIZE,
        pagination::{PaginatedApiResponse, PaginatedApiResult, PaginationMeta, PaginationParams},
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

/// Returns statistics for all locations
///
/// # Returns
/// Returns an `WireguardNetworkStats` based on stats from all locations in requested time period
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

/// Returns statistics for requested location
///
/// # Returns
/// Returns an `WireguardNetworkStats` based on requested location and time period
pub(crate) async fn location_stats(
    _role: AdminRole,
    State(appstate): State<AppState>,
    Path(network_id): Path<i64>,
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

/// Returns paginated list of connected users for a given location
///
/// # Returns
/// Returns a paginated list of `LocationConnectedUser` objects for requested location and time period
pub(crate) async fn location_connected_users(
    _role: AdminRole,
    State(appstate): State<AppState>,
    Path(location_id): Path<i64>,
    Query(query_from): Query<QueryFrom>,
    pagination: Query<PaginationParams>,
) -> PaginatedApiResult<LocationConnectedUserStats> {
    debug!(
        "Displaying connected users for location {location_id} with time window {query_from:?} and pagination {pagination:?}"
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
            pagination.page,
            DEFAULT_API_PAGE_SIZE,
        )
        .await?;

    let pagination = PaginationMeta::new(pagination.page, total_items, DEFAULT_API_PAGE_SIZE);

    Ok(PaginatedApiResponse {
        data: connected_users,
        pagination,
    })
}

/// Returns paginated list of connected network devices for a given location
///
/// # Returns
/// Returns a paginated list of `LocationConnectedNetworkDevice` objects for requested location and time period
pub(crate) async fn location_connected_network_devices(
    _role: AdminRole,
    State(appstate): State<AppState>,
    Path(location_id): Path<i64>,
    Query(query_from): Query<QueryFrom>,
    pagination: Query<PaginationParams>,
) -> PaginatedApiResult<LocationConnectedNetworkDevice> {
    debug!(
        "Displaying connected network devices for location {location_id} with time window {query_from:?} and pagination {pagination:?}"
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
            pagination.page,
            DEFAULT_API_PAGE_SIZE,
        )
        .await?;

    let pagination = PaginationMeta::new(pagination.page, total_items, DEFAULT_API_PAGE_SIZE);

    Ok(PaginatedApiResponse {
        data: connected_network_devices,
        pagination,
    })
}

#[derive(Deserialize)]
pub(crate) struct ConnectedUserDevicesPath {
    location_id: i64,
    user_id: i64,
}

/// Returns list of connected devices for a specific user at a given location
///
/// # Returns
/// Returns a list of `LocationConnectedUserDevice` objects for requested user, location and time period
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
