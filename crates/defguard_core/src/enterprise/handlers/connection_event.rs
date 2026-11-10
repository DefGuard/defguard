use std::net::IpAddr;

use axum::{extract::State, http::StatusCode};
use axum_extra::extract::Query;
use chrono::{DateTime, NaiveDateTime, Utc};
use defguard_common::db::{
    Id,
    models::connection_event::{ConnectionDirection, ConnectionStatus},
};
use sqlx::{Encode, FromRow, Postgres, QueryBuilder, Type, postgres::PgHasArrayType, query_as};

use super::{EnterpriseTierFeature, LicenseGated};
#[cfg(feature = "openapi")]
use crate::handlers::ApiErrorResponse;
use crate::{
    appstate::AppState,
    auth::AdminRole,
    handlers::{
        ApiResponse, ApiResult,
        pagination::{PaginatedApiResponse, PaginatedApiResult, PaginationParams, SortOrder},
    },
};

/// Resolves the VPN device taking part in a connection by matching its VPN addresses in the
/// event's location. The device side is the source of outbound traffic and the destination of
/// inbound traffic; the other side is only used as a fallback.
const DEVICE_LATERAL_JOIN: &str = " LEFT JOIN LATERAL ( \
        SELECT d.id device_id, d.name device_name \
        FROM wireguard_network_device wnd JOIN device d ON d.id = wnd.device_id \
        WHERE wnd.wireguard_network_id = ce.location_id \
        AND (ce.source_ip = ANY(wnd.wireguard_ips) OR ce.destination_ip = ANY(wnd.wireguard_ips)) \
        ORDER BY (ce.source_ip = ANY(wnd.wireguard_ips)) = (ce.direction = 'outbound') DESC \
        LIMIT 1 \
    ) dev ON TRUE ";

/// Joins resolving names of the location, Gateway and ACL rule.
const NAME_JOINS: &str = " JOIN wireguard_network wn ON wn.id = ce.location_id \
    LEFT JOIN gateway g ON g.id = ce.gateway_id \
    LEFT JOIN aclrule r ON r.id = ce.rule_id ";

#[derive(Debug, Deserialize)]
pub struct FilterParams {
    pub from: Option<DateTime<Utc>>,
    pub until: Option<DateTime<Utc>>,
    #[serde(default)]
    pub status: Vec<ConnectionStatus>,
    #[serde(default)]
    pub direction: Vec<ConnectionDirection>,
    #[serde(default)]
    pub protocol: Vec<i32>,
    #[serde(default)]
    pub location: Vec<Id>,
    #[serde(default)]
    pub gateway: Vec<Id>,
    #[serde(default)]
    pub rule: Vec<Id>,
    #[serde(default)]
    pub device: Vec<Id>,
    pub search: Option<String>,
}

impl FilterParams {
    fn search_term(&self) -> Option<&str> {
        self.search
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
    }

    /// Device resolution is costly, so only join it when a filter needs it.
    fn needs_device(&self) -> bool {
        !self.device.is_empty() || self.search_term().is_some()
    }
}

#[derive(Debug, Deserialize)]
pub struct SortParams {
    #[serde(default)]
    pub sort_by: SortKey,
    #[serde(default)]
    pub sort_order: SortOrder,
}

#[derive(Clone, Copy, Debug, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SortKey {
    #[default]
    Timestamp,
    UploadBytes,
    DownloadBytes,
    Count,
}

impl SortKey {
    const fn column(self) -> &'static str {
        match self {
            Self::Timestamp => "timestamp",
            Self::UploadBytes => "upload_bytes",
            Self::DownloadBytes => "download_bytes",
            Self::Count => "count",
        }
    }
}

/// Connection event as returned by the API.
#[derive(Debug, FromRow, Serialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ApiConnectionEvent {
    pub id: Id,
    pub timestamp: NaiveDateTime,
    pub direction: ConnectionDirection,
    pub status: ConnectionStatus,
    /// IANA protocol number (e.g. 6 for TCP, 17 for UDP).
    pub protocol: i32,
    #[cfg_attr(feature = "openapi", schema(value_type = String))]
    pub source_ip: IpAddr,
    pub source_port: Option<i32>,
    #[cfg_attr(feature = "openapi", schema(value_type = String))]
    pub destination_ip: IpAddr,
    pub destination_port: Option<i32>,
    pub location_id: Id,
    pub location_name: String,
    pub gateway_id: Id,
    /// `None` if the Gateway has been deleted.
    pub gateway_name: Option<String>,
    /// `None` if the default policy applied.
    pub rule_id: Option<Id>,
    /// `None` if no rule matched or the rule has been deleted.
    pub rule_name: Option<String>,
    /// VPN device taking part in the connection, if it could be resolved.
    pub device_id: Option<Id>,
    pub device_name: Option<String>,
    /// Bytes sent by the VPN device.
    pub upload_bytes: i64,
    /// Bytes received by the VPN device.
    pub download_bytes: i64,
    /// Number of aggregated packets.
    pub count: i64,
}

