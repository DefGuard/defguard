use std::fmt::{self, Display, Formatter};

use axum::extract::State;
use axum_extra::extract::Query;
use chrono::NaiveDateTime;
use ipnetwork::IpNetwork;
use sqlx::{FromRow, Postgres, QueryBuilder, Type};
use tracing::Instrument;

use crate::{
    appstate::AppState,
    auth::AdminRole,
    db::{models::audit_log::AuditModule, Id},
};

use super::{
    pagination::{PaginatedApiResponse, PaginatedApiResult, PaginationMeta, PaginationParams},
    DEFAULT_API_PAGE_SIZE,
};

#[derive(Debug, Deserialize, Default)]
pub struct FilterParams {
    pub from: Option<NaiveDateTime>,
    pub until: Option<NaiveDateTime>,
    #[serde(default = "default_user")]
    pub user: Vec<Id>,
    #[serde(default = "default_event")]
    pub event: Vec<String>,
    #[serde(default = "default_module")]
    pub module: Vec<AuditModule>,
    // pub search: Option<String>,
}

fn default_user() -> Vec<Id> {
    Vec::new()
}

fn default_event() -> Vec<String> {
    Vec::new()
}

fn default_module() -> Vec<AuditModule> {
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
    #[default]
    Asc,
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

/// Audit log event with additional info as returned by the API
#[derive(Serialize, FromRow)]
pub struct ApiAuditEvent {
    pub id: Id,
    pub timestamp: NaiveDateTime,
    pub user_id: Id,
    pub username: String,
    pub ip: IpNetwork,
    pub event: String,
    pub module: AuditModule,
    pub device: String,
    pub metadata: Option<serde_json::Value>,
}

// TODO: add utoipa API schema
/// Filtered list of audit log events
///
/// Retrives a paginated list of audit log events filtered by following query parameters:
/// TODO: add explanations
/// - from
/// - until
/// - module
/// - event_type
/// - username
/// - search
///
/// # Returns
/// Returns a paginated list of `ApiAuditEvent` objects or `WebError` if error occurs.
pub async fn get_audit_log_events(
    _role: AdminRole,
    State(appstate): State<AppState>,
    pagination: Query<PaginationParams>,
    filters: Query<FilterParams>,
    sorting: Query<SortParams>,
) -> PaginatedApiResult<ApiAuditEvent> {
    debug!("Fetching audit log with filters {filters:?} and pagination {pagination:?}");
    // start with base SELECT query
    // dummy WHERE filter is use to enable composable filtering
    let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
        "SELECT ae.id, timestamp, user_id, username, ip, event, module, device, metadata FROM audit_event ae JOIN \"user\" u ON ae.user_id = u.id WHERE 1=1 ",
    );

    // add optional filters
    apply_filters(&mut query_builder, &filters);

    // apply ordering
    apply_sorting(&mut query_builder, &sorting);

    // add limit and offset to fetch a specific page
    let limit = DEFAULT_API_PAGE_SIZE;
    query_builder.push(" LIMIT ").push_bind(limit as i64);
    let offset = (pagination.page - 1) * DEFAULT_API_PAGE_SIZE;
    query_builder.push(" OFFSET ").push_bind(offset as i64);

    // fetch filtered events
    let events = query_builder
        .build_query_as::<ApiAuditEvent>()
        .fetch_all(&appstate.pool)
        .instrument(info_span!("audit_log"))
        .await?;

    // execute count query
    // fetch total number of filtered events
    let mut count_query_builder: QueryBuilder<Postgres> =
        QueryBuilder::new("SELECT COUNT(*) FROM audit_event WHERE 1=1 ");
    apply_filters(&mut count_query_builder, &filters);
    let total_items: i64 = count_query_builder
        .build_query_scalar()
        .fetch_one(&appstate.pool)
        .await?;

    let pagination = get_pagination_metadata(pagination.page, total_items as u32);

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
        query_builder.push(" AND timestamp >= ").push_bind(from);
    }
    if let Some(until) = filters.until {
        query_builder.push(" AND timestamp <= ").push_bind(until);
    }

    // user filter
    if !filters.user.is_empty() {
        query_builder
            .push(" AND user_id = ANY(")
            .push_bind(filters.user.clone())
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

/// Prepares pagination metadata that's part of the response
fn get_pagination_metadata(current_page: u32, total_items: u32) -> PaginationMeta {
    let total_pages = (total_items).div_ceil(DEFAULT_API_PAGE_SIZE);
    let next_page = if current_page < total_pages {
        Some(current_page + 1)
    } else {
        None
    };

    PaginationMeta {
        current_page,
        page_size: DEFAULT_API_PAGE_SIZE,
        total_items,
        total_pages,
        next_page,
    }
}
