use axum::{
    body::Body,
    http::{HeaderName, HeaderValue},
    response::{IntoResponse, Response},
};
use reqwest::StatusCode;
use serde::Serialize;

use crate::{error::WebError, VERSION};

/// Query params for paginated endpoints
#[derive(Debug, Deserialize, Default)]
pub struct PaginationParams {
    #[serde(default = "default_page")]
    pub page: u32,
}

fn default_page() -> u32 {
    1
}

/// Metadata about the pagination included in response
#[derive(Debug, Serialize)]
pub struct PaginationMeta {
    pub current_page: u32,
    pub page_size: u32,
    pub total_items: u32,
    pub total_pages: u32,
    pub next_page: Option<u32>,
}

pub type PaginatedApiResult<T> = Result<PaginatedApiResponse<T>, WebError>;

#[derive(Debug, Serialize)]
pub struct PaginatedApiResponse<T> {
    pub data: Vec<T>,
    pub pagination: PaginationMeta,
}

impl<T> IntoResponse for PaginatedApiResponse<T>
where
    T: Serialize,
{
    fn into_response(self) -> Response {
        // Convert the data to JSON
        let json = match serde_json::to_string(&self) {
            Ok(json) => json,
            Err(err) => {
                error!("Failed to convert paginated response into JSON: {err}");
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
        };

        let mut response = Response::new(Body::from(json));

        response.headers_mut().insert(
            HeaderName::from_static("x-defguard-version"),
            HeaderValue::from_static(VERSION),
        );

        response
    }
}
