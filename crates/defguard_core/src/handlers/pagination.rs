use std::fmt;

use axum::{
    body::Body,
    response::{IntoResponse, Response},
};
use reqwest::StatusCode;
use serde::Serialize;

use crate::error::WebError;

/// Query params for paginated endpoints
#[derive(Deserialize)]
#[serde(default)]
pub(crate) struct PaginationParams {
    page: u32,
    per_page: u32,
}

impl PaginationParams {
    /// Page getter.
    #[must_use]
    pub fn page(&self) -> u32 {
        self.page
    }

    /// Page size getter.
    #[must_use]
    pub fn per_page(&self) -> u32 {
        self.per_page
    }

    /// Calculate offset.
    #[must_use]
    pub fn offset(&self) -> u32 {
        (self.page - 1) * self.per_page
    }
}

impl Default for PaginationParams {
    fn default() -> Self {
        Self {
            page: 1,
            per_page: 50,
        }
    }
}

impl fmt::Display for PaginationParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "page {}", self.page)
    }
}

/// Metadata about the pagination included in response
#[derive(Serialize)]
pub(crate) struct PaginationMeta {
    current_page: u32,
    page_size: u32,
    total_items: u32,
    total_pages: u32,
    next_page: Option<u32>,
}

impl PaginationMeta {
    /// Prepares pagination metadata that's part of the response.
    #[must_use]
    pub(crate) fn from_pagination(pagination: PaginationParams, total_items: u32) -> Self {
        let PaginationParams { page, per_page } = pagination;
        let total_pages = if per_page <= 1 {
            // For 0 and 1, assume per_page is 1.
            total_items
        } else {
            total_items.div_ceil(per_page)
        };
        let next_page = if page < total_pages {
            Some(page + 1)
        } else {
            None
        };

        Self {
            current_page: page,
            page_size: per_page,
            total_items,
            total_pages,
            next_page,
        }
    }
}

pub(crate) type PaginatedApiResult<T> = Result<PaginatedApiResponse<T>, WebError>;

#[derive(Serialize)]
pub(crate) struct PaginatedApiResponse<T> {
    data: Vec<T>,
    pagination: PaginationMeta,
}

impl<T> PaginatedApiResponse<T> {
    #[must_use]
    pub(crate) fn new(data: Vec<T>, pagination: PaginationMeta) -> Self {
        Self { data, pagination }
    }
}

impl<T> IntoResponse for PaginatedApiResponse<T>
where
    T: Serialize,
{
    fn into_response(self) -> Response {
        // Convert the data to JSON
        match serde_json::to_string(&self) {
            Ok(json) => Response::new(Body::from(json)),
            Err(err) => {
                error!("Failed to convert paginated response into JSON: {err}");
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
        }
    }
}