#[derive(Debug, FromRow, Serialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct FilterOption {
    pub id: Id,
    pub name: String,
    /// Disambiguates options sharing a name: location of a Gateway, owner of a device.
    pub context: Option<String>,
}

/// Values offered by connection event filters.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ConnectionEventFilterOptions {
    pub locations: Vec<FilterOption>,
    pub gateways: Vec<FilterOption>,
    pub rules: Vec<FilterOption>,
    pub devices: Vec<FilterOption>,
}

/// List connection events
///
/// Traffic flows reported by Gateways. Supports filtering by time range, status, direction,
/// protocol, location, Gateway, ACL rule and device, plus a free-text search over IP addresses
/// and names of the related objects.
/// Requires an Enterprise license.
#[cfg_attr(feature = "openapi", utoipa::path(
    get,
    path = "/api/v1/connection_event",
    tag = "connection events",
    params(
        ("page" = Option<u32>, Query, description = "Page number. Defaults to 1."),
        ("per_page" = Option<u32>, Query, description = "Number of items per page, from 1 to 100. Defaults to 50."),
        ("from" = Option<String>, Query, description = "Start of the reported period as an RFC 3339 timestamp."),
        ("until" = Option<String>, Query, description = "End of the reported period as an RFC 3339 timestamp."),
        ("status" = Option<String>, Query, description = "Filter by status: `allowed` or `blocked`. Repeatable."),
        ("direction" = Option<String>, Query, description = "Filter by direction: `inbound` or `outbound`. Repeatable."),
        ("protocol" = Option<i32>, Query, description = "Filter by IANA protocol number. Repeatable."),
        ("location" = Option<i64>, Query, description = "Filter by location ID. Repeatable."),
        ("gateway" = Option<i64>, Query, description = "Filter by Gateway ID. Repeatable."),
        ("rule" = Option<i64>, Query, description = "Filter by ACL rule ID. Repeatable."),
        ("device" = Option<i64>, Query, description = "Filter by device ID. Repeatable."),
        ("search" = Option<String>, Query, description = "Free-text search across IP addresses, and names of the location, Gateway, ACL rule and device."),
        ("sort_by" = Option<String>, Query, description = "Sort key: `timestamp`, `upload_bytes`, `download_bytes`, or `count`. Defaults to `timestamp`."),
        ("sort_order" = Option<String>, Query, description = "Sort direction: `asc` or `desc`. Defaults to `desc`."),
    ),
    responses(
        (status = 200, description = "Paginated list of connection events.", body = PaginatedApiResponse<ApiConnectionEvent>),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Admin role or Enterprise license required.", body = ApiErrorResponse, example = json!({"msg": "Enterprise features are disabled"})),
        (status = 500, description = "Unable to list connection events.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
))]
pub async fn list_connection_events(
    _admin: AdminRole,
    _license: LicenseGated<EnterpriseTierFeature>,
    State(appstate): State<AppState>,
    Query(pagination): Query<PaginationParams>,
    Query(filters): Query<FilterParams>,
    Query(sorting): Query<SortParams>,
) -> PaginatedApiResult<ApiConnectionEvent> {
    debug!("Fetching connection events with filters {filters:?} and pagination {pagination}");

    // Pick one page of events first, so names and devices are resolved for that page only.
    let mut query_builder = QueryBuilder::<Postgres>::new(
        "SELECT ce.id, ce.timestamp, ce.direction, ce.status, ce.protocol, \
        ce.source_ip, ce.source_port, ce.destination_ip, ce.destination_port, \
        ce.location_id, wn.name location_name, ce.gateway_id, g.name gateway_name, \
        ce.rule_id, r.name rule_name, dev.device_id, dev.device_name, \
        ce.upload_bytes, ce.download_bytes, ce.count \
        FROM (SELECT ce.*",
    );
    push_filtered_source(&mut query_builder, &filters);
    push_sorting(&mut query_builder, &sorting);
    query_builder
        .push(" LIMIT ")
        .push_bind(i64::from(pagination.per_page()))
        .push(" OFFSET ")
        .push_bind(i64::from(pagination.offset()))
        .push(") ce");
    query_builder.push(NAME_JOINS).push(DEVICE_LATERAL_JOIN);
    push_sorting(&mut query_builder, &sorting);

    let mut count_query_builder = QueryBuilder::new("SELECT COUNT(*)");
    push_filtered_source(&mut count_query_builder, &filters);

    let (events, total_items) = tokio::try_join!(
        query_builder
            .build_query_as::<ApiConnectionEvent>()
            .fetch_all(&appstate.pool),
        count_query_builder
            .build_query_scalar::<i64>()
            .fetch_one(&appstate.pool),
    )?;

    Ok(PaginatedApiResponse::new(
        events,
        pagination,
        u32::try_from(total_items).unwrap_or(u32::MAX),
    ))
}

