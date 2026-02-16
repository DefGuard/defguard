use std::str::FromStr;

use axum::extract::{Path, Query, State};
use chrono::{DateTime, NaiveDateTime, TimeDelta, Utc};
use defguard_common::db::models::{
    DeviceType, WireguardNetwork,
    wireguard::{
        DateTimeAggregation, WireguardDeviceStatsRow, WireguardNetworkStats, WireguardUserStatsRow,
        networks_stats,
    },
};
use reqwest::StatusCode;

use crate::{
    appstate::AppState,
    auth::AdminRole,
    error::WebError,
    handlers::{ApiResponse, ApiResult},
};

#[derive(Deserialize)]
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

#[derive(Serialize)]
pub(crate) struct DevicesStatsResponse {
    user_devices: Vec<WireguardUserStatsRow>,
    network_devices: Vec<WireguardDeviceStatsRow>,
}

/// Returns network statistics for users and their devices
///
/// # Returns
/// Returns an `DevicesStatsResponse` for requested network and time period
pub(crate) async fn devices_stats(
    _role: AdminRole,
    State(appstate): State<AppState>,
    Path(network_id): Path<i64>,
    Query(query_from): Query<QueryFrom>,
) -> ApiResult {
    debug!("Displaying WireGuard user stats for network {network_id}");
    let Some(network) = WireguardNetwork::find_by_id(&appstate.pool, network_id).await? else {
        return Err(WebError::ObjectNotFound(format!(
            "Requested network ({network_id}) not found",
        )));
    };
    let from = query_from.parse_timestamp()?.naive_utc();
    let aggregation = get_aggregation(from)?;
    let user_devices_stats = network
        .user_stats(&appstate.pool, &from, &aggregation)
        .await?;
    let network_devices_stats = network
        .distinct_device_stats(&appstate.pool, &from, &aggregation, DeviceType::Network)
        .await?;
    let response = DevicesStatsResponse {
        user_devices: user_devices_stats,
        network_devices: network_devices_stats,
    };

    debug!("Displayed WireGuard user stats for network {network_id}");

    Ok(ApiResponse::json(response, StatusCode::OK))
}
