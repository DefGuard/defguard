use axum::extract::{Query, State};
use chrono::NaiveDateTime;
use sqlx::{Postgres, QueryBuilder};

use crate::{
    appstate::AppState,
    auth::AdminRole,
    db::{
        models::audit_log::{AuditEvent, AuditModule},
        Id,
    },
};

use super::{
    pagination::{PaginatedApiResponse, PaginatedApiResult, PaginationMeta, PaginationParams},
    API_PAGE_SIZE,
};

#[derive(Debug, Deserialize, Default)]
pub struct FilterParams {
    pub from: Option<NaiveDateTime>,
    pub until: Option<NaiveDateTime>,
    pub search: Option<String>,
    // pub event_type: Option<Vec<>>
    #[serde(default = "default_module")]
    pub module: Vec<AuditModule>,
}

fn default_module() -> Vec<AuditModule> {
    Vec::new()
}

#[derive(Debug, Deserialize, Default)]
pub struct SortParams {
    pub sort_by: Option<String>,
}

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
/// Returns a paginated list of `AuditEvent` objects or `WebError` if error occurs.
pub async fn get_audit_log_events(
    _role: AdminRole,
    State(appstate): State<AppState>,
    pagination: Query<PaginationParams>,
    filters: Query<FilterParams>,
) -> PaginatedApiResult<AuditEvent<Id>> {
    // start with base SELECT query
    // dummy WHERE filter is use to enable composable filtering
    let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
        "SELECT id, timestamp, user_id, ip, event, module, device, details, metadata FROM audit_event WHERE 1=1 ",
    );

    // prepate count query base
    let mut count_query_builder: QueryBuilder<Postgres> =
        QueryBuilder::new("SELECT COUNT(*) FROM audit_event WHERE 1=1 ");

    // add optional filters
    apply_filters(&mut query_builder, &filters);
    apply_filters(&mut count_query_builder, &filters);

    // execute count query
    // fetch total number of filtered events
    let total_items: i64 = count_query_builder
        .build_query_scalar()
        .fetch_one(&appstate.pool)
        .await?;
    let total_pages = ((total_items as u32) + API_PAGE_SIZE - 1) / API_PAGE_SIZE;
    let next_page = if pagination.page < total_pages {
        Some(pagination.page + 1)
    } else {
        None
    };

    // TODO: add ordering

    // add limit and offset to fetch a specific page
    let limit = API_PAGE_SIZE;
    let offset = (pagination.page - 1) * API_PAGE_SIZE;
    query_builder
        .push(" ORDER BY timestamp LIMIT ")
        .push_bind(limit as i64);
    query_builder.push(" OFFSET ").push_bind(offset as i64);

    let events = query_builder
        .build_query_as::<AuditEvent<Id>>()
        .fetch_all(&appstate.pool)
        .await?;

    let pagination = PaginationMeta {
        current_page: pagination.page,
        page_size: API_PAGE_SIZE,
        total_items: total_items as u32,
        total_pages,
        next_page,
    };

    Ok(PaginatedApiResponse {
        data: events,
        pagination,
    })
}

/// Adds optional filtering statements to SQL query based on request query params
fn apply_filters(query_builder: &mut QueryBuilder<Postgres>, filters: &FilterParams) {
    // time filters
    if let Some(from) = filters.from {
        query_builder.push(" AND timestamp >= ").push_bind(from);
    }
    if let Some(until) = filters.until {
        query_builder.push(" AND timestamp <= ").push_bind(until);
    }

    // module filter
    if !filters.module.is_empty() {
        query_builder
            .push(" AND module = ANY(")
            .push_bind(filters.module.clone())
            .push(") ");
    }
}