/// Get connection event filter options
///
/// Returns locations, Gateways, ACL rules and devices that connection events can be filtered by.
/// Requires an Enterprise license.
#[cfg_attr(feature = "openapi", utoipa::path(
    get,
    path = "/api/v1/connection_event/filter_options",
    tag = "connection events",
    responses(
        (status = 200, description = "Filter options.", body = ConnectionEventFilterOptions),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Admin role or Enterprise license required.", body = ApiErrorResponse, example = json!({"msg": "Enterprise features are disabled"})),
        (status = 500, description = "Unable to fetch filter options.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
))]
pub async fn get_connection_event_filter_options(
    _admin: AdminRole,
    _license: LicenseGated<EnterpriseTierFeature>,
    State(appstate): State<AppState>,
) -> ApiResult {
    let pool = &appstate.pool;
    let (locations, gateways, rules, devices) = tokio::try_join!(
        query_as::<_, FilterOption>(
            "SELECT id, name, NULL context FROM wireguard_network ORDER BY name",
        )
        .fetch_all(pool),
        query_as::<_, FilterOption>(
            "SELECT g.id, g.name, wn.name context FROM gateway g \
            JOIN wireguard_network wn ON wn.id = g.location_id ORDER BY g.name, wn.name",
        )
        .fetch_all(pool),
        // Pending modifications point at their original rule, which is the one Gateways report.
        query_as::<_, FilterOption>(
            "SELECT id, name, NULL context FROM aclrule WHERE parent_id IS NULL ORDER BY name",
        )
        .fetch_all(pool),
        query_as::<_, FilterOption>(
            "SELECT d.id, d.name, u.username context FROM device d \
            JOIN \"user\" u ON u.id = d.user_id ORDER BY d.name, u.username",
        )
        .fetch_all(pool),
    )?;

    Ok(ApiResponse::json(
        ConnectionEventFilterOptions {
            locations,
            gateways,
            rules,
            devices,
        },
        StatusCode::OK,
    ))
}

/// Pushes `FROM ... WHERE ...` selecting connection events (aliased `ce`) matching `filters`.
fn push_filtered_source(query_builder: &mut QueryBuilder<Postgres>, filters: &FilterParams) {
    let search_term = filters.search_term();

    query_builder.push(" FROM connection_event ce");
    if search_term.is_some() {
        query_builder.push(NAME_JOINS);
    }
    if filters.needs_device() {
        query_builder.push(DEVICE_LATERAL_JOIN);
    }
    query_builder.push(" WHERE TRUE");

    if let Some(from) = filters.from {
        query_builder
            .push(" AND ce.timestamp >= ")
            .push_bind(from.naive_utc());
    }
    if let Some(until) = filters.until {
        query_builder
            .push(" AND ce.timestamp <= ")
            .push_bind(until.naive_utc());
    }
    push_any(query_builder, "ce.status", &filters.status);
    push_any(query_builder, "ce.direction", &filters.direction);
    push_any(query_builder, "ce.protocol", &filters.protocol);
    push_any(query_builder, "ce.location_id", &filters.location);
    push_any(query_builder, "ce.gateway_id", &filters.gateway);
    push_any(query_builder, "ce.rule_id", &filters.rule);
    push_any(query_builder, "dev.device_id", &filters.device);
    if let Some(search_term) = search_term {
        query_builder
            .push(
                " AND CONCAT_WS(' ', host(ce.source_ip), host(ce.destination_ip), wn.name, \
                g.name, r.name, dev.device_name) ILIKE ",
            )
            .push_bind(format!("%{search_term}%"));
    }
}

/// Restricts `column` to `values`, unless `values` is empty.
fn push_any<T>(query_builder: &mut QueryBuilder<Postgres>, column: &str, values: &[T])
where
    T: for<'q> Encode<'q, Postgres> + Type<Postgres> + PgHasArrayType,
{
    if !values.is_empty() {
        query_builder
            .push(" AND ")
            .push(column)
            .push(" = ANY(")
            .push_bind(values)
            .push(")");
    }
}

fn push_sorting(query_builder: &mut QueryBuilder<Postgres>, sorting: &SortParams) {
    let order = sorting.sort_order.as_sql();
    query_builder
        .push(" ORDER BY ce.")
        .push(sorting.sort_by.column())
        .push(" ")
        .push(order)
        .push(", ce.id ")
        .push(order);
}
