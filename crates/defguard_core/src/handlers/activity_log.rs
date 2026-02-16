use std::fmt::{self, Display, Formatter};

use axum::extract::State;
use axum_extra::extract::Query;
use chrono::{DateTime, NaiveDateTime, Utc};
use defguard_common::db::Id;
use ipnetwork::IpNetwork;
use sqlx::{FromRow, Postgres, QueryBuilder, Type};

use super::{
    DEFAULT_API_PAGE_SIZE,
    pagination::{PaginatedApiResponse, PaginatedApiResult, PaginationMeta, PaginationParams},
};
use crate::{appstate::AppState, auth::SessionInfo, db::models::activity_log::ActivityLogModule};

#[derive(Debug, Deserialize, Default)]
pub struct FilterParams {
    pub from: Option<DateTime<Utc>>,
    pub until: Option<DateTime<Utc>>,
    #[serde(default = "default_username")]
    pub username: Vec<String>,
    #[serde(default = "default_location")]
    pub location: Vec<String>,
    #[serde(default = "default_event")]
    pub event: Vec<String>,
    #[serde(default = "default_module")]
    pub module: Vec<ActivityLogModule>,
    pub search: Option<String>,
}

fn default_username() -> Vec<String> {
    Vec::new()
}

fn default_location() -> Vec<String> {
    Vec::new()
}

fn default_event() -> Vec<String> {
    Vec::new()
}

fn default_module() -> Vec<ActivityLogModule> {
    Vec::new()
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub struct SortParams {
    #[serde(default)]
    pub sort_by: SortKey,
    #[serde(default)]
    pub sort_order: SortOrder,
}

#[derive(Debug, Deserialize, Type, Serialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SortKey {
    #[default]
    Timestamp,
    Username,
    Location,
    Ip,
    Event,
    Module,
    Device,
}

impl Display for SortKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Timestamp => write!(f, "timestamp"),
            Self::Username => write!(f, "username"),
            Self::Location => write!(f, "location"),
            Self::Ip => write!(f, "ip"),
            Self::Event => write!(f, "event"),
            Self::Module => write!(f, "module"),
            Self::Device => write!(f, "device"),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Default, Type)]
#[serde(rename_all = "lowercase")]
pub enum SortOrder {
    Asc,
    #[default]
    Desc,
}

impl Display for SortOrder {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Asc => write!(f, "ASC"),
            Self::Desc => write!(f, "DESC"),
        }
    }
}

/// Activity log event with additional info as returned by the API
#[derive(Serialize, FromRow)]
pub struct ApiActivityLogEvent {
    pub id: Id,
    pub timestamp: NaiveDateTime,
    pub user_id: Id,
    pub username: String,
    pub location: Option<String>,
    pub ip: IpNetwork,
    pub event: String,
    pub module: ActivityLogModule,
    pub device: String,
    pub description: Option<String>,
}

// TODO: add utoipa API schema
/// Filtered list of activity log events
///
/// Retrieves a paginated list of activity log events filtered by following query parameters:
/// TODO: add explanations
/// - from
/// - until
/// - module
/// - event_type
/// - username
/// - search
///
/// # Returns
/// Returns a paginated list of `ApiActivityLogEvent` objects or `WebError` if error occurs.
pub async fn get_activity_log_events(
    session_info: SessionInfo,
    State(appstate): State<AppState>,
    pagination: Query<PaginationParams>,
    filters: Query<FilterParams>,
    sorting: Query<SortParams>,
) -> PaginatedApiResult<ApiActivityLogEvent> {
    debug!("Fetching activity log with filters {filters:?} and pagination {pagination:?}");
    // start with base SELECT query
    // dummy WHERE filter is use to enable composable filtering
    let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
        "SELECT id, timestamp, user_id, username, location, ip, event, module, device, description FROM activity_log_event WHERE 1=1 ",
    );

    // filter events for non-admin users to show only their own events
    if !session_info.is_admin {
        query_builder
            .push(" AND username = ")
            .push_bind(session_info.user.username)
            .push(" ");
    }

    // add optional filters
    apply_filters(&mut query_builder, &filters);

    // apply ordering
    apply_sorting(&mut query_builder, &sorting);

    // add limit and offset to fetch a specific page
    let limit = DEFAULT_API_PAGE_SIZE;
    query_builder.push(" LIMIT ").push_bind(i64::from(limit));
    let offset = (pagination.page - 1) * DEFAULT_API_PAGE_SIZE;
    query_builder.push(" OFFSET ").push_bind(i64::from(offset));

    // fetch filtered events
    let events = query_builder
        .build_query_as::<ApiActivityLogEvent>()
        .fetch_all(&appstate.pool)
        .await?;

    // execute count query
    // fetch total number of filtered events
    let mut count_query_builder: QueryBuilder<Postgres> =
        QueryBuilder::new("SELECT COUNT(*) FROM activity_log_event WHERE 1=1 ");
    apply_filters(&mut count_query_builder, &filters);
    let total_items: i64 = count_query_builder
        .build_query_scalar()
        .fetch_one(&appstate.pool)
        .await?;

    let pagination =
        PaginationMeta::new(pagination.page, total_items as u32, DEFAULT_API_PAGE_SIZE);

    Ok(PaginatedApiResponse {
        data: events,
        pagination,
    })
}

/// Adds optional filtering statements to SQL query based on request query params
fn apply_filters(query_builder: &mut QueryBuilder<Postgres>, filters: &FilterParams) {
    debug!("Applying query filters: {filters:?}");

    // time filters
    if let Some(from) = filters.from {
        query_builder
            .push(" AND timestamp >= ")
            .push_bind(from.naive_utc());
    }
    if let Some(until) = filters.until {
        query_builder
            .push(" AND timestamp <= ")
            .push_bind(until.naive_utc());
    }

    // user filter
    if !filters.username.is_empty() {
        query_builder
            .push(" AND username = ANY(")
            .push_bind(filters.username.clone())
            .push(") ");
    }

    // location filter
    if !filters.location.is_empty() {
        query_builder
            .push(" AND location = ANY(")
            .push_bind(filters.location.clone())
            .push(") ");
    }

    // event filter
    if !filters.event.is_empty() {
        query_builder
            .push(" AND event = ANY(")
            .push_bind(filters.event.clone())
            .push(") ");
    }

    // module filter
    if !filters.module.is_empty() {
        query_builder
            .push(" AND module = ANY(")
            .push_bind(filters.module.clone())
            .push(") ");
    }

    // search by provided term
    // following columns are supported:
    // - username
    // - location
    // - module
    // - event
    // - device
    // - description
    if let Some(search_term) = &filters.search {
        query_builder
            .push(" AND CONCAT(username, ' ', location, ' ', module, ' ', event, ' ', device, ' ', description, ' ') ILIKE ")
            .push_bind(format!("%{search_term}%"))
            .push(" ");
    }
}

/// Adds ORDER BY clause to SQL query based on request query params
fn apply_sorting(query_builder: &mut QueryBuilder<Postgres>, sorting: &SortParams) {
    debug!("Applying query sorting: {sorting:?}");

    query_builder
        .push(" ORDER BY ")
        .push(sorting.sort_by.to_string())
        .push(" ")
        .push(sorting.sort_order.to_string());
}
