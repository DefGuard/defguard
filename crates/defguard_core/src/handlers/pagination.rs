use std::fmt;

use axum::{
    body::Body,
    response::{IntoResponse, Response},
};
use reqwest::StatusCode;
use serde::{Deserialize, Deserializer, Serialize, de};

use crate::error::WebError;

const DEFAULT_PER_PAGE: u32 = 50;
const MIN_PAGE: u32 = 1;
const MIN_PER_PAGE: u32 = 1;
const MAX_PER_PAGE: u32 = 100;

/// Query params for paginated endpoints
pub(crate) struct PaginationParams {
    page: u32,
    per_page: u32,
}

/// Implement custom deserializer to control default values and limits.
impl<'de> Deserialize<'de> for PaginationParams {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "snake_case")]
        enum Field {
            Page,
            PerPage,
        }

        struct PaginationParamsVisitor;

        impl<'de> de::Visitor<'de> for PaginationParamsVisitor {
            type Value = PaginationParams;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("struct PaginationParams")
            }

            fn visit_map<V>(self, mut map: V) -> Result<PaginationParams, V::Error>
            where
                V: de::MapAccess<'de>,
            {
                let mut page = None;
                let mut per_page = None;
                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Page => {
                            if page.is_some() {
                                return Err(de::Error::duplicate_field("page"));
                            }
                            page = Some(map.next_value()?);
                        }
                        Field::PerPage => {
                            if per_page.is_some() {
                                return Err(de::Error::duplicate_field("per_page"));
                            }
                            per_page = Some(map.next_value()?);
                        }
                    }
                }
                let page = page.unwrap_or(MIN_PAGE);
                let per_page = per_page.unwrap_or(DEFAULT_PER_PAGE);
                Ok(PaginationParams::new(page, per_page))
            }
        }

        const FIELDS: &[&str] = &["page", "per_page"];
        deserializer.deserialize_struct("PaginationParams", FIELDS, PaginationParamsVisitor)
    }
}

impl PaginationParams {
    /// Constructor.
    #[must_use]
    pub fn new(page: u32, per_page: u32) -> Self {
        Self {
            page: page.max(MIN_PAGE),
            per_page: per_page.max(MIN_PER_PAGE).min(MAX_PER_PAGE),
        }
    }

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
            page: MIN_PAGE,
            per_page: DEFAULT_PER_PAGE,
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
struct PaginationMeta {
    current_page: u32,
    page_size: u32,
    total_items: u32,
    total_pages: u32,
    next_page: Option<u32>,
}

impl PaginationMeta {
    /// Prepares pagination metadata that's part of the response.
    #[must_use]
    fn from_pagination(pagination: PaginationParams, total_items: u32) -> Self {
        let PaginationParams { page, per_page } = pagination;
        let total_pages = total_items.div_ceil(per_page);
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
    pub(crate) fn new(data: Vec<T>, pagination: PaginationParams, total_items: u32) -> Self {
        Self {
            data,
            pagination: PaginationMeta::from_pagination(pagination, total_items),
        }
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
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::PaginationParams;

    #[test]
    fn deserialize_pagination_params_defaults() {
        let params = serde_urlencoded::from_str::<PaginationParams>("").unwrap();
        assert_eq!(params.page(), 1);
        assert_eq!(params.per_page(), 50);
        assert_eq!(params.offset(), 0);
    }

    #[test]
    fn deserialize_pagination_params_zero_values() {
        let params = serde_urlencoded::from_str::<PaginationParams>("page=0&per_page=0").unwrap();
        assert_eq!(params.page(), 1);
        assert_eq!(params.per_page(), 1);
        assert_eq!(params.offset(), 0);
    }

    #[test]
    fn deserialize_pagination_params_large_values() {
        let params =
            serde_urlencoded::from_str::<PaginationParams>("page=1000&per_page=1000").unwrap();
        assert_eq!(params.page(), 1000);
        assert_eq!(params.per_page(), 100);
        assert_eq!(params.offset(), 99900);
    }

    #[test]
    fn deserialize_pagination_params_valid_values() {
        let params = serde_urlencoded::from_str::<PaginationParams>("page=3&per_page=25").unwrap();
        assert_eq!(params.page(), 3);
        assert_eq!(params.per_page(), 25);
        assert_eq!(params.offset(), 50);
    }
}
